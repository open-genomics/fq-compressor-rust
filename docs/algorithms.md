# Compression Algorithms

## Overview

fqc uses different compression strategies depending on read length class:

| Read Length | Sequence Codec | Quality Codec | Reordering |
|-------------|---------------|---------------|------------|
| Short (< 300bp) | ABC (consensus + delta) | SCM Order-2 | Yes (minimizer-based) |
| Medium (300bp – 10kbp) | Zstd | SCM Order-2 | No |
| Long (> 10kbp) | Zstd | SCM Order-1 | No |

## ABC Algorithm (Alignment-Based Compression)

ABC is used for short reads (e.g. Illumina) where high sequence similarity enables delta encoding against a consensus.

### Pipeline

```
Reads → Global Reorder → Block Partition → Per-Block:
  1. Build Contigs (consensus + alignment)
  2. Delta Encode (mismatches only)
  3. Serialize + Zstd compress
```

### Step 1: Consensus Building

For each block, reads are grouped into **contigs** — clusters of aligned reads:

1. Pick an unassigned read as seed; initialize consensus from its sequence
2. For each remaining unassigned read, try alignment at shifts `[-max_shift, +max_shift]` (both forward and reverse complement)
3. If Hamming distance ≤ threshold, add read to contig and update consensus base counts
4. After all reads assigned (or tried), recompute final consensus from majority base at each position

**Consensus** is stored as the majority base at each position using per-position base frequency counts (`[A, C, G, T]`).

### Step 2: Delta Encoding

Each read in a contig is delta-encoded against the final consensus:

- **position_offset** (i16): alignment shift relative to consensus
- **is_rc** (bool): whether read was reverse-complemented
- **mismatch_positions** (Vec<u16>): positions where read differs from consensus
- **mismatch_chars** (Vec<u8>): encoded mismatch characters
  - For positions overlapping consensus: noise-encoded (`XOR`-like encoding)
  - For positions outside consensus: raw base

### Step 3: Serialization

Per contig:
1. Consensus sequence (length-prefixed)
2. Delta count
3. For each delta: original_order, offset, is_rc, read_length, mismatch count, positions, chars

The entire block is then Zstd-compressed.

### Negative Shift Handling

When `shift < 0`, the read extends before the consensus start:
- `cons_start = 0`, `read_start = |shift|`
- Bases before the overlap (positions 0..read_start) are stored as raw bases in mismatch data
- Reconstruction restores these directly (not noise-decoded)

## Global Read Reordering

For short reads, a minimizer-based reordering groups similar reads together to improve ABC compression:

1. Extract canonical k-mer minimizer from each read
2. Sort reads by minimizer value
3. Store bidirectional reorder map (forward + reverse) in the archive

The reorder map uses **ZigZag delta + varint encoding** for compact storage.

## SCM Quality Compression

Quality scores are compressed using a **Statistical Context Model** with arithmetic coding:

- **Order-2** (short/medium reads): context = previous 2 quality values
- **Order-1** (long reads): context = previous 1 quality value

### Arithmetic Coding

- Adaptive frequency model per context
- Rescaling when total frequency exceeds threshold
- 32-bit precision arithmetic encoder/decoder

### Quality Modes

| Mode | Description |
|------|-------------|
| Lossless | Exact quality preservation |
| Illumina8Bin | Bin to 8 representative values (2,6,15,22,27,33,37,40) |
| Discard | Replace all quality with '!' (Phred 0) |

## Paired-End Optimization

For paired-end data, the PE optimizer exploits reverse-complement complementarity:

1. Reverse-complement R2 sequence
2. Compare with R1
3. If similarity > threshold, store only diff positions + bases (delta encoding)
4. Quality deltas stored similarly

### PE Layouts

| Layout | Storage | Description |
|--------|---------|-------------|
| Interleaved | R1, R2, R1, R2, ... | Alternating read pairs |
| Consecutive | R1, R1, ..., R2, R2, ... | All R1 first, then all R2 |

## ID Compression

Read identifiers are compressed based on ID mode:

| Mode | Description |
|------|-------------|
| Exact | Full ID preservation |
| StripComment | Remove content after first space |
| Discard | Replace with sequential `@read_N` |

IDs are Zstd-compressed as a block with newline separators.

## Zstd Codec (Medium/Long Reads)

For reads > 300bp, sequences are stored with length-prefixed encoding:

```
[u16: read_length][sequence bytes] × N reads
```

Then Zstd-compressed as a single block. The compression level is configurable (1-19, default 3).
