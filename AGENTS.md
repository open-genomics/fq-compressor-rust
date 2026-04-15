# AGENTS.md — AI Agent Guidelines for fqc

> This document provides context and guidelines for AI assistants working on the fqc codebase.

## Identity

**fqc** is a high-performance FASTQ compressor in Rust. It compresses genomic sequencing data (FASTQ format) using domain-specific algorithms:

- **ABC Algorithm** — Consensus + delta encoding for short reads (< 300bp)
- **Zstd** — General-purpose compression for medium/long reads
- **SCM** — Statistical Context Model with arithmetic coding for quality scores

## Quick Start

```bash
# Build and verify before any commit
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
│   ├── id_compressor.rs      # ID tokenization + delta encoding
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

| Rule | Description |
|------|-------------|
| **MSRV 1.75** | Do not use APIs stabilized after Rust 1.75 |
| **No unsafe** | `unsafe_code = "deny"` in Cargo.toml; only `#[allow(unsafe_code)]` on Windows FFI (`memory_budget.rs`) |
| **Clippy pedantic** | All code must pass with 0 warnings |
| **Formatting** | Run `cargo fmt` before committing |
| **All 131 tests** | Must pass before any commit: `cargo test --lib --tests` |
| **Use `log` crate** | `log::info!`, `log::warn!`, `log::debug!`; never `println!` for status |
| **Use `thiserror`** | Add new error variants to `FqcError`, map in `exit_code()` |
| **Feature-gated deps** | gz/bz2/xz behind `#[cfg(feature)]` in `compressed_stream.rs` |

### Code Style

- 4-space indentation, max line width 120 (`rustfmt.toml`)
- Section separators: `// ====...====` block comments for major sections
- Test pattern: compress → decompress → compare record-by-record

### Error Handling Pattern

```rust
// Good: Use ? operator with FqcError
pub fn my_function() -> Result<()> {
    let data = std::fs::read(path)?;
    // ...
    Ok(())
}

// Bad: unwrap() in library code
pub fn my_function() -> Result<()> {
    let data = std::fs::read(path).unwrap(); // Don't do this
}
```

## Key Types

| Type | Location | Purpose |
|------|----------|---------|
| `ReadRecord` | `types.rs` | Single FASTQ record (id, comment, sequence, quality) |
| `FqcError` | `error.rs` | Error enum with 11 variants |
| `ExitCode` | `error.rs` | CLI exit codes 0-5 |
| `GlobalHeader` | `format.rs` | Archive header (flags, read count, filename) |
| `BlockHeader` | `format.rs` | Per-block header (codec, counts, sizes) |
| `CompressOptions` | `commands/compress.rs` | All compression parameters |
| `DecompressOptions` | `commands/decompress.rs` | All decompression parameters |
| `BlockCompressorConfig` | `algo/block_compressor.rs` | Compression algorithm config |
| `CompressionPipelineConfig` | `pipeline/compression.rs` | Pipeline configuration |

## Common Tasks

### Add a CLI Flag

1. Add field to `CompressOptions` or `DecompressOptions` in `src/commands/<cmd>.rs`
2. Add `#[arg]` attribute in the `Commands` enum (`main.rs`)
3. Wire the field in the match arm (`main.rs`)

**Example:**

```rust
// Step 1: In src/commands/compress.rs
pub struct CompressOptions {
    // ...
    pub my_new_flag: bool,
}

// Step 2: In src/main.rs
#[derive(Subcommand)]
enum Commands {
    Compress {
        // ...
        #[arg(long)]
        my_new_flag: bool,
    },
}

// Step 3: In main.rs match arm
Commands::Compress { my_new_flag, .. } => {
    let opts = CompressOptions {
        my_new_flag,
        ..Default::default()
    };
    // ...
}
```

### Add an Error Variant

1. Add to `FqcError` in `src/error.rs`
2. Map in `exit_code()` method

```rust
// Step 1
#[derive(Debug, Error)]
pub enum FqcError {
    // ... existing variants
    #[error("My new error: {0}")]
    MyNewError(String),
}

// Step 2
impl FqcError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            // ... existing mappings
            FqcError::MyNewError(_) => ExitCode::Usage,
        }
    }
}
```

### Add a Test

Use helper functions from `test_e2e.rs`:

```rust
use super::*;

#[test]
fn test_my_feature() -> Result<()> {
    let input = "tests/data/test_se.fastq";
    let output = TempFile::new(".fqc")?;
    
    compress_file(input, output.path(), Default::default())?;
    
    let records = decompress_file(output.path(), Default::default())?;
    let original = read_fastq_records(input)?;
    
    assert_roundtrip_match(&original, &records)?;
    Ok(())
}
```

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
| `tikv-jemallocator` (0.6) | Jemalloc for musl static builds |

## Performance Tips

1. **Use pipeline mode** for large files: `--pipeline` flag enables 3-stage parallel processing
2. **Adjust block size** based on read length: smaller blocks for long reads, larger for short
3. **Use streaming mode** for stdin or memory-constrained environments: `--streaming`
4. **Discard quality** if not needed: `--lossy-quality discard` gives smallest output

## Docker Build Chain

| Component | Choice | Reason |
|-----------|--------|--------|
| Build image | `rust:1.75-bookworm` (Debian 12) | Official Rust image, matches MSRV |
| Runtime image | `debian:bookworm-slim` | Same family, shared base layers, minimal size |

## CI/CD

- **ci.yml** — Push/PR: check, test (3 OS), clippy, fmt, MSRV 1.75, cargo-deny
- **release.yml** — `v*` tag: validate version, test, build 5 targets, GitHub Release with checksums

## Things to Avoid

- `unsafe` code without `#[allow(unsafe_code)]` and justification
- Deleting or weakening existing tests
- Changing FQC binary format without bumping version
- `println!` for status output — use `log::info!`/`log::warn!`/`log::debug!`
- Hard-coded platform-specific paths — use `std::path::Path`
- `unwrap()` in library code — use `?` with proper error handling
