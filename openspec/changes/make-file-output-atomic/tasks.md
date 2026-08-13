# Tasks: make-file-output-atomic

## 1. Abstraction

- [x] 1.1 Move `tempfile` to runtime dependency
- [x] 1.2 Add `src/io/output_transaction.rs` + `FqcWriter::from_file`
- [x] 1.3 Unit tests for begin/drop/force/commit/stdout rejection

## 2. Wire writers

- [x] 2.1 Compression engine archive + streaming paths
- [x] 2.2 Compression pipeline (SE/PE/interleaved) with `force_overwrite`
- [x] 2.3 Decompress ordinary + split-PE + decompression pipeline

## 3. Integration tests and docs

- [x] 3.1 `tests/test_output_atomic.rs` (refuse, force success, force failure keeps old)
- [x] 3.2 CHANGELOG + CLI guide note
- [x] 3.3 OpenSpec delta/verification

## 4. Verification

- [x] 4.1 `cargo fmt --all -- --check`
- [x] 4.2 `cargo clippy --all-targets -- -D warnings`
- [x] 4.3 `cargo test --lib --tests`
- [x] 4.4 `cargo doc --no-deps`
