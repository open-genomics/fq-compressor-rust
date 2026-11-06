# Agent Note: 用长度直方图替代全量长度向量

Status: implemented

## 问题

`--scan-all-lengths` 的语义是“检查完整输入”，但实现会先把每条读段长度放进
`Vec<usize>`，再复制一份排序：

- archive/pipeline 的 `length_stats_from_records_sampled(scan_all=true)`；
- streaming 的 `inspect_input_lengths_for_streaming(scan_all=true)`。

在 1 亿条读段上，单是长度向量就约 800 MB（64 位），排序副本再翻一倍；这与
streaming 的低内存目标直接冲突。即使默认采样只保留 4,096 条，`scan_all_lengths`
仍是最坏路径。

## 决定

`LengthStats` 增加直方图构造：

- `from_lengths_iter(impl IntoIterator<Item = usize>)`：逐条累加
  `BTreeMap<usize, u64>`；
- `from_length_counts(&BTreeMap<usize, u64>)`：从已有计数计算
  `sample_size` / `avg_length` / `median_length` / `max_length`；
- `from_lengths(&[usize])` 改为委托 `from_lengths_iter`，保持公共行为不变。

中位数仍取排序后的下标 `N / 2`（偶数个取靠下的中间值），通过累计计数
`> N / 2` 定位，和旧实现逐值一致。`length_stats_from_records_sampled` 的全量
分支改为迭代器；streaming 扫描改为直接累加计数。

内存从 `O(reads)` 降为 `O(distinct lengths)`。常见 FASTQ 的不同读长通常只有
个位数到几十个；即使长度种类很多，也不会比“全量 Vec + 排序副本”更差。

## 备选方案

- **只保留采样、不真正全量扫描**：违背 `--scan-all-lengths` 的用户预期；
- **原地排序 `Vec<usize>`**：仍需要 O(reads) 内存，只是去掉副本；
- **近似中位数/蓄水池抽样**：会改变分类结果，不能作为“全量扫描”的实现；
- **固定上限的计数数组（如 1M 桶）**：读长无硬上限，越界处理会引入新的截断语义；
  `BTreeMap` 更简单且无上限。

## 后果

- `--scan-all-lengths` 不再随读段数线性分配长度向量；
- 分类结果与旧实现逐值一致（有单元测试对拍）；
- 时间从 O(n log n) 排序降为 O(n log d)，d 为不同长度数；
- 不改变 `.fqc` 格式、CLI 参数或默认采样策略。

## 验证

- `tests/test_types.rs::
  length_stats_slice_and_histogram_agree`：空、单值、奇偶、重复、零长度等用例，
  `from_lengths` 与 `from_lengths_iter` 四个字段逐项相等；
- `tests/test_types.rs::
  length_stats_histogram_median_uses_lower_middle_for_even_counts`：偶数个读段
  取靠下的中间值；
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --locked -- -D warnings`
- `cargo test --lib --tests --locked`
- `cargo doc --no-deps --locked`
- `cargo +1.75.0 check --all-targets --locked`
- `npm run verify-notes`
