# Agent Note: archive / pipeline 收敛到同一个完整摄入实现

Status: implemented

## 问题

`archive_runner` 与 `CompressionPipeline` 各自实现同一条完整摄入路径：

- 读取全部读段、按拓扑选择 parser；
- 统计长度、分类读长；
- 规划块大小与 worker；
- 建 flags、可选重排、切块；
- 调用共享块流水线、组装 `OperationStats`。

两条路径在真实语料上已经产出字节相同的归档（除 global header 时间戳），说明
它们不是“两种不同策略”，而是同一实现的两次维护。继续保留会让块规划、flags、
重排开关、空输入错误和统计口径再次漂移。

## 决定

删除 `archive_runner`，新增 `full_ingest_runner` 同时服务 `Archive` 与
`Pipeline` 两种模式：

- `CompressionEngine` 把 `Archive | Pipeline` 路由到
  `full_ingest_runner::run(request, mode)`，`Streaming` 仍走独立 runner；
- runner 负责模式标签与日志，把完整摄入、分类、重排和块阶段全部交给
  `CompressionPipeline`；
- 返回的 `CompressionOutcome.mode` 仍是调用方请求的模式，CLI/API 语义不变；
- `--pipeline` 保留“每块至少 100 读段”的校验，archive 继续允许更小的块
  （`tests/test_audit_regressions.rs::repro_6_inverted_range_stats` 固定了该行为）。
  该差异通过 `CompressionPipelineConfig::enforce_min_block_size` 表达，而不是
  再复制一套 runner；
- 空输入错误统一为 `Input file contains no FASTQ records`（此前 archive 与
  pipeline 分别是 `contains no FASTQ records` / `Input file is empty`）。

## 备选方案

- **保留 `archive_runner` 作为薄包装**：仍要维护第二套 ingest/分类/切块代码，
  正是本次要消除的重复；否决。
- **删除 `--pipeline` 压缩标志**：会破坏 CLI/API 兼容性，也影响测试和脚本；
  本轮只收敛实现，保留模式标签；否决。
- **让 archive 依赖 `CompressionPipeline` 的块阶段而不是完整 pipeline**：上一轮
  已经这样做过，结果就是两套完整摄入继续并存；否决。
- **把 streaming 也塞进 full-ingest runner**：两者摄入语义相反（全量 vs 按需），
  会重新引入模式分支；streaming 保持独立 runner，只共享块阶段。

## 后果

- 完整摄入编排只剩一份；archive/pipeline 的归档体、块数、读段统计和解压结果
  由同一代码决定；
- archive 继承 pipeline 已有的内存预算、分类和重排逻辑；`--reorder` 行为由
  `GlobalAnalyzer` 的 Short 类别门控保证与旧 archive 一致；
- 两种模式的日志前缀仍按请求模式区分，`--pipeline` 的块大小校验保留；
- `.fqc` wire format、codec 字节和压缩结果不变。

## 验证

- 真实语料（Illumina、Nanopore、mito SE、mito PE）用同一输入分别跑 archive 与
  pipeline：归档体逐字节相同（较大的两个文件仅时间戳 1 字节不同），解压结果
  逐字节一致；
- `tests/test_mode_matrix.rs::archive_and_pipeline_share_full_ingest_output`
  固定两种模式的 block 数、总读段数和解压内容一致；
- 全量 `cargo fmt` / `clippy -D warnings` / `test --lib --tests` / `doc` /
  `+1.75.0 check` / `npm run verify-notes` 通过。
