# Compression Algorithms

> See also: [architecture.md](architecture.md) (module structure), [format-spec.md](format-spec.md) (binary format)

## Overview

fqc selects different compression strategies based on read length classification:

| Read Length | Sequence Codec | Quality Codec | Global Reordering |
|-------------|----------------|---------------|-------------------|
| Short (< 300bp) | ABC (Consensus + Delta) | SCM Order-2 | ✓ Yes (minimizer) |
| Medium (300bp – 10kbp) | Zstd | SCM Order-2 | ✗ No |
| Long (> 10kbp) | Zstd | SCM Order-1 | ✗ No |

Implementation is located in the `src/algo/` directory:

| Module | Responsibility |
|--------|----------------|
| `block_compressor.rs` | Block-level ABC/Zstd compression and decompression |
| `global_analyzer.rs` | Minimizer extraction and global ordering |
| `quality_compressor.rs` | SCM arithmetic coding for quality scores |
| `id_compressor.rs` | Read ID compression |
| `pe_optimizer.rs` | Paired-end reverse complement optimization |

---

## ABC Algorithm (Alignment-Based Compression)

ABC is used for short reads (e.g., Illumina), leveraging high sequence similarity for consensus + delta encoding.

### Processing Pipeline

```
Read → Global Ordering → Block Partition → Per Block:
  1. Build Contig (consensus + alignment)
  2. Delta encoding (store differences only)
  3. Serialization + Zstd compression
```

### Step 1: Consensus Building

Within each block, reads are clustered into **contigs** (clusters of aligned reads):

1. Select an unassigned read as seed, initialize consensus with its sequence
2. For each remaining read, attempt alignment in `[-max_shift, +max_shift]` range (forward + reverse complement)
3. If Hamming distance ≤ threshold, add read to contig, update base frequency counts
4. After all reads processed, recompute final consensus using majority base at each position

**Consensus** is stored as per-position base frequencies `[A, C, G, T]`, with final consensus being the majority base.

### Step 2: Delta Encoding

Each read in a contig is delta-encoded relative to the final consensus:

| Field | Type | Description |
|-------|------|-------------|
| `position_offset` | i16 | Alignment offset relative to consensus |
| `is_rc` | bool | Whether reverse complimented |
| `mismatch_positions` | Vec\<u16\> | Positions differing from consensus |
| `mismatch_chars` | Vec\<u8\> | Encoded difference bases |

Difference base encoding rules:
- Positions within overlap: noise encoding (XOR-like)
- Positions outside consensus range: raw bases

### Step 3: Serialization

Each contig is serialized as:

1. Consensus sequence (length-prefixed)
2. Delta count
3. Each delta: original_order, offset, is_rc, read_length, mismatch count, positions, chars

The entire block is then compressed with Zstd.

### Negative Offset Handling

When `shift < 0`, the read extends before the consensus start position:

- `cons_start = 0`, `read_start = |shift|`
- Bases before overlap (positions `0..read_start`) are stored as raw bases in mismatch data
- Decompressed directly (no noise decoding)

---

## Global Read Reordering

For short reads, minimizer ordering clusters similar reads together, improving ABC compression ratio.

Implementation is in `src/algo/global_analyzer.rs`.

### Algorithm

1. Extract **canonical k-mer minimizer** from each read (smaller of forward and reverse complement)
2. Sort all reads by minimizer value
3. Generate bidirectional `ReorderMap` (forward + reverse mapping) storing in archive

### ReorderMap Encoding

Reordering mapping uses **ZigZag delta + varint** encoding (`src/reorder_map.rs`):

1. Compute differences (delta) between adjacent IDs
2. ZigZag encoding handles negatives: `(n << 1) ^ (n >> 63)`
3. Unsigned varint encoding for compression

---

## SCM Quality Compression

Quality scores are compressed using **Statistical Context Model (SCM)** + arithmetic coding.

Implementation is in `src/algo/quality_compressor.rs`.

### Context Model

| Read Type | Context Order | Context Source |
|-----------|---------------|----------------|
| Short / Medium | Order-2 | Previous 2 quality values |
| Long reads | Order-1 | Previous 1 quality value |

### Arithmetic Coding

- Each context maintains an adaptive frequency model
- Frequency rescaling when total exceeds threshold
- 32-bit precision arithmetic encoder/decoder

### Quality Modes

| Mode | Description | Compression Impact |
|------|-------------|-------------------|
| Lossless | Exact quality value preservation | Baseline |
| Illumina8Bin | Quantized to 8 bins (2,6,15,22,27,33,37,40) | ~30% improvement |
| Discard | All replaced with `!` (Phred 0) | Maximum |

---

## ID Compression

Read identifiers are compressed based on ID mode.

Implementation is in `src/algo/id_compressor.rs`.

| Mode | Description | Typical Use Case |
|------|-------------|------------------|
| Exact | Preserve ID exactly | Requires exact ID matching |
| StripComment | Remove content after first space | General use |
| Discard | Replace with sequence number `@read_N` | Maximum compression |

IDs are concatenated newline-separated and compressed with Zstd as a single data stream.

---

## Zstd Codec (Medium/Long Reads)

For reads > 300bp, sequences use length-prefixed encoding followed by Zstd compression:

```
[u16: read_length][sequence bytes] × N reads
```

Zstd compression level is configurable (1-19, default 3).

---

## Paired-End (PE) Optimization

Paired-end data leverages R1/R2 reverse complement relationships for optimized compression.

Implementation is in `src/algo/pe_optimizer.rs`.

### Algorithm

1. Reverse complement R2 sequence
2. Compare similarity with R1
3. If similarity > threshold, only store difference positions + bases (delta encoding)
4. Quality differences processed similarly

### PE Storage Layout

| Layout | Storage | Description |
|--------|---------|-------------|
| Interleaved | R1, R2, R1, R2, ... | Alternating read pairs |
| Consecutive | R1, R1, ..., R2, R2, ... | All R1 then all R2 |
