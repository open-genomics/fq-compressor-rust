# Agent Note: 按需块源与 streaming 接入共享块流水线

Status: implemented

## 问题

`block_pipeline` 已经把 archive 与 pipeline 的“并行压缩 + 顺序写入”收敛成
一份实现，但它只接受 `Vec<Vec<ReadRecord>>`：

- 输入必须预先全部切块，无法表达 streaming 的按需解析；
- streaming 仍保留独立的 `run_core`，自己创建 writer、写 header、串行压缩、
  写块、patch 总读段数并 finalize；
- 任何 writer/错误传播/统计口径修复都必须在两套实现里各做一次；
- `streaming` 的 `write_ms`、事务提交和零块边界与共享实现存在漂移风险。

目标不是让 archive/pipeline 变成增量摄入，而是让三种模式的**块阶段**只有一套
实现，同时让 streaming 的摄入保持有界。

## 决定

把 `block_pipeline::run` 的输入改为按需块源：

```rust
pub(crate) fn run(
    next_chunk: impl FnMut() -> Result<Option<Vec<ReadRecord>>> + Send + 'static,
    output_path: impl AsRef<Path>,
    config: BlockPipelineConfig,
) -> Result<BlockPipelineReport>
```

reader 线程在闭包上拉取块，`None` 表示源结束。archive/pipeline 用
`chunks.into_iter()` 适配预切块；streaming 直接把 single/paired/interleaved
parser 的 `next_block()` 交给它。

`BlockPipelineConfig` 用两个 `Option` 表达两种输入语义：

- `total_reads = Some(n)`：archive/pipeline 已知总读段数，global header 直接写；
- `total_reads = None`：streaming，写完块后 `patch_total_read_count`；
- `expected_blocks = Some(n)`：archive/pipeline 必须写满 n 块；
- `expected_blocks = None`：streaming 以源结束为准，允许空输入归档。

空输入边界分开处理：`expected_blocks == Some(0)` 在 `run` 开头返回
`InvalidArgument("no blocks to write")`，保持 archive/pipeline 原来的拒绝行为；
streaming 仍允许 0 块，保持其空输入语义。

writer 在拉取任何输入之前、在调用线程上打开输出事务。这样不可写路径会立即
失败，而不是先启动一个可能阻塞在交互式 stdin 上的 reader 线程。

reader 统计源交出的读段数，writer 统计每个压缩块声明的 `read_count`；两者必须
一致，否则返回压缩完整性错误。这个校验同时消除了 `ReaderOutcome.total_reads`
的未使用字段，并能捕获块在解析与归档之间丢读段的问题。

writer 只负责把临时文件写完并 finalize，不再自己 commit。`OutputTransaction`
随 `WriterOutcome` 返回主线程，等所有 stage join、错误排序和读段一致性检查都
通过后才提交。对 streaming 的 `expected_blocks = None` 尤其重要：解析器在
中途失败时，writer 可能已经写出若干连续块并 finalize；如果仍在 writer 线程
提交，就会发布截断归档。现在任一早于提交的失败都会 drop 事务并删除临时文件。

streaming 也改为调用共享块流水线：

- `num_workers` 由 `codec_profile` 的线程上限、worker 预算、块大小和平均读长
  共同决定（复用 `plan::memory_limited_worker_count`）；
- 只预取 **1** 个块。解析器是串行的，加深队列只会成倍增加常驻块，不能更快
  喂饱 worker；archive/pipeline 继续使用原来的 8 块上限；
- `parse_ms` = 长度预扫描 + reader 拉块；`process_ms` 与 `write_ms` 由共享
  流水线统一提供；
- writer 的 setup 时间计入 `write_ms`，与旧口径一致。

## 备选方案

- **保留 streaming 的串行 `run_core`，只共享 writer/header**：仍然维护第二套
  压缩循环、错误传播和统计；否决。
- **引入泛型 `BlockSource` trait**：当前只有“预切块”和“按需解析”两种闭包适配，
  trait 会增加类型参数和对象安全成本，收益不足；闭包已经表达同一协议。
- **让 streaming 也使用 8 块在途上限**：370 MB / 16 块实测峰值 RSS 约 700 MB，
  比串行 130 MB 高 5 倍；streaming 的定位是低内存，1 块预取在 4 worker 下仍有
  ~2.9× 吞吐提升，因此拒绝更深队列。
- **把 archive/pipeline 改成增量摄入**：会改变全局重排、总读段数和 reorder map
  语义，必须独立评审；本轮只统一块阶段。

## 后果

- 三种压缩模式共用 reader/compressor/writer、事务提交、错误优先级和阶段计时；
- streaming 的归档体（除 global header 的原始文件名/时间戳外）与旧实现逐字节
  一致；块划分和 codec 字节不变；
- streaming 仍是单遍增量摄入，常驻内存与输入大小无关，但现在可以并行压缩块；
  峰值由块大小、worker 数和 1 块预取决定，并受 `--memory-limit` 约束；
- `--memory-limit` 较小时 worker 数会下降，必要时回到 1 个 worker；这使
  streaming 的 worker 规划与 archive 使用同一套内存估算；
- 文档中“streaming 是单线程增量路径”的表述已过期，改为“共享块流水线 +
  有界增量摄入”。

## 验证

真实语料（Illumina 10k×150bp、Nanopore 4k reads、mito SE 161,675 reads、
mito PE 161,675 pairs）streaming 压缩后解压，与输入逐字节 `cmp` 一致；归档大小
与旧 streaming 实现完全相同（641,148 / 1,612,235 / 7,413,877 / 12,806,271 B），
差异只在 global header 的原始文件名/时间戳。

370 MB / 16 块合成语料（mito_1 × 8）release 冒烟：

| 配置 | 耗时 | 峰值 RSS |
|---|---:|---:|
| 旧串行 streaming | 6.9 s | 130 MB |
| 新 streaming，1 worker，1 预取 | 7.0 s | 218 MB |
| 新 streaming，2 worker，1 预取 | 3.9 s | 342 MB |
| 新 streaming，3 worker，1 预取 | 2.7 s | 462 MB |
| 新 streaming，4 worker，1 预取 | 2.3–2.5 s | 约 575 MB |
| 新 streaming，`--memory-limit 512` | 2.9 s | 463 MB |
| 新 streaming，`--memory-limit 256` | 6.6 s | 218 MB |

验证命令：

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --locked -- -D warnings`
- `cargo test --lib --tests --locked`
- `cargo doc --no-deps --locked`
- `cargo +1.75.0 check --all-targets --locked`
- `npm run verify-notes`
- 新增 `tests/test_mode_matrix.rs::streaming_parallel_blocks_keep_order_and_content`
  固定 3 块、2 worker 的 streaming 顺序与内容；
- 新增 `tests/test_output_atomic.rs::streaming_malformed_input_aborts_without_publishing`
  固定“前一块已写出、后一块解析失败”时不发布归档且不残留临时文件。
