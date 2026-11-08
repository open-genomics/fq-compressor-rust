# Agent Note: zstd 有界解压不再按不可信上限预分配

Status: implemented

## 问题

`zstd_decompress_bounded(data, max_out, ...)` 原先直接调用
`zstd::bulk::decompress(data, max_out)`。当 zstd 帧头没有 `Content_Size`
时，`bulk` 解码器会按调用方给的 `max_out` 一次性预分配输出缓冲。

`max_out` 在多个解码路径上由 `.fqc` 内容推导，属于不可信输入：

- `src/algo/zstd_sequence.rs:58-66`：
  `uniform_read_length × read_count + read_count × 4 + 64`；
- `src/algo/aux_compressor.rs:72`：`read_count × 5 + 64`；
- reorder map、id、quality 等路径同样使用头部/索引声明值。

`uniform_read_length` 只在 `format.rs` 判断是否大于 0，`reader.rs` 没有
上限校验。构造约 3 KB 的归档即可声明
`uniform_read_length = 0xFFFFFFFF`、`read_count = 100000`，并把 seq 流替换为
不含 `Content_Size` 的 25 字节原始 zstd 块帧：

```text
$ fqc --memory-limit 16 --no-progress decompress -i craft_huge_len.fqc -f
memory allocation of 429496729900064 bytes failed
EXIT=134
```

`429496729900064 = 0xFFFFFFFF × 100000 + 100000 × 4 + 64`，正是
`zstd_sequence` 的 `max_out`。`--memory-limit 16` 完全无法约束这条路径，
进程直接 abort。

## 决定

`zstd_decompress_bounded` 改用 `zstd::stream::read::Decoder` 流式读取：

- 起始容量取 `max_out.min(64 KiB)`，绝不按 `max_out` 预留；
- 每次读 64 KiB，累计输出超过 `max_out` 时返回 `ResourceLimit`；
- 正常读完返回实际输出，内存随真实解压结果增长。

调用方签名与语义不变，所有 9 个调用点无需修改。

## 备选方案

- **校验 `uniform_read_length` 上限。** 只能堵住这一条来源；非均匀读长来自
  aux varint（单条可达 `u32::MAX`），reorder map 与 id/quality 路径也各有
  声明值，仍需统一修复；且合理的长读上限需要格式层定义，容易误伤。
- **保留 `bulk::decompress`，仅把 `max_out` 再 clamp 到 `max_alloc_bytes`。**
  默认 `--memory-limit 0` 时 `max_alloc_bytes` 仍可达数 GB，攻击者照样能让
  进程预分配数 GB；显式小预算下才勉强安全，不彻底。
- **`decoder.take(max_out).read_to_end()`。** 实现更短，但 `take` 在超出时
  静默截断，调用方会把截断结果当成完整输出继续解析，可能掩盖攻击/损坏；
  显式计数才能返回 `ResourceLimit`。
- **先读 zstd 帧头 `Content_Size` 再决定容量。** 帧头可缺失或伪造，不能作为
  信任边界。

## 后果

- 解压峰值内存与“实际解压字节数”成正比，不再与攻击者声明的长度成正比；
- 真正的解压炸弹仍在输出超过 `max_out` 时被 `ResourceLimit` 拦截；
- 每次调用多一个 64 KiB 读缓冲，正常数据多一次 64 KiB 拷贝；
- 损坏帧的错误信息改为流式解码器的错误文本（`Decompression`），
  超限仍是 `ResourceLimit`。

## 验证

- 构造归档 `/tmp/fqc-audit/craft_huge_len.fqc`（3054 字节）修复前
  `EXIT=134` / `memory allocation of 429496729900064 bytes failed`；修复后：

  ```text
  $ fqc --memory-limit 16 --no-progress decompress -i craft_huge_len.fqc -f
  Decompression failed: Format error: Zstd sequence length 0 disagrees with aux length 4294967295 at read 0
  EXIT=3
  ```

- 对照归档 `craft_huge_len_origframe.fqc`、`craft_readcount.fqc` 均返回
  `Format error`（`EXIT=3`），不再 abort。
- 新增 `zstd_bounded_does_not_preallocate_untrusted_ceiling`：以
  `usize::MAX / 2` 为上限解压 1 KiB 数据，修复前会预分配失败 abort，
  修复后返回原始数据。
- `cargo test --lib --tests`：348 passed / 0 failed。
- `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`：
  通过。

## 边界

本修复只约束单次 zstd 解压的输出上限，不改变 `max_out` 本身的推导规则。
`max_out` 仍可能大于真实数据，但内存不再按它预留；若未来需要更严格的
“声明长度 vs 实际长度”校验，应在格式层单独定义读长上限并补测试。
