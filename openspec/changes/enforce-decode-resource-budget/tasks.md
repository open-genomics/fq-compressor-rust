# Tasks: enforce-decode-resource-budget

## 1. Budget core

- [x] 1.1 `DecodeBudget` + `ResourceLimit` + `zstd_decompress_bounded`
- [x] 1.2 `BlockIndex::read_with_budget` / `FqcReader::open_with_budget`
- [x] 1.3 Bound reorder map and block stream allocations

## 2. Codec + CLI wire-up

- [x] 2.1 Replace remaining `decode_all` in stream codecs (incl. IDs)
- [x] 2.2 Decompress / verify / pipeline take `memory_limit_mb`
- [x] 2.3 Original-order peak check before outputs; bounded parallel batch

## 3. Docs and tests

- [x] 3.1 README + CLI: automatic ≠ unlimited; limit applies to decompress/verify
- [x] 3.2 CHANGELOG entry
- [x] 3.3 Regression tests (huge index, zstd bound, tiny fixture under low budget, original-order peak)

## 4. Verification

- [x] 4.1 `cargo fmt --all -- --check`
- [x] 4.2 `cargo clippy --all-targets -- -D warnings`
- [x] 4.3 `cargo test --lib --tests`
- [x] 4.4 `cargo doc --no-deps`
