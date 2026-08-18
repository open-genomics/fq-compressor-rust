# Verification: enforce-compress-archive-budget

- Status: `Completed`
- Ready to archive: `yes`

| Case | Evidence | Result |
|---|---|---|
| Automatic is finite | `resolve_compress_limit_mb_zero_is_finite` | passed |
| Over-budget fails | `archive_compress_rejects_over_budget_before_output` | passed |
| Tiny + 0/16 | `archive_compress_accepts_tiny_fixture_under_auto_and_min` | passed |
| Streaming exception | `streaming_compress_accepts_input_that_archive_rejects` | passed |
| Pipeline over-budget | `pipeline_compress_rejects_over_budget_before_output` | passed |
| Pipeline tiny | `pipeline_compress_accepts_tiny_fixture_under_min` | passed |

Commands: `cargo fmt --check`, `clippy -D warnings`, `test --lib --tests`, `doc --no-deps`.
