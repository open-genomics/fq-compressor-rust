# CLAUDE.md — Project Guide for Claude Code

## Project Overview

**fqc** is a high-performance FASTQ compressor written in Rust, featuring the ABC (Alignment-Based Compression) algorithm for short reads and Zstd for medium/long reads. It is a Rust port of the C++ fq-compressor project with feature parity.

## Quick Reference

```bash
# Build
cargo build              # dev build
cargo build --release    # optimized release build

# Test (131 tests across 8 suites)
cargo test --lib --tests

# Lint (clippy pedantic enabled, 0 warnings expected)
cargo clippy --all-targets

# Format check
cargo fmt --all -- --check

# Single test suite
cargo test --test test_algo
cargo test --test test_dna
cargo test --test test_e2e
cargo test --test test_format
cargo test --test test_parser
cargo test --test test_reorder_map
cargo test --test test_roundtrip
cargo test --test test_types
```

## Architecture

```
src/
├── main.rs              # CLI entry point (clap derive), command dispatch
├── lib.rs               # Library root, re-exports all modules
├── error.rs             # FqcError enum (11 variants) + ExitCode mapping (0-5)
├── types.rs             # Core types: ReadRecord, QualityMode, IdMode, PeLayout, ReadLengthClass
├── format.rs            # FQC binary format: magic bytes, GlobalHeader, BlockHeader, Footer
├── fqc_reader.rs        # Archive reader with block index + random access
├── fqc_writer.rs        # Archive writer with block index + finalize
├── reorder_map.rs       # Bidirectional read reorder map (ZigZag delta + varint encoding)
├── algo/
│   ├── block_compressor.rs  # ABC algorithm (consensus + delta) + Zstd codec
│   ├── dna.rs               # Shared DNA encoding tables + reverse complement
│   ├── global_analyzer.rs   # Minimizer-based global read reordering
│   ├── quality_compressor.rs # SCM order-1/2 arithmetic coding for quality scores
│   └── pe_optimizer.rs      # Paired-end complementarity optimization
├── commands/
│   ├── compress.rs      # CompressCommand: default / streaming / pipeline modes
│   ├── decompress.rs    # DecompressCommand: sequential / parallel / reorder / pipeline
│   ├── info.rs          # Archive info display (text / JSON / detailed)
│   └── verify.rs        # Block-by-block integrity verification
├── common/
│   └── memory_budget.rs # System memory detection (Win/Linux/macOS), ChunkingStrategy
├── fastq/
│   └── parser.rs        # FASTQ parser with validation, stats, PE support, chunk reading
├── io/
│   ├── async_io.rs      # AsyncReader/AsyncWriter/BufferPool/DoubleBuffer
│   └── compressed_stream.rs # Transparent gz/bz2/xz/zst decompression (feature-gated)
└── pipeline/
    ├── mod.rs           # PipelineControl, PipelineStats, ReadChunk
    ├── compression.rs   # 3-stage Reader→Compressor→Writer (crossbeam channels)
    └── decompression.rs # 3-stage Reader→Decompressor→Writer (with AsyncWriter)
```

## Key Design Decisions

- **MSRV 1.75** — pinned in `Cargo.toml` and tested in CI
- **`unsafe_code = "deny"`** — only allowed via `#[allow(unsafe_code)]` on Windows FFI in `memory_budget.rs`
- **Clippy pedantic** — enabled globally in `[lints.clippy]` with domain-specific allows (casts, etc.)
- **Feature flags** — `gz`, `bz2`, `xz` are optional (default enabled); `compressed_stream.rs` uses `#[cfg(feature)]`
- **Error handling** — `FqcError` with `thiserror`, maps to CLI exit codes 0-5
- **Binary format** — custom block-indexed format with magic header, xxHash64 checksums, optional reorder map
- **Parallelism** — `rayon` for batch mode, `crossbeam-channel` for pipeline mode

## Testing Conventions

- **Test data** — `tests/data/` contains `test_se.fastq` (20 reads) and `test_pe_R1/R2.fastq`
- **Temp files** — tests use `TempFile` RAII guard for automatic cleanup
- **Helper functions** — `compress_file()`, `decompress_file()`, `read_fastq_records()`, `assert_roundtrip_match()` in `test_e2e.rs`
- **Round-trip pattern** — compress → decompress → compare record-by-record (id, sequence, quality)
- **All 131 tests must pass** before any commit: `cargo test --lib --tests`

## Common Patterns

### Adding a new CLI flag
1. Add field to options struct in `src/commands/<cmd>.rs`
2. Add `#[arg]` to the `Commands` enum in `src/main.rs`
3. Wire the field in the match arm in `main.rs`

### Adding a new error variant
1. Add variant to `FqcError` in `src/error.rs`
2. Add mapping in `FqcError::exit_code()`

### Adding a new compression format
1. Add variant to `CompressionFormat` in `src/io/compressed_stream.rs`
2. Add magic bytes detection in `detect_format_from_bytes()`
3. Add extension in `detect_format_from_extension()`
4. Add reader in `open_compressed_reader()` behind `#[cfg(feature)]`
5. Add optional dependency + feature flag in `Cargo.toml`

## CI/CD

- **CI** (`ci.yml`) — check, test (3 OS), clippy, fmt, MSRV, cargo-deny
- **Release** (`release.yml`) — tag-triggered, builds 5 targets, creates GitHub Release with checksums

## Do NOT

- Add `unsafe` code without `#[allow(unsafe_code)]` and justification
- Delete or weaken existing tests
- Change the FQC binary format without updating version numbers
- Use `println!` for logging — use `log::info!`/`log::warn!`/`log::debug!`
- Hard-code platform-specific paths — use `std::path::Path`
