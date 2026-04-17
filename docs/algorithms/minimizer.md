# Minimizer Hashing and Reordering

This document describes the minimizer-based read reordering system used by fqc to improve compression locality.

## Overview

fqc reorders reads before compression so that reads with similar sequences are placed adjacent to each other. This significantly improves the ABC algorithm's ability to build accurate consensus sequences and encode reads as small deltas.

The reordering uses **minimizers** — representative k-mers that serve as compact "signatures" for each read. Reads sharing minimizers are likely to be similar and are grouped together.

## What Are Minimizers?

A **minimizer** is the smallest (by hash value) k-mer in a sliding window along a sequence. Instead of storing all k-mers for a read, we store only the minimizers, which provides:

1. **Compact representation**: A few minimizers per read instead of hundreds of k-mers
2. **Sensitivity**: Similar reads will share at least one minimizer
3. **Canonical form**: Each minimizer uses the minimum of forward and reverse-complement hash, making it strand-independent

### Example

For a read `ACGTACGTACGT` with k=3 and w=3:

```
K-mers:  ACG CGT GTA TAC ACG CGT GTA TAC ACG CGT
Window 1 [ACG CGT GTA] → min(GTA) = minimizer
Window 2    [CGT GTA TAC] → min(CGT) = minimizer
Window 3       [GTA TAC ACG] → min(ACG) = minimizer
...
```

## Minimizer Extraction

### k-mer Hashing

Each k-mer is hashed using a simple 2-bit encoding:

```rust
fn compute_kmer_hash(seq: &[u8]) -> u64 {
    let k = seq.len();
    let mut hash: u64 = 0;
    let mut rc_hash: u64 = 0;
    for i in 0..k {
        let base = BASE_TO_INDEX[seq[i] as usize] as u64;
        hash = (hash << 2) | base;
        let rc_base = 3 - BASE_TO_INDEX[seq[k - 1 - i] as usize] as u64;
        rc_hash = (rc_hash << 2) | rc_base;
    }
    hash.min(rc_hash)  // Canonical: minimum of forward and RC
}
```

The hash uses **2 bits per base** (A=00, C=01, G=10, T=11), so for k=15, the hash fits in 30 bits of a u64.

**Canonical form**: The hash is the minimum of the forward and reverse-complement hash, ensuring the same k-mer produces the same hash regardless of strand orientation.

### Sliding Window Minimizer Selection

```rust
pub fn extract_minimizers(seq: &[u8], k: usize, w: usize) -> Vec<Minimizer>
```

Parameters:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `k` | 15 | k-mer length in bases |
| `w` | 10 | Window size (number of consecutive k-mers) |

Algorithm:

1. Compute hashes for all k-mers in the sequence
2. Slide a window of size `w` across the k-mers
3. For each window, find the k-mer with the minimum hash
4. If the minimizer position changed from the previous window, record it

```rust
for window_start in 0..=(num_kmers.saturating_sub(window_size)) {
    let mut min_hash = u64::MAX;
    let mut min_pos = 0;

    for i in 0..window_size {
        let pos = window_start + i;
        if hashes[pos] < min_hash {
            min_hash = hashes[pos];
            min_pos = pos;
        }
    }

    if min_pos != prev_min_pos {
        // Check if canonical form is the reverse complement
        let mut fwd_hash: u64 = 0;
        for i in 0..k {
            let base = BASE_TO_INDEX[seq[min_pos + i] as usize] as u64;
            fwd_hash = (fwd_hash << 2) | base;
        }
        let is_rc = min_hash != fwd_hash;
        minimizers.push(Minimizer {
            hash: min_hash,
            position: min_pos as u32,
            is_rc,
        });
        prev_min_min_pos = min_pos;
    }
}
```

The `is_rc` flag indicates whether the canonical minimizer hash came from the reverse-complement strand, which is useful for downstream alignment.

### Minimizer Output

```rust
pub struct Minimizer {
    pub hash: u64,      // Canonical k-mer hash
    pub position: u32,  // Position within the read
    pub is_rc: bool,    // True if canonical form is reverse complement
}
```

For a typical 150bp read with k=15, w=10, approximately 10-20 minimizers are extracted.

## Global Analyzer

The `GlobalAnalyzer` orchestrates the entire reordering process:

```rust
pub struct GlobalAnalyzer {
    config: GlobalAnalyzerConfig,
}

pub struct GlobalAnalyzerConfig {
    pub reads_per_block: usize,       // Block size
    pub minimizer_k: usize,           // k-mer length (default 15)
    pub minimizer_w: usize,           // Window size (default 10)
    pub enable_reorder: bool,         // Enable reordering
    pub memory_limit: usize,          // Memory limit
    pub max_search_reorder: usize,    // Search limit (default 64)
    pub read_length_class: Option<ReadLengthClass>,
}
```

## Reordering Algorithm

### Step 1: Parallel Minimizer Extraction

Minimizers are extracted for all reads in parallel using `rayon`:

```rust
let all_buckets: Vec<Vec<(u64, u64)>> = sequences
    .par_iter()
    .enumerate()
    .map(|(i, seq)| {
        let mins = extract_minimizers(seq.as_bytes(), k, w);
        mins.into_iter().map(|m| (m.hash, i as u64)).collect()
    })
    .collect();
```

### Step 2: Build Bucket Index

A hash map maps each minimizer hash to the list of reads containing it:

```rust
let mut bucket_map: HashMap<u64, Vec<u64>> = HashMap::new();
for entries in &all_buckets {
    for &(hash, read_id) in entries {
        bucket_map.entry(hash).or_default().push(read_id);
    }
}
```

This creates an inverted index: `minimizer_hash → [read_ids]`.

### Step 3: Greedy Reordering

The reordering approximates a Hamiltonian path through the read similarity graph:

```rust
let mut used = vec![false; total_reads];
let mut ordered: Vec<ReadId> = Vec::with_capacity(total_reads);

ordered.push(0);  // Start with first read
used[0] = true;

while ordered.len() < total_reads {
    let last_read = *ordered.last();
    let last_mins = extract_minimizers(sequences[last_read].as_bytes(), k, w);

    let mut best_match: Option<u64> = None;
    let mut best_score = usize::MAX;

    // Search candidates from minimizer buckets
    for m in &last_mins {
        if let Some(bucket) = bucket_map.get(&m.hash) {
            for &candidate_id in bucket {
                if used[candidate_id as usize] { continue; }

                // Score by length similarity
                let len_diff = last_len.abs_diff(sequences[candidate_id as usize].len());
                if len_diff < best_score {
                    best_score = len_diff;
                    best_match = Some(candidate_id);
                }

                searched += 1;
                if searched >= max_search_reorder { break; }
            }
        }
    }

    // If no match found, use first unused read
    let next = best_match ?? (0..total_reads).find(|&i| !used[i as usize]).unwrap_or(0);
    ordered.push(next);
    used[next as usize] = true;
}
```

**Scoring**: Candidates are scored by **length difference** — reads with similar lengths are preferred. This is a fast proxy for sequence similarity that avoids expensive alignment.

**Search bound**: The `max_search_reorder` parameter (default 64) limits the number of candidates examined per step, ensuring the algorithm runs in reasonable time.

**Fallback**: If no similar read is found among the searched candidates, the algorithm falls back to the first unused read, ensuring progress.

## Output

The `GlobalAnalysisResult` contains:

```rust
pub struct GlobalAnalysisResult {
    pub total_reads: u64,
    pub max_read_length: usize,
    pub length_class: ReadLengthClass,
    pub reordering_performed: bool,
    pub forward_map: Vec<ReadId>,   // original_id → archive_id
    pub reverse_map: Vec<ReadId>,   // archive_id → original_id
    pub block_boundaries: Vec<BlockBoundary>,
    pub num_blocks: usize,
}
```

### Block Boundaries

After reordering, block boundaries are computed based on the recommended block size for the read length class:

```rust
fn compute_block_boundaries(&self, total_reads: u64, reads_per_block: usize) -> Vec<BlockBoundary> {
    let num_blocks = (total_reads as usize).div_ceil(reads_per_block);
    // Create boundaries with start/end archive IDs
}
```

| Read Length Class | Block Size |
|-------------------|------------|
| Short | 100,000 reads |
| Medium | 50,000 reads |
| Long | 10,000 reads |

### Block Lookup

The result supports efficient block lookup via binary search:

```rust
pub fn find_block(&self, archive_id: ReadId) -> Option<BlockId> {
    let idx = self.block_boundaries
        .partition_point(|b| b.archive_id_start <= archive_id);
    if idx == 0 { return None; }
    let b = &self.block_boundaries[idx - 1];
    if archive_id >= b.archive_id_start && archive_id < b.archive_id_end {
        Some(b.block_id)
    } else { None }
}
```

## When Reordering is Applied

Reordering is **only** performed when all conditions are met:

| Condition | Requirement | Reason |
|-----------|-------------|--------|
| `enable_reorder` | `true` | User disabled |
| `ReadLengthClass` | `Short` | ABC needs locality; Zstd doesn't benefit |
| Paired-end | `false` | PE data has its own ordering constraints |
| Streaming mode | `false` | Requires all reads in memory |

## Performance Characteristics

### Time Complexity

| Phase | Complexity | Notes |
|-------|------------|-------|
| Minimizer extraction | O(n × L) | n reads, L read length, parallel |
| Bucket building | O(n × m) | m minimizers per read |
| Greedy reordering | O(n × max_search × m) | Bounded search per step |
| Block boundary computation | O(n) | Simple division |

For typical data (n=100M reads, m=15 minimizers/read, max_search=64):
- Minimizer extraction: ~30 seconds (parallel)
- Bucket building: ~10 seconds
- Reordering: ~2-5 minutes

### Space Complexity

| Component | Memory | Notes |
|-----------|--------|-------|
| Minimizer hashes | O(n × m × 8 bytes) | ~12 GB for 100M reads × 15 minimizers |
| Bucket map | O(n × m × 16 bytes) | HashMap overhead |
| Ordering arrays | O(n × 16 bytes) | Forward + reverse maps |

### Parameter Tuning

| Parameter | Increase | Decrease |
|-----------|----------|----------|
| `k` (k-mer length) | More specific minimizers, fewer false matches | More sensitive, more minimizers per read |
| `w` (window size) | Fewer minimizers per read, faster | More minimizers, better sensitivity |
| `max_search_reorder` | Better reordering, slower | Faster, potentially worse grouping |

**Default parameters** (k=15, w=10, max_search=64) work well for typical Illumina data (150bp reads).

## Example

Consider 5 reads with sequences that have the following minimizer hashes:

```
R0: [0x100, 0x200, 0x300]
R1: [0x200, 0x400, 0x500]   -- shares 0x200 with R0
R2: [0x600, 0x700, 0x800]   -- no shared minimizers
R3: [0x300, 0x900, 0xA00]   -- shares 0x300 with R0
R4: [0x400, 0xB00, 0xC00]   -- shares 0x400 with R1
```

Bucket map:
```
0x100 → [R0]
0x200 → [R0, R1]
0x300 → [R0, R3]
0x400 → [R1, R4]
0x500 → [R1]
0x600 → [R2]
0x700 → [R2]
0x800 → [R2]
0x900 → [R3]
0xA00 → [R3]
0xB00 → [R4]
0xC00 → [R4]
```

Greedy reordering starting from R0:
1. Start: ordered = [R0]
2. R0's minimizers → candidates: R1 (via 0x200), R3 (via 0x300) → pick R3 (similar length)
3. R3's minimizers → no unused candidates → fallback to first unused (R1)
4. R1's minimizers → candidates: R4 (via 0x400) → pick R4
5. R4's minimizers → no candidates → pick R2

Result: `[R0, R3, R1, R4, R2]`

## Integration with Compression

After reordering:
1. Reads are accessed in archive order via `reverse_map[archive_id]`
2. Block boundaries are computed on the reordered sequence
3. The reorder map is serialized and stored in the archive
4. On decompression, `reverse_map` restores original order if requested

```
Original:     R0   R1   R2   R3   R4
                \  / \  /    |    |
Reordered:    R0  R3  R1  R4  R2
                  |   |   |   |  |
Blocks:        [Block 0  ] [B1]
```

## Related Documents

- [Reorder Map Architecture](../architecture/reorder-map.md)
- [Strategy Selection](./strategy-selection.md)
- [ABC Algorithm](./abc.md)
- [Source Module Overview](../architecture/modules.md)
