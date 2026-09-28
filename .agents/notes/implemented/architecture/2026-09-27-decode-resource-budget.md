# Agent Note: 解压与 verify 在操作级 DecodeBudget 下运行

Status: implemented

## 问题

`--memory-limit` 原本只约束压缩侧。归档 reader 按攻击者可控的字段分配内存（`num_blocks`、各 stream size、reorder map），且 zstd 解压走无界的 `decode_all`——损坏或恶意构造的归档可以让 `decompress`/`verify` 耗尽内存。不做处理意味着任何能被喂给 `fqc decompress` 的文件都是潜在的 OOM 向量。

## 决定

`decompress`/`verify` 的每次操作在一个显式的 `DecodeBudget` 下运行，无全局可变状态。`--memory-limit` 为 0 时解析为可用内存约 75%，钳制在 `[MIN_DECODE_MEMORY_MB, HARD_MAX_DECODE_MEMORY_MB]`。预算经 `FqcReader::open_with_budget` 传入并约束：读总数与索引 `num_blocks`、文件区容量、各流分配（`check_alloc` + checked `u64 → usize`）、reorder map 的压缩态与解压态上限、单 block 解压峰值（`compressed_size * 3`）。流 codec 与 reorder map 的 `zstd::stream::decode_all` 一律由 `zstd_decompress_bounded(max_out)` 替代。original-order 输出前先 `check_original_order_peak`；parallel/pipeline 的批大小为 `min(threads*2, budget_batches)` 且绝不为 0，单 block 都放不下即报 `ResourceLimit`。`verify` 与 `decompress` 共用同一 resolve 路径；`--quick` 跳过 block 解码但仍在此预算内 open。

出处：openspec 归档变更 `enforce-decode-resource-budget`（`FQCR-LIMIT-001`，2026-08-18 归档）。

## 备选方案

- **全局可变预算** — 实现最省：不用把预算穿进 open/read/reorder/decode 整条调用链；否决理由：设计明确禁止全局可变状态，操作级预算才能让并发调用互不干扰、测试可直接注入不同预算。
- **精确 allocator 记账** — 字节级精确，没有启发式误差；否决理由：当时即列为 non-goal，复杂度远超收益——结构性上限（先拒绝、后分配）已能挡住 OOM 类输入。
- **以 fuzz harness 作主防线** — 能覆盖更多畸形输入形态；否决理由：同样列为 non-goal，结构性拒绝先于任何分配发生，比依赖 fuzz 事后发现更直接。

## 后果

- **收益**：恶意或损坏归档无法令解压侧 OOM；`verify` 与 `decompress` 共享同一套预算语义，行为一致。
- **代价与已知上限**：默认预算是启发式（可用内存 75% 钳制 + 峰值系数 3），极端但合法的输入可能被误拒。重访信号：合法归档遭 `ResourceLimit` 误拒的报告出现时，重估钳制区间与峰值系数，而不是绕过预算。

## 验证

`src/memory_budget.rs` 定义预算类型与钳制；`src/archive/reader.rs` 的 `open_with_budget` 是入口；`ResourceLimit` 错误在 `src/error.rs`。
