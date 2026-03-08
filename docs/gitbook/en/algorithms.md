# Core Algorithms

> See also: [Architecture](architecture.md), [FQC Format](format-spec.md)

## Overview

fqc selects compression strategies based on read length:

| Read Length | Sequence Codec | Quality Codec | Global Reorder |
|-------------|---------------|---------------|----------------|
| Short (< 300bp) | ABC (consensus + delta) | SCM Order-2 | Yes (minimizer) |
| Medium (300bp – 10kbp) | Zstd | SCM Order-2 | No |
| Long (> 10kbp) | Zstd | SCM Order-1 | No |

## ABC Algorithm (Alignment-Based Compression)

ABC is used for short reads (e.g., Illumina), exploiting high sequence similarity via consensus + delta encoding.

### Processing Flow

```
Reads → Global Sort → Block Partition → Per block:
  1. Build Contig (consensus + alignment)
  2. Delta Encode (store only differences)
  3. Serialize + Zstd compress
```

### Step 1: Consensus Building

Within each block, reads are clustered into **contigs** (aligned read clusters):

1. Pick an unassigned read as seed, initialize consensus with its sequence
2. For each remaining read, try alignment within `[-max_shift, +max_shift]` (forward + reverse complement)
3. If Hamming distance ≤ threshold, add read to contig, update base frequency counts
4. After all reads processed, recompute final consensus as majority base at each position

### Step 2: Delta Encoding

Each read in a contig is delta-encoded against the final consensus:

| Field | Type | Description |
|-------|------|-------------|
| `position_offset` | i16 | Alignment offset relative to consensus |
| `is_rc` | bool | Whether reverse complemented |
| `mismatch_positions` | Vec\<u16\> | Positions differing from consensus |
| `mismatch_chars` | Vec\<u8\> | Encoded difference bases |

### Step 3: Serialization

Each contig serializes as: consensus sequence (length-prefixed), delta count, then each delta with its metadata. The entire block is then Zstd-compressed.

## Global Read Reordering

Minimizer-based sorting groups similar reads together, improving ABC compression ratio.

### Algorithm

1. Extract **canonical k-mer minimizer** from each read (smaller of forward and reverse complement)
2. Sort all reads by minimizer value
3. Generate bidirectional `ReorderMap` (forward + reverse mapping) stored in archive

### ReorderMap Encoding

Uses **ZigZag delta + varint** encoding (`src/reorder_map.rs`):

```
delta = current_id - previous_id
zigzag = (delta << 1) ^ (delta >> 63)    // Map negatives to positives
varint: 7 bits/byte, MSB=1 indicates continuation
```

## SCM Quality Compression

Quality scores use **Statistical Context Model (SCM)** + arithmetic coding.

### Context Model

| Read Type | Context Order | Context Source |
|-----------|--------------|----------------|
| Short / Medium | Order-2 | Previous 2 quality values |
| Long | Order-1 | Previous 1 quality value |

### Quality Modes

| Mode | Description | Ratio Impact |
|------|-------------|-------------|
| Lossless | Exact quality preservation | Baseline |
| Illumina8Bin | Quantize to 8 representative values | ~30% better |
| Discard | Replace all with `!` (Phred 0) | Maximum |

## ID Compression

| Mode | Description | Use Case |
|------|-------------|----------|
| Exact | Preserve full ID | Exact ID matching needed |
| StripComment | Remove content after first space | General use |
| Discard | Replace with sequential `@read_N` | Maximum compression |

## Paired-End Optimization

Paired-end data exploits R1/R2 reverse complement relationship:

1. Take reverse complement of R2 sequence
2. Compare similarity with R1
3. If similarity > threshold, store only difference positions + bases (delta encoding)
