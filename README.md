# fqc - High-Performance FASTQ Compressor

<div align="center">

[![CI](https://github.com/LessUp/fq-compressor-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/LessUp/fq-compressor-rust/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/LessUp/fq-compressor-rust?include_prereleases&label=release&color=blue)](https://github.com/LessUp/fq-compressor-rust/releases)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-green.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![MSRV](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Docs](https://img.shields.io/badge/docs-VitePress-52b3f0.svg)](https://lessup.github.io/fq-compressor-rust/)
[![Crates.io](https://img.shields.io/crates/v/fqc.svg)](https://crates.io/crates/fqc)
[![Downloads](https://img.shields.io/github/downloads/LessUp/fq-compressor-rust/total?label=downloads)](https://github.com/LessUp/fq-compressor-rust/releases)

**A high-performance FASTQ compressor written in Rust**  
Featuring the **ABC** algorithm for short reads and **Zstd** for long reads

[Features](#-features) • [Quick Start](#-quick-start) • [Installation](#-installation) • [Documentation](#-documentation) • [Contributing](#-contributing)

[English](README.md) | [简体中文](docs/zh/README.md)

</div>

---

## 🎯 Overview

**fqc** compresses genomic sequencing data (FASTQ format) with:

- ⚡ **3.9x compression ratio** — 75% smaller files
- 🚀 **Parallel processing** — 3-stage pipeline, multi-core support
- 🔒 **Memory safe** — Zero `unsafe` code, Rust guarantees
- 📦 **Block-indexed** — Random access, partial decompression
- 🌐 **Cross-platform** — Linux, macOS, Windows

Compatible with the [fq-compressor](https://github.com/LessUp/fq-compressor) C++ implementation's `.fqc` format.

---

## ✨ Features

### Compression Algorithms

| Algorithm | Use Case | Compression | Speed |
|-----------|----------|-------------|-------|
| **ABC** (Alignment-Based) | Short reads (< 300bp) | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| **Zstd** | Medium/long reads (> 300bp) | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **SCM** (Statistical Context Model) | Quality scores | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |

### Key Capabilities

<details open>
<summary><b>Core Features</b></summary>

- ✅ **ABC Algorithm** — Consensus + delta encoding for short reads
- ✅ **Zstd Compression** — Length-prefixed encoding for long reads
- ✅ **SCM Quality Compression** — Arithmetic coding with context models
- ✅ **Global Read Reordering** — Minimizer-based optimization
- ✅ **Random Access** — Block-indexed archive format
- ✅ **Parallel Processing** — Rayon-based multi-core compression

</details>

<details>
<summary><b>Advanced Features</b></summary>

- 🔄 **Pipeline Mode** — 3-stage Reader→Compressor→Writer with backpressure
- 🌊 **Streaming Mode** — Low-memory compression from stdin
- 📊 **Lossless & Lossy** — Quality score preservation options
- 📁 **Compressed Input** — Auto-decompress `.gz`, `.bz2`, `.xz`, `.zst`
- 🔀 **Paired-End Support** — Interleaved and separate file modes
- 💾 **Memory Budget** — Auto-detect system memory, dynamic chunking

</details>

---

## 📊 Performance

### Benchmark Results

Tested on Intel Core i7-9700 @ 3.00GHz (8 cores), 2.27M Illumina reads (511 MB):

| Mode | Compression | Decompression | Ratio | Memory |
|------|-------------|---------------|-------|--------|
| **Default** | ~10 MB/s | ~55 MB/s | **3.9x** | ~2 GB |
| **Pipeline** | ~12 MB/s | ~60 MB/s | **3.9x** | ~3 GB |
| **Streaming** | ~8 MB/s | ~50 MB/s | **3.5x** | ~200 MB |

### Comparison with Other Tools

| Tool | Compression | Speed | Ratio | Memory Safe |
|------|-------------|-------|-------|-------------|
| **fqc** (this project) | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | **3.9x** | ✅ Yes |
| fqzcomp | ⭐⭐⭐ | ⭐⭐⭐ | 3.5x | ❌ No |
| Spring | ⭐⭐⭐⭐ | ⭐⭐⭐ | 3.8x | ❌ No |
| gzip | ⭐⭐ | ⭐⭐ | 2.5x | N/A |

### Compression Strategies

| Read Length | Sequence Codec | Quality Codec | Reordering |
|-------------|----------------|---------------|------------|
| Short (< 300bp) | ABC (consensus + delta) | SCM Order-2 | ✅ Yes |
| Medium (300bp – 10kbp) | Zstd | SCM Order-2 | ❌ No |
| Long (> 10kbp) | Zstd | SCM Order-1 | ❌ No |

---

## 🚀 Quick Start

### 1. Install

```bash
# From source (requires Rust 1.75+)
git clone https://github.com/LessUp/fq-compressor-rust.git
cd fq-compressor-rust
cargo build --release
sudo mv target/release/fqc /usr/local/bin/

# Or download pre-built binaries from GitHub Releases
```

### 2. Compress

```bash
# Basic compression
fqc compress -i reads.fastq -o reads.fqc

# Maximum compression
fqc compress -i reads.fastq -o reads.fqc -l 9

# Pipeline mode (fastest)
fqc compress -i reads.fastq -o reads.fqc --pipeline

# Low memory mode
fqc compress -i reads.fastq -o reads.fqc --streaming
```

### 3. Decompress

```bash
# Full decompression
fqc decompress -i reads.fqc -o reads.fastq

# Extract subset of reads
fqc decompress -i reads.fqc -o subset.fastq --range 1:1000

# Split paired-end files
fqc decompress -i reads.fqc -o output.fastq --split-pe
```

### 4. Verify & Inspect

```bash
# Check archive integrity
fqc verify -i reads.fqc

# View archive info
fqc info -i reads.fqc --detailed
```

---

## 📦 Installation

### Package Managers

| Platform | Command | Notes |
|----------|---------|-------|
| **Cargo** | `cargo install fqc` | Requires Rust 1.75+ |
| **Homebrew** (macOS) | `brew install fqc` | Coming soon |
| **APT** (Ubuntu) | Download `.deb` from Releases | Debian/Ubuntu |

### Pre-built Binaries

Download from [GitHub Releases](https://github.com/LessUp/fq-compressor-rust/releases):

| Platform | Architectures | Format |
|----------|---------------|--------|
| **Linux** | x86_64, ARM64 | `.tar.gz`, `.deb` |
| **macOS** | Intel, Apple Silicon | `.tar.gz` |
| **Windows** | x86_64 | `.zip` |

### Docker

```bash
# Pull from GitHub Container Registry
docker pull ghcr.io/lessup/fq-compressor-rust:latest

# Or build locally
docker build -t fqc .

# Run (mount data directory)
docker run --rm -v $(pwd):/data fqc \
  compress -i /data/reads.fastq -o /data/reads.fqc
```

### Build from Source

**Requirements:**
- Rust 1.75.0 or later
- zstd library (`libzstd-dev` on Ubuntu)

```bash
# Clone repository
git clone https://github.com/LessUp/fq-compressor-rust.git
cd fq-compressor-rust

# Build release version (optimized)
cargo build --release

# Run
./target/release/fqc --help
```

**Optional Features:**

```bash
# Enable gzip/bzip2/xz support
cargo build --release --features gz,bz2,xz

# Static binary (Linux)
cargo build --release --target x86_64-unknown-linux-musl
```

---

## 💡 Usage Examples

### Common Workflows

#### Single-End Sequencing

```bash
# Compress
fqc compress -i sample.fastq -o sample.fqc --pipeline

# Verify
fqc verify -i sample.fqc

# Decompress
fqc decompress -i sample.fqc -o sample_out.fastq
```

#### Paired-End Sequencing

```bash
# Separate files
fqc compress -i sample_R1.fastq -2 sample_R2.fastq -o paired.fqc

# Interleaved file
fqc compress -i interleaved.fastq -o paired.fqc --interleaved

# Decompress and split
fqc decompress -i paired.fqc -o output.fastq --split-pe
```

#### Large Files (> 10GB)

```bash
# Streaming mode (low memory)
fqc compress -i huge.fastq -o huge.fqc --streaming

# Or pipeline with limited threads
fqc compress -i huge.fastq -o huge.fqc --pipeline --threads 8
```

#### Compressed Input

```bash
# Auto-detect and decompress input
fqc compress -i reads.fastq.gz -o reads.fqc
fqc compress -i reads.fastq.bz2 -o reads.fqc --features bz2
```

### Advanced Options

#### Quality Score Control

```bash
# Lossless (default)
fqc compress -i reads.fastq -o reads.fqc --quality-mode lossless

# Lossy (smaller, ~95% fidelity)
fqc compress -i reads.fastq -o reads.fqc --quality-mode lossy

# Discard quality (smallest)
fqc compress -i reads.fastq -o reads.fqc --quality-mode discard
```

#### Partial Decompression

```bash
# Extract reads 100-200 (1-based, inclusive)
fqc decompress -i archive.fqc -o subset.fastq --range 100:200

# Extract from read 1000 to end
fqc decompress -i archive.fqc -o subset.fastq --range 1000:

# Headers only (IDs)
fqc decompress -i archive.fqc -o headers.txt --header-only
```

#### Read Order

```bash
# Restore original order (if archive was reordered)
fqc decompress -i archive.fqc -o ordered.fastq --original-order

# Skip corrupted blocks
fqc decompress -i archive.fqc -o output.fastq --skip-corrupted
```

### Info & Verification

```bash
# Basic info
fqc info -i reads.fqc

# JSON output (for scripts)
fqc info -i reads.fqc --json

# Detailed block-level info
fqc info -i reads.fqc --detailed

# Show codecs used per block
fqc info -i reads.fqc --show-codecs

# Verify integrity
fqc verify -i reads.fqc

# Verbose verification (per-block)
fqc verify -i reads.fqc --verbose

# Quick check (header + footer only)
fqc verify -i reads.fqc --quick
```

---

## 🗂️ FQC File Format

```
┌─────────────────────┐
│   Magic Header (9B) │  "\x89FQC\r\n\x1a\n" + version
├─────────────────────┤
│   Global Header     │  Flags, read count, filename, timestamp
├─────────────────────┤
│   Block 0           │  Block header + IDs + Sequences + Quality
├─────────────────────┤
│   Block 1           │  ...
├─────────────────────┤
│   ...               │
├─────────────────────┤
│   Reorder Map (opt) │  Forward + reverse maps (delta + varint)
├─────────────────────┤
│   Block Index       │  Offsets for random access
├─────────────────────┤
│   File Footer (32B) │  Index offset, checksum, magic tail
└─────────────────────┘
```

See [Format Specification](specs/product/file-format.md) for complete details.

---

## 📚 Documentation

### User Guides

| Guide | Description | Link |
|-------|-------------|------|
| **Getting Started** | What is FQC, installation, quick start | [Guide](docs/guide/) |
| **CLI Reference** | All commands with options | [CLI](docs/guide/cli/) |
| **Features** | Streaming, pipeline, paired-end | [Features](docs/guide/features/) |
| **Performance** | Benchmarks and tuning | [Performance](docs/guide/performance/) |
| **Algorithms** | ABC, SCM, Zstd deep dive | [Algorithms](docs/algorithms/) |
| **Architecture** | Module structure, data flow | [Architecture](docs/architecture/) |
| **Changelog** | Release notes | [Changelog](docs/changelog/) |

### Online Documentation

🌐 **VitePress Site**: [https://lessup.github.io/fq-compressor-rust/](https://lessup.github.io/fq-compressor-rust/)

### Specifications (SDD)

This project follows **Spec-Driven Development** ([AGENTS.md](AGENTS.md)):

| Spec Type | Location | Purpose |
|-----------|----------|---------|
| **Product** | [specs/product/](specs/product/) | Feature definitions |
| **RFCs** | [specs/rfc/](specs/rfc/) | Technical decisions |
| **API** | [specs/api/](specs/api/) | Interface definitions |
| **Testing** | [specs/testing/](specs/testing/) | BDD test cases |

---

## 🧪 Testing

```bash
# Run all tests (131 tests)
cargo test --lib --tests

# Run specific test suite
cargo test --test test_roundtrip    # Round-trip tests
cargo test --test test_algo         # Algorithm tests
cargo test --test test_e2e          # End-to-end tests

# Lint and format check
cargo clippy --all-targets          # Must pass with 0 warnings
cargo fmt --all -- --check          # Formatting check
```

**Test Coverage:**

| Suite | Tests | Coverage |
|-------|-------|----------|
| Algorithms | 19 | ID/quality compressor, PE optimizer |
| DNA Utilities | 15 | Encoding tables, reverse complement |
| End-to-End | 15 | Full compression workflows |
| Binary Format | 15 | Header/footer validation |
| FASTQ Parser | 19 | Parser edge cases |
| Reorder Map | 23 | Map operations |
| Round-Trip | 14 | Compress → decompress → compare |
| Types | 11 | Constants and validation |
| **Total** | **131** | **0 failures** |

---

## 🏗️ Architecture

```
src/
├── main.rs              # CLI entry point (clap)
├── lib.rs               # Library root
├── types.rs             # Core types (ReadRecord, etc.)
├── error.rs             # Error handling (11 variants)
├── format.rs            # FQC binary format
├── fqc_reader.rs        # Archive reader + random access
├── fqc_writer.rs        # Archive writer
├── reorder_map.rs       # Read reordering (ZigZag + varint)
├── algo/                # Compression algorithms
│   ├── block_compressor.rs   # ABC + Zstd
│   ├── dna.rs                # DNA encoding tables
│   ├── global_analyzer.rs    # Minimizer reordering
│   ├── quality_compressor.rs # SCM arithmetic coding
│   ├── id_compressor.rs      # ID compression
│   └── pe_optimizer.rs       # Paired-end optimization
├── commands/            # CLI commands
│   ├── compress.rs
│   ├── decompress.rs
│   ├── info.rs
│   └── verify.rs
├── pipeline/            # 3-stage pipelines
│   ├── compression.rs
│   └── decompression.rs
├── fastq/               # FASTQ parser
│   └── parser.rs
├── io/                  # I/O operations
│   ├── async_io.rs
│   └── compressed_stream.rs
└── common/              # Shared utilities
    └── memory_budget.rs
```

---

## ❓ Troubleshooting

### Common Issues

**Build fails with "could not find system library 'zstd'"**

```bash
# Ubuntu/Debian
sudo apt install libzstd-dev

# macOS
brew install zstd

# Fedora
sudo dnf install zstd-devel
```

**Outdated Rust version**

```bash
rustup update
```

**Permission denied when building**

```bash
chmod -R 755 target/
```

### Getting Help

- 📖 **Documentation**: [VitePress Site](https://lessup.github.io/fq-compressor-rust/)
- 🐛 **Bug Reports**: [GitHub Issues](https://github.com/LessUp/fq-compressor-rust/issues)
- 💬 **Discussions**: [GitHub Discussions](https://github.com/LessUp/fq-compressor-rust/discussions)
- 📧 **Security Issues**: [SECURITY.md](SECURITY.md)

---

## 🤝 Contributing

We welcome contributions! Here's how to get started:

1. **Fork** the repository
2. **Create** a feature branch (`git checkout -b feature/my-feature`)
3. **Make** your changes
4. **Test** your changes (`cargo test --lib --tests`)
5. **Lint** your code (`cargo clippy --all-targets`)
6. **Format** your code (`cargo fmt --all`)
7. **Commit** with a clear message (`git commit -m "feat: add my feature"`)
8. **Push** and open a **Pull Request**

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.

### Code of Conduct

Please read our [Code of Conduct](CODE_OF_CONDUCT.md) before participating.

---

## 📄 License

This project is licensed under the [GNU General Public License v3.0](LICENSE).

```
fqc - High-Performance FASTQ Compressor
Copyright (C) 2024 fqc contributors

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.
```

---

## 🙏 Acknowledgments

- **Spring** — Original ABC algorithm paper
- **zstd** — Facebook's Zstandard compression library
- **fq-compressor** — Original C++ implementation

---

## 📈 Project Stats

- **Lines of Code**: ~15,000 Rust
- **Test Coverage**: 131 tests, 0 failures
- **Dependencies**: 15 crates (minimal)
- **Unsafe Code**: 0 lines (`unsafe_code = "deny"`)
- **MSRV**: Rust 1.75.0

---

<div align="center">

**Made with ❤️ by the fqc contributors**

[⭐ Star this repo](https://github.com/LessUp/fq-compressor-rust) • [🐛 Report issue](https://github.com/LessUp/fq-compressor-rust/issues) • [💬 Discuss](https://github.com/LessUp/fq-compressor-rust/discussions)

</div>
