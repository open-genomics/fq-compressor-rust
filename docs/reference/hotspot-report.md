# 热点测量报告（阶段计时）

> 历史说明：下表是 2026-08 的测量，当时默认 `--reorder true`。2026-11 起默认改为 `--reorder auto`，并加入按实际 codec worker 上限/碱基预算的自适应分块与 Fenwick 质量模型；新默认在有序真实短读上会自动跳过重排。本文保留原始对照数据，不把旧默认当成当前行为。

> 范围：`docs/explanation/architecture/performance-roadmap.md` 中"热点测量"切片（2026-08-18 起，7d）的收口。
>
> **2026-10-26 说明**：长读 archive/pipeline 已改为默认自动分块；下表中的 ONT 单 block
> 与吞吐是改动前的历史测量，保留用于对照，不代表当前默认。
> 环境：WSL2（Linux 6.18 / 12 核），release 构建，`--threads 8`，默认参数。机器为共享环境、测量时存在并发任务，
> 墙钟波动约 ±20–30%；结论只看量级与相对关系，不把绝对值当基准。语料见 `docs/reference/real-corpus.md`（与 C++ 侧同一切片）。

## 阶段计时（单位 ms）

压缩各阶段含义：`parse`（读入 + 长度分类）、`reorder`（全局分析 + 重排 + 建块）、`process`（并行 block 压缩，聚合 CPU 时间）、`write`（顺序写盘 + reorder map + finalize）。解压：`parse`（读块）、`process`（解压，聚合 CPU）、`write`（写出）。

| 语料 | 模式 | 总耗时 s | parse | reorder | process | write | 备注 |
|---|---:|---:|---:|---:|---:|---|
| Illumina archive（默认 reorder） | 压缩 | 11.05 | 140 | **8838** | 3841 | 37 | reorder 占 ≈80% |
| Illumina archive（`--reorder false`） | 压缩 | 1.93 | 134 | 62 | 3290 | 15 | 见「关键发现 1」 |
| Illumina pipeline（reorder on） | 压缩 | 11.19 | 133 | **8954** | 3362 | 2085 | pipeline 的 write 含等待信道 |
| Illumina streaming | 压缩 | 3.75 | 120 | 62 | 2447 | — | 无全局重排 |
| ONT archive | 压缩 | 9.62 | 181 | 76 | **9216** | 144 | 单 block，process≈墙钟 |
| ONT pipeline（已修复） | 压缩 | 9.56 | 151 | 74 | 8808 | 9138 | 单 block，write 含信道等待 |
| ONT streaming | 压缩 | 9.33 | — | — | 9954 | — | 单 block |
| Illumina archive | 解压 | 2.08 | 8 | — | 1923 | 98 | |
| ONT archive | 解压 | 10.02 | 66 | — | 9842 | 93 | 单 block 串行 |

## 关键发现

### 1. 全局重排在真实 Illumina WXS 上是**纯负收益**：更慢 + 更大

同一 56.6 MB 切片，唯一差别是重排开关：

| 参数 | 总耗时 | 磁盘文件 | 磁盘压缩比 |
|---|---:|---:|---:|
| `--reorder true`（默认） | 11.05 s | 14,036,446 B | 4.03× |
| `--reorder false` | 1.93 s | 12,508,029 B | 4.52× |

- 慢 **5.7×**（reorder 阶段占 ≈8.8s/11s，即 80%），输出反而大 **+1.53 MB（+12.2%）**。
- 机制：Illumina WXS 的 FASTQ 原序已是按基因组位置的**空间局部排序**（相邻读段重叠）；minimizer 贪心重排
  破坏了这份局部性，再按 hash 相似度重排后块内冗余反而变差。`--streaming`（不重排）输出与
  `--reorder false` 几乎逐字节一致（12,507,647 vs 12,508,029，仅头部差异），印证 streaming == 无重排归档。
- 结论：对真实短读，重排是净亏。**下一步路线图切片应是「自适应/默认关闭重排」**（先做离线判定：
  采样算重叠率，只在输入非空间有序时启用）。这不是本切片顺手优化能解决的算法级问题。

### 2. 长读压缩曾为单 block 串行

ONT 切片（4000 条长读，65.36 Mbp）自动归为 Long，旧默认 `--max-block-bases 0` 时整份只有 1 个 block，
`process` 聚合 CPU ≈ 墙钟（9216ms vs 9.62s），8 线程只用了 1 个。2026-10-26 起 archive/pipeline
在默认情况下按 5 Mbp 自动切块；下表保留旧测量作为历史基线，当前默认块数与吞吐应重新测量。
streaming 不自动改块，显式 `--max-block-bases <N>` 仍可覆盖 archive/pipeline 的自动策略。

### 3. pipeline 的 `write_ms` 含信道等待

pipeline 的 writer 线程在 `write_ms` 内包含阻塞收信道的等待时间：单 block 时 `write_ms` ≈ `process_ms`
（ONT 9138 vs 8808；Illumina 2085 vs 3362），多 block 时被真实写盘稀释。报告时不要把它当纯 I/O 时间。

### 4. 顺手优化（Task 6）：重排/建块改移动

`reorder_ms` 占 Illumina archive 80%，满足决策门（≥20%）→ 执行移动优化：
- `run_archive` / pipeline `run()` 的建块从逐条 `clone()` 改为 `std::mem::take` 移动（pipeline 切块改为
  `split_off`），块内容与顺序不变，压缩输出**逐字节不变**（reorder true: 14,036,446 B；reorder false: 12,508,029 B，与优化前一致）。
- 实测影响：建块克隆不是 reorder 阶段的大头（大头是 `analyze()` 本身），墙钟差异在并发负载噪声内；
  收益是**内存/分配减少**（省掉 200k 条读的整份克隆）。
- 结论：正确且无害，但未触及真实热点。真实热点见发现 1/2，留给后续算法切片。

## 对路线图的回写

- "热点测量"切片完成：旧热点已定位（短读 = reorder/analyze；长读 = 单 block 串行）；长读默认分块已随后修复。
- 已完成：重排自适应、长读 archive/pipeline 自动切块（5 Mbp）。下一刀建议：bounded streaming pipeline 与真实 WXS/ONT 默认参数复测。

## 2026-11 优化后复测（合成数据，机制验证）

> 100,000 条 150 bp、约 33 MB 的合成短读；release 构建；本机 8 线程。合成数据只用于验证机制，绝对值不代表真实 WXS。

| 配置 | 墙钟 | 归档大小 | 峰值 RSS | 说明 |
|---|---:|---:|---:|---|
| 旧默认（reorder on，1 block） | 2.75 s | 9.06 MB | 222 MB | 重排慢且更大 |
| 扁平模型 + worker 规划后默认（8 blocks） | 0.39 s | 6.93 MB | 171 MB | auto 跳过重排；块数跟随实际 worker 上限 |
| 上一版默认（reorder auto，10 blocks，Fenwick per-context） | 0.42 s | 6.94 MB | 187 MB | 保留用于机制对照 |
| 新默认 + `--threads 8`（10 blocks） | 0.32 s | 6.94 MB | 229 MB | 显式线程覆盖自动上限；更多 block |
| 新默认 + `--memory-limit 256`（8 blocks） | 0.37 s | 6.93 MB | 171 MB | worker/块规划按内存收紧 |
| 打乱输入，reorder auto | 1.88 s | 9.20 MB | 271 MB | 检测到无序后执行重排 |
| 打乱输入，`--reorder false` | 0.45 s | 10.70 MB | 189 MB | 不重排更快，但归档大 16% |

解压同一切片：最新默认约 0.42 s / 175 MB；`--threads 8` 约 0.32 s / 277 MB；`--memory-limit 256` 约 0.33 s / 221 MB。上一版默认为约 0.48 s / 255 MB（旧默认约 1.69 s / 178 MB）。

质量模型从 `u32` 频率 + cumulative 改为 `u16` Fenwick，再把约 7 万个上下文各自的 `Vec` 合并为连续 `trees`/`totals`。固定质量样本的压缩流 xxHash64 为 `839fc7efbf59b7d3`，改造前后一致；`--reorder auto` 的归档变化来自块边界，而不是质量编码字节变化。

`--reorder auto` 还要求采样读段存在全局共享 k-mer；独立随机读段即使相邻重叠为 0 也会跳过重排，避免触发无收益的重排路径。
