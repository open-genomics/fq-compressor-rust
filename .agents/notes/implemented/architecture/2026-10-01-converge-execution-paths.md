# Agent Note: 压缩/解压执行路径收敛为骨架 + 参数

Status: implemented

## 问题

压缩主流程存在 7 个具体实现：engine 侧 `run_archive` 与 `run_streaming_single/paired/interleaved`，pipeline 侧 `run/run_paired/run_interleaved`——同一"parse → 全局分析/reorder → 建块 → 压缩 → 写出/commit"流程被复制两大套、每套又按拓扑复制三份；`pipeline/compression.rs:189-193` 与 `:467-471` 有逐字相同的 40 行 reorder-map 构造（含同一段注释），是"修一处漏其余"已经发生过的物证（`55da121` 修的长读空归档 bug 正是三处同步修补）。解压有 2 条路径：`commands/decompress.rs` 自带全套编排（serial/parallel/original_order），使 `pipeline/decompression.rs::DecompressionPipeline` 形同虚设；`skip_corrupted`、`--range`、`original_order` 语义在两条路径各写一遍。另有两处行为不一致：`run_paired` 单实例串行（其余拓扑并行）；解压经典路径的 stdout 无缓冲而 pipeline 有 BufWriter。

## 决定

按三阶段收敛，全程行为不变（wire 格式、内存特征、CLI、退出码冻结）：

- **Phase A（解压下沉）**：`commands/decompress.rs` 退化为薄 CLI 层（选项验证 + 分支到 pipeline/engine API + 汇总打印），编排逻辑（`run_parallel`/`process_block`/`run_original_order`）下沉至 `pipeline::decompression`；`DecompressionPipeline` 成为解压编排唯一实现。`split_pe` 与 `original_order` 属于经典路径独有能力，作为下沉编排的两种输出策略随迁，而非塞进三阶段线程管线。
- **Phase B（压缩骨架）**：7 条路径收敛为"一个骨架函数 + 拓扑（Single/Paired/Interleaved）× 执行模式（Archive/Streaming/Pipeline）参数"；逐字重复的 reorder-map 构造归一；streaming 三兄弟的 header/timestamp/stats 样板提取共享；`run_paired` 与其余拓扑同等并行（解码结果与原输入逐字节一致为验收线）。
- **Phase C（拆豁免）**：拆掉 2 处 `#[allow(clippy::too_many_lines)]`（pipeline 两侧的 `run`）与 3 处 `needless_pass_by_value`（engine），`main.rs:7` 的 bin 级 `#![allow(dead_code)]` 通过 bin 复用 lib target 消除或缩到最小范围。

解压两侧行为审计结论（Phase 0，重构依据）：块级 range 预筛（只解码覆盖请求范围的块）仅 pipeline 有，输出语义与经典路径逐条一致（1-based 闭区间，已由 `tests/test_mode_matrix.rs::range_extraction_agrees_between_classic_and_pipeline_paths` 双路径对拍钉死）；`skip_corrupted` 的占位符默认值（`N`）与 id 模式两侧一致；`--pipeline` 在 `original_order`/`split_pe` 时由 CLI 层守卫回退经典路径；pipeline 统计的 `total_bases` 恒为 0（summary 展示缺陷，随下沉一并修正）。经典路径的部分输出防护依赖串行结构 + 事务末端 commit，pipeline 的 "Decompression incomplete" 守卫保留。

相关：[三种执行模式](2026-09-27-three-execution-modes.md)。

## 备选方案

- **维持现状** — 最强论据：测试矩阵厚（259+ 集成测试），分叉风险已被对冲；否决理由：每次修 bug 人工同步 2-7 处的成本持续累积，且逐字复制的注释证明分叉不止一次。
- **引入泛型 trait 框架统一拓扑**（`ReadSource`/`BlockSink` 抽象 + 泛型组装）— 最强论据：类型层面根除复制；否决理由：业余维护项目养不起抽象税，7 个调用点的差异（paired 布局、streaming 逐块 ingest、pipeline 背压）用 enum 参数 + 少量 match 表达更直白。
- **删除 pipeline 执行模式** — 最强论据：少一条路径少一份维护；否决理由：`--pipeline` 是已发布的 CLI 契约（docs/reference/cli.md、README），删除是行为变化，超出本次"行为不变收敛"的范围。

## 后果

- **收益**：bug 修复单点生效；`run_paired` 并行性能与其他拓扑对齐；解压语义只有一份定义；4 个巨型函数随骨架化自然缩减，为拆 clippy 豁免铺路。
- **代价**：骨架函数引入"拓扑 × 模式"参数矩阵，新拓扑需理解骨架的阶段顺序；下沉后 `commands/decompress.rs` 与 pipeline 层的接口是新跨文件契约，需要靠 `tests/test_mode_matrix.rs` 的 11 项对拍长期守护；并行化 `run_paired` 后其块写入顺序依赖有序写路径，若骨架实现有误最坏情况是空归档回归（已由 `test_e2e_pipeline_long_reads_no_reorder_nonempty_archive` 与矩阵测试双重守护）。

## 验证

`tests/test_mode_matrix.rs`（9 格矩阵 + range 双路径对拍 + 经典/pipeline 解压等价）为安全网；四门禁 + `cargo bench --quiet` 无回退；budget 测试原样通过（内存特征冻结）。
