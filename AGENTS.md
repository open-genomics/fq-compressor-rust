# AGENTS.md — AI Agent Guidelines for fqc

## Identity

This is **fqc**, a high-performance FASTQ compressor in Rust. It compresses genomic sequencing data (FASTQ format) using domain-specific algorithms for sequences, quality scores, and read identifiers.

## Build & Verify

```bash
# Always run before committing
cargo build                    # must compile cleanly
cargo test --lib --tests       # 131 tests, 0 failures expected
cargo clippy --all-targets     # 0 warnings expected (pedantic enabled)
cargo fmt --all -- --check     # must pass
```

## Project Structure

```
src/
├── main.rs           # CLI (clap derive): compress, decompress, info, verify
├── lib.rs            # Library re-exports
├── error.rs          # FqcError (11 variants) → ExitCode (0-5)
├── types.rs          # ReadRecord, QualityMode, IdMode, PeLayout, ReadLengthClass
├── format.rs         # Binary format: magic, GlobalHeader, BlockHeader, Footer
├── fqc_reader.rs     # Block-indexed archive reader
├── fqc_writer.rs     # Archive writer with finalize
├── reorder_map.rs    # ZigZag delta + varint encoded bidirectional map
├── algo/             # Compression algorithms
│   ├── block_compressor.rs   # ABC (consensus + delta) / Zstd
│   ├── dna.rs               # Shared DNA encoding tables + reverse complement
│   ├── global_analyzer.rs    # Minimizer reordering
│   ├── quality_compressor.rs # SCM arithmetic coding
│   └── pe_optimizer.rs       # Paired-end optimization
├── commands/         # CLI commands
│   ├── compress.rs   # default / streaming / pipeline modes
│   ├── decompress.rs # sequential / parallel / reorder / pipeline
│   ├── info.rs       # archive info
│   └── verify.rs     # integrity check
├── common/
│   └── memory_budget.rs  # System memory detection, chunking
├── fastq/
│   └── parser.rs     # FASTQ parser, validation, PE, stats
├── io/
│   ├── async_io.rs           # Async read/write with buffer pool
│   └── compressed_stream.rs  # Feature-gated gz/bz2/xz/zst
└── pipeline/
    ├── mod.rs            # Shared types (PipelineControl, PipelineStats)
    ├── compression.rs    # 3-stage compression pipeline
    └── decompression.rs  # 3-stage decompression pipeline

tests/
├── test_algo.rs         # 19 algorithm tests (ID/quality compressor, PE optimizer)
├── test_dna.rs          # 15 DNA utility tests (encoding tables, reverse complement)
├── test_e2e.rs          # 15 end-to-end tests
├── test_format.rs       # 15 binary format tests
├── test_parser.rs       # 19 FASTQ parser tests
├── test_reorder_map.rs  # 23 reorder map tests
├── test_roundtrip.rs    # 14 compress→decompress round-trip
└── test_types.rs        # 11 type/constant tests
```

## Coding Rules

### Must Follow
- **MSRV 1.75** — do not use APIs stabilized after Rust 1.75
- **No unsafe** — `unsafe_code = "deny"` in Cargo.toml; only `#[allow(unsafe_code)]` on Windows FFI (`memory_budget.rs`)
- **Clippy pedantic clean** — `[lints.clippy] pedantic = "warn"` with tuned allows
- **All 131 tests pass** — run `cargo test --lib --tests` before any change
- **Use `log` crate** — `log::info!`, `log::warn!`, `log::debug!`; never `println!` for status
- **Use `thiserror`** — add new error variants to `FqcError`, map in `exit_code()`
- **Feature-gated compression** — gz/bz2/xz behind `#[cfg(feature)]` in `compressed_stream.rs`

### Style
- 4-space indentation, max line width 120 (`rustfmt.toml`)
- Imports grouped: std → external → crate (`imports_granularity = "Crate"`)
- Section separators: `// ====...====` block comments for major sections
- Test pattern: compress → decompress → compare record-by-record

### Avoid
- Changing the FQC binary format without bumping version
- Deleting or weakening tests
- Platform-specific paths — use `std::path::Path`
- Adding dependencies without justification
- `unwrap()` in library code — use `?` operator with `FqcError`

## Key Types

| Type | Location | Purpose |
|------|----------|---------|
| `ReadRecord` | `types.rs` | Single FASTQ record (id, sequence, quality) |
| `FqcError` | `error.rs` | Error enum with 11 variants |
| `ExitCode` | `error.rs` | CLI exit codes 0-5 |
| `GlobalHeader` | `format.rs` | Archive header (flags, read count, filename) |
| `BlockHeader` | `format.rs` | Per-block header (codec, counts, sizes) |
| `CompressOptions` | `commands/compress.rs` | All compression parameters |
| `DecompressOptions` | `commands/decompress.rs` | All decompression parameters |
| `CompressionPipelineConfig` | `pipeline/compression.rs` | Pipeline configuration |

## Common Tasks

### Add a CLI flag
1. Add field to `CompressOptions` or `DecompressOptions`
2. Add `#[arg]` variant in `Commands` enum (`main.rs`)
3. Wire in the match arm (`main.rs`)

### Add an error variant
1. Add to `FqcError` in `error.rs`
2. Map in `exit_code()` method

### Add a test
1. Use `compress_file()` / `decompress_file()` / `read_fastq_records()` helpers from `test_e2e.rs`
2. Use `TempFile` RAII guard for automatic cleanup
3. Assert record-by-record equality for round-trip tests

## Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` (4, derive) | CLI argument parsing |
| `zstd` (0.13) | Zstd compression (always required) |
| `xxhash-rust` (0.8) | xxHash64 checksums |
| `rayon` (1.10) | Parallel block processing |
| `crossbeam-channel` (0.5) | Pipeline stage communication |
| `thiserror` (1) | Error derive macros |
| `byteorder` (1) | Little-endian binary I/O |
| `log` + `env_logger` | Logging |
| `flate2` (optional) | Gzip support |
| `bzip2` (optional) | Bzip2 support |
| `xz2` (optional) | XZ/LZMA support |
| `tikv-jemallocator` (0.6) | Jemalloc allocator for musl static builds |

## CI/CD

- **ci.yml** — push/PR: check, test (3 OS), clippy, fmt, MSRV 1.75, cargo-deny
- **release.yml** — `v*` tag: validate version, test, build 5 targets, GitHub Release with checksums
