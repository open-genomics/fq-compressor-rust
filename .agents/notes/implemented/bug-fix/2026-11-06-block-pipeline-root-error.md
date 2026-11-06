# Agent Note: 块流水线优先报告根本错误

Status: implemented

## 问题

共享块流水线有三个并发阶段：reader、compressor、writer。任一阶段退出都会关闭
自己持有的 channel，使其它阶段看到 `channel closed`。旧实现按
`reader_result?; compressor_result?; writer_result?` 的顺序返回错误：

- writer 在 `begin_fqc_writer` 就失败（例如输出目录不存在）时，reader 和
  compressor 会先观察到 channel 关闭；
- compressor 在 `BlockCompressor::compress` 失败时，writer 会先观察到块数不足并
  返回 `Compression incomplete`；
- 用户最终看到的可能是 “Reader: channel closed” 或 “Compression incomplete”，
  而不是输出路径不可写、磁盘写入失败等真正的根因。

这会误导排障，也让不同线程调度下的错误信息不稳定。

## 决定

在 `src/pipeline/block_pipeline.rs` 中把阶段结果区分为三类：

- `Root(FqcError)`：阶段自身工作失败，例如 writer 的 `begin`/`write_block`/
  `finalize`/`commit`，或 compressor 的 `compress`；
- `Derived(String)`：仅表示对端停止，例如 `channel closed` 或
  `Compression incomplete`；
- `Panicked(&'static str)`：阶段线程 panic。

所有阶段 join 后收集失败，按 `Root < Panicked < Derived` 的优先级稳定排序；同等级
保持 writer、compressor、reader 的插入顺序，因此错误选择是确定性的。只有没有任何
根本错误时才返回派生症状。

不使用共享 `Mutex<Option<FqcError>>` 记录“第一个错误”：`FqcError::Io` 包装的
`std::io::Error` 不可 `Clone`，而按字符串降级会丢失错误分类和退出码。阶段结果枚举
保留原始 `FqcError`，无需克隆，也无需给公共错误类型增加约束。

## 备选方案

- **共享失败槽 + 首错优先**：需要克隆或序列化 `FqcError`；首错还可能是 channel
  closed 症状，反而掩盖根因。
- **固定 writer 优先**：writer 的 `Compression incomplete` 本身可能是 compressor
  失败的症状，会遮住真正的压缩错误。
- **只保留 writer 错误、丢弃其它阶段错误**：worker panic 或压缩失败时缺少可读
  原因，否决。
- **引入结构化错误分类体系**：当前 `FqcError` 仍以字符串为主，全面改造超出本次
  修复范围；先用阶段来源区分根因与症状。

## 后果

- 输出路径不可写、目录不存在、写入/commit 失败等 writer 根因会原样返回；
- compressor 的编码错误优先于 writer 的“块数不足”症状；
- 线程 panic 优先于 channel 症状，但仍低于明确的阶段根因；
- `.fqc` 格式、块顺序、事务提交语义不变；
- 错误信息只在失败路径变化，正常压缩行为不受影响。

## 验证

- 新增 `tests/test_pipeline_concurrency.rs` 中的
  `writer_root_error_is_not_masked_by_closed_channel_symptoms`：构造不存在的输出父目录，
  20 个块、`max_in_flight_blocks = 1`，强制 writer 在开始阶段失败；断言返回
  `FqcError::Io`，而不是 reader/compressor 的 channel closed。
- `cargo test --test test_pipeline_concurrency --locked`：6 个测试通过。
- `cargo clippy --all-targets --locked -- -D warnings` 通过。
