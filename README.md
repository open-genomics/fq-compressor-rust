# fqc - High-Performance FASTQ Compressor

[![Docs](https://img.shields.io/badge/Docs-GitHub%20Pages-blue?logo=github)](https://lessup.github.io/fq-compressor-rust/)

English | [简体中文](README.zh-CN.md) | [C++ Version (fq-compressor)](https://github.com/LessUp/fq-compressor)

> **fq-compressor** 的 Rust 实现，两个版本共享相同的 `.fqc` 归档格式与 ABC/SCM 压缩算法。
> Rust 版本以 Rayon + crossbeam 替代 Intel TBB，并引入异步 I/O。

A high-performance FASTQ compressor written in Rust, featuring the ABC (Alignment-Based Compression) algorithm for short reads and Zstd for medium/long reads.

## Features

- **ABC Algorithm** — Consensus-based delta encoding for short reads (< 300bp), achieving high compression ratios
- **Zstd Compression** — For medium/long reads with length-prefixed encoding
- **SCM Quality Compression** — Statistical Context Model with arithmetic coding for quality scores
- **Global Read Reordering** — Minimizer-based read reordering to improve compression
- **Random Access** — Block-indexed archive format for efficient partial decompression
- **Parallel Processing** — Rayon-based parallel block compression/decompression
- **Pipeline Mode** — 3-stage Reader→Compressor→Writer pipeline with backpressure (`--pipeline`)
- **Async I/O** — Background prefetch and write-behind for improved throughput
- **Streaming Mode** — Low-memory compression from stdin without global reordering
- **Lossless & Lossy** — Supports lossless, Illumina 8-bin, and discard quality modes
- **Compressed Input** — Transparent decompression of `.gz`, `.bz2`, `.xz`, `.zst` FASTQ files
- **Paired-End** — Interleaved and separate-file paired-end support
- **Memory Budget** — Auto-detect system memory, dynamic chunking for large datasets

## Installation

### From Source

```bash
cargo build --release
```

The binary will be at `target/release/fqc` (or `fqc.exe` on Windows).

### Docker

```bash
# Pull from GitHub Container Registry
docker pull ghcr.io/lessup/fq-compressor-rust:latest

# Or build locally
docker build -t fqc .

# Run (mount data directory)
docker run --rm -v $(pwd):/data fqc compress -i /data/reads.fastq -o /data/reads.fqc
docker run --rm -v $(pwd):/data fqc decompress -i /data/reads.fqc -o /data/reads.fastq
```

## Usage

### Compress

```bash
# Basic compression (auto-detects read length)
fqc compress -i reads.fastq -o reads.fqc

# Specify compression level (1-9)
fqc compress -i reads.fastq -o reads.fqc -l 9

# Compress from gzip input
fqc compress -i reads.fastq.gz -o reads.fqc

# Streaming mode (low memory, from stdin)
cat reads.fastq | fqc compress --streaming -i - -o reads.fqc

# Pipeline mode (3-stage parallel pipeline with backpressure)
fqc compress -i reads.fastq -o reads.fqc --pipeline

# Paired-end (separate files)
fqc compress -i reads_R1.fastq -2 reads_R2.fastq -o reads.fqc

# Paired-end (interleaved single file)
fqc compress -i interleaved.fastq -o reads.fqc --interleaved

# Discard quality scores
fqc compress -i reads.fastq -o reads.fqc --lossy-quality discard

# Force medium/long read mode
fqc compress -i long_reads.fastq -o reads.fqc --long-read-mode long

# Compressed input (auto-detected)
fqc compress -i reads.fastq.gz -o reads.fqc
fqc compress -i reads.fastq.bz2 -o reads.fqc
```

### Decompress

```bash
# Full decompression
fqc decompress -i reads.fqc -o reads.fastq

# Extract range of reads (1-based)
fqc decompress -i reads.fqc -o subset.fastq --range 1:1000

# Output to stdout
fqc decompress -i reads.fqc -o -

# Headers only
fqc decompress -i reads.fqc -o headers.txt --header-only
```

### Info

```bash
# Human-readable summary
fqc info -i reads.fqc

# JSON output
fqc info -i reads.fqc --json

# Detailed block index
fqc info -i reads.fqc --detailed
```

### Verify

```bash
# Verify archive integrity
fqc verify -i reads.fqc

# Verbose verification
fqc verify -i reads.fqc --verbose
```

## FQC File Format

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

## Compression Strategies

| Read Length | Sequence Codec | Quality Codec | Reordering |
|-------------|---------------|---------------|------------|
| Short (<300bp) | ABC (consensus + delta) | SCM Order-2 | Yes |
| Medium (300bp-10kbp) | Zstd | SCM Order-2 | No |
| Long (>10kbp) | Zstd | SCM Order-1 | No |

## Architecture

```
src/
├── algo/                   # Compression algorithms
│   ├── block_compressor.rs # ABC + Zstd block compression/decompression
│   ├── dna.rs              # Shared DNA encoding tables + reverse complement
│   ├── global_analyzer.rs  # Minimizer-based read reordering
│   ├── quality_compressor.rs # SCM arithmetic coding for quality scores
│   └── pe_optimizer.rs     # Paired-end complementarity optimization
├── commands/               # CLI command implementations
│   ├── compress.rs         # Compress command (default + streaming + pipeline)
│   ├── decompress.rs       # Decompress command (sequential + parallel + reorder)
│   ├── info.rs             # Archive info display
│   └── verify.rs           # Integrity verification
├── common/
│   └── memory_budget.rs    # System memory detection, dynamic chunking
├── fastq/
│   └── parser.rs           # FASTQ parser (stats, validation, PE, chunk reading)
├── io/
│   ├── async_io.rs         # AsyncReader/AsyncWriter with prefetch/write-behind
│   └── compressed_stream.rs# Transparent gz/bz2/xz/zst decompression
├── pipeline/
│   ├── compression.rs      # 3-stage compression pipeline (crossbeam channels)
│   └── decompression.rs    # 3-stage decompression pipeline
├── error.rs                # FqcError enum + ExitCode mapping (0-5)
├── format.rs               # FQC binary format structures
├── fqc_reader.rs           # Archive reader with random access
├── fqc_writer.rs           # Archive writer with block index
├── reorder_map.rs          # Bidirectional read reorder map (ZigZag varint)
└── types.rs                # Core types and constants
```

## Testing

```bash
# Run all 131 tests
cargo test

# Run specific test suite
cargo test --test test_algo         # 19 algorithm tests (ID/quality compressor, PE optimizer)
cargo test --test test_dna          # 15 DNA utility tests
cargo test --test test_e2e          # 15 end-to-end tests
cargo test --test test_format       # 15 format tests
cargo test --test test_parser       # 19 parser tests
cargo test --test test_reorder_map  # 23 reorder map tests
cargo test --test test_roundtrip    # 14 round-trip compression tests
cargo test --test test_types        # 11 type tests
```

## License

See LICENSE file.
