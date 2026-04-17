# QWEN.md — fqc Project Context

## Project Overview

**fqc** is a high-performance FASTQ compressor written in Rust. It compresses genomic sequencing data (FASTQ format) using domain-specific algorithms:

- **ABC Algorithm** — Consensus + delta encoding for short reads (< 300bp)
- **Zstd** — General-purpose compression for medium/long reads
- **SCM** — Statistical Context Model with arithmetic coding for quality scores

The project achieves **3.9x compression ratio** (75% smaller files) and supports random access via a block-indexed `.fqc` archive format. It is compatible with the [fq-compressor](https://github.com/LessUp/fq-compressor) C++ implementation.

### Key Metrics

| Metric | Value |
|--------|-------|
| **Lines of Code** | ~15,000 Rust |
| **Test Coverage** | 131 tests, 0 failures |
| **Dependencies** | 15 crates (minimal) |
| **Unsafe Code** | 0 lines (`unsafe_code = "deny"`) |
| **MSRV** | Rust 1.75.0 |
| **License** | GPL-3.0 |

---

## Building and Running

### Prerequisites

- **Rust 1.75+** (MSRV pinned in `Cargo.toml`)
- **zstd library** (`libzstd-dev` on Ubuntu, `zstd` on macOS via Homebrew)

### Build Commands

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run
./target/release/fqc --help

# Install globally
cargo install --path .
```

### Testing

```bash
# Run all tests (131 tests)
cargo test --lib --tests

# Run specific test suite
cargo test --test test_algo         # Algorithm tests (19)
cargo test --test test_dna          # DNA utility tests (15)
cargo test --test test_e2e          # End-to-end tests (15)
cargo test --test test_format       # Binary format tests (15)
cargo test --test test_parser       # FASTQ parser tests (19)
cargo test --test test_reorder_map  # Reorder map tests (23)
cargo test --test test_roundtrip    # Round-trip tests (14)
cargo test --test test_types        # Type/constant tests (11)
```

### Linting and Formatting

```bash
# Clippy (must pass with 0 warnings, pedantic enabled)
cargo clippy --all-targets

# Format check
cargo fmt --all -- --check

# Auto-format
cargo fmt --all
```

### Optional Features

```bash
# Enable gzip/bzip2/xz input support
cargo build --release --features gz,bz2,xz

# Static binary (Linux, requires musl toolchain)
cargo build --release --target x86_64-unknown-linux-musl
```

---

## Development Conventions

### Spec-Driven Development (SDD)

This project strictly follows **Spec-Driven Development**. All code implementations must use the `/specs` directory as the single source of truth:

| Directory | Purpose |
|-----------|---------|
| `/specs/product/` | Product feature definitions and acceptance criteria |
| `/specs/rfc/` | Technical design documents and architecture decisions |
| `/specs/api/` | API interface definitions (CLI and library APIs) |
| `/specs/testing/` | BDD test case specifications and acceptance criteria |

**AI Agent Workflow:**
1. **Review specs** before writing code
2. **Propose spec changes** before implementation
3. **100% comply with specs** during implementation
4. **Write tests** based on spec acceptance criteria

### Code Style

- **Edition**: Rust 2021
- **MSRV**: 1.75.0 (do not use APIs stabilized after 1.75)
- **Indentation**: 4 spaces
- **Max line width**: 120 characters (`rustfmt.toml`)
- **No unsafe code**: `unsafe_code = "deny"` in Cargo.toml
- **Clippy pedantic**: All code must pass with 0 warnings
- **Error handling**: Use `thiserror` derive and `?` operator (never `unwrap()` in library code)
- **Logging**: Use `log` crate (`log::info!`, `log::warn!`, `log::debug!`), never `println!` for status
- **Section separators**: Use `// ====...====` block comments for major sections

### Testing Practices

- **Integration tests** in `tests/` directory (external to `src/`)
- **Unit tests** alongside implementation using `#[cfg(test)]` modules
- **Test pattern**: compress → decompress → compare record-by-record
- **Use helper functions** from `test_e2e.rs`: `compress_file()`, `decompress_file()`, `read_fastq_records()`, `assert_roundtrip_match()`
- **Test data** in `tests/data/` directory (FASTQ sample files)

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `test:` Test changes
- `refactor:` Code refactoring
- `perf:` Performance improvements
- `ci:` CI/CD changes

---

## Project Structure

```
fq-compressor-rust/
├── src/                          # Source code
│   ├── main.rs                   # CLI entry point (clap derive)
│   ├── lib.rs                    # Library root, re-exports all modules
│   ├── error.rs                  # FqcError (11 variants) → ExitCode (0-5)
│   ├── types.rs                  # Core types: ReadRecord, QualityMode, etc.
│   ├── format.rs                 # FQC binary format: magic, headers, footer
│   ├── fqc_reader.rs             # Block-indexed archive reader
│   ├── fqc_writer.rs             # Archive writer with finalize
│   ├── reorder_map.rs            # ZigZag delta + varint encoded bidirectional map
│   ├── algo/                     # Compression algorithms
│   │   ├── block_compressor.rs   # ABC (consensus + delta) / Zstd
│   │   ├── dna.rs                # Shared DNA encoding tables + reverse complement
│   │   ├── global_analyzer.rs    # Minimizer reordering
│   │   ├── quality_compressor.rs # SCM arithmetic coding
│   │   ├── id_compressor.rs      # ID tokenization + delta encoding
│   │   └── pe_optimizer.rs       # Paired-end optimization
│   ├── commands/                 # CLI commands
│   │   ├── compress.rs           # default / streaming / pipeline modes
│   │   ├── decompress.rs         # sequential / parallel / reorder / pipeline
│   │   ├── info.rs               # archive info
│   │   └── verify.rs             # integrity check
│   ├── pipeline/                 # 3-stage parallel pipelines
│   │   ├── compression.rs        # Compression pipeline
│   │   └── decompression.rs      # Decompression pipeline
│   ├── common/                   # Shared utilities
│   │   └── memory_budget.rs      # System memory detection, chunking
│   ├── fastq/                    # FASTQ parsing
│   │   └── parser.rs             # FASTQ parser, validation, PE, stats
│   └── io/                       # I/O operations
│       ├── async_io.rs           # Async read/write with buffer pool
│       └── compressed_stream.rs  # Feature-gated gz/bz2/xz/zst
│
├── tests/                        # Integration tests
│   ├── data/                     # Test fixture files (FASTQ samples)
│   └── test_*.rs                 # Test suites (131 tests total)
│
├── specs/                        # Specifications (SDD)
│   ├── product/                  # Product feature specs
│   ├── rfc/                      # Technical RFCs
│   ├── api/                      # API interface specs
│   └── testing/                  # BDD test specs
│
├── docs/                         # VitePress documentation site
│   ├── guide/                    # User guide (English)
│   ├── architecture/             # Architecture docs
│   ├── algorithms/               # Algorithm documentation
│   ├── changelog/                # Release notes
│   └── zh/                       # Chinese documentation
│
├── .github/workflows/            # CI/CD pipelines
│   ├── ci.yml                    # Cross-platform tests, MSRV, audit
│   ├── quality.yml               # Clippy, fmt, docs check
│   ├── docker.yml                # Docker image build
│   ├── pages-vitepress.yml       # Documentation deployment
│   └── release.yml               # Release builds (5 targets)
│
└── Configuration files
    ├── Cargo.toml                # Rust dependencies, features, profiles
    ├── rustfmt.toml              # Formatter config (4-space, 120 width)
    ├── clippy.toml               # Clippy thresholds
    ├── deny.toml                 # License/dependency audit
    ├── Dockerfile                # Production Docker image
    └── package.json              # VitePress docs site
```

---

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

---

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

---

## CI/CD Pipelines

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| **CI** | Push/PR | Cross-platform tests (Linux, macOS, Windows), MSRV check, dependency audit |
| **Code Quality** | Push/PR | Clippy (pedantic), format check, docs check |
| **Docker** | Tag `v*` | Build and push Docker images to GHCR |
| **VitePress Pages** | Push to `main` | Build and deploy documentation site |
| **Release** | Tag `v*` | Build 5 targets, create GitHub Release with checksums |

---

## Common Tasks

### Add a CLI Flag

1. Add field to `CompressOptions` or `DecompressOptions` in `src/commands/<cmd>.rs`
2. Add `#[arg]` attribute in the `Commands` enum (`main.rs`)
3. Wire the field in the match arm (`main.rs`)

### Add an Error Variant

1. Add to `FqcError` in `src/error.rs`
2. Map in `exit_code()` method

### Add a Test

Use helper functions from `test_e2e.rs`:

```rust
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

---

## Performance Tips

1. **Use pipeline mode** for large files: `--pipeline` flag enables 3-stage parallel processing
2. **Adjust block size** based on read length: smaller blocks for long reads, larger for short
3. **Use streaming mode** for stdin or memory-constrained environments: `--streaming`
4. **Discard quality** if not needed: `--lossy-quality discard` gives smallest output

---

## Docker Build

```bash
# Build image
docker build -t fqc .

# Run (mount data directory)
docker run --rm -v $(pwd):/data fqc compress -i /data/reads.fastq -o /data/reads.fqc

# Pull from GHCR
docker pull ghcr.io/lessup/fq-compressor-rust:latest
```

**Build chain:**
- **Build image**: `rust:1.75-bookworm` (Debian 12, matches MSRV)
- **Runtime image**: `debian:bookworm-slim` (minimal size)

---

## Documentation

- **Online docs**: [https://lessup.github.io/fq-compressor-rust/](https://lessup.github.io/fq-compressor-rust/)
- **VitePress config**: `docs/.vitepress/config.mts`
- **Chinese docs**: `docs/zh/`
- **Build locally**: `cd docs && npm install && npm run docs:build`

---

## Things to Avoid

- ❌ `unsafe` code without `#[allow(unsafe_code)]` and justification
- ❌ Deleting or weakening existing tests
- ❌ Changing FQC binary format without bumping version
- ❌ `println!` for status output — use `log::info!`/`log::warn!`/`log::debug!`
- ❌ Hard-coded platform-specific paths — use `std::path::Path`
- ❌ `unwrap()` in library code — use `?` with proper error handling
- ❌ Adding features not defined in specs (no gold-plating)
