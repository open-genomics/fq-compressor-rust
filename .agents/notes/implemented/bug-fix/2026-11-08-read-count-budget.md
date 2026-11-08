# Agent Note: 单块解码字节预算（ReadRecord 骨架 + 读段长度）

Status: implemented

## Problem

`BlockCompressor::decompress_raw` 的第一条语句就按文件头 `read_count` 分配：

```rust
let mut result = DecodedBlock {
    reads: vec![ReadRecord::default(); read_count as usize],
};
```

`read_count` 来自 `BlockHeader.uncompressed_count`，在 reader 打开阶段只做了
结构校验（与块索引一致、总和不超 `global_header.total_read_count`）。
`DecodeBudget::max_total_reads` 按 `limit_bytes / 32` 估算，而
`size_of::<ReadRecord>()` 是 96 B（4 × `String`）。`--memory-limit 16`
允许 524,288 reads，骨架可达 48 MB，超过 `max_alloc_bytes = 8 MiB`。
审计构造的 500k reads 归档实测 RSS 比合法 4 reads 归档高约 35 MB。

随后 `zstd_sequence.rs` 的 `Vec::with_capacity(read_count)` 等分配同样在
任何逐块预算检查之前发生。

## Decision

`BlockCompressorConfig` 新增：

```rust
pub max_block_decoded_bytes: u64, // 默认 u64::MAX（压缩路径不限制）
```

`decompress_raw` 做两级检查（都使用同一单块预算）：

```rust
let skeleton_bytes = u64::from(read_count)
    .saturating_mul(std::mem::size_of::<ReadRecord>() as u64);
if skeleton_bytes > self.config.max_block_decoded_bytes {
    return Err(FqcError::ResourceLimit { .. });
}
// ... 解出 lengths 并校验数量后：
let total_bases: u64 = lengths.iter().map(|&l| u64::from(l)).fold(0, u64::saturating_add);
let decoded_bytes = skeleton_bytes.saturating_add(total_bases.saturating_mul(2));
if decoded_bytes > self.config.max_block_decoded_bytes {
    return Err(FqcError::ResourceLimit { .. });
}
```

解压 pipeline 与 verify 在构造 `BlockCompressorConfig` 时填
`reader.budget().max_alloc_bytes`；压缩路径保留默认值，不受影响。

第二个检查覆盖长度维度：`uniform_read_length` 或 aux 流解出的每个长度都是
文件可控值，序列与质量解码各分配约 `len` 字节，旧路径在
`uniform_read_length=0xFFFFFFFF` 时会尝试 4 GiB 级分配/循环。总和检查在
任何解码器之前拒绝，同时兜住 `Vec::with_capacity(read_count)`、
`vec![0u8; len]`、`String::with_capacity(len)` 等下游分配。

## Alternatives considered

- **把 `DecodeBudget::max_total_reads` 的除数从 32 改成 96。** 会收紧所有
  归档的全局读段上限（包括可以流式处理的大归档），而且仍无法表达
  `max_alloc_bytes`；按块检查更精确。
- **只检查骨架、不检查总长度。** 骨架检查挡住 `read_count`，但
  `uniform_read_length=0xFFFFFFFF` 仍会让单条 read 的 `vec![0u8; len]`
  尝试 4 GiB 分配；必须同时约束长度总和。
- **在 pipeline 中按块索引的 `read_count` 提前拒绝。** 需要把预算传进
  每个解码入口（pipeline、verify、测试辅助），且 `decompress_raw` 被直接
  调用时仍无防护；配置字段 + 单点检查覆盖全部入口。
- **改用 `Vec::try_reserve` 并把 OOM 转成错误。** 无法区分合法大分配与
  超预算分配，且 48 MB 分配在多数机器上会成功，起不到预算约束作用。
- **把 `max_alloc_bytes` 再细分出块级份额。** 当前字段语义就是“单次分配
  上限”，直接复用它，避免新增预算维度。

## Consequences

Benefits:

- 伪造/损坏归档无法再用 `read_count` 驱动超出 `--memory-limit` 的分配；
  错误是 `ResourceLimit`（可诊断），不是 OOM/abort。
- 合法但单块过大的归档在预算不足时提前失败，而不是先分配再失败。

Costs and limits:

- 这是行为收紧：`--memory-limit 16` 下，单块超过约 87k reads 的归档现在
  会在解码前失败。此前它们可能“成功”，但 RSS 已超预算。
- 预算按“骨架 + 2 × 总碱基”估算，质量丢弃模式与实际 ID/注释开销未单独
  计入，属于保守近似；目标是把文件可控声明值限制在预算量级，不是精确 RSS
  预测。
- 压缩路径默认不限制，`BlockCompressorConfig` 的 `Debug` 输出会多一个字段。

## Verification

- `cargo test --test test_decode_budget`：14 passed，其中新增
  `forged_read_count_rejected_before_record_skeleton_alloc`、
  `forged_uniform_length_rejected_before_decoders` 与
  `decompress_rejects_block_whose_record_skeleton_exceeds_budget`
  （90k reads × 96 B = 8.24 MB > 16 MiB 预算的 8 MiB 上限，端到端返回非 0）。
- `cargo test --lib --tests`：**351 passed / 0 failed / 0 ignored**（33 个
  suite）。
- `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`：
  通过。
- 未复测审计中的 500k reads 构造归档 RSS；端到端测试以 90k reads 越界为
  代表，检查发生在分配前，因此 RSS 不应再出现 48 MB 骨架。
