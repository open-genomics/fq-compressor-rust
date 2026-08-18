# Verification: enforce-compress-archive-budget

- Status: `Completed`
- Ready to archive: `yes`

| Case | Evidence | Result |
|---|---|---|
| Automatic is finite | `resolve_compress_limit_mb_zero_is_finite` | passed |
| Over-budget fails | `archive_compress_rejects_over_budget_before_output` | passed |
| Tiny + 0/16 | `archive_compress_accepts_tiny_fixture_under_auto_and_min` | passed |
| Streaming exception | `streaming_compress_accepts_input_that_archive_rejects` | passed |

Commands: `cargo fmt --check`, `clippy -D warnings`, `test --lib --tests`, `doc --no-deps` — exit 0.

Criterion (repeated tiny fixture, 2026-08-18): parser 453 MiB/s; archive roundtrip 249 ms / 70 KiB/s; verify 144 ms. Not production throughput.
