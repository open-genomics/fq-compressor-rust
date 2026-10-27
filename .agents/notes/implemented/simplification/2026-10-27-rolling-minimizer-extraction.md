# Agent Note: 滚动哈希与单调队列加速 minimizer 提取

Status: implemented

## 问题

`extract_minimizers` 原先对每个 k-mer 调用一次 O(k) 的正反向哈希计算，随后对每个
窗口做 O(w) 的线性最小值扫描。默认参数 `k=15, w=10` 下，100k 条 150 bp 短读在
aarch64 Kunpeng 920 上的隔离微基准约 3.07 s；在重排已完成一次重复提取消除后，
它仍是可测量的标量热点。

## 决策

- 对 `k <= 32` 使用 2-bit 滚动正链哈希和反向互补哈希，每个后续 k-mer 只做常数
  次移位/掩码；
- 用单调队列维护滑动窗口最小值，保持旧实现“相等时取最左位置”的 tie-break；
- `k > 32` 保留原线性实现，因为 2-bit 状态会超过 `u64`，且旧实现对超出 64 位的
  截断语义需要原样保留；
- `k=0`、`w=0`、`seq.len() < k` 的行为不变。

窗口输出只包含 hash，不包含位置；回归测试直接把新实现与旧线性实现逐项对比，
覆盖 `k=1..40`、多个窗口大小、歧义字符和小写输入。

## 备选方案

- **继续使用线性窗口扫描** — 实现简单，但隔离微基准约 3.07 s，滚动加单调队列约
  0.27 s；不保留明显的热路径浪费。
- **用 NEON/SVE intrinsics** — 对这个小窗口操作收益不足以抵消 `unsafe` 边界；
  项目又硬性禁止新增 `unsafe`，否决。
- **用 `u128` 支持任意更长的 k** — 会增加状态宽度和复杂度，而且实际配置是
  `k=15`；对 `k > 32` 使用旧路径更能保证历史行为，否决。
- **改为哈希集合/minimizer sketch** — 会改变 reorder map 和压缩输出，超出本次
  性能切片的边界，否决。

## 后果

- 隔离微基准：100k × 150 bp，`extract_minimizers(k=15,w=10)` 约
  3.07 s → 0.27 s（约 11×，重复 3 轮稳定）；哈希总和完全一致。
- 端到端重排仍约 5.4 s，因为并行压缩和贪心排序/桶查询占剩余主导；本次不改变
  重排算法或输出布局，也不让 reorder 在空间有序 Illumina 数据上变成正收益。
- 归档 payload 除全局 timestamp 外与旧实现一致；解压后 FASTQ 与
  `fqc verify` 均通过。
- 没有引入架构专用指令，x86_64 与 aarch64 共用同一安全实现。

## 验证

- 单元测试 `rolling_minimizers_match_legacy_scan` 覆盖 `k=1..40` 和多个
  `len/w` 组合；`rolling_minimizers_preserve_ambiguous_base_mapping` 覆盖
  `N`、小写和 `k=32/33` 边界。
- 本机 aarch64 release 微基准：旧实现 3.065–3.067 s，新实现 0.273 s（3 轮）。
- 同一 100k × 150 bp 输入重排端到端约 5.4 s；关闭重排约 3.8 s；长读保持约
  1.18 s；解压内容、payload 和校验结果均一致。
- 最终 `cargo build --release --locked`、`cargo clippy --all-targets --locked
  -- -D warnings`、`cargo test --lib --tests --locked`、`cargo fmt --all --
  --check`、`cargo doc --no-deps --locked` 通过。
