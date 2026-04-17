# Paired-End Read Optimization

This document describes fqc's paired-end (PE) optimization, which exploits the complementarity between paired reads to reduce storage requirements.

## Overview

In paired-end sequencing, two reads (R1 and R2) are generated from opposite ends of the same DNA fragment. For short fragments, R1 and R2 often overlap, meaning R2 is approximately the reverse complement of R1's tail. This biological property can be exploited for compression.

fqc's PE optimizer detects complementary read pairs and stores R2 as a **differential encoding** against R1's reverse complement, rather than storing both reads independently.

## Biology Background

In paired-end sequencing:

```
5'---[R1--->            <---R2]---3'
    |--------------------|
         Fragment (insert size)
```

- R1 sequences from the 5' end of the fragment
- R2 sequences from the 3' end (stored as reverse complement)
- For short fragments, R1 and R2 overlap
- In the overlap region, R2 ≈ reverse_complement(R1)

## PE Layout Support

fqc supports two paired-end layouts:

| Layout | Description | File Format |
|--------|-------------|-------------|
| **Interleaved** | R1, R2, R1, R2, ... in a single file | Single file |
| **Consecutive** | All R1s, then all R2s | Two files or one file with R1s first |

The layout is specified via `PeLayout` and stored in the global header flags:

```rust
pub enum PeLayout {
    Interleaved = 0,  // R1, R2, R1, R2, ...
    Consecutive = 1,  // All R1s, then all R2s
}
```

### Flag Storage

```rust
f = (f & !flags::PE_LAYOUT_MASK) | ((pe_layout as u64) << flags::PE_LAYOUT_SHIFT);
// PE_LAYOUT_SHIFT = 8, PE_LAYOUT_MASK = 0x3 << 8
```

## Complementarity Detection

The PE optimizer checks whether R2 is approximately the reverse complement of R1:

```rust
pub fn check_complementarity(&self, r1_seq: &[u8], r2_seq: &[u8]) -> (bool, usize) {
    let min_len = r1_seq.len().min(r2_seq.len());
    if min_len < self.config.min_overlap {   // default: 20bp
        return (false, 0);
    }

    let r1_rc = reverse_complement(r1_seq);

    let mut diff_count = 0usize;
    for i in 0..min_len {
        if r1_rc[i] != r2_seq[i] {
            diff_count += 1;
            if diff_count > self.config.complementarity_threshold {  // default: 15
                return (false, diff_count);
            }
        }
    }

    // Add length difference
    diff_count += r1_seq.len().abs_diff(r2_seq.len());

    let beneficial = diff_count <= self.config.complementarity_threshold;
    (beneficial, diff_count)
}
```

### Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `complementarity_threshold` | 15 | Maximum differences (mismatches + length diff) |
| `min_overlap` | 20 bp | Minimum overlap length for complementarity check |
| `enable_complementarity` | true | Master switch for PE optimization |

### Decision Criteria

A pair is considered complementary when:
1. Both reads are at least `min_overlap` (20bp) long
2. The Hamming distance between R1-RC and R2 is within threshold
3. Total differences (mismatches + length difference) <= threshold

If the complementarity check fails, R2 is stored in full.

## Encoding

When complementarity is detected, R2 is encoded as a differential from R1's reverse complement:

```rust
pub struct PEEncodedPair {
    pub id1: String,               // R1 ID (stored in ID stream)
    pub seq1: String,              // R1 sequence (stored in seq stream)
    pub qual1: String,             // R1 quality (stored in qual stream)
    pub id2: String,               // R2 ID (stored in ID stream)
    pub seq2: String,              // R2 sequence (stored in seq stream, or empty if complementarity)
    pub qual2: String,             // R2 quality (stored in qual stream, or empty if complementarity)
    pub use_complementarity: bool,  // True if diff encoding is used
    pub diff_positions: Vec<u16>,  // Positions where R2 differs from R1-RC
    pub diff_bases: Vec<u8>,       // Actual bases at diff positions in R2
    pub qual_delta: Vec<i8>,       // Quality score deltas at diff positions
}
```

### Diff Computation

```rust
fn compute_diff(seq1: &[u8], seq2: &[u8]) -> (Vec<u16>, Vec<u8>) {
    let min_len = seq1.len().min(seq2.len());
    let mut positions = Vec::new();
    let mut bases = Vec::new();

    // Differences in overlap region
    for i in 0..min_len {
        if seq1[i] != seq2[i] {
            positions.push(i as u16);
            bases.push(seq2[i]);
        }
    }

    // Extra bases in seq2 (length difference)
    for (i, &b) in seq2.iter().enumerate().skip(min_len) {
        positions.push(i as u16);
        bases.push(b);
    }

    (positions, bases)
}
```

### Quality Deltas

Quality scores are also encoded differentially:

```rust
let r1_qual_rev: Vec<u8> = r1.quality.as_bytes().iter().rev().copied().collect();
let mut qual_delta = Vec::with_capacity(positions.len());
for &pos in &positions {
    let p = pos as usize;
    if p < r1_qual_rev.len() && p < r2.quality.len() {
        let delta = r2.quality.as_bytes()[p] as i16 - r1_qual_rev[p] as i16;
        qual_delta.push(delta.clamp(-128, 127) as i8);
    } else {
        qual_delta.push(0);
    }
}
```

Quality deltas are clamped to i8 range (-128 to 127) to fit in a single byte.

## Serialization

The encoded pair is serialized with the following format:

```
+-------------------+
| Flags (u8)        |  Bit 0: use_complementarity
+-------------------+
| ID2 Length (varint)|  Length of R2 ID
+-------------------+
| ID2 Bytes         |  R2 ID (may be empty if pattern-detected)
+-------------------+
| If complementarity:     |
|   Diff Count      |  varint
|   Diff Positions  |  delta-varint encoded u16 positions
|   Diff Bases      |  raw bytes
|   Quality Deltas  |  raw i8 bytes
+-------------------+
| If not complementarity: |
|   Seq2 Length     |  varint
|   Seq2 Bases      |  raw bytes
|   Qual2 Length    |  varint
|   Qual2 Bytes     |  raw bytes
+-------------------+
```

### Position Delta Encoding

Diff positions are delta-encoded for compactness:

```rust
let mut prev: u16 = 0;
for &pos in &pair.diff_positions {
    push_varint(&mut buf, (pos - prev) as u64);
    prev = pos;
}
```

Since diff positions are typically in ascending order, the deltas are small and encode to 1-2 bytes each.

## Decoding

Decoding reconstructs R2 from R1 and the diff data:

### Sequence Decoding

```rust
pub fn decode_r2_sequence(&self, r1_seq: &str) -> String {
    if !self.use_complementarity {
        return self.seq2.clone();
    }

    // Start with R1 reverse complement
    let r1_rc = reverse_complement(r1_seq.as_bytes());
    let mut result: Vec<u8> = r1_rc;

    // Apply differences
    for (i, &pos) in self.diff_positions.iter().enumerate() {
        if i >= self.diff_bases.len() { break; }
        let p = pos as usize;
        if p >= result.len() {
            result.resize(p + 1, b'N');
        }
        result[p] = self.diff_bases[i];
    }

    String::from_utf8_lossy(&result).into_owned()
}
```

### Quality Decoding

```rust
pub fn decode_r2_quality(&self, r1_qual: &str) -> String {
    if !self.use_complementarity {
        return self.qual2.clone();
    }

    // Start with reversed R1 quality
    let mut result: Vec<u8> = r1_qual.as_bytes().iter().rev().copied().collect();

    // Apply quality deltas
    for (i, &pos) in self.diff_positions.iter().enumerate() {
        if i >= self.qual_delta.len() { break; }
        let p = pos as usize;
        if p >= result.len() {
            result.resize(p + 1, b'!');  // Q0 default
        }
        let new_qual = (result[p] as i16) + (self.qual_delta[i] as i16);
        result[p] = new_qual.clamp(33, 126) as u8;
    }

    String::from_utf8_lossy(&result).into_owned()
}
```

## R2 ID Generation

When IDs follow a predictable pattern, R2 IDs can be derived from R1 IDs:

```rust
pub fn generate_r2_id(r1_id: &str) -> String {
    let bytes = r1_id.as_bytes();
    let len = bytes.len();

    // Check for /1 or .1 suffix
    if len >= 2 && bytes[len - 1] == b'1'
        && (bytes[len - 2] == b'/' || bytes[len - 2] == b'.')
    {
        let mut id = r1_id[..len - 1].to_string();
        id.push('2');
        return id;
    }

    // Check for space-separated: "id 1:..." -> "id 2:..."
    if let Some(space_pos) = r1_id.find(' ') {
        if space_pos + 1 < len && bytes[space_pos + 1] == b'1' {
            let mut id = r1_id[..=space_pos].to_string();
            id.push('2');
            id.push_str(&r1_id[space_pos + 2..]);
            return id;
        }
    }

    // Default: append /2
    format!("{}/2", r1_id)
}
```

Supported ID conventions:

| R1 ID | R2 ID | Convention |
|-------|-------|------------|
| `read/1` | `read/2` | Suffix `/1` → `/2` |
| `read.1` | `read.2` | Suffix `.1` → `.2` |
| `read 1:N:0:ATGC` | `read 2:N:0:ATGC` | Space + `1:` → `2:` |
| `read` | `read/2` | Default: append `/2` |

## PE Pair Validation

fqc validates PE pair IDs using common conventions:

```rust
pub fn validate_pe_pair_ids(id1: &str, id2: &str) -> bool {
    // Identical IDs
    if id1 == id2 { return true; }

    // /1 /2 suffix
    if id1.ends_with("/1") && id2.ends_with("/2") {
        return id1[..id1.len() - 2] == id2[..id2.len() - 2];
    }

    // Space-separated: "id 1:..." + "id 2:..."
    if let (Some(p1), Some(p2)) = (id1.find(' '), id2.find(' ')) {
        let base1 = &id1[..p1];
        let base2 = &id2[..p2];
        if base1 == base2 {
            let suffix1 = &id1[p1 + 1..];
            let suffix2 = &id2[p2 + 2..];
            return suffix1.starts_with("1:") && suffix2.starts_with("2:");
        }
    }
    false
}
```

## Interleaved Format Detection

fqc can auto-detect whether a FASTQ file is in interleaved PE format:

```rust
pub fn detect_interleaved_format(path: &str) -> Result<bool> {
    let mut parser = open_fastq(path)?;
    let mut pairs_checked = 0;

    for _ in 0..4 {
        let r1 = match parser.next_record()? {
            Some(r) => r,
            None => break,
        };
        let r2 = match parser.next_record()? {
            Some(r) => r,
            None => return Ok(false),
        };
        if !validate_pe_pair_ids(&r1.id, &r2.id) {
            return Ok(false);
        }
        pairs_checked += 1;
    }
    Ok(pairs_checked > 0)
}
```

Checks the first 4 read pairs. If all pairs pass ID validation, the file is considered interleaved PE.

## Statistics

The PE optimizer tracks compression statistics:

```rust
pub struct PEOptimizerStats {
    pub total_pairs: u64,           // Total pairs processed
    pub complementarity_used: u64,  // Pairs where complementarity was detected
    pub bytes_saved: u64,           // Approximate bytes saved via diff encoding
}
```

## Integration with Compression Pipeline

### Current State

The PE optimizer provides the encoding/decoding primitives. In the current compression pipeline:

1. **Paired-end data** is identified via the `IS_PAIRED` flag in the global header
2. **Reads are stored** as individual `ReadRecord` entries in blocks (R1 followed by R2 in interleaved layout, or all R1s then all R2s in consecutive layout)
3. **The PE layout** is recorded in the global header flags for decompression ordering
4. **Reordering is disabled** for paired-end data (reads must maintain pairing relationship)

```rust
let enable_reorder = self.opts.enable_reorder
    && !self.opts.streaming_mode
    && !is_paired       // Disabled for PE
    && effective_length_class == ReadLengthClass::Short;
```

### Storage Efficiency

When complementarity is detected:
- R2 is stored as diff positions + bases + quality deltas
- For typical Illumina data with high overlap, this can save **50-90%** of R2's sequence storage
- Quality score storage is also reduced via differential encoding

**Example**: For a pair of 150bp reads with 8 differences:

| Component | Without PE Optimization | With PE Optimization |
|-----------|------------------------|----------------------|
| R2 sequence | 150 bytes | 8 bases + 8 positions ≈ 24 bytes |
| R2 quality | 150 bytes | 8 deltas ≈ 8 bytes |
| **Total** | **300 bytes** | **~32 bytes** |

## CLI Usage

```bash
# Paired-end, interleaved layout
fqc -1 r1.fastq -2 r2.fastq output.fqc

# Paired-end, consecutive layout (two separate files)
fqc -1 r1.fastq -2 r2.fastq --pe-layout consecutive output.fqc

# Single-file interleaved PE
fqc --interleaved interleaved.fastq output.fqc

# Streaming mode with PE
fqc --streaming -1 r1.fastq -2 r2.fastq output.fqc
```

## Related Documents

- [Strategy Selection](./strategy-selection.md)
- [Source Module Overview](../architecture/modules.md)
- [Full FQC Format Specification](../architecture/format-spec.md)
- [Core Compression Spec](../../specs/product/core-compression.md)
- [Compression Algorithms RFC](../../specs/rfc/0002-compression-algorithms.md)
