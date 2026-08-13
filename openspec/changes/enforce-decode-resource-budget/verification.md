# Verification: enforce-decode-resource-budget

- Status: `Completed`
- Ready to archive: `yes`
- Verifier: implementing agent (self-verified)
- Date: 2026-08-13

## Requirement -> Evidence

| Requirement | Scenario | Evidence | Result |
|---|---|---|---|
| Operation-scoped budget | Automatic is finite | `automatic_budget_is_finite` | passed |
| Operation-scoped budget | Explicit limit honored | `original_order_peak_rejected_before_output`; `parallel_batch_never_zero_and_rejects_oversized_block` | passed |
| Declared sizes checked | Huge block count | `forged_huge_num_blocks_rejected_before_alloc` | passed |
| Declared sizes checked | File-region cap | `forged_num_blocks_exceeding_file_region_rejected` | passed |
| Zstd bounded | Expansion past ceiling | `zstd_bounded_rejects_expansion_past_ceiling` | passed |
| Original-order before outputs | Peak over budget | `original_order_peak_rejected_before_output` | passed |
| Tiny fixture low budget | Happy path | `tiny_archive_opens_and_verifies_under_min_budget` | passed |

## Commands

| Command | Exit | Summary |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean |
| `cargo test --lib --tests` | 0 | prior suites + 7 budget tests |
| `cargo doc --no-deps` | 0 | clean |
| `git diff --check` | 0 | clean |

## Scope audit

Touched: `src/memory_budget.rs`, `src/error.rs`, `src/archive/{format,reader}.rs`,
`src/algo/*` (bounded zstd), `src/commands/{decompress,verify}.rs`, `src/main.rs`,
`src/pipeline/decompression.rs`, `tests/test_decode_budget.rs`, docs/CHANGELOG,
`openspec/changes/enforce-decode-resource-budget/`.

No format magic/product-name changes.
