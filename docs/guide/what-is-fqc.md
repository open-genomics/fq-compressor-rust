# What is fqc?

**fqc** (fq-compressor-rust) is a high-performance FASTQ compressor written in Rust. It implements the [**ABC (Alignment-Based Compression)**](https://github.com/shubhamchandak94/Spring) algorithm for short reads and uses Zstd for medium/long reads.

## Why fqc?

### 🚀 Performance

- **Compression**: ~10 MB/s (default), ~12 MB/s (pipeline mode)
- **Decompression**: ~55 MB/s (default), ~60 MB/s (pipeline mode)
- **Compression Ratio**: 3.9x on Illumina paired-end data

### 🧬 Smart Algorithms

| Read Length | Algorithm | Compression Strategy |
|-------------|-----------|---------------------|
| Short (< 300bp) | **ABC** | Consensus + Delta encoding with global reordering |
| Medium (300bp – 10kbp) | **Zstd** | Direct compression with SCM Order-2 |
| Long (> 10kbp) | **Zstd** | Direct compression with SCM Order-1 |

### 💾 Quality Compression

Statistical Context Model (SCM) with arithmetic coding:
- **Order-2 context** for short/medium reads
- **Order-1 context** for long reads
- Three modes: `lossless`, `illumina8bin`, `discard`

## Key Features

- **Random Access**: Block-indexed format for partial decompression
- **Streaming Mode**: Low-memory compression from stdin
- **Pipeline Mode**: 3-stage parallel processing with backpressure
- **Paired-End Support**: Interleaved or separate files
- **Compressed Input**: Transparent decompression of `.gz`, `.bz2`, `.xz`, `.zst`
- **Memory Budget**: Auto-detect system memory with dynamic chunking

## Compatibility

fqc shares the `.fqc` archive format with the original [C++ fq-compressor](https://github.com/LessUp/fq-compressor), ensuring cross-compatibility between implementations.

## Next Steps

- [Installation](./installation.md) - Get fqc installed
- [Quick Start](./quick-start.md) - Your first compression
- [Architecture](/architecture/) - Deep dive into design
