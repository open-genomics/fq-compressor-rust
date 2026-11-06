# Agent Note: 依赖方向收敛、统一计划/统计与稳定 facade

Status: implemented

## 问题

2026-11-06 的架构审查把项目问题归结为“按文件分层，而不是按依赖方向分层”：

1. `algo::block_compressor` 依赖 `archive::traits::BlockData`，`archive::reader/writer`
   又依赖 `algo` 的 `CompressedBlockData` 与 delta varint，形成 `algo ↔ archive` 双向依赖。
   Rust 允许模块循环，四门禁不会报错。
2. 统计有多套表示：`ProcessingStats`、`PipelineStats`、`CompressionOutcome` 重复字段；
   archive/streaming 曾把 `total_bases` 当 `input_bytes`，pipeline 用估算读段字节，
   `output_bytes` 只累计 block payload，压缩比方向也不一致。
3. 请求校验分散在 CLI、engine、pipeline；`enable_reorder + auto_reorder`、
   `streaming_mode + use_pipeline` 用布尔组合表示三态，存在非法组合。
4. `CompressionEngine` 同时承担模式路由、archive 编排、streaming 拓扑、pipeline 适配、
   header/reorder/finalize/统计组装，变更原因过多；pipeline 仍先完整摄入，但 CLI help
   宣称 3-stage 背压。
5. 块/worker 规划器放在 `BlockCompressorConfig` 上，engine 与 pipeline 各自复制
   thread limit、worker budget、avg length 的调用胶水；`memory_budget.rs` 同时容纳
   ingest/encode/decode 三套预算与未使用的 `MemoryEstimator`。
6. `io` 通过 `begin_fqc_writer` 反向依赖 `archive`；`fastq` 为 `is_valid_base` 依赖 `algo`；
   公共 API 整棵模块树暴露，测试直接依赖内部实现。

## 决定

按可验证切片推进，保持 `.fqc` wire format、codec 字节和解码结果不变：

- **Slice 1：统计单源**。`OperationStats` 统一为唯一统计类型；删除 `PipelineStats` /
  `ProcessingStats` 别名与 outcome 重复字段。`input_bytes` 用实际输入文件字节
  （stdin 用 parser 估算兜底），`total_bases` 单独保留；`output_bytes` 由
  `FqcWriter::finalize()` 返回真实归档大小；`compression_ratio()` 固定为
  `input_bytes / output_bytes`。
- **Slice 2：中立 block**。新增 `block::{BlockMeta, EncodedBlock, DecodedBlock}` 与
  `block::id_codec`；`archive` 只做 `BlockMeta <-> BlockHeader` 转换和字节 I/O，
  `algo` 只处理 reads ↔ `EncodedBlock`。`archive`/`algo` 不再互相 import。
- **Slice 3：计划与校验**。新增 `plan::compress` / `plan::decompress`：
  `CompressionRequest`、`DecompressRequest` 字段私有，`Validated*Plan` 是唯一结构校验入口；
  `ReorderMode { Auto, Always, Never }` 取代两个布尔；`CompressionEngine::run` 接收
  request by value，在入口校验一次。CLI 的 `show_progress` 留在命令层。
- **Slice 4：runner 拆分**。`engine/compression_engine.rs` 只保留路由/outcome，
  模式编排移到 `archive_runner` / `streaming_runner` / `pipeline_runner`；global header
  与 timestamp 收敛到 `archive::writer::write_global_header`。删除
  `CompressionPipelineConfig.streaming_mode`，重排统一为 `ReorderMode`；CLI/文档明确
  compress pipeline 是“完整摄入后并行压缩块”，不是 bounded streaming。
- **Slice 5：规划/内存上移**。`plan::block_plan` 拥有 `CompressionPlanInput`、
  固定点 `plan_compression_layout`、`codec_thread_limit`、`worker_budget_bytes`、
  `avg_read_length`；`memory/` 拆为 `budget` / `decode` / `encode` / `system`，
  删除未使用的 `MemoryEstimator`，保留有测试的 `ChunkingStrategy`。
- **Slice 6：稳定 facade**。新增 `api`：`compress`、`decompress`、`info`、`verify`，
  request/settings/outcome/report 类型在 facade 重导出；内部模块标记 `#[doc(hidden)]`
  并在 crate 文档声明为 internal API。`VerifyCommand` 的校验/解码核心抽成
  `verify_archive(options) -> VerifyReport`，命令层只打印摘要。
- **Slice 7：命名与文档**。`DecompressionPipelineConfig` → `DecompressionConfig`；
  `archive/traits.rs` 已删除；`fastq` 的 DNA 校验移到中性 `dna`；`io` 不再依赖
  `archive`；AGENTS、架构总览、白皮书、CLI 参考与 `.agents/notes` 中的旧路径同步。

## 备选方案

- **直接把所有内部模块改成 `pub(crate)`**：最能保证 API 收窄；否决理由是 28 个集成
  测试直接依赖 `algo`/`archive`/`pipeline` 内部类型，一次性切换会把架构清理变成测试
  重写，风险和收益不匹配。先用 facade + `#[doc(hidden)]` 明确契约，后续再逐模块迁移。
- **为执行模式引入泛型 `ReadSource`/`BlockSink` 框架**：类型层面可根除拓扑重复；
  否决理由与 2026-10-01 note 一致：业余维护项目养不起抽象税，enum + runner 更直白。
- **保留 `ProcessingStats`/`PipelineStats` 兼容别名**：迁移成本最低；否决理由是双统计
  正是口径漂移的根源，项目允许 breaking change。
- **为 `archive`/`algo` 补 Reader/Writer trait 以保留 `traits.rs` 名字**：否决理由是
  当前只有一个实现，空 trait 只增加维护面；直接迁移类型并删除文件。
- **在架构清理同时改 `.fqc` wire format 或实现 ScmV2**：否决理由是格式/编解码变更应
  单独评审；本轮所有改动以“归档字节不变、解码结果不变”为验收线。
- **把 compress pipeline 改成真正的 bounded streaming**：收益大，但涉及 parser →
  bounded channel → N compressor → ordered writer 的新数据流和总读段数回填；本轮只
  校准语义与文档，不把架构清理和并发模型重写绑在一起。

## 后果

- `algo`/`archive` 的依赖方向由 `tests/test_architecture.rs` 守护；新增跨层 import 会
  在 `cargo test` 失败，而不是靠人工 review。
- `CompressionRequest`/`DecompressRequest` 不能再靠结构体字面量构造非法组合；
  `ReorderMode` 消除了 `enable=false, auto=true` 这类状态。
- `CompressionOutcome` 只保留 `mode`、`detected_read_length_class`、`stats`；统计字段
  只有 `OperationStats` 一处定义。
- `fqc::api` 是稳定嵌入面；内部模块仍 public（集成测试需要），但文档隐藏且不承诺稳定。
- `verify` 的 verbose 细节改走 `log`（`-v` 可见），顶层 PASSED/FAILED 摘要仍由命令打印。
- compress pipeline 的行为没有变成低内存：它仍完整摄入；只是 CLI/文档不再误导。
- `.fqc` magic、版本、block header、codec 字节、质量流字节均未改变。

## 验证

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --locked -- -D warnings`
- `cargo test --lib --tests --locked`（含新增 `tests/test_architecture.rs`）
- `cargo doc --no-deps --locked`
- `npm run verify-notes`
- `cargo +1.75.0 check --all-targets --locked`
- `cargo build --release --locked`
- 100k×150bp 合成短读：archive 与 pipeline 默认路径 round-trip 逐字节一致，归档仅
  header 时间戳不同；统计的 `input_bytes`/`output_bytes` 与 `fs::metadata` 对拍。
- 固定质量样本压缩流 xxHash64 保持 `839fc7efbf59b7d3`；rescale 样本保持
  `aa422204a59bb473`，证明质量编码字节未漂移。
