# ID 流解码上限

## Problem

`decompress_exact` / `decompress_tokenize` / `decompress_legacy` 允许每条 ID
平均 64 KiB 的声明输出，且没有绝对上限。`read_count` 来自归档头部，伪造一个
大 `read_count` 的小文件即可让 `zstd_decompress_bounded` 接受数百 GiB 的
`max_out`，绕过 `DecodeBudget`。

## Decision

- 单条 ID（含 comment）平均上限降为 4 KiB；正常 header 远小于该值。
- 增加 512 MiB 的绝对流上限，覆盖“read 数极大且 ID 合法偏长”的归档。
- 三条路径共用同一对常量，避免再次漂移。

## Consequences

超过 4 KiB 平均长度的 ID 流会被拒绝；这是对异常 header 的防御，不是正常数据
边界。512 MiB 绝对上限对现有测试与常见归档无影响。

## Verification

- 新增 `test_id_declared_size_is_capped_before_decompression`：伪造 600 MiB
  声明，断言 `ResourceLimit`。
- `cargo test --lib --tests`、`cargo clippy --all-targets -- -D warnings` 通过。
