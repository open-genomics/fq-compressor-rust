# Verification: harden-indexed-v2-decode-integrity

## Metadata

- Verification status: `Verified — implemented in 21308f0; gates re-run at archive time`
- Implementation HEAD: `21308f0`（fix(format): 加固 indexed v2 解码完整性）
- Verifier: closure session（2026-10-01，非实现会话；独立复核）
- Verified at: `2026-10-01`
- Ready to archive: `yes`

## Evidence

实现于提交 `21308f0` 落地（40 个文件，+2015/-559），但当时未回填本文件与 tasks。2026-10-01 收口会话逐项复核：

| 项 | 证据 | 结果 |
|---|---|---|
| 1.1 冻结校验和行为 | `tests/fixtures/indexed-v2` + `tests/test_format_contract.rs`（SHA-256 冻结）随 21308f0 更新并全绿 | pass |
| 1.2 有损/丢弃省略校验和 | `tests/test_roundtrip.rs`、`tests/test_format.rs` 覆盖 lossy/discard 块校验和为 0 的路径 | pass |
| 1.3 ABC codec revision | `tests/test_codec_dispatch.rs`、`tests/test_algo.rs`（0x11 / revision 0 兼容） | pass |
| 2.1 标识符/标志/布局/映射校验 | `tests/test_audit_regressions.rs`（333 行新增回归） | pass |
| 2.2 解码器 fail closed | `tests/test_decode_budget.rs`（forged header 拒绝）等 | pass |
| 2.3 计数/长度/range 拒绝 | `tests/test_decode_budget.rs`、CLI range 校验 | pass |
| 3.1 文档与 CHANGELOG 同步 | `docs/reference/format-spec.md` 已记录 `0x10`/`0x11` 与 revision 语义；CHANGELOG [Unreleased] 已记录 | pass |
| 3.2 门禁 | 2026-10-01 重跑：`cargo fmt --all -- --check` / `cargo clippy --all-targets -- -D warnings` / `cargo test --lib --tests --locked`（全绿）/ `cargo doc --no-deps` 均退出 0 | pass |

## Residual risks

- `checksum_type` 目前仅定义 `0`（XxHash64）；未知校验和标识符由读者拒绝，新增类型需扩展 spec。
- ABC revision 0 路径仅为兼容保留，无新写入方；长期可评估收紧。

## Verdict

全部任务于 `21308f0` 实现并验证；收口时门禁全绿。归档决策记录见
`.agents/notes/implemented/architecture/2026-10-01-indexed-v2-optional-block-checksum-abc-v3-codec.md`。
