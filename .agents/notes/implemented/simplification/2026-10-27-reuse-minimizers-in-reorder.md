# Agent Note: 重排阶段复用已提取的 minimizer

Status: implemented

## 问题

`GlobalAnalyzer::perform_reordering` 先在第一步并行提取每条 read 的 minimizer，
并在 `all_buckets` 中保存；进入贪心排序后，又对当前 read 调用一次
`extract_minimizers`。同一份标量 k-mer 计算因此做了两遍，在 aarch64
Kunpeng 920 的短读重排路径上是可直接消除的重复工作。

## 决策

排序阶段直接读取 `all_buckets[last_read]`，只保留读长用于长度差评分；删除第二次
`extract_minimizers`。minimizer 顺序、候选遍历顺序、tie-break 和最终 reorder map
均不变。

## 备选方案

- **同时把 k-mer hash 改成滚动实现** — 本次先做局部、可证明等价的重复工作
  消除；滚动哈希与单调队列随后作为独立切片完成，见
  `2026-10-27-rolling-minimizer-extraction.md`。
- **用 `-C target-cpu=tsv110` 或 NEON intrinsics 加速** — 在同机 A/B 中
  `target-cpu=tsv110` 对短读、长读和重排均无稳定收益，且会牺牲二进制可移植性；
  项目又禁止新增 `unsafe`，所以不以架构专用指令作为首选。
- **保持现状** — 实现最简单，但把已经算出的结果丢弃后重复计算，属于不应保留的
  ARM 热路径成本。

## 后果

- 100k × 150 bp 合成短读、8 线程、`--reorder true`：重排墙钟约 5.90–5.93 s
  降至约 5.49–5.56 s（约 7%）；RSS 约 257 MiB 不变。共享机器负载下只应看量级。
- 关闭重排的短读和长读路径没有行为变化；重排开启时归档 payload 与旧实现逐字节
  一致，唯一差异是全局头部的压缩时间戳。
- 该优化不引入架构专用代码，x86_64 与 aarch64 共用同一实现。

## 验证

- 与优化前的通用 aarch64 release 二进制对同一输入比较：解压后 FASTQ SHA-256
  完全一致，`fqc verify` 通过；排除 8 字节 header timestamp 后归档字节一致。
- `cargo test --lib --tests --locked`、`cargo clippy --all-targets --locked
  -- -D warnings`、`cargo fmt --all -- --check`、`cargo doc --no-deps --locked`
  通过（详见本轮交付记录）。
