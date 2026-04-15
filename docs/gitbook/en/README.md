# fqc (fq-compressor-rust)

[![CI](https://github.com/LessUp/fq-compressor-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/LessUp/fq-compressor-rust/actions/workflows/ci.yml)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

**fqc** is a high-performance FASTQ compressor written in Rust, featuring the ABC (Alignment-Based Compression) algorithm for short reads and Zstd for medium/long reads.

> This is the Rust implementation of [fq-compressor](https://lessup.github.io/fq-compressor/). Both versions share the same `.fqc` archive format and ABC/SCM compression algorithms. The Rust version uses Rayon + crossbeam instead of Intel TBB, and introduces async I/O.

## Key Features

| Feature | Description |
|---------|-------------|
| **ABC Algorithm** | Consensus-based delta encoding for short reads (< 300bp) |
| **Zstd Compression** | Length-prefixed encoding for medium/long reads |
| **SCM Quality** | Statistical Context Model with arithmetic coding |
| **Global Reordering** | Minimizer-based read reordering to improve compression |
| **Random Access** | Block-indexed archive format for partial decompression |
| **Pipeline Mode** | 3-stage Reader→Compressor→Writer with backpressure |
| **Async I/O** | Background prefetch and write-behind buffering |
| **Compressed Input** | Transparent `.gz`, `.bz2`, `.xz`, `.zst` decompression |
| **Paired-End** | Interleaved and separate-file PE support |
| **Memory Budget** | Auto-detect system memory, dynamic chunking |

## Performance at a Glance

| Mode | Compression | Decompression | Ratio |
|------|-------------|---------------|-------|
| Default | ~10 MB/s | ~55 MB/s | 3.9x |
| Pipeline | ~12 MB/s | ~60 MB/s | 3.9x |

*Tested on Intel Core i7-9700 @ 3.00GHz (8 cores), 2.27M Illumina reads (511 MB uncompressed)*

## Quick Start

```bash
# Install from source
cargo build --release

# Compress
fqc compress -i reads.fastq -o reads.fqc

# Decompress
fqc decompress -i reads.fqc -o reads.fastq

# View archive info
fqc info -i reads.fqc

# Verify integrity
fqc verify -i reads.fqc
```

## Documentation Index

- [Installation](installation.md) — Build from source or Docker
- [Quick Start](quickstart.md) — Compress your first FASTQ file
- [CLI Reference](cli-reference.md) — All commands and options
- [Architecture](architecture.md) — How it works under the hood
  - [Core Algorithms](algorithms.md) — ABC, SCM, ID compression
  - [FQC File Format](format-spec.md) — Binary format specification
  - [Parallel Pipeline](pipeline.md) — 3-stage pipeline design
- [Performance Tuning](performance.md) — Optimize for your workload
- [Development Guide](development.md) — Contribute to fqc
- [FAQ](faq.md) — Common questions and answers

## License

GNU General Public License v3.0 — see [LICENSE](https://github.com/LessUp/fq-compressor-rust/blob/main/LICENSE).
