# fqc - High-Performance FASTQ Compressor

A Rust implementation of the FQC compressor for FASTQ files, featuring the ABC (Alignment-Based Compression) algorithm for short reads and Zstd for medium/long reads.

## Features

- **ABC Algorithm** — Consensus-based delta encoding for short reads (< 300bp), achieving high compression ratios
- **Zstd Compression** — For medium/long reads with length-prefixed encoding
- **SCM Quality Compression** — Statistical Context Model with arithmetic coding for quality scores
- **Global Read Reordering** — Minimizer-based read reordering to improve compression
- **Random Access** — Block-indexed archive format for efficient partial decompression
- **Parallel Processing** — Rayon-based parallel block compression/decompression
- **Streaming Mode** — Low-memory compression from stdin without global reordering
- **Lossless & Lossy** — Supports lossless, Illumina 8-bin, and discard quality modes
- **Gzip Input** — Transparent decompression of `.gz` FASTQ files

## Installation

```bash
cargo build --release
```

The binary will be at `target/release/fqc` (or `fqc.exe` on Windows).

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
cat reads.fastq | fqc --streaming compress -i - -o reads.fqc

# Discard quality scores
fqc compress -i reads.fastq -o reads.fqc --lossy-quality discard

# Force medium/long read mode
fqc compress -i long_reads.fastq -o reads.fqc --long-read-mode long
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

## License

See LICENSE file.
