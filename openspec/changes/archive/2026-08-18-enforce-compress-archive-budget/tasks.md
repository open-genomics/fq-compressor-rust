# Tasks: enforce-compress-archive-budget

- [x] `resolve_compress_limit_mb` / `check_archive_ingest` / incremental ingest
- [x] Wire `run_archive` read paths; keep streaming unchecked
- [x] Tests: auto is finite; over-budget archive fails with no output; tiny + `0`/`16` succeed; streaming still accepts the over-budget file
- [x] README / CLI / modes / roadmap / CHANGELOG
- [x] Gates: fmt, clippy, test, doc
  - `cargo fmt --all -- --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test --lib --tests`
  - `cargo doc --no-deps`
