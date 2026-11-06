# Agent Note: 解压调度收敛到单一 pipeline

Status: implemented

## 问题

解压仍有两条调度：

- `DecompressionPipeline`：Reader thread → N decompressor threads → Writer
  thread，有界 channel，支持 range/header-only/skip-corrupted；
- `decompression_classic`：serial 或 rayon batch，支持 original-order、
  split-PE、range/header-only/skip-corrupted。

`api::decompress` 按 `use_pipeline && !original_order && !split_pe` 路由，导致
默认路径与 `--pipeline` 路径行为、统计和性能特征分叉。共享输出层只解决了写入
格式，块调度仍然两套。

## 决定

让 `DecompressionPipeline` 成为唯一解压调度：

- 删除 `decompression_classic.rs`；
- `api::decompress` 总是调用 `DecompressionPipeline`；
- `--pipeline` 保留为兼容别名（CLI/API 字段仍在，但不再切换路径）；
- pipeline 新增：
  - `original_order`：加载 reorder map，writer 先按 archive 顺序收集
    `Vec<ReadRecord>`，channel 关闭后按 `forward_map` 输出；
  - `split_pe`：writer 使用共享 `OutputWriters::buffered` 创建 R1/R2 事务；
  - 正常 block 顺序下按读段范围过滤，并向 `write_record` 传
    `block_local_idx` 以保持 streaming consecutive 的 R1/R2 路由；
  - corrupted block 在正常模式直接写占位读段，在 original-order 模式先把
    占位读段放进 archive 顺序缓冲区，再由 reorder map 定位。
- `find_block_range` 仍只用于非 original-order；original-order 必须处理全部
  块，因为它需要完整 archive 顺序。
- original-order 的峰值预算检查仍在创建输出之前执行。

## 备选方案

- **保留 classic 专门处理 original-order/split-PE**：这正是本次要消除的两套
  调度；共享输出层后剩余差异已经可以放进 writer 分支，否决。
- **让 classic 改成调用 pipeline 的薄包装**：仍保留两套入口和路由条件，收益
  不足；直接删除 classic 更彻底。
- **为 original-order 单独保留一套 writer**：它会再次复制范围过滤、split
  路由和事务提交；改为在同一个 writer thread 内分支。
- **删除 `--pipeline` 标志**：会破坏 CLI/API 兼容性；保留为别名，行为与默认
  一致。

## 后果

- 解压只有一条 Reader→Decompressor→Writer 调度；original-order、split-PE、
  range、header-only、skip-corrupted 共用同一套块读取、解码、统计和提交逻辑；
- 默认解压从 classic 的 rayon batch 切换到 threaded pipeline：
  - 370 MB 以下真实语料上 multi-block 略快（mito SE 约 0.26 s vs 0.34 s，
    mito PE 约 0.51 s vs 0.60 s），单 block 小文件有约 0.01 s 线程开销；
  - RSS 略低（mito SE 187 MB vs 197 MB，mito PE 253–260 MB vs 272 MB）；
- `--pipeline` 不再是行为开关，仅保留兼容；文档已注明；
- `OperationStats.blocks` 统一为“本次选中的块数”（含 corrupted 块），
  与原 `--pipeline` 语义一致。

## 验证

- 全量 fmt / clippy / test / doc / MSRV / verify-notes 通过；
- 真实语料旧 classic 二进制 vs 新统一 pipeline：
  - Illumina、Nanopore、mito SE 默认解压逐字节一致；
  - `--range 100:200`、`--header-only` 逐字节一致；
  - `--original-order` 逐字节一致；
  - split-PE interleaved、consecutive、streaming consecutive 的 R1/R2
    逐字节一致；
- 新增 `test_original_order_skip_corrupted_emits_placeholders`，覆盖
  original-order + skip-corrupted 的占位读段路径；
- 现有 `test_skip_corrupted_emits_placeholder_reads` 与
  `test_pipeline_skip_corrupted_emits_placeholder_reads` 继续通过。
