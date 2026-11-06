# Agent Note: streaming 模式补齐阶段计时

Status: implemented

## 问题

`OperationStats` 的 `parse_ms` / `process_ms` / `write_ms` 对 archive/pipeline
有实际值，但 streaming 的 `run_core` 一直写 0：

- CLI summary 的 `Stage timings` 在 streaming 下永远是 0；
- API 调用方无法判断时间花在 parser、codec 还是 writer；
- CHANGELOG 已声称阶段计时覆盖 streaming，与实现不一致。

streaming 是串行循环，三个阶段的边界清楚，不需要额外的线程或协议才能计时。

## 决定

在 `streaming_runner::run_core` 的串行循环中分别累计：

- `next_block()` 调用时间 → `parse_ns`（包含 parser 读取与块组装）；
- `BlockCompressor::compress` → `process_ns`；
- `begin_fqc_writer` / global header / `write_block_with_id` /
  `patch_total_read_count` / `finalize` / `commit` → `write_ns`；
- `reorder_ms` 保持 0（streaming 不做重排）；
- `elapsed_seconds` 仍由 `CompressionEngine::run` 在返回前统一填充。

`write_ns` 从 writer 创建开始计时，保证 header 和事务提交也计入串行 I/O，而不是
只算 block payload。

## 备选方案

- **保持 0，只在文档中说明**：CLI 的 summary 字段会继续误导用户；
- **把整个循环时间都算进 `process_ms`**：会把 parser 和磁盘 I/O 混进 codec 时间；
- **为 streaming 引入 reader/compressor/writer 三线程**：能并行，但属于 bounded
  pipeline 的独立设计，不应为了计时引入并发；
- **只测 compress**：仍无法解释 streaming 的端到端时间。

## 后果

- streaming 的 `parse_ms` / `process_ms` / `write_ms` 反映实际串行阶段；
- 三者之和通常小于 `elapsed_seconds`，差额是循环/统计开销，属预期；
- 不改变块边界、码流、归档格式或 CLI 参数；
- 为以后实现并行 bounded-ingest 提供同一套统计口径。

## 验证

- 新增 `tests/unit_stats_timing.rs::streaming_stats_carry_stage_timings`：
  1,000 条读段、`streaming_mode = true`，断言
  `parse_ms + process_ms + write_ms > 0` 且 mode 为 Streaming；
- 既有 `archive_stats_carry_stage_timings` 继续通过；
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --locked -- -D warnings`
- `cargo test --lib --tests --locked`
- `cargo doc --no-deps --locked`
- `cargo +1.75.0 check --all-targets --locked`
- `npm run verify-notes`
