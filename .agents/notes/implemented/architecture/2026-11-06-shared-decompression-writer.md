# Agent Note: 解压输出层共享

Status: implemented

## 问题

解压的 sequential（`decompression_classic`）与 threaded pipeline
（`decompression`）各自维护输出层：

- 两套单文件/stdout 创建逻辑；
- 两套 FASTQ / header-only 写入与字节计数；
- 两套 corrupted placeholder 生成；
- split R1/R2 路径推导与 `streaming_block_layout` 判定只在 classic 中；
- 两套事务 flush / drop / commit 顺序。

这些代码逐行几乎相同，但任何格式或提交语义修正都可能只改到一条路径。
两条解压路径本身仍有不同职责（original-order、split-PE、range、并行调度），
所以本轮先收敛输出层，不动调度。

## 决定

新增 `src/pipeline/decompression_writer.rs`：

- `OutputWriters` 统一管理：
  - `Single`（文件或 stdout）；
  - `Split`（R1/R2、`PeLayout`、streaming 连续布局判定）；
  - `write_record`、`flush`、`commit`。
- `buffered` 供 sequential 路径创建 BufWriter 目标；
- `single_raw` 供 threaded pipeline 注入自己的 `AsyncWriter`；
- `placeholder_record`、`derive_split_output_paths` 和 FASTQ 字节计数只在
  该模块维护；
- split 路径的非 UTF-8 文件名测试随实现移动。

`DecompressionConfig` 仍保留在 `decompression.rs`，因为两条路径共享同一组
配置，后续若调度收敛再考虑迁移。

## 备选方案

- **一次性合并两条解压调度**：需要统一 original-order、split-PE、range、
  skip-corrupted、并行批次和统计口径，风险远大于本轮输出层收敛；先保留两条
  调度，后续单独评审。
- **继续各自维护输出层**：格式化、事务提交和 split 路由继续双份，正是本次要
  消除的漂移源；否决。
- **把 AsyncWriter 做成 OutputWriters 的默认实现**：sequential 路径不需要
  写后台上限，且 split 两个 AsyncWriter 会放大资源；保留 `buffered` 与
  `single_raw` 两个入口更直接。

## 后果

- 解压输出的格式、header-only 字节数、placeholder、split 路径推导和事务提交
  只有一份实现；
- pipeline 仍使用 AsyncWriter，sequential 仍使用 BufWriter，性能路径不变；
- 解压调度仍是两条路径；这是下一步的独立工作。

## 验证

- 全量 `cargo fmt` / `clippy -D warnings` / `test --lib --tests` / `doc` /
  `+1.75.0 check` / `verify-notes` 通过；
- 真实语料：classic 与 `--pipeline` 对 Illumina、Nanopore、mito SE 解压输出
  逐字节一致；`--range`、`--header-only` 两条路径输出一致；
- `--original-order` 输出大小与输入一致；
- split-PE：interleaved、consecutive、streaming consecutive 三种归档的
  R1/R2 均与输入逐字节一致。
