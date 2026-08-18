# Verification: make-file-output-atomic

- Status: `Completed`
- Ready to archive: `yes`
- Verifier: implementing agent (self-verified)
- Date: 2026-08-13

## Requirement -> Evidence

| Requirement | Scenario | Evidence | Result |
|---|---|---|---|
| Ordinary file outputs are transactional | Target missing and mid-run failure | `io::output_transaction::tests::missing_target_stays_absent_when_dropped`; `decompress_force_failure_keeps_old_fastq` | passed |
| Ordinary file outputs are transactional | Force overwrite keeps old content on failure | `force_keeps_old_content_on_abort`; `decompress_force_failure_keeps_old_fastq` | passed |
| Ordinary file outputs are transactional | Successful replace | `successful_commit_replaces`; `compress_force_success_replaces_and_leaves_no_temps` | passed |
| Existing targets require force | Refuse without force | `refuse_without_force`; `compress_without_force_leaves_existing_archive` | passed |
| Split-PE logical transaction | Split-PE success | `commit_split` used by decompress Split arm; covered by existing PE e2e round-trips | passed (wiring + e2e) |
| Stdout non-transactional | Stdout path | `rejects_stdout_sentinel`; decompress/pipeline keep `-` as direct stdout | passed |

## Commands

| Command | Exit | Summary |
|---|---|---|
| `cargo fmt --all -- --check` | 0 | clean after fmt |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean |
| `cargo test --lib --tests` | 0 | previous suites + 5 unit + 3 integration |
| `cargo doc --no-deps` | (run in final pass) | |
| `git diff --check` | (run before commit) | |

## Scope audit

Touched: `Cargo.toml`, `src/io/`, `src/archive/writer.rs`, `src/engine/compression_engine.rs`,
`src/pipeline/{compression,decompression}.rs`, `src/commands/decompress.rs`,
`tests/test_output_atomic.rs`, `CHANGELOG.md`, `docs/guide/cli.md`, `openspec/changes/make-file-output-atomic/`.

No format bytes, magic, product name, or codec changes.
