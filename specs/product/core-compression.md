# Core Compression Functionality

**Status**: ✅ Implemented  
**Version**: 1.0

## Overview

fqc provides high-performance compression for FASTQ sequencing data using domain-specific algorithms optimized for different read lengths.

## Features

### 1. ABC Algorithm (Short Reads < 300bp)

**Description**: Consensus-based delta encoding algorithm

**Acceptance Criteria**:
- ✅ Computes consensus sequence from read population
- ✅ Encodes reads as delta from consensus
- ✅ Achieves better compression than general-purpose algorithms for short reads
- ✅ Handles variable-length reads within block

**Performance Target**:
- Compression ratio: ≥ 4x for Illumina short reads
- Speed: ≥ 10 MB/s on modern hardware

### 2. Zstd Compression (Medium/Long Reads ≥ 300bp)

**Description**: General-purpose compression with length-prefix encoding

**Acceptance Criteria**:
- ✅ Uses zstd library for compression
- ✅ Length-prefixed encoding for variable-length reads
- ✅ Configurable compression level (1-9)
- ✅ Handles reads up to megabase length

**Performance Target**:
- Compression ratio: ≥ 3x for long reads
- Speed: ≥ 50 MB/s on modern hardware

### 3. SCM Quality Score Compression

**Description**: Statistical Context Model with arithmetic coding

**Modes**:
- ✅ **Lossless**: Full quality score preservation
- ✅ **Illumina 8-bin**: Bin quality scores to 8 levels
- ✅ **Discard**: Remove quality scores entirely (smallest output)

**Acceptance Criteria**:
- ✅ Order-2 context model for short reads
- ✅ Order-1 context model for long reads
- ✅ Arithmetic coding achieves entropy-limit compression

### 4. ID Compression

**Description**: Tokenization and delta encoding for read identifiers

**Acceptance Criteria**:
- ✅ Tokenizes repetitive ID patterns
- ✅ Delta encodes numeric components
- ✅ Handles comments (preserved or stripped per mode)

### 5. Paired-End Optimization

**Description**: Exploits complementarity between paired reads

**Acceptance Criteria**:
- ✅ Detects paired-end data
- ✅ Optimizes storage of complementary sequences
- ✅ Supports interleaved and separate file layouts

### 6. Global Read Reordering

**Description**: Minimizer-based reordering to improve compression

**Acceptance Criteria**:
- ✅ Computes minimizers for each read
- ✅ Reorders reads to maximize locality
- ✅ Stores bidirectional reorder map for restoration
- ✅ Map uses ZigZag delta + varint encoding

## Configuration Options

| Option | Type | Description |
|--------|------|-------------|
| Compression level | 1-9 | Zstd compression level |
| Quality mode | lossless/illumina8/discard | Quality score handling |
| ID mode | exact/strip-comment/discard | ID preservation level |
| Block size | auto/custom | Number of reads per block |
| Pipeline mode | bool | Enable 3-stage parallel pipeline |

## Error Handling

| Error Condition | Exit Code | Message |
|-----------------|-----------|---------|
| Input file not found | 1 | "File not found: {path}" |
| Invalid FASTQ format | 2 | "Invalid FASTQ format at line {n}" |
| Compression failure | 3 | "Compression failed: {reason}" |
| I/O error | 4 | "I/O error: {reason}" |

## Related Documents

- [CLI Commands Spec](./cli-commands.md)
- [File Format Spec](./file-format.md)
- [Compression Algorithms RFC](../rfc/0002-compression-algorithms.md)
