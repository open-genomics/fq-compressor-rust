# fqc - High-Performance FASTQ Compressor

[![CI](https://github.com/LessUp/fq-compressor-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/LessUp/fq-compressor-rust/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/LessUp/fq-compressor-rust?include_prereleases)](https://github.com/LessUp/fq-compressor-rust/releases)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![MSRV](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Docs](https://img.shields.io/badge/docs-VitePress-blue)](https://lessup.github.io/fq-compressor-rust/)

[English](README.md) | [简体中文](README.zh-CN.md) | [C++ Version](https://github.com/LessUp/fq-compressor)

> **fqc** is a high-performance FASTQ compressor written in Rust, featuring the **ABC** (Alignment-Based Compression) algorithm for short reads and **Zstd** for medium/long reads. It shares the `.fqc` archive format with the original [fq-compressor](https://github.com/LessUp/fq-compressor) C++ implementation.

---

## ✨ Features

| Category | Features |
|----------|----------|
| **Compression** | ABC (consensus + delta) for short reads, Zstd for long reads |
| **Quality** | SCM (Statistical Context Model) with arithmetic coding |
| **Performance** | Parallel processing, 3-stage pipeline, async I/O |
| **Flexibility** | Streaming mode, lossy/lossless quality, random access |
| **Compatibility** | Paired-end support, compressed input (gz/bz2/xz/zst) |

<details>
<summary><b>Full Feature List</b></summary>

- **ABC Algorithm** — Consensus-based delta encoding for short reads (< 300bp), achieving high compression ratios
- **Zstd Compression** — For medium/long reads with length-prefixed encoding
- **SCM Quality Compression** — Statistical Context Model with arithmetic coding for quality scores
- **Global Read Reordering** — Minimizer-based read reordering to improve compression
- **Random Access** — Block-indexed archive format for efficient partial decompression
- **Parallel Processing** — Rayon-based parallel block compression/decompression
- **Pipeline Mode** — 3-stage Reader→Compressor→Writer pipeline with backpressure (`--pipeline`)
- **Async I/O** — Background prefetch and write-behind for improved throughput
- **Streaming Mode** — Low-memory compression from stdin without global reordering (`--streaming`)
- **Lossless & Lossy** — Supports lossless, Illumina 8-bin, and discard quality modes
- **Compressed Input** — Transparent decompression of `.gz`, `.bz2`, `.xz`, `.zst` FASTQ files
- **Paired-End** — Interleaved and separate-file paired-end support
- **Memory Budget** — Auto-detect system memory, dynamic chunking for large datasets

</details>

---

## 📊 Performance

| Mode | Compression | Decompression | Ratio |
|------|-------------|---------------|-------|
| Default | ~10 MB/s | ~55 MB/s | 3.9x |
| Pipeline | ~12 MB/s | ~60 MB/s | 3.9x |

*Tested on Intel Core i7-9700 @ 3.00GHz (8 cores), 2.27M Illumina reads (511 MB uncompressed)*

### Compression Strategies

| Read Length | Sequence Codec | Quality Codec | Reordering |
|-------------|----------------|---------------|------------|
| Short (< 300bp) | ABC (consensus + delta) | SCM Order-2 | ✅ Yes |
| Medium (300bp – 10kbp) | Zstd | SCM Order-2 | ❌ No |
| Long (> 10kbp) | Zstd | SCM Order-1 | ❌ No |

---

## 📦 Installation

### From Source

```bash
# Clone and build
git clone https://github.com/LessUp/fq-compressor-rust.git
cd fq-compressor-rust
cargo build --release

# Binary location
./target/release/fqc --help
```

### Docker

```bash
# Pull from GitHub Container Registry
docker pull ghcr.io/lessup/fq-compressor-rust:latest

# Or build locally
docker build -t fqc .

# Run (mount data directory)
docker run --rm -v $(pwd):/data fqc compress -i /data/reads.fastq -o /data/reads.fqc
```

### Pre-built Binaries

Download from [GitHub Releases](https://github.com/LessUp/fq-compressor-rust/releases) for:
- Linux (x64, ARM64) — glibc and musl (static)
- macOS (Intel, Apple Silicon)
- Windows x64

---

## 🚀 Quick Start

### Compress

```bash
# Basic compression (auto-detects read length)
fqc compress -i reads.fastq -o reads.fqc

# With compression level (1-9)
fqc compress -i reads.fastq -o reads.fqc -l 9

# Streaming mode (low memory, from stdin)
cat reads.fastq | fqc compress --streaming -i - -o reads.fqc

# Pipeline mode (3-stage parallel pipeline)
fqc compress -i reads.fastq -o reads.fqc --pipeline

# Paired-end (separate files)
fqc compress -i reads_R1.fastq -2 reads_R2.fastq -o paired.fqc

# Paired-end (interleaved single file)
fqc compress -i interleaved.fastq -o paired.fqc --interleaved

# Compressed input (auto-detected)
fqc compress -i reads.fastq.gz -o reads.fqc
fqc compress -i reads.fastq.bz2 -o reads.fqc

# Discard quality scores (smallest output)
fqc compress -i reads.fastq -o reads.fqc --lossy-quality discard

# Force long-read mode
fqc compress -i long_reads.fastq -o reads.fqc --long-read-mode long

# Overwrite existing file
fqc compress -i reads.fastq -o reads.fqc -f
```

### Decompress

```bash
# Full decompression
fqc decompress -i reads.fqc -o reads.fastq

# Extract range of reads (1-based, inclusive)
fqc decompress -i reads.fqc -o subset.fastq --range 1:1000
fqc decompress -i reads.fqc -o subset.fastq --range 100:    # from 100 to end

# Output to stdout
fqc decompress -i reads.fqc -o -

# Headers only (IDs)
fqc decompress -i reads.fqc -o headers.txt --header-only

# Restore original order (requires reorder map)
fqc decompress -i reads.fqc -o reads.fastq --original-order

# Split paired-end to separate files
fqc decompress -i paired.fqc -o output.fastq --split-pe
# Creates output_R1.fastq and output_R2.fastq

# Pipeline mode decompression
fqc decompress -i reads.fqc -o reads.fastq --pipeline

# Skip corrupted blocks instead of failing
fqc decompress -i reads.fqc -o reads.fastq --skip-corrupted
```

### Info & Verify

```bash
# Human-readable summary
fqc info -i reads.fqc

# JSON output
fqc info -i reads.fqc --json

# Detailed block index
fqc info -i reads.fqc --detailed

# Show codec information per block
fqc info -i reads.fqc --show-codecs

# Verify archive integrity
fqc verify -i reads.fqc

# Verbose verification (per-block progress)
fqc verify -i reads.fqc --verbose

# Quick verification (header + footer only)
fqc verify -i reads.fqc --quick
```

---

## 📁 FQC File Format

```
┌─────────────────────┐
│   Magic Header (9B) │  "\x89FQC\r\n\x1a\n" + version
├─────────────────────┤
│   Global Header     │  Flags, read count, filename, timestamp
├─────────────────────┤
│   Block 0           │  Block header + IDs + Sequences + Quality + Aux
├─────────────────────┤
│   Block 1           │
├─────────────────────┤
│   ...               │
├─────────────────────┤
│   Reorder Map (opt) │  Forward + reverse maps (delta + varint encoded)
├─────────────────────┤
│   Block Index       │  Offsets for random access
├─────────────────────┤
│   File Footer (32B) │  Index offset, checksum, magic tail
└─────────────────────┘
```

See [specs/product/file-format.md](specs/product/file-format.md) for complete specification.

---

## 🏗️ Architecture

```
src/
├── main.rs              # CLI entry point (clap derive), command dispatch
├── lib.rs               # Library root, re-exports all modules
├── error.rs             # FqcError enum (11 variants) + ExitCode mapping (0-5)
├── types.rs             # Core types: ReadRecord, QualityMode, IdMode, PeLayout
├── format.rs            # FQC binary format: magic, GlobalHeader, BlockHeader, Footer
├── fqc_reader.rs        # Archive reader with block index + random access
├── fqc_writer.rs        # Archive writer with block index + finalize
├── reorder_map.rs       # Bidirectional read reorder map (ZigZag delta + varint)
├── algo/                # Compression algorithms
│   ├── block_compressor.rs  # ABC algorithm (consensus + delta) + Zstd codec
│   ├── dna.rs               # Shared DNA encoding tables + reverse complement
│   ├── global_analyzer.rs   # Minimizer-based global read reordering
│   ├── quality_compressor.rs # SCM order-1/2 arithmetic coding for quality
│   ├── id_compressor.rs      # ID tokenization + delta encoding
│   └── pe_optimizer.rs       # Paired-end complementarity optimization
├── commands/            # CLI commands
│   ├── compress.rs      # default / streaming / pipeline modes
│   ├── decompress.rs    # sequential / parallel / reorder / pipeline
│   ├── info.rs          # archive info
│   └── verify.rs        # integrity check
├── common/
│   └── memory_budget.rs # System memory detection, chunking
├── fastq/
│   └── parser.rs        # FASTQ parser, validation, PE, stats
├── io/
│   ├── async_io.rs           # Async read/write with buffer pool
│   └── compressed_stream.rs  # Feature-gated gz/bz2/xz/zst
└── pipeline/
    ├── mod.rs            # Shared types (PipelineControl, PipelineStats)
    ├── compression.rs    # 3-stage compression pipeline
    └── decompression.rs  # 3-stage decompression pipeline
```

---

## 🧪 Testing

```bash
# Run all 131 tests
cargo test --lib --tests

# Run specific test suite
cargo test --test test_algo         # 19 algorithm tests
cargo test --test test_dna          # 15 DNA utility tests
cargo test --test test_e2e          # 15 end-to-end tests
cargo test --test test_format       # 15 binary format tests
cargo test --test test_parser       # 19 parser tests
cargo test --test test_reorder_map  # 23 reorder map tests
cargo test --test test_roundtrip    # 14 round-trip tests
cargo test --test test_types        # 11 type/constant tests

# Lint and format
cargo clippy --all-targets          # Must pass with 0 warnings
cargo fmt --all -- --check          # Must pass
```

---

## 📚 Documentation

| Document | Description |
|----------|-------------|
| [Architecture](docs/architecture/index.md) | Project architecture and module structure |
| [Algorithms](docs/algorithms/index.md) | ABC, SCM, and reordering algorithms |
| [Format Spec](specs/product/file-format.md) | FQC binary format specification |
| [Development Guide](CONTRIBUTING.md) | Development guide and contribution process |

### Specifications (Spec-Driven Development)

Formal specifications are in `/specs`:

| Directory | Purpose |
|-----------|---------|
| [specs/product/](specs/product/) | Product feature definitions and acceptance criteria |
| [specs/rfc/](specs/rfc/) | Technical design documents (RFCs) |
| [specs/api/](specs/api/) | API interface definitions |
| [specs/testing/](specs/testing/) | BDD test specifications |

### Online Documentation

Full documentation: [https://lessup.github.io/fq-compressor-rust/](https://lessup.github.io/fq-compressor-rust/)

---

## 🤝 Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

- [Code of Conduct](CODE_OF_CONDUCT.md)
- [Development Guide](CONTRIBUTING.md#development-setup)
- [Pull Request Process](CONTRIBUTING.md#pull-request-process)

### Security

For security issues, please see [SECURITY.md](SECURITY.md) for responsible disclosure guidelines.

---

## 📄 License

This project is licensed under the [GNU General Public License v3.0](LICENSE).

---

## 🔗 Related Projects

- [fq-compressor](https://github.com/LessUp/fq-compressor) — Original C++ implementation
- [Spring](https://github.com/shubhamchandak94/Spring) — Reference ABC algorithm paper
