# 任务：fqc 架构收敛——统一压缩/解压执行路径

> 计划文件：`subagents/plans/2026-10-01-architecture-convergence.md`
> 状态：**Phase 0/A 已完成**（每完成一个 Phase，勾选对应复选框并更新本行，不要留下未勾选的僵尸计划）

你在仓库 `fq-compressor-rust`（FASTQ 压缩器 `fqc`）。本次任务是一次**保持行为不变的架构收敛重构**：消除压缩/解压的多套平行实现。除下文明示的 run_paired 并行化外，禁止改变任何用户可见行为。

开工前必读（按序）：

1. `AGENTS.md` — 仓库约定：Agent Note 制度、四门禁、编辑护栏
2. `CONTEXT.md` — 领域词汇（contig / minimizer / SCM / streams）
3. `.agents/skills/write-notes-like-deepseek/SKILL.md` — 本任务必须写决策笔记
4. `src/engine/compression_engine.rs`、`src/pipeline/compression.rs`、`src/pipeline/decompression.rs`、`src/commands/decompress.rs`、`src/commands/verify.rs`、`src/main.rs`
5. `docs/` 中描述执行模式与模块职责的文档（重构后需同步）

## 硬性约束

- **wire 格式冻结**：不改 `.fqc` 二进制布局、magic、版本号。`tests/test_format_contract.rs`（冻结字节 + SHA-256）必须**原样**通过，不许改测试本身。
- **内存语义冻结**：三种模式的内存特征是用户可见契约——Archive=全量 ingest、Streaming=有界、Pipeline=分阶段。`tests/test_compress_budget.rs`、`tests/test_decode_budget.rs` 必须原样通过。
- **CLI 与退出码不变**：参数、默认值、`src/error.rs` 的退出码映射都不动（因此 README 与 `docs/reference/cli.md` 无需改动）。
- MSRV 1.75：不得使用更高版本才有的 std API；不新增 unsafe；`cargo clippy --all-targets -- -D warnings` 保持零告警。
- 已知 flaky：`tests/test_pipeline_concurrency.rs:190` 附近的时间戳断言（秒级粒度）会偶发红。这是已登记的独立问题，**不在本任务范围内**；撞上时重跑确认与本重构无关即可，不要顺手修。
- 测试/构建报 `__tunable_is_initialized@GLIBC_PRIVATE` 时，给 cargo 命令加前缀 `PATH="/usr/bin:/bin:/usr/local/bin:$HOME/.cargo/bin"`（见 AGENTS.md Troubleshooting）。

## Phase 0 —— 摸底与安全网（先做）

- [x] **行为审计**：逐行对比 `src/commands/decompress.rs` 的编排（`run_parallel`/`process_block`/`run_original_order`，约 :387/:510/:552）与 `src/pipeline/decompression.rs::DecompressionPipeline::run`（约 :162），列出所有语义差异：`skip_corrupted`、`--range`、`original_order`、占位符写入、分批策略、错误处理时机。同样对比 engine 侧与 pipeline 侧的压缩路径差异。产出一份差异清单，附在决策笔记里。
- [x] **补齐行为钉死测试**：审计 `tests/` 对 {single, paired, interleaved} × {archive, streaming, pipeline} 的覆盖矩阵，为缺失组合补 roundtrip + verify + `info` 字段断言（数据用 `tests/data/` 现有文件或程序化生成，小而快）。重构前让每条路径都有测试钉住，这是整个任务的安全网。
- [x] **注意**：归档输出含时间戳（`src/pipeline/compression.rs:331` 一带），**不要**用"对归档文件做字节级 golden 比对"来钉行为——用 roundtrip（`decompress(compress(x)) == x`）和 `info`/`verify` 字段断言。

## Phase A —— 解压路径下沉

- [x] 目标：`src/commands/decompress.rs` 退化为薄 CLI 层（参数解析 → 调 pipeline/engine API → 汇报结果），不再 import `algo::block_compressor`、`archive::reader`、`archive::traits`、`fastq::parser` 内部符号；`DecompressionPipeline` 成为解压编排的**唯一**实现。
- [x] 以 Phase 0 差异清单为准统一语义：两路径行为不一致处，选更防御/更正确的一侧，用测试钉死，并在决策笔记里记录每一次取舍。
- [x] `verify.rs` 目前也直接驱动 `FqcReader` + `BlockCompressor`（约 :89、:155-162）：评估是否复用同一批 pipeline 原语。verify 的语义是"校验而非产出"，允许保留独立路径，但结论（复用或保留及原因）必须写进笔记。
- [x] 验收：grep 确认 `commands/decompress.rs` 无 algo/archive 内部 import；`test_e2e.rs`、`test_stdin_stdout.rs`、`test_verify_detection.rs`、`test_output_atomic.rs`、`test_compressed_inputs.rs` 原样全绿。

## Phase B —— 压缩路径收敛

- [ ] 目标：七条路径收敛为**一个骨架 + 两组参数**（engine 侧 4 个 + pipeline 侧 3 个）——拓扑（Single/Paired/Interleaved）× 执行模式（Archive/Streaming/Pipeline）。骨架内阶段：parse → global analysis/reorder → block 构建 → 压缩 → 写出/commit。涉及：
  - `engine/compression_engine.rs`：`run_archive`（约 :152）与 `run_streaming_single/paired/interleaved`（约 :694/:806/:923）
  - `pipeline/compression.rs`：`run`/`run_paired`/`run_interleaved`（约 :141/:427/:572）
- [ ] 先依 Phase 0 差异清单定骨架 API，再迁移。逐字重复的 reorder-map 构造（`pipeline/compression.rs:189-193` vs `:467-471`）必须是同一份代码。
- [ ] **消除 run_paired 的串行不一致**（约 :535-539 单实例串行）：改为与其他拓扑同等的并行。验收标准：roundtrip 解码结果与原输入逐字节一致，`info` 输出的块数等结构与串行版一致。
- [ ] streaming 三兄弟重复的约 40 行样板（header 构造、timestamp、`ProcessingStats`/`CompressionOutcome` 填充）提取为共享辅助函数。
- [ ] 内存预算是红线：三模式内存特征一个都不能变，budget 测试原样通过。
- [ ] 验收：grep 确认无第二份 reorder-map 构造；拓扑×模式矩阵测试全绿；`cargo bench --quiet` 跑 `parser_throughput` 与 `archive_workflow` 确认无明显回退。

## Phase C —— 拆豁免

- [ ] `grep -rn "allow(clippy::too_many_lines)\|allow(clippy::needless_pass_by_value)" src/`（扫描时确认：`pipeline/compression.rs:140`、`pipeline/decompression.rs:161` 的 too_many_lines，`engine/compression_engine.rs:151/399/467` 一带的 needless_pass_by_value，共 2+3 处（已逐条实测确认））：把相关函数拆到 `clippy.toml` 阈值（200 行）以内、修正参数传递方式，删除豁免。Phase A/B 完成后这些函数应已大幅缩短，剩余工作多是顺势拆分。
- [ ] `src/main.rs:7` 的 bin 侧 crate 级 `#![allow(dead_code)]`（代码注释表明作者知情，但豁免仍覆盖整个 bin）：先查 git 历史与笔记搞清 bin 为什么重声明整个模块树；优先方案是 `main.rs` 只留 CLI 薄层、复用 lib target（`fqc::…`），彻底删除该豁免；若确有原因不可行，把 allow 缩到最小范围并写进笔记。
- [ ] 验收：上述 allow 零命中（或仅剩最小范围且有笔记说明）；四门禁 + `cargo doc --no-deps` 通过。

## 过程要求

- 本任务是 AGENTS.md 定义的典型 non-trivial change：**开工前先写决策笔记**（用 `write-notes-like-deepseek` skill，放 `.agents/notes/implemented/architecture/`，记录收敛方案、Phase 0 发现的语义差异与取舍、被否决的替代方案）。`npm run verify-notes` 必须通过。Phase A 与 Phase B 若各有重大取舍，可拆两篇笔记并互相链接。
- 提交粒度：Phase 0/A/B/C 各一个 commit，conventional commits 中文格式（如 `refactor(decompress): 解压编排下沉 DecompressionPipeline`）。
- 在 `CHANGELOG.md` 的 `[Unreleased]` 记一条（Changed：内部重构，无行为变化）。
- 遇到与本文行号/数量不符的代码现状，以代码为准，在笔记里记录偏差。
- **范围外**（发现的其他问题记到笔记末尾"顺带发现"清单即可，不顺手修）：wire 格式、错误码映射、flaky 时间戳测试、`openspec/`、新功能。

## 完成定义（DoD）

- [ ] Phase 0：语义差异清单落盘 + 覆盖矩阵补齐，钉死测试全绿
- [ ] Phase A：解压单一路径，commands 层无算法/归档内部依赖
- [ ] Phase B：压缩骨架 + 拓扑×模式参数化，run_paired 并行，budget 测试原样通过
- [ ] Phase C：too_many_lines / needless_pass_by_value 豁免清零，dead_code 豁免删除或最小化
- [ ] 四门禁全绿 + verify-notes 通过 + bench 无明显回退
- [ ] Agent Note 落盘；docs 中描述模块职责的段落已同步
