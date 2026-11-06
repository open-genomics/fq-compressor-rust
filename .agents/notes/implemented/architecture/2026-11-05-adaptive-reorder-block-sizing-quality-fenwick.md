# Agent Note: 自适应重排、块粒度与质量模型紧凑化

Status: implemented

## 问题

2026-11 的开发审计发现五个相互放大的问题：

1. `--reorder` 默认恒为 `true`。仓库已有真实 Illumina WXS 测量显示空间有序输入上重排慢 5.7×、输出大 12.2%；但打乱输入上重排仍有约 14% 压缩收益，因此不能简单改成 `false`。
2. 块大小只按读长类别和固定读段数决定。默认短读 100k 条、长读 10k 条；长读 10k 条可达 160 Mbp，实际只有 1 个 block。200k 短读默认只有 2 个 block，8 线程压缩仅约 1.9×。
3. `--max-block-bases` 用 `max_length` 估算，且只对非 Short 生效；一条长离群读段会把块压得极小，短读则完全无法按碱基切块。
4. order-2 质量模型为 94²×8 个上下文，每上下文两套 `u32` 数组，约 57 MB/worker，更新是 O(94)。`--memory-limit 256` 实测压缩/解压 RSS 可到 370/410 MB。
5. ABC 在 4,096 条上限附近是 O(n²) 性能悬崖：4,000 条合成短读约 25.9 s，而 4,097 条走 Zstd 约 0.13 s，且该样本上 ABC 没有压缩收益。默认 100k 块又让 ABC 在生产路径上几乎不可达。

另外发现 classic 解压把 `num_threads=0` 直接传给 `parallel_batch_size`，后者按 `max(1)` 处理，导致自动线程模式下批大小退化为 1，实际串行。

## 决定

- `--reorder` 改为 `auto|true|false`，默认 `auto`。`auto` 采样 8 个连续窗口、每窗口最多 256 条读段，同时计算相邻读段共享 canonical 15-mer 的比例和采样读段的全局共享比例：相邻占比 ≥25% 时视为已空间有序并跳过；相邻低但全局 ≥25% 时视为无序但有重排冗余；两者都低时跳过重排。显式 `true`/`false` 不跑检测。
- 新增 `adaptive_block_size`：目标块数为实际 codec worker 上限的 `workers × 2`（clamp 4..=64），再套类别最小读段数（Short 10k、Medium 2k、Long 500）和碱基上限（Long 默认 5 Mbp，其他类别 8 Mbp；streaming 不套自动上限）；`--max-block-bases` 扩展到所有类别并按平均长度估算；新增 `--block-reads` 显式覆盖，并在 `CompressCommand` CLI 入口拦截超过 `1,000,000` 的显式块；archive/streaming 保留小块能力，`--pipeline` 仍要求至少 `100`。
- pipeline 在读完全部 reads 后重新按实际 total reads/bases 计算块大小；archive 直接使用精确总量；streaming 用采样长度估算。
- ABC 上限从 4,096 降到 512。保留 codec 与小文件路径，待 minimizer seeding 替换 O(n²) contig 扫描后再扩大。
- 质量上下文模型改为扁平 `u16` Fenwick 树：所有上下文的树和总频数分别存放在两个连续数组中，消除约 7 万次 per-context `Vec` 分配；保留完全相同的频率更新与 rescale 顺序，编码输出逐字节一致；模型从约 57 MB 降到约 14 MB，更新/查询从 O(94) 降到 O(log 94)。
- `BlockCompressorConfig::estimated_worker_bytes` 估算质量模型、记录与 Zstd 缓冲；archive/pipeline/decompress 按 `--memory-limit` 收紧 worker。自动线程且未显式给内存时默认最多 4 个 worker；显式 `--threads` 或 `--memory-limit` 可覆盖。archive 与 pipeline 使用 `plan_compression_layout` 在 worker 上限和块大小之间迭代到稳定点，避免按请求线程数生成过多 block。
- 解压优先使用所有 block 共有的 `uniform_read_length`；只有每个 block 都声明同一个非零长度且没有 auxiliary length stream 时才返回精确值，混合长度、任一 block 为 0 或 header 与 aux 流矛盾时按 `ReadLengthClass` 回退到上界（Short 511、Medium 10,240、Long 100,000 的实用上界）。worker 预算保留 1/3 给 reader/writer、线程栈和 allocator，移除固定 256 bp 的长读低估。
- classic 解压改用 `effective_threads()`，修复 `--threads 0` 退化为串行批大小的问题。
- 重排无候选时的 fallback 从 `(0..total_reads).find` 改为 `BTreeSet` 维护未使用读段，最坏复杂度从 O(n²) 降到 O(n log n)。

## 备选方案

- **直接把默认改成 `--reorder false`**：最强论据是真实有序 WXS 上更快更小；否决理由是打乱输入会损失约 14% 压缩比，且 `auto` 已能用很低成本区分两者。
- **只按线程数把短读块降到 4k 以启用 ABC**：最强论据是复用现成 ABC；否决理由是 4k ABC 实测 25.9 s 且无收益，必须先做 seeding。
- **保持 `u32` 两数组，只减少 worker**：实现最小；否决理由是内存仍约 57 MB/worker，且每符号 O(94) 更新继续主导质量编码 CPU。
- **本轮直接上每块 alphabet + ScmV2**：压缩比/内存还有收益；否决理由是涉及新 codec revision 与 frozen fixture，先交付不改变 wire 语义的 Fenwick 更稳。
- **用静态位置 rANS 替换 SCM**：解码潜力更大；否决理由是格式与模型设计面更大，应先用真实语料验证 order-1/order-2 的收益边界。

## 后果

- 默认有序短读不再写 reorder map；`--original-order` 对这类新归档不适用（输入顺序本身已保留）。文档已注明。
- 默认块数增加，归档 block index 变大但单块更小；range extraction 粒度同步改善。
- 自动线程模式默认最多 4 个 codec worker，是质量模型仍占内存时的保护；显式 `--threads`/`--memory-limit` 可恢复更高并行。
- archive wire format、magic、codec 字节未变；Fenwick 只改内存表示，质量流字节保持兼容。
- `auto` 的全局共享信号尚未掩码 adapter/低复杂度高频 15-mer；真实语料校准前，阈值与启发式仍应视为可调整策略。

## 验证

- `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --lib --tests`、`cargo doc --no-deps` 通过。
- 新增 `adaptive_block_size` / `adaptive_block_size_with_policy`、worker/block 规划、读长类别内存上界、uniform/variable/mixed-uniform 归档估算、质量流小样本与 rescale hash、`auto_reorder` 有序/打乱/独立随机测试，以及无共享 k-mer 的 fallback permutation 测试；streaming 不自动切长读的策略有显式单元测试。
- 新增 `CompressCommand` 超大 `--block-reads` 拒绝测试；`--pipeline` 的最小 100 条/块约束仍由 pipeline 自身校验。
- 100k×150bp 合成短读：旧默认 2.75 s / 9.06 MB / 222 MB → 新默认 0.42 s / 6.94 MB / 187 MB；解压 1.69 s → 0.48 s。
- 质量上下文扁平化前后，固定质量样本的压缩流 xxHash64 均为 `839fc7efbf59b7d3`；测试断言防止字节漂移。
- worker/block 规划把同一 100k×150bp 合成片段的默认块数从 10 降到 8；归档 6,927,470 B、压缩 0.37 s / 170,840 KB、解压 0.32 s / 173,988 KB，解压后与原 FASTQ `cmp` 逐字节一致。archive 与 `--pipeline` 本轮都生成 8 blocks、6,927,470 B；最终复测只有 header 时间戳字节（offset 60）不同，其余字节一致。块边界变化会改变归档大小，不能用归档哈希替代质量流哈希。
- 打乱输入：`auto` 输出 9.20 MB，`--reorder false` 10.70 MB，确认 auto 仍保留重排收益。
- 20k 独立随机 150 bp：旧 auto 0.60 s / 961,921 B，`--reorder false` 0.08 s / 938,679 B；修正后 auto 0.12 s / 938,679 B，全局共享仅 5.4%，确认不再误触发重排。
- 2,000×20kb 合成长读：5 blocks、80,043 B；压缩 0.31 s / 165,760 KB，解压 0.34 s / 161,360 KB。解压 `--memory-limit 64/96/128` 均为约 36.9 MB RSS（0.66–0.72 s），`160/192` 为约 128.4 MB（0.48–0.51 s）；旧实现 128 MB 限制曾约 135,848 KB，现已回到限制内。
- 20k×250bp 短读 `--original-order` 在 16/32/64/96/128 MB 限制下均逐字节一致，约 0.11–0.12 s、45 MB RSS；这暴露的是进程基线和软预算边界，不是 uniform length 估算错误。
- 20k 打乱有序短读：相邻重叠 0.049%、全局共享 75.0%；auto 输出 1,815,151 B，`--reorder false` 2,112,699 B，保留约 14% 压缩收益。
- ABC：4k 输入现在走 Zstd；ABC 仅保留 ≤512 条块，原有 ABC 单元/回归测试继续通过。
