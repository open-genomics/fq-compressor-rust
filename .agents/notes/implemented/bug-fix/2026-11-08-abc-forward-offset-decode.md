# Agent Note: ABC 解码接受短读段的正向对齐偏移

Status: implemented

## 问题

`decode_contig` 原先要求 `position_offset.unsigned_abs() < read_length`
（`src/algo/abc.rs` 旧 604-609 行），即对齐偏移必须小于读段自身长度。但
编码器 `find_best_alignment` 对正向 shift 只要求 `shift` 小于*对齐当时*的
共识长度，不限制其与读段长度的关系；共识只增不减，`compute_delta` /
`reconstruct_from_delta` 也按 `consensus[shift + i]` 正常处理这类 delta。
因此编码器可以合法产出偏移大于短读段长度的正向对齐，解码器却报
`ABC alignment offset N has no read overlap`。

最小复现（两条 read）：

- r1：58 bp `ACGTTGCAAGTCCTAGGATCCGATGCTAGCTAGGCTAACGTTAGGCATCGATCGTACG`
- r2：r1 在 offset 20 处的 10 bp 子串 `CGATGCTAGC`

`fqc compress --reorder false` exit 0，归档 1 个 block；`fqc decompress`
exit 3（上述 Format error），`fqc verify` exit 1，而 `verify --quick` exit 0
（只比较压缩字节，掩盖了不可解码）。真实 22 bp 短读（miRNA 量级）同样可触发；
默认 `max_shift = 32`，长度不超过 32 bp 的读段最易命中。

## 决定

把重叠校验按偏移方向拆开：

- `read_length == 0`：只允许 `position_offset == 0`（保持原语义）；
- `position_offset >= 0`：要求 `position_offset < consensus.len()`。编码器在
  对齐时已保证 `shift < 当时共识长度`，而共识只增不减，所以最终共识长度
  仍成立；
- `position_offset < 0`：保留 `|position_offset| < read_length`。负向 shift
  在 `find_best_alignment` 中已由 `read_start < read.len()` 保证。

这样接受编码器可产生的全部 delta，同时仍拒绝无重叠的伪造偏移。

## 备选方案

- **删除全部重叠校验。** `reconstruct_from_delta` 对越界位置回退到原始
  碱基，格式上不会越界，但会接受完全无重叠的伪造偏移，削弱格式校验；
  保留方向性下界成本更低。
- **把正向偏移上限改为 `consensus.len() + read_length` 等更宽的值。** 没有
  编码器契约依据，只会放宽解析器而不增加正确性。
- **在压缩端限制 `max_shift <= 读段长度`。** 会降低长读对短读的锚定能力，
  且无法修复已有归档的解压；解码器仍应接受格式允许的偏移。

## 后果

- 由短读段 + 较大正向偏移构成的归档可以正常解压和完整 verify；
- 无损格式与既有归档布局不变，仅放宽了解码校验；
- 伪造的越界偏移（正向不小于共识长度、负向不小于读段长度）仍被拒绝；
- `read_length == 0` 的语义保持不变。

## 验证

- 新增单元测试 `test_abc_short_read_forward_offset_roundtrip`
  （`src/algo/abc.rs`）：58 bp + 10 bp 子串直接经 `AbcCompressor`
  压缩/解压，逐条断言一致。
- 新增端到端测试 `repro_abc_short_read_forward_offset_archive_roundtrip`
  （`tests/test_audit_regressions.rs`）：经 `CompressCommand` /
  `DecompressCommand` / `VerifyCommand`，断言压缩、解压、完整 verify 均
  exit 0 且输出与输入逐字节一致。
- 修复前同一输入经 CLI 复现：compress exit 0、decompress exit 3、
  verify exit 1；修复后三者均为 0。
- `cargo test --lib --tests`、`cargo fmt --all -- --check`、
  `cargo clippy --all-targets -- -D warnings` 全部通过。

## 边界

本修复只调整解码端的偏移校验；编码器的 `max_shift` 配置与哈希阈值不变。
