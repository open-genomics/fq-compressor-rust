# Agent Note: archive / pipeline 共享有界块级流水线

Status: implemented

## 问题

archive 和 pipeline 两条压缩路径各自维护“并行压缩块 + 顺序写入”：

- archive 的 `common::compress_blocks` 用 Rayon 一次收集
  `Vec<Result<EncodedBlock>>`，所有压缩块在写盘前同时驻留，放大了已经很高的
  全量摄入峰值；
- pipeline 有一套线程 + 有界 channel + `BTreeMap` 顺序写实现，archive 又有一套
  先收集再顺序写的实现；两边的 header、transaction、reorder map、错误传播和
  统计各自维护；
- pipeline 的 `write_ms` 从 writer 线程启动时计时，包含了等待压缩块的时间，
  不是真正的串行写时间；archive 改成共享实现后也会继承这个口径错误。

这不是要把 pipeline 改成 bounded ingest；archive/pipeline 仍需要完整摄入以支持
全局分析和可选重排。问题是**摄入之后的块阶段**应该有且只有一套有界实现。

## 决定

新增 `src/pipeline/block_pipeline.rs`，作为 archive 与 pipeline 共用的块阶段：

- 输入是已经按归档顺序切好的 `Vec<Vec<ReadRecord>>`；
- reader 线程通过有界 channel 发送块；
- N 个 compressor 线程并行编码，按 chunk id 发送完成块；
- writer 线程用 `BTreeMap` 按 id 顺序写块，最后写 reorder map、finalize 并
  commit 事务输出；
- 任一线程失败时关闭 channel、拒绝提交半成品；主线程 join 所有阶段后再返回；
- `process_ms` 仍是 worker 聚合 CPU 时间；`write_ms` 只累计 header、block、
  reorder map、finalize/commit 的实际串行 I/O 时间，不再包含等待压缩的时间。

`archive_runner` 不再调用 `common::compress_blocks`，也不再自己持有
`FqcWriter`；它只负责全局分析、重排、块切分和统计组装。`CompressionPipeline`
也删除自己的线程/channel/writer 副本，调用同一个 block pipeline。

另外把 pipeline 的 `split_into_chunks` 从反复 `Vec::split_off` 改成单次
`into_iter().take(block_size)` 移动扫描。旧实现每切一块都复制剩余读段，
既产生 O(n²) 拷贝，又让首块的 `Vec` 保留接近整个输入的超大容量；这正是
release 冒烟中 pipeline RSS 远高于 archive 的原因。

## 备选方案

- **保留两套实现，只改 archive 为流式写**：能降低 archive 峰值，但会继续维护
  两份顺序写/错误传播/统计口径；否决。
- **让 archive 直接复用整个 `CompressionPipeline`**：会把“读入 + 全局分析”
  再次耦合进 pipeline 的配置和拓扑；archive 只需要块阶段，不应依赖整条 pipeline。
- **把 block pipeline 做成泛型 `BlockSource`/`BlockSink`**：理论最通用，但当前
  只有“预切块”一种输入；为尚未存在的第二实现引入 trait 抽象税，与本项目
  “业余维护、避免投机抽象”的定位冲突。
- **同时把 pipeline 改成 bounded ingest**：会改变全局重排、总读段数回填和
  reorder map 的语义，必须独立评审；本轮只收敛块阶段，不改变 CLI 行为。

## 后果

- archive 不再在写盘前保留全部 `EncodedBlock`，只保留有界数量的在途压缩块；
  输入读段仍然完整驻留，这是全局分析/重排的既有成本。
- archive 与 pipeline 的块顺序、header、reorder map、事务提交和错误传播由同一
  份代码决定；以后修 writer 或时序只需改一处。
- `write_ms` 对 archive 和 pipeline 都变为“实际写 I/O 时间”，不再把等待
  compressor 的时间算作写时间；`process_ms` 仍是聚合 CPU 时间。
- `.fqc` wire format、codec 字节、block id 顺序和解压结果不变。

## 验证

- release 冒烟（100k × 150bp 随机读段，`--block-reads 1000 --reorder false`，
  `-t 4`）：archive 与 pipeline 都产出 100 个 block、4,725,031 字节，解压结果
  与输入逐字节一致；archive 峰值 RSS 139,188 KB / 0.26 s，pipeline 111,836 KB /
  0.26 s。修复 `split_into_chunks` 前同一 pipeline 用例为 564,784 KB / 0.72 s，
  说明“共享块流水线 + 单次移动分块”消除了实际的内存放大。
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --locked -- -D warnings`
- `cargo test --lib --tests --locked`
- `cargo doc --no-deps --locked`
- `cargo +1.75.0 check --all-targets --locked`
- `cargo build --release --locked`
- `npm run verify-notes`
- `tests/test_pipeline_concurrency.rs`：多 block、4/8 worker、重复运行的乱序完成
  与顺序写入路径。
- `tests/test_e2e.rs` / `tests/test_mode_matrix.rs`：archive、pipeline、streaming
  三种模式的 round-trip、reorder map 和多 block 行为。
