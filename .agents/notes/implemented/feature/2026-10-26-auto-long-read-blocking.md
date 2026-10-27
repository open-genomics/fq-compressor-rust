# Agent Note: 长读 archive/pipeline 自动分块

Status: implemented

## Problem

长读默认以 10,000 reads/block 起始；对常见的 ONT 数据，这往往意味着整份
输入只有一个 block。2026-10-26 的本机合成长读（4,000 × 10 kb，约 40 Mbp；10,000 bp 在现有分类中
仍是 Medium，性能对照用 `--long-read-mode long` 强制进入 Long）上，默认 archive
压缩 11.13s，8 个线程实际只用了 1 个 block，解压 10.45s；
长读的 `--max-block-bases` 虽然能改善，但要求用户知道并手动设置，默认路径
没有利用多核。

同一轮测试里，把块限制到 5M bases（8 blocks）后压缩降到 2.13s、解压降到
2.52s，磁盘压缩比从 2.445× 变为 2.422×；2.5M bases（16 blocks）反而不更快
（2.27s）且压缩比更差（2.398×）。短读不能沿用这个方案：ABC 在 4,096 reads
块上实测 87.5s，且每 read 输出不比 Zstd 好，属于错误方向。

## Decision

当 `--max-block-bases 0`、读长分类为 Long、模式为 archive 或 pipeline 时，
自动按 `DEFAULT_LONG_READ_BLOCK_BASES = 5_000_000` bases/block 限制块大小。
自动限制以观测到的平均读长计算，避免单个超长 outlier 把块压得过小。

边界保持明确：

- 显式 `--max-block-bases N` 仍优先，并保留历史上的 max-length 计算；
- Short 路径完全不变；Medium 不自动改；
- streaming 不自动改（显式 `--max-block-bases` 仍生效），避免改变低内存单遍
  路径的块边界；
- 归档格式不变，只改变块数量和默认输出字节布局。

## Alternatives considered

- **维持单 block（现状）** — 最低改动，但无法利用当前 21 核机器的并行压缩/
  解压；否决。
- **2.5M bases（16 blocks）** — 理论并行度更高；实测 2.27s，比 5M/8 blocks 的
  2.13s 更慢，压缩比也更差；否决为默认。
- **10M/20M bases** — 分别 4/2 blocks，压缩 3.21s/5.88s，仍明显慢于 8 blocks；
  保留为显式调参空间，不作为默认。
- **把短读也缩小到 ABC 可用的 ≤4096 块** — 会让 ABC 的 O(n²) 路径进入生产
  默认；4,096 reads 实测 87.5s，且没有净压缩收益；否决。
- **按线程数动态计算 block 上限** — 更“自适应”，但缺少稳定的真实语料校准，
  且会让同一输入在不同线程数下改变归档布局；先固定 5M bases，待真实语料
  复测后再评估。

## Consequences

- **收益**：本机合成长读上 archive 压缩约 5.2×、解压约 4.3× 加速；改动用
  既有 `--max-block-bases` 语义即可回退。
- **代价**：长读默认归档会多出 block 头和索引项，压缩比有约 1% 量级的损失；
  `docs/reference/real-corpus.md` 与 `hotspot-report.md` 中历史上的 ONT 单
  block 数字不再代表当前默认，只保留为旧配置证据。
- **风险控制**：显式 `--max-block-bases`、Short/Medium、streaming 三个边界
  都有测试或代码路径约束；格式版本不升级。

## Verification

新增 E2E 测试覆盖 600 × 12 kb 长读在 archive 与 pipeline 下的自动多块输出；
既有的显式 `--max-block-bases` 测试继续验证用户覆盖不被吞掉。另用本机
release 二进制复测 4,000 × 10 kb 的 1/2/4/8/16 block 对照、archive 与
pipeline round-trip、verify 和 `cmp`；结果见交付说明。
