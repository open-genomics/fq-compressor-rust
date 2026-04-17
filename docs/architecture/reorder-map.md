# Reorder Map Architecture

This document explains how fqc's minimizer-based read reordering works, including the bidirectional map structure, encoding scheme, and integration with the compression pipeline.

## Overview

fqc optionally reorders reads before compression to maximize locality: reads with similar sequences are placed adjacent to each other, which significantly improves the ABC algorithm's consensus quality and delta encoding efficiency.

To enable restoration of the original read order, fqc stores a **bidirectional reorder map** that maps between original read IDs and archive (reordered) read IDs.

```
Original order:  [R0, R1, R2, R3, R4, R5, ...]
                      \   |   /     \   |   /
Reorder map:       [R2, R0, R1, R5, R3, R4, ...]
                      |   |   |      |   |   |
Archive order:     [A0, A1, A2, A3, A4, A5, ...]
```

## Bidirectional Map Structure

The reorder map consists of two complementary vectors:

| Map | Direction | Purpose |
|-----|-----------|---------|
| **Forward Map** | `original_id → archive_id` | "Where did read N end up in the archive?" |
| **Reverse Map** | `archive_id → original_id` | "Which original read is at archive position N?" |

Both maps are permutations of `[0, 1, 2, ..., n-1]` and are mathematical inverses of each other.

### Construction

Given a reverse map (the reordering order), the forward map is computed automatically:

```rust
pub fn from_reverse_map(reverse_map: Vec<ReadId>) -> Self {
    let n = reverse_map.len();
    let mut forward_map = vec![0u64; n];
    for (archive_id, &orig_id) in reverse_map.iter().enumerate() {
        if (orig_id as usize) < n {
            forward_map[orig_id as usize] = archive_id as ReadId;
        }
    }
    Self { forward_map, reverse_map }
}
```

### Consistency Invariant

The maps must satisfy the inverse relationship:

```
forward_map[reverse_map[i]] == i    for all i
reverse_map[forward_map[j]] == j    for all j
```

The `is_valid()` method checks this invariant:

```rust
pub fn is_valid(&self) -> bool {
    if self.forward_map.len() != self.reverse_map.len() {
        return false;
    }
    let n = self.forward_map.len();
    for i in 0..n {
        let archive_id = self.forward_map[i] as usize;
        if archive_id >= n {
            return false;
        }
        if self.reverse_map[archive_id] != i as u64 {
            return false;
        }
    }
    true
}
```

## Encoding Scheme

Reorder maps are compressed using a three-stage encoding process:

```
Read IDs → Delta Encoding → ZigZag Varint → Zstd Compression
```

### Stage 1: Delta Encoding

Consecutive IDs are encoded as deltas (differences from the previous value):

```
Original:  [0, 2, 1, 5, 3, 4]
Deltas:    [0, 2, -1, 4, -2, 1]
```

Delta encoding reduces the magnitude of values, which makes subsequent varint encoding more efficient.

### Stage 2: ZigZag Varint Encoding

Signed deltas are converted to unsigned using ZigZag encoding, then encoded as variable-length integers:

**ZigZag mapping**:

| Signed | ZigZag (unsigned) |
|--------|-------------------|
| 0 | 0 |
| -1 | 1 |
| 1 | 2 |
| -2 | 3 |
| 2 | 4 |
| ... | ... |

```rust
pub fn encode_signed_varint(value: i64) -> Vec<u8> {
    let zigzag = ((value << 1) ^ (value >> 63)) as u64;
    encode_varint(zigzag)
}
```

**Varint encoding** (7 bits per byte, MSB = continuation flag):

| Value | Bytes |
|-------|-------|
| 0 | `[0x00]` |
| 1 | `[0x01]` |
| 127 | `[0x7F]` |
| 128 | `[0x80, 0x01]` |
| 16383 | `[0xFF, 0x7F]` |

Small deltas (common after reordering) encode to 1 byte; large deltas use 2-3 bytes.

### Stage 3: Zstd Compression

The varint-encoded byte streams for both forward and reverse maps are independently compressed with zstd (level 3):

```rust
let forward_compressed = zstd::bulk::compress(&forward_encoded, 3)?;
let reverse_compressed = zstd::bulk::compress(&reverse_encoded, 3)?;
```

### Serialized Format

```
+-------------------+
| Version (u32)     |  Always 1
+-------------------+
| Total Reads (u64) |
+-------------------+
| Fwd Size (u64)    |  Compressed forward map size
+-------------------+
| Rev Size (u64)    |  Compressed reverse map size
+-------------------+
| Fwd Data          |  zstd(delta(varint(forward_map)))
+-------------------+
| Rev Data          |  zstd(delta(varint(reverse_map)))
+-------------------+
```

Header size: 28 bytes. Total size: `28 + fwd_size + rev_size`.

### Memory Overhead

The target memory overhead is approximately **4 bytes per read** for the serialized map:

```rust
pub fn estimate_serialized_size(&self) -> usize {
    let n = self.forward_map.len();
    (n as f64 * TARGET_BYTES_PER_READ) as usize + 28 // header overhead
}
```

For a 100-million-read file, this is roughly 400 MB.

## Reordering Process

The reordering is performed by `GlobalAnalyzer` and follows these steps:

### Step 1: Minimizer Extraction

For each read, extract minimizers (representative k-mers) using parameters `k` (k-mer length) and `w` (window size):

```rust
pub fn extract_minimizers(seq: &[u8], k: usize, w: usize) -> Vec<Minimizer>
```

Each minimizer records:
- `hash`: Canonical k-mer hash (min of forward and reverse-complement)
- `position`: Position within the read
- `is_rc`: Whether the canonical form is the reverse complement

### Step 2: Bucketing

Build a hash map from minimizer hash to the list of reads containing that minimizer:

```rust
let mut bucket_map: HashMap<u64, Vec<u64>> = HashMap::new();
for entries in &all_buckets {
    for &(hash, read_id) in entries {
        bucket_map.entry(hash).or_default().push(read_id);
    }
}
```

### Step 3: Greedy Reordering

Starting from read 0, greedily select the next read that shares minimizers and has the most similar length:

```rust
ordered.push(0);
used[0] = true;

while ordered.len() < total_reads {
    let last_read = *ordered.last();
    let last_mins = extract_minimizers(sequences[last_read], k, w);

    // Search candidates from minimizer buckets
    for m in &last_mins {
        if let Some(bucket) = bucket_map.get(&m.hash) {
            for &candidate_id in bucket {
                if !used[candidate_id] {
                    // Score by length similarity
                    let len_diff = last_len.abs_diff(candidate_len);
                    if len_diff < best_score {
                        best_score = len_diff;
                        best_match = Some(candidate_id);
                    }
                }
            }
        }
    }

    ordered.push(best_match ?? first_unused);
    used[best_match] = true;
}
```

The search is bounded by `max_search_reorder` (default 64) candidates to limit runtime.

## When Reordering is Applied

Reordering is **only** applied when all conditions are met:

| Condition | Requirement |
|-----------|-------------|
| `enable_reorder` | `true` |
| `ReadLengthClass` | `Short` (< 300bp reads) |
| Paired-end | `false` (disabled for PE data) |
| Streaming mode | `false` (requires all reads in memory) |

For medium and long reads, reordering is skipped because:
- ABC algorithm is not used (Zstd is used instead)
- Zstd benefits less from read reordering
- The overhead of reordering outweighs the benefit for long reads

## Reorder Map in the Archive

The reorder map is stored between the last block and the block index:

```
+----------------+
|    Block N     |
+----------------+
| Reorder Map    |  ← Optional (present if HAS_REORDER_MAP flag is set)
+----------------+
|   Block Index  |
+----------------+
|  File Footer   |
+----------------+
```

The `FileFooter` stores the reorder map offset:

```rust
pub struct FileFooter {
    pub index_offset: u64,
    pub reorder_map_offset: u64,  // 0 if no reorder map
    pub global_checksum: u64,
    pub magic_end: [u8; 8],
}
```

If `reorder_map_offset == 0`, no reorder map is present (original order preserved).

### Global Header Flag

The `HAS_REORDER_MAP` flag (bit 7) in the global header indicates whether a reorder map exists:

```rust
pub const HAS_REORDER_MAP: u64 = 1 << 7;
```

## Decompression with Reorder Map

During decompression, the reverse map is used to restore reads to their original order:

```rust
// Load reverse map
reader.load_reorder_map()?;

// Map archive ID back to original ID
let original_id = reader.lookup_original_id(archive_id)?;
```

When the `--original-order` flag is passed to decompress:
1. Load the reverse map from the archive
2. After decompressing each block, reorder reads using `reverse_map[archive_id]`
3. Write reads in original order

## Chunk Combination (Divide-and-Conquer Mode)

For very large files processed in chunks, individual reorder maps are combined:

```rust
pub fn combine_chunks(chunks: &[ReorderMapData], chunk_sizes: &[u64]) -> Self {
    let mut combined = ReorderMapData { ... };
    let mut archive_offset: u64 = 0;
    let mut original_offset: u64 = 0;

    for (i, chunk) in chunks.iter().enumerate() {
        combined.append_chunk(chunk, archive_offset, original_offset);
        archive_offset += chunk_sizes[i];
        original_offset += chunk_sizes[i];
    }
    combined
}
```

Each chunk's IDs are offset by the cumulative sizes of preceding chunks.

## Validation Functions

The module provides two validation functions:

### `verify_map_consistency()`

Verifies that forward and reverse maps are consistent inverses:

```rust
pub fn verify_map_consistency(forward_map: &[ReadId], reverse_map: &[ReadId]) -> Result<()>
```

Checks:
- Same length
- All values within bounds
- `reverse_map[forward_map[i]] == i` for all i

### `validate_permutation()`

Verifies that a map is a valid permutation:

```rust
pub fn validate_permutation(map: &[ReadId]) -> Result<()>
```

Checks:
- All values in range `[0, n)`
- No duplicate values

## Performance Characteristics

| Metric | Value |
|--------|-------|
| Target memory overhead | ~4 bytes/read |
| Minimizer extraction | Parallel via `rayon` |
| Reordering complexity | O(n × k × max_search) |
| Serialization | Delta + varint + zstd (level 3) |
| Deserialization | Zstd + varint decode + delta decode |

## Example

For a 5-read file with original order `[R0, R1, R2, R3, R4]`:

**After reordering** (reads 2,0,1,4,3 grouped by similarity):

```
reverse_map = [2, 0, 1, 4, 3]   // archive_id → original_id
forward_map = [1, 2, 0, 4, 3]   // original_id → archive_id
```

**Delta encoding of reverse map**:

```
reverse_map:   [2, 0, 1, 4, 3]
deltas:        [2, -2, 1, 3, -1]
zigzag:        [4, 3, 2, 6, 1]
varint bytes:  [0x04, 0x03, 0x02, 0x06, 0x01]  // 5 bytes (all single-byte)
```

## Related Documents

- [Minimizer Algorithm](../algorithms/minimizer.md)
- [Source Module Overview](./modules.md)
- [Full FQC Format Specification](./format-spec.md)
- [Block Format](./block-format.md)
