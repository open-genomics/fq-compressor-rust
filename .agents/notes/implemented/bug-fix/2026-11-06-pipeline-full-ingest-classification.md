# Agent Note: pipeline 在完整摄入后统一分类读长

Status: implemented

## 问题

pipeline 模式在摄入前用 `inspect_input_lengths_for_streaming` 预扫描文件：

- 默认只读前 4,096 条读段；
- `--scan-all-lengths` 会把整个输入读一遍，随后完整摄入再读一遍；
- stdin 无法预扫描，直接回退到 150 bp / Short。

archive 模式则在完整摄入后，用 `length_stats_from_records_sampled` 在全库均匀
抽样。两种抽样方式对“长读段只出现在尾部”的输入会给出不同分类。

实测构造 4,100 条 100 bp 短读 + 100 条 5,000 bp 长读：

```text
archive:  medium, 5 blocks, 40,329 B
pipeline: short,  4 blocks, 40,025 B
```

pipeline 的 Short 分类不会损坏数据（实际序列长度超过 ABC 上限时仍走 Zstd），但
会使用 Short 的 quality context order、块大小和 worker 规划，与 archive 产生不同
归档布局，并可能降低长读压缩效果。

## 决定

pipeline 不再预扫描，统一在完整摄入后分类：

- `CompressionPipeline::finish_ingest` 是所有拓扑的唯一汇合点：先
  `length_stats_from_records_sampled(&all_reads, scan_all_lengths)`，再
  `effective_read_length_class(requested, &stats)`；
- 分类结果存入 `detected_read_length_class`，engine 用它填充 `CompressionOutcome`；
- `CompressionPipelineConfig` 改为携带与类别无关的
  `BlockCodecSettings { quality_mode, id_mode, zstd_level }`，加上
  `requested_read_length_class: Option<ReadLengthClass>` 和
  `scan_all_lengths: bool`；摄入后调用
  `BlockCodecSettings::with_read_length_class(class)` 生成最终
  `BlockCompressorConfig`；
- `engine::common::codec_profile` 也改用同一构造入口，保留 archive/streaming 的
  单次映射；
- 读长抽样与分类策略移到 `plan::length`，archive 与 pipeline 共用；
- `effective_in_flight_blocks` 移到 `plan::block_plan`，显式接收调用方的
  `max_in_flight` 上限；pipeline 在拿到精确 block size 和统计后再计算，archive
  仍使用 `DEFAULT_MAX_IN_FLIGHT_BLOCKS` 作为上限；
- paired 拓扑在 `finish_ingest` 内强制 `ReorderMode::Never`，与 archive 一致，
  避免把成对读段当成可任意重排的平坦列表。

`CompressionPipelineConfig` 是 internal API，本次直接改字段，不保留兼容层；仓库
定位允许破坏性变更。

## 备选方案

- **保留预扫描，但改成尾部也采样**：需要随机访问或完整扫描；gzip/stdin 无法随机
  访问，完整扫描又回到双倍 I/O。
- **pipeline 永远 `scan_all_lengths = true`**：分类更准，但默认路径白白多读一遍
  文件；本次直接消除预扫描。
- **保留 `CompressionPipelineConfig.compressor_config`，摄入后覆盖
  `read_length_class`**：字段里会存在一个“占位类别”，调用方容易误用；显式拆成
  `BlockCodecSettings` + requested class 更诚实。
- **只在 `pipeline_runner` 里把前 4,096 条改成均匀抽样**：仍是额外一遍 I/O，
  且 stdin 无法处理；不解决根因。
- **让 `BlockCompressorConfig` 自己持有可选类别**：会把“未分类”状态扩散到 codec
  层，编码器不应该接受未解析的类别。

## 后果

- pipeline 不再预扫描输入：默认少一遍 4,096 条读段读取，
  `--scan-all-lengths` 少一整遍文件读取；
- archive 与 pipeline 对同一输入得到相同的读长分类、block size、worker 数和块数；
- stdin pipeline 也能根据实际读段分类，而不是固定 150 bp / Short；
- `max_in_flight_blocks` 仍尊重调用方上限，但会按精确 block size 与内存预算收紧；
- paired pipeline 的重排行为与 archive 对齐；
- `.fqc` wire format、codec 字节格式不变；块布局可能因分类修正而变化，这是预期
  的行为修复。

## 验证

- 新增 `tests/test_compression_engine.rs::
  archive_and_pipeline_classify_mixed_length_tail_identically`：4,100 条短读 +
  100 条长读，断言两者都分类为 Medium、块数相同、归档大小相同；修复前该用例为
  archive Medium/5 blocks、pipeline Short/4 blocks。
- 真实数据（nf-core Sarek NA12878 mito，161,675 对 130 bp reads）：
  archive 与 pipeline 均为 8 blocks、12,856,367 B，解压 R1/R2 与输入逐字节一致；
  pipeline 0.85 s / 288,436 KB，archive 0.91 s / 317,268 KB。
- 真实数据（seqkit Illumina 10,000 × 150 bp）：archive/pipeline 均 1 block、
  471,814 B、Short、带 reorder map；`--original-order` 解压与输入逐字节一致。
- 真实数据（seqkit Nanopore 4,000 reads，median 318 bp、max 6,006 bp）：
  archive/pipeline 均 Medium、2 blocks、1,664,392 B，解压逐字节一致。
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --locked -- -D warnings`
- `cargo test --lib --tests --locked`
- `cargo doc --no-deps --locked`
- `cargo +1.75.0 check --all-targets --locked`
- `npm run verify-notes`
