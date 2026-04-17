# AGENTS.md — AI Agent Guidelines for fqc

> This document provides context and guidelines for AI assistants working on the fqc codebase.

## Project Philosophy: Spec-Driven Development (SDD)

This project strictly follows the **Spec-Driven Development (SDD)** paradigm. All code implementations must use the `/specs` directory as the single source of truth.

### Directory Context

| Directory | Purpose |
|-----------|---------|
| `/specs/product/` | Product feature definitions and acceptance criteria |
| `/specs/rfc/` | Technical design documents and architecture decisions |
| `/specs/api/` | API interface definitions (CLI and library APIs) |
| `/specs/db/` | Database schema definitions (not used - fqc is file-based) |
| `/specs/testing/` | BDD test case specifications and acceptance criteria |
| `/docs/` | User guides, tutorials, and architecture documentation (VitePress) |
| `/docs/zh/` | Chinese language documentation |
| `/src/` | Implementation code |
| `/tests/data/` | Test fixture files (FASTQ samples) |

### AI Agent Workflow Instructions

When you (AI) are asked to develop a new feature, modify existing functionality, or fix a bug, **you MUST strictly follow this workflow without skipping any steps**:

#### Step 1: Review Specs

- Before writing any code, first read the relevant documents in `/specs` (product specs, RFCs, API definitions).
- If the user's instruction conflicts with existing specs, **stop immediately** and point out the conflict, asking the user whether to update the spec first.

#### Step 2: Spec-First Update

- If this is a new feature or requires changes to existing interfaces/structures, **you MUST propose changes to the relevant spec documents first** (e.g., RFCs, API specs).
- Wait for user confirmation of spec changes before entering the coding phase.

#### Step 3: Implementation

- When writing code, **100% comply with spec definitions** (including variable naming, API paths, data types, status codes, etc.).
- **Do not add features not defined in specs** (No Gold-Plating).

#### Step 4: Test against Spec

- Write unit and integration tests based on acceptance criteria in `/specs`.
- Ensure test cases cover all boundary conditions described in specs.

### Code Generation Rules

- Any externally exposed API changes must sync with `/specs/api/` documents.
- If uncertain about technical details, consult `/specs/rfc/` for architecture conventions; do not invent design patterns on your own.

---

## Identity

**fqc** is a high-performance FASTQ compressor written in Rust. It compresses genomic sequencing data (FASTQ format) using domain-specific algorithms:

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
├── main.rs              # CLI entry point (clap derive), command dispatch
├── lib.rs               # Library root, re-exports all modules
├── error.rs             # FqcError (11 variants) → ExitCode (0-5)
├── types.rs             # Core types: ReadRecord, QualityMode, IdMode, PeLayout, ReadLengthClass
├── format.rs            # FQC binary format: magic, GlobalHeader, BlockHeader, Footer
├── fqc_reader.rs        # Block-indexed archive reader
├── fqc_writer.rs        # Archive writer with finalize
├── reorder_map.rs       # ZigZag delta + varint encoded bidirectional map
├── algo/                # Compression algorithms
│   ├── block_compressor.rs   # ABC (consensus + delta) / Zstd
│   ├── dna.rs               # Shared DNA encoding tables + reverse complement
│   ├── global_analyzer.rs   # Minimizer reordering
│   ├── quality_compressor.rs # SCM arithmetic coding
│   ├── id_compressor.rs     # ID tokenization + delta encoding
│   └── pe_optimizer.rs      # Paired-end optimization
├── commands/            # CLI commands
│   ├── compress.rs      # default / streaming / pipeline modes
│   ├── decompress.rs    # sequential / parallel / reorder / pipeline
│   ├── info.rs          # archive info
│   └── verify.rs        # integrity check
├── common/
│   └── memory_budget.rs     # System memory detection, chunking
├── fastq/
│   └── parser.rs            # FASTQ parser, validation, PE, stats
├── io/
│   ├── async_io.rs          # Async read/write with buffer pool
│   └── compressed_stream.rs # Feature-gated gz/bz2/xz/zst
└── pipeline/
    ├── mod.rs               # Shared types (PipelineControl, PipelineStats)
    ├── compression.rs       # 3-stage compression pipeline
    └── decompression.rs     # 3-stage decompression pipeline

specs/
├── README.md            # Specs index and SDD philosophy
├── product/             # Product feature definitions
│   ├── core-compression.md
│   ├── cli-commands.md
│   └── file-format.md
├── rfc/                 # Technical design documents
│   ├── 0001-core-architecture.md
│   ├── 0002-compression-algorithms.md
│   └── 0003-pipeline-architecture.md
├── api/                 # API interface definitions
├── db/                  # Database schemas (not used)
└── testing/             # BDD test specifications

docs/
├── .vitepress/          # VitePress configuration
├── guide/               # User guide (English)
├── architecture/        # Architecture docs
├── algorithms/          # Algorithm documentation
├── changelog/           # Release notes (English)
│   ├── index.md
│   └── releases/
│       ├── v0.1.0.md
│       ├── v0.1.1.md
│       └── zh/          # Chinese release notes
├── zh/                  # Chinese documentation
│   ├── README.md
│   ├── guide/
│   ├── architecture/
│   ├── algorithms/
│   └── changelog/
└── index.md             # VitePress landing page

tests/
├── data/                # Test fixture files
│   ├── README.md
│   ├── test_se.fastq
│   ├── test_R1.fastq
│   ├── test_R2.fastq
│   └── test_interleaved.fastq
├── test_algo.rs         # 19 algorithm tests
├── test_dna.rs          # 15 DNA utility tests
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
    // ...existing fields
    pub my_new_flag: bool,
}

// Step 2: In src/main.rs
#[derive(Subcommand)]
enum Commands {
    Compress {
        // ...existing args
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
