# Agent Note: 统一 settings 到 codec 配置的映射

Status: implemented

## 问题

`CompressionSettings` 到 `BlockCompressorConfig` 的映射在三个执行路径各写一遍：

- `archive_runner` 构造 codec 配置，再用它估算 quality model 内存；
- `pipeline_runner` 先把 `quality_mode` / `id_mode` / `level` / `read_length_class`
  装进 `CompressionPipelineConfig`，`CompressionPipeline` 内部再映射一次；
- `streaming_runner` 在 `run_core` 里再构造一份。

同一份 CLI 设置被翻译多次，新增一个 codec 字段时必须记得同步所有路径，否则
archive / pipeline / streaming 会静默产生不同码流。`CompressionPipelineConfig`
还重复暴露 `quality_mode`、`id_mode`、`compression_level` 等字段，让调用方误以为
这些字段独立于 codec 配置。

## 决定

在 `src/engine/compression_engine/common.rs` 增加单一解析入口：

`CodecProfile` 携带 `config: BlockCompressorConfig`、`thread_limit`、
`worker_budget_bytes` 和 `model_bytes`；`codec_profile()` 接收
`&CompressionSettings` 与已解析的 `ReadLengthClass`。

它一次性完成：

- `quality_mode` / `id_mode` / `level` / `read_length_class` → `BlockCompressorConfig`；
- `threads` + `memory_limit_mb` → codec 线程上限；
- 内存限制 → worker 预算；
- codec 配置 → quality model 驻留字节数（供块/worker 规划）。

archive、pipeline、streaming 都只调用 `common::codec_profile(...)`。同时：

- `CompressionPipelineConfig` 用与类别无关的 `codec: BlockCodecSettings` 取代
  四个 codec 字段，删除内部的 `block_compressor_config()` 二次映射；读长类别由
  pipeline 在完整摄入后解析（见
  `2026-11-06-pipeline-full-ingest-classification.md`）；
- `build_flags`、`GlobalAnalyzer`、`BlockPipelineConfig` 需要的 codec 字段直接读
  `compressor_config`；
- 删除只做转发的 `common::codec_thread_limit` 包装。

`CompressionPipelineConfig` 是测试和高级调用方可见的 internal API；本次直接改字段，
不保留兼容构造器。仓库定位允许破坏性变更。

## 备选方案

- **让 `BlockCompressorConfig` 直接接收 `CompressionSettings`**：会把请求/CLI 层
  耦合进 `algo`，破坏“algo 只描述编码”的边界。
- **只在 pipeline 内复用 `BlockCompressorConfig`，archive/streaming 继续各自映射**：
  不能消除三份映射中的两份，字段漂移仍会发生。
- **给 `CompressionPipelineConfig` 保留旧字段并加 `From` 转换**：会继续维护两套
  配置面，且无法阻止两边字段不一致；否决。
- **把 `CodecProfile` 放进 `plan` 模块**：`plan` 目前已禁止依赖更高层，但它也不需要
  理解 `BlockCompressorConfig`；放在 engine 的共享 helper 中更贴近调用方。

## 后果

- codec 配置、线程上限、worker 预算和 model 估算只有一个来源；
- `CompressionPipelineConfig` 不再暴露可漂移的重复字段，测试也改为直接提供
  `BlockCompressorConfig`；
- archive / pipeline / streaming 的编码参数保持逐字段一致；
- 不改变 CLI 默认值、`.fqc` 格式或块规划算法；
- `common` 增加对 `algo::block_compressor` 的依赖，但 engine 本来就负责编排 codec，
  不引入循环。

## 验证

- `cargo check --all-targets --locked`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --locked -- -D warnings`
- `cargo test --lib --tests --locked`
- `cargo doc --no-deps --locked`
- `cargo +1.75.0 check --all-targets --locked`
- `npm run verify-notes`
