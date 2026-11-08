# Agent Note: 读长分类上限与解压预算统一

Status: implemented

## Problem

解压侧的预算决策（codec worker 数、`--original-order` 峰值估算）建立在
「归档属于哪个读长分类」上，而分类只是全局头 flags 里的 2 个比特：

- `memory_estimate_read_length()` 对 `Long` 返回
  `LONG_READ_MEMORY_ESTIMATE_BP = 100_000`——注释自己写着这是「经验值，不是
  硬上限」。真实 ONT 超长读可以到 2.5 Mbp，此时 `--original-order` 的峰值
  估算低估 20 倍以上。
- 块头声明的 `uniform_read_length` 与分类之间没有任何一致性校验：伪造归档
  可以声明 Short 分类（小上限）而块头写 2000 bp，估算因此小得多。
- 于是「不可信的分类」直接决定了内存安全相关的估算。

## Decision

把「分类上限」变成 decode 真正执行的约束，让所有由分类推导的估算都成为上界：

1. `ReadLengthClass::max_read_length()`：Short = `SPRING_MAX_READ_LENGTH`(511)、
   Medium = `LONG_READ_THRESHOLD - 1`(10239)、Long = 新增
   `MAX_LONG_READ_LENGTH`(4 MiB)。前两个与 `classify_read_length` 的判定一致，
   是真实上限；Long 用 4 MiB 这个高于任何真实读长的硬上限。
2. `validate_block_headers` 在打开归档时拒绝 `uniform_read_length` 超过分类上限
   的块头。
3. `BlockCompressorConfig::max_read_length_bytes`（默认 `None`）由解压与 verify
   路径从分类上限填充；解码器在解出 `lengths` 后逐条校验，超过即格式错误。
   默认 `None` 保持库/测试里「显式指定分类解码」的历史行为（例如
   `repro_5_forced_short_class_long_reads` 用 Short 分类解 2000 bp 块）。
4. `memory_estimate_read_length()` 直接返回 `max_read_length()`，删掉
   `Long` 的「经验值」分支；`LONG_READ_MEMORY_ESTIMATE_BP` 常量随之删除。

## Alternatives considered

- **保留 100 kbp 经验值，只在 `--original-order` 里加运行时累计检查**：
  需要在 decode 循环里逐条记账，改动面更大，而且「先检查后输出」的既有契约
  （`original_order_peak_rejected_before_output`）会退化成运行中失败。
- **按块压缩字节 × 64倍 估算混合长度块**：与 decode 的 zstd 上限一致、绝对
  安全，但 64 倍把几乎所有长读归档都拒之门外，实际不可用。
- **不引入 4 MiB 上限，只在块头校验分类一致性**：混合长度块（ONT 常态）
  仍然没有上界，问题只解决一半。
- **在 `decompress_block` 里无条件按分类上限校验**：会让「显式用 Short 分类
  解码长读」的既有测试和库用法失效，故改为配置项，由归档路径开启。

## Consequences

- 伪造分类不再能压低预算估算：三类上限都真实执行。
- 单条读长 > 4 MiB 的归档无法解码。真实数据（最长 ONT 约 2.5 Mbp）远低于该值，
  但这是一条新的硬性拒绝边界，已写入 CHANGELOG。
- `--original-order` 对 Long 归档的估算从「100 kbp/read」变成「4 MiB/read」，
  更保守，可能拒绝此前能通过的运行；失败信息仍指向 `ResourceLimit`，用户可提高
  `--memory-limit` 或改用默认顺序输出。
- 库层（`BlockCompressorConfig::default()`）行为不变，只有归档解压/verify
  开启上限。

## Verification

- `cargo test --quiet < /dev/null`：34 个测试目标全部通过。
- 新增 `test_decode_budget::forged_short_class_with_long_reads_is_rejected`：
  2000 bp 归档正常打开（估算取块头 2000），把 flags 的分类位改成 Short 后
  `FqcReader::open` 返回 `Format("... exceeds the short class limit 511")`。
- `test_types::memory_estimates_use_class_upper_bounds` 断言三个分类的
  `memory_estimate_read_length() == max_read_length()`。
- `cargo clippy --all-targets -- -D warnings`、`cargo fmt --all -- --check` 通过。
