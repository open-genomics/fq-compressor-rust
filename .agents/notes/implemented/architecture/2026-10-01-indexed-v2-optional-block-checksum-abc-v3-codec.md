# Agent Note: indexed v2 可选块校验和与 ABC v3 codec 标识

Status: implemented

## 问题

`harden-indexed-v2-decode-integrity` 变更期间暴露三个正确性缺口：一是块校验和若按"总是重算逻辑记录布局"处理，已冻结的 v2 fixture 将无法通过 `verify`（有损/丢弃块的逻辑布局本就不可复现）；二是畸形布局（流偏移重叠、留缝、与声明载荷不符）会被接受到解码深处才失败；三是 ABC 序列流的无损 v3 载荷没有显式 codec 标识，与历史 v1/v2 载荷无法区分，header 与载荷可能各说各话。

## 决定

- `block_xxhash64` 为**可选**字段：非零时按历史"完全无损逻辑记录布局"计算 XxHash64；丢弃 ID 或采用有损质量模式的块存 0，其结构完整性仍由归档级压缩流校验和覆盖。`checksum_type = 0` 的 XxHash64 语义不变。
- ABC 外层 codec revision nibble 与内部载荷版本对齐：revision 0 读历史 ABC v1/v2 载荷，revision 1 读无损 ABC v3 载荷（序列流 codec 字节 `0x11`）；header 与载荷 revision 矛盾一律拒绝。
- 读者在分配或产出任何记录之前 fail closed：校验标识符与标志合法性、规范连续的流布局、精确计数与 varint 终止、UTF-8 与 sequence/quality 长度一致、reorder map 互为 `0..total_reads` 的逆置换；CLI 在创建输出前拒绝倒置 range；生产 FASTQ 入口校验 Phred+33 字节（底层 parser 对库调用者保持可配置）。

wire 契约：块级逻辑校验和 must 视为可选——无损块 may 写，有损/丢弃块 never 写（存 0）；codec 字节 `0x11` must 只用于 ABC v3 载荷，`0x01`/`0x02` 语义不变；解码器 never 信任未经校验的偏移、计数或映射。

## 备选方案

- **新开归档大版本 + 密码学校验和** — 最强论据：一次性消除所有语义歧义并提供防篡改能力；否决理由：破坏向后兼容且超出工具定位（防意外损坏而非恶意攻击），既有 v2 归档生态会被整体作废。
- **重写冻结 fixture 以适配"块校验和必在"** — 最强论据：单一语义（校验和恒存在）让读路径更简单；否决理由：frozen fixture + SHA-256 就是已发布的 wire 契约（见[块索引格式](2026-09-27-block-indexed-format.md)），重写等于毁约。
- **有损块也强制写校验和** — 最强论据：块级验证覆盖面均匀、无特例；否决理由：丢弃 ID 或有损质量后逻辑布局不可复现，"应该哈希什么"无从定义。

## 后果

- **收益**：冻结 fixture 继续作为回归锚点；有损/丢弃/无损三种块共用同一套校验框架；ABC v3 获得唯一标识，解码器可在读入载荷前拒绝 revision 矛盾；畸形输入在分配内存前即被拒绝。
- **代价**：块校验和从"必有"变为"可选"，`verify` 与解码路径需同时处理两种情形（`src/archive/reader.rs`）；格式文档与能力规格需持续维护这组可选语义。

## 验证

`tests/fixtures/indexed-v2`（SHA-256 冻结）由 `tests/test_format_contract.rs` 钉死；畸形布局拒绝覆盖于 `tests/test_audit_regressions.rs` 与 `tests/test_decode_budget.rs`；ABC revision 分派见 `tests/test_codec_dispatch.rs`。实现提交 `21308f0`；出处为 openspec 变更 `harden-indexed-v2-decode-integrity`（2026-10-01 归档），解码侧资源上限另见[解码资源预算](2026-09-27-decode-resource-budget.md)。
