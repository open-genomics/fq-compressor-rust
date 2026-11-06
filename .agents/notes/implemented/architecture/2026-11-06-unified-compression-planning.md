# Agent Note: 三种压缩模式共用同一块/worker 规划器

Status: implemented

## 问题

块大小与 codec worker 数仍有三套入口：

- archive 通过 `plan_compression_layout(CompressionPlanInput { ... })`；
- pipeline 在自己的 `CompressionPipelineConfig::block_plan` 里再拼一份
  `CompressionPlanInput`；
- streaming 先用 `streaming_support::effective_block_size` 调
  `adaptive_block_size_with_policy`，再用 `memory_limited_worker_count` 单独
  算 worker。

streaming 的 `effective_block_size` 只是对 `adaptive_block_size_with_policy`
的转发，worker 估算又与 `plan_compression_layout` 内部逻辑重复。任何“内存受限
时如何放大块/减少 worker”的修正都可能在 streaming 漏掉。

## 决定

让 `plan_compression_layout` 成为三种模式唯一的块/worker 规划入口：

- `CompressionPlanInput` 新增 `auto_split_long_reads: bool`：
  - archive/pipeline 传 `true`，保留 Long 类别按 5 Mbp 自动切块的既有策略；
  - streaming 传 `false`，保留其历史块大小（显式 `--max-block-bases` 仍生效）；
- `plan_compression_layout` 内部改用
  `adaptive_block_size_with_policy(..., auto_split_long_reads)`；
- `memory_limited_worker_count` 收回为私有实现细节，不再从 `plan` 再导出；
- 删除 `streaming_support::effective_block_size`；streaming 直接调用
  `plan_compression_layout`，传 `total_reads = None`、`total_bases = None`，
  以采样平均长度作为回退。

`total_reads = None` 时规划器的 `by_count` 取类别推荐块大小，不依赖线程数；
因此 streaming 的块大小与旧实现逐值一致。worker 数仍由
`worker_budget_bytes / estimated_worker_bytes` 限制，并受 `thread_limit` 约束。

## 备选方案

- **保留 streaming 的独立规划**：块大小虽暂时一致，但 worker/内存策略会继续
  分叉；否决。
- **给 streaming 复制一份 `plan_compression_layout` 变体**：同样会产生两套
  迭代关系；用 `auto_split_long_reads` 一个字段表达差异更小。
- **把 streaming 的块大小也改成内存自适应**：会改变归档块划分和压缩率，需要
  单独评估与真实语料回归；本轮只统一规划入口，不改变策略。

## 后果

- 块大小、worker 数、内存受限时的块放大逻辑只维护一处；
- archive/pipeline 的 `auto_split_long_reads = true` 与旧行为一致；
- streaming 的块大小、worker 数、归档体保持不变（真实语料验证）；
- `plan::memory_limited_worker_count` 不再是公开内部 API，减少一个可误用入口。

## 验证

- `tests/test_types.rs::compression_plan_sizes_blocks_from_memory_limited_workers`
  继续固定内存受限时的 worker/块大小关系；
- 真实语料 streaming 归档大小与旧实现完全相同：
  641,148 / 1,612,235 / 7,413,877 / 12,806,271 B，块数 1/1/2/4；
- 日志：mito SE streaming 仍为 `block_size=100000, workers=4, in_flight=1`；
- 全量 fmt / clippy / test / doc / MSRV / verify-notes 通过。
