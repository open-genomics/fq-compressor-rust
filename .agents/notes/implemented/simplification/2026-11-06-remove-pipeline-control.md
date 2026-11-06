# Agent Note: 移除未实现的 PipelineControl

Status: implemented

## 问题

`pipeline::PipelineControl` 暴露了取消和进度两个原子状态：

- `is_cancelled()` 被 reader / decompressor / writer 三个循环检查；
- `add_reads()` 只在 decompressor 中累加。

但代码库没有任何地方调用 cancel，也没有读取 `reads_processed`。因此
`is_cancelled()` 永远为 false，所有检查都是死分支；`add_reads()` 的原子累加
也没有消费者。这让解压 pipeline 看起来支持取消/进度，实际上没有对应行为。

## 决定

删除 `PipelineControl` 及解压 pipeline 中的全部检查和参数传递。reader、
decompressor、writer 的退出条件只由 channel 关闭、线程错误和事务完整性决定。
模块文档不再宣称提供取消/进度支持。

## 备选方案

- **补一个真正的取消入口**：需要 CLI 信号处理、跨阶段取消语义和测试；
  当前没有用户请求，先不引入新的并发契约。
- **保留类型但标记 `#[doc(hidden)]`**：仍然保留死分支和原子开销，也会继续误导
  维护者；否决。
- **只删除 `reads_processed`，保留 cancellation**：会留下一个没有触发点的
  `is_cancelled()`，仍然是投机抽象。

## 后果

- 删除约 60 行未使用状态与检查，reader/decompressor/writer 热路径少一次原子读。
- 解压错误路径更直接：channel 关闭或 worker 错误会让写入阶段拒绝提交半成品。
- 不改变 CLI、`.fqc` 格式、解压结果或统计字段。
- 如果将来真的要支持取消，应作为独立设计实现信号源、跨阶段传播和回归测试，
  而不是复活这个无触发点的结构。

## 验证

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --locked -- -D warnings`
- `cargo test --lib --tests --locked`
- `cargo doc --no-deps --locked`
- `cargo +1.75.0 check --all-targets --locked`
- `npm run verify-notes`
