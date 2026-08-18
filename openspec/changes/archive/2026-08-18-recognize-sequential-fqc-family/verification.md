# Verification: recognize-sequential-fqc-family

- Status: `Completed`
- Date: 2026-08-13

| Case | Evidence | Result |
|---|---|---|
| Own indexed fixture | `accepts_own_indexed_frozen_fixture` | passed |
| Sequential family reject | `rejects_sequential_frozen_fixture_as_known_family` | passed |
| Unknown magic | `rejects_unknown_magic` | passed |
| Truncated magic | `rejects_truncated_magic` | passed |
| CLI entries | `info_verify_decompress_reject_sequential_without_output` | passed |

Commands: `cargo fmt --check`, `clippy -D warnings`, `test --lib --tests`, `doc --no-deps` — exit 0.
