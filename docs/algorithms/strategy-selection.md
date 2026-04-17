# Strategy Selection

This document explains how fqc chooses which compression algorithms to apply based on input data characteristics and user configuration.

## Overview

fqc uses a multi-strategy approach that selects different algorithms for different components of FASTQ data (sequences, quality scores, IDs, auxiliary data) based on the **read length class**, **quality mode**, **ID mode**, and **compression level**.

## Read Length Classification

The first decision point is classifying the reads into one of three length classes. This classification determines which algorithms are used for sequences and quality scores.

### Classification Logic

```rust
pub fn classify_read_length(median_length: usize, max_length: usize) -> ReadLengthClass {
    if max_length >= LONG_READ_THRESHOLD {       // 10,240 bytes
        return ReadLengthClass::Long;
    }
    if max_length > SPRING_MAX_READ_LENGTH {      // 511 bytes
        return ReadLengthClass::Medium;
    }
    if median_length >= MEDIUM_READ_THRESHOLD {   // 1,024 bytes
        return ReadLengthClass::Medium;
    }
    ReadLengthClass::Short
}
```

### Thresholds

| Parameter | Value | Description |
|-----------|-------|-------------|
| `SPRING_MAX_READ_LENGTH` | 511 bp | Maximum read length for ABC algorithm |
| `MEDIUM_READ_THRESHOLD` | 1,024 bp | Median length threshold for medium class |
| `LONG_READ_THRESHOLD` | 10,240 bp | Maximum length threshold for long class |
| `ULTRA_LONG_READ_THRESHOLD` | 102,400 bp | Ultra-long read threshold (metadata) |

### Classes

| Class | Criteria | Typical Data | Sequence Algorithm | Quality Algorithm |
|-------|----------|--------------|--------------------|--------------------|
| **Short** | max <= 511bp AND median < 1,024bp | Illumina short reads (75-300bp) | ABC (consensus + delta) | SCM Order-2 |
| **Medium** | max in (511, 10,240) OR median >= 1,024bp | Amplicon, longer Illumina | Zstd plain | SCM Order-2 |
| **Long** | max >= 10,240bp | PacBio, Nanopore, assembled contigs | Zstd plain | SCM Order-1 |

## Default Block Sizes

Each read length class has a recommended block size (number of reads per block):

| Class | Default Block Size | Rationale |
|-------|-------------------|-----------|
| Short | 100,000 reads | Small reads → more fits in memory, better consensus |
| Medium | 50,000 reads | Medium reads → balanced memory usage |
| Long | 10,000 reads | Large reads → fewer per block to limit memory |

## Compression Level to Zstd Level Mapping

User-facing compression levels (1-9) map to zstd compression levels:

| User Level | Zstd Level | Description |
|------------|------------|-------------|
| 1-2 | 1 | Fast compression |
| 3-4 | 3 | Balanced |
| 5-6 | 5 | Default (level 5) |
| 7-8 | 9 | High compression |
| 9 | 15 | Maximum compression |

## Codec Selection Matrix

The `BlockCompressorConfig` selects codecs based on the current configuration:

### Sequence Codec

```rust
pub fn get_sequence_codec(&self) -> u8 {
    match self.read_length_class {
        ReadLengthClass::Short => encode_codec(CodecFamily::AbcV1, 0),   // 0x10
        _ => encode_codec(CodecFamily::ZstdPlain, 0),                     // 0x70
    }
}
```

| Read Length Class | Codec Family | Codec Byte | Description |
|-------------------|--------------|------------|-------------|
| Short | `AbcV1` | `0x10` | Consensus + delta encoding |
| Medium | `ZstdPlain` | `0x70` | Plain zstd on length-prefixed reads |
| Long | `ZstdPlain` | `0x70` | Plain zstd on length-prefixed reads |

### Quality Codec

```rust
pub fn get_quality_codec(&self) -> u8 {
    if self.quality_mode == QualityMode::Discard {
        return encode_codec(CodecFamily::Raw, 0);   // 0x00
    }
    match self.read_length_class {
        ReadLengthClass::Long => encode_codec(CodecFamily::ScmOrder1, 0),  // 0x80
        _ => encode_codec(CodecFamily::ScmV1, 0),                          // 0x20
    }
}
```

| Quality Mode | Short/Medium | Long |
|--------------|--------------|------|
| Lossless | `ScmV1` (0x20) | `ScmOrder1` (0x80) |
| Illumina8 | `ScmV1` (0x20) | `ScmOrder1` (0x80) |
| Discard | `Raw` (0x00) | `Raw` (0x00) |
| Qvz | `ScmV1` (0x20) | `ScmOrder1` (0x80) |

### ID Codec

```rust
pub fn get_id_codec(&self) -> u8 {
    if self.id_mode == IdMode::Discard {
        return encode_codec(CodecFamily::Raw, 0);   // 0x00
    }
    encode_codec(CodecFamily::DeltaZstd, 0);         // 0x40
}
```

| ID Mode | Codec | Description |
|---------|-------|-------------|
| Exact | `DeltaZstd` (0x40) | Tokenization + delta + zstd |
| Tokenize | `DeltaZstd` (0x40) | Tokenization + delta + zstd |
| Discard | `Raw` (0x00) | Empty stream, regenerated on decompress |

### Aux Codec

```rust
pub fn get_aux_codec(&self) -> u8 {
    encode_codec(CodecFamily::DeltaVarint, 0);   // 0x50
}
```

The aux stream always uses delta-varint encoding for read lengths, regardless of read length class.

## Complete Selection Tables

### Short Reads (Illumina 150bp)

| Component | Codec | Encoding Chain |
|-----------|-------|----------------|
| IDs | `DeltaZstd` | Tokenize → delta-varint → zstd |
| Sequences | `AbcV1` | Consensus → delta encode → zstd |
| Quality | `ScmV1` | Order-2 SCM → arithmetic coding → zstd |
| Aux (lengths) | `DeltaVarint` | Delta → varint |

### Medium Reads (500-5000bp)

| Component | Codec | Encoding Chain |
|-----------|-------|----------------|
| IDs | `DeltaZstd` | Tokenize → delta-varint → zstd |
| Sequences | `ZstdPlain` | Length-prefixed → zstd |
| Quality | `ScmV1` | Order-2 SCM → arithmetic coding → zstd |
| Aux (lengths) | `DeltaVarint` | Delta → varint |

### Long Reads (> 10KB)

| Component | Codec | Encoding Chain |
|-----------|-------|----------------|
| IDs | `DeltaZstd` | Tokenize → delta-varint → zstd |
| Sequences | `ZstdPlain` | Length-prefixed → zstd |
| Quality | `ScmOrder1` | Order-1 SCM → arithmetic coding → zstd |
| Aux (lengths) | `DeltaVarint` | Delta → varint |

## Reordering Decision

Read reordering is conditionally applied:

| Condition | Applied |
|-----------|---------|
| `enable_reorder` flag | Must be `true` |
| `ReadLengthClass` | Must be `Short` |
| Paired-end data | Must be `false` |
| Streaming mode | Must be `false` |

Reordering is only beneficial for short reads using the ABC algorithm because:
- ABC relies on grouping similar reads together for better consensus
- Zstd does not benefit significantly from read reordering
- The overhead of reordering (minimizer extraction, bucketing, greedy search) is not justified for long reads

## Compression Mode Decision Tree

```
Input FASTQ
    │
    ├── [streaming mode?] ──Yes──→ No reordering, block-by-block compression
    │                                  │
    │                                  └──→ Use Zstd for sequences (no ABC)
    │
    └── No
         │
         └── [analyze all reads]
              │
              └── classify_read_length()
                   │
                   ├── Short (< 511bp)
                   │    │
                   │    ├── [paired-end?] ──Yes──→ No reordering
                   │    │                              │
                   │    │                              └──→ ABC for sequences
                   │    │
                   │    └── No
                   │         │
                   │         ├── [enable_reorder?] ──Yes──→ Minimizer reordering
                   │         │                                 │
                   │         │                                 └──→ ABC for sequences
                   │         │
                   │         └── No
                   │              │
                   │              └──→ ABC for sequences (no reordering)
                   │
                   ├── Medium (511bp - 10KB)
                   │    │
                   │    └──→ Zstd for sequences, SCM Order-2 for quality
                   │
                   └── Long (> 10KB)
                        │
                        └──→ Zstd for sequences, SCM Order-1 for quality
```

## Configuration Override

Users can override automatic classification:

```bash
# Force long-read mode (Zstd for sequences)
fqc input.fastq --read-length-class long

# Force short-read mode (ABC for sequences)
fqc input.fastq --read-length-class short

# Disable reordering even for short reads
fqc input.fastq --no-reorder
```

When `--read-length-class` is explicitly set, `classify_read_length()` is bypassed.

## Quality Mode Impact on Codec Selection

### Lossless Mode

Full 94-symbol quality alphabet (Phred Q0-Q93). Uses SCM with Order-2 context (short/medium) or Order-1 (long).

### Illumina 8 Mode

Quality scores quantized to 8 bins before SCM encoding:

```rust
fn illumina8_to_bin(q: u8) -> u8 {
    for (i, &b) in BIN_BOUNDARIES.iter().enumerate() {
        if q < b { return i as u8; }
    }
    7
}
```

Reduces the effective alphabet size, improving SCM compression ratio.

### Discard Mode

Quality stream is empty (`size_qual = 0`, codec = `Raw`). On decompression, placeholder `'!'` (Q0) characters are generated to fill each read's length.

## Performance Trade-offs

| Strategy | Compression Ratio | Speed | Memory |
|----------|-------------------|-------|--------|
| ABC (short) | Best (4-8x) | Moderate | O(block_size × read_length) |
| Zstd (medium/long) | Good (3-5x) | Fast | O(block_size × read_length) |
| SCM Order-2 (quality) | Best | Moderate | O(94² × 8 models) |
| SCM Order-1 (quality) | Good | Faster | O(94 × 8 models) |
| DeltaZstd (IDs) | Good (3-10x) | Fast | O(block_size × id_length) |

## Related Documents

- [ABC Algorithm](./abc.md)
- [SCM Quality Compression](./scm.md)
- [Zstd Integration](./zstd.md)
- [Minimizer Reordering](./minimizer.md)
- [Source Module Overview](../architecture/modules.md)
- [Block Format](../architecture/block-format.md)
