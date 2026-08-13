# Foreign sequential fixture (for family rejection)

Copied from `open-genomics/fq-compressor` `tests/fixtures/sequential-v2/frozen_se.fqc`
for `FQC-FAMILY-001` / `recognize-sequential-fqc-family`.

| Field | Value |
|---|---|
| Format family | `fqc-sequential/v2` |
| Magic | `46 51 43 56 32 0D 0A 1A` |
| SHA-256 | `2b1cc50edfa47dd8bd7881a4ee7bb7f4980e693c14d90d90b640fdd62ccc4edf` |
| Expected Rust reader behavior | reject with unsupported format family |

This archive is **not** decoded by this repository.
