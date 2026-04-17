# ABC Algorithm (Consensus + Delta Encoding)

This document describes the ABC (Alignment-Based Compression) algorithm used by fqc for compressing short-read DNA sequences (< 300bp).

## Overview

ABC is a domain-specific compression algorithm that exploits the high sequence similarity among reads from the same genomic sample. Instead of storing each read independently, ABC computes a **consensus sequence** representing the population and encodes each read as **deltas** (differences) from that consensus.

The algorithm is inspired by the [Spring](https://github.com/shubhamchandak94/Spring) compressor and the ABC method described in ["FASTQ Compression" (2014)](https://web.archive.org/web/20140302093354/http://www.ebi.ac.uk/~swah/FastqCompression.pdf).

## Algorithm Pipeline

```
Reads → Contig Building → Consensus Computation → Delta Encoding → Serialization → Zstd
```

### Step 1: Contig Building

Reads are grouped into **contigs** — clusters of reads that align well to a common consensus sequence. The process is greedy:

1. Start with the first unassigned read as the seed of a new contig
2. For each remaining unassigned read, attempt to align it to the contig's current consensus
3. If alignment succeeds (Hamming distance <= threshold), add the read to the contig
4. The consensus is updated incrementally as new reads are added
5. Repeat until all reads are assigned to contigs

```rust
fn build_contigs(reads: &[ReadRecord], max_shift: usize, hamming_threshold: usize) -> Vec<Contig> {
    let mut contigs: Vec<Contig> = Vec::new();
    let mut assigned = vec![false; reads.len()];

    for i in 0..reads.len() {
        if assigned[i] { continue; }

        // Start new contig with read i
        let mut contig = Contig {
            consensus: ConsensusSequence::init_from_read(reads[i].sequence.as_bytes()),
            deltas: Vec::new(),
        };
        assigned[i] = true;

        // Try to add remaining reads
        for j in (i + 1)..reads.len() {
            if assigned[j] { continue; }

            if let Some((shift, is_rc)) = find_best_alignment(
                reads[j].sequence.as_bytes(),
                &contig.consensus.sequence,
                max_shift,
                hamming_threshold,
            ) {
                contig.consensus.add_read(reads[j].sequence.as_bytes(), shift, is_rc);
                assigned[j] = true;
            }
        }

        // Recompute all deltas against final consensus
        contigs.push(contig);
    }
    contigs
}
```

### Step 2: Consensus Computation

The consensus is not simply the first read in the contig. It is computed dynamically using **base counting** at each position:

```rust
struct ConsensusSequence {
    sequence: Vec<u8>,              // Current consensus bases
    base_counts: Vec<[u16; 4]>,     // Counts of A, C, G, T at each position
    contributing_reads: u32,        // Number of reads contributing
}
```

When a read is added:
1. The read is aligned to the consensus (with optional shift and reverse complement)
2. Base counts are incremented for each position in the overlap
3. The consensus is recomputed as the majority base at each position

```rust
fn recompute_consensus(&mut self) {
    self.sequence.resize(self.base_counts.len(), b'N');
    for (i, counts) in self.base_counts.iter().enumerate() {
        let total: u16 = counts.iter().sum();
        if total == 0 {
            continue;  // No valid bases, keep 'N'
        }
        let max_idx = counts.iter().enumerate()
            .max_by_key(|(_, &c)| c)
            .map(|(idx, _)| idx)
            .unwrap_or(0);
        self.sequence[i] = INDEX_TO_BASE[max_idx];  // A, C, G, or T
    }
}
```

Base counts only include valid bases (A=0, C=1, G=2, T=3). `N` bases are ignored and do not contribute to the consensus.

### Step 3: Alignment Search

Each candidate read is aligned to the consensus using a bounded shift search:

```rust
fn find_best_alignment(
    read: &[u8],
    reference: &[u8],
    max_shift: usize,        // default: 32
    hamming_threshold: usize // default: 16
) -> Option<(i32, bool)>
```

The search considers:
- **Forward orientation**: Read as-is
- **Reverse complement**: Read reversed and complemented (A↔T, C↔G)
- **Shift range**: `-max_shift` to `+max_shift` (default: -32 to +32)

For each shift position, the Hamming distance is computed between the overlapping regions. A penalty is applied for non-overlapping regions:

```rust
let penalty = read_seq.len() - compare_len;
let dist = hamming_distance(...) + penalty;
```

Alignment succeeds if the best distance is <= `hamming_threshold` (default 16).

### Step 4: Delta Encoding

Once the final consensus is computed for a contig, all reads in the contig are delta-encoded against it:

```rust
struct DeltaEncodedRead {
    original_order: u32,         // Original position within the block
    position_offset: i32,        // Alignment shift
    is_rc: bool,                 // Reverse complemented
    read_length: u32,            // Read length in bases
    mismatch_positions: Vec<u32>,// Positions where read differs from consensus
    mismatch_chars: Vec<u8>,     // Encoded mismatch characters
}
```

**Mismatch encoding**:

For positions within the consensus overlap, mismatches are encoded using a 4-character noise scheme:

```rust
fn encode_noise(ref_base: u8, read_base: u8) -> u8 {
    match (ref_base | 32, read_base | 32) {
        (b'a', b'c') => b'0',   // A→C
        (b'a', b'g') => b'1',   // A→G
        (b'a', b't') => b'2',   // A→T
        (b'a', _)    => b'3',   // A→N/other
        // ... similar for C, G, T, N reference bases
    }
}
```

For positions outside the consensus overlap (read extends beyond consensus), raw bases are stored directly.

### Step 5: Serialization (ABC Format v2)

Contigs are serialized into a binary format:

```
+-------------------+
| Version (u8)      |  Always 0x02
+-------------------+
| Num Contigs (u32) |
+-------------------+
| For each contig:  |
|   Consensus Len   |  u32 (supports long reads)
|   Consensus Bases |  raw bytes
|   Num Deltas      |  u32
|   For each delta: |
|     Original Order|  u32
|     Position Offset| i32 (supports long reads)
|     Flags         |  u8 (bit 0: is_rc)
|     Read Length   |  u32
|     Num Mismatches|  u32
|     Mismatch Pos  |  u32 each
|     Mismatch Chars|  raw bytes
+-------------------+
```

The serialized buffer is then compressed with zstd.

### ABC Format Versions

| Version | Length Fields | Max Read Length | Notes |
|---------|---------------|-----------------|-------|
| V1 (0x01) | `u16` | 65,535 bp | Original format |
| V2 (0x02) | `u32` | 4,294,967,295 bp | Supports long reads |

The decompressor auto-detects the version from the first byte.

## Reverse Complement Handling

fqc considers both orientations when aligning reads:

```rust
pub fn reverse_complement(seq: &[u8]) -> Vec<u8> {
    seq.iter()
        .rev()
        .map(|&c| COMPLEMENT[c as usize])
        .collect()
}
```

The complement table:

| Base | Complement |
|------|------------|
| A | T |
| C | G |
| G | C |
| T | A |
| N | N |

The `is_rc` flag in `DeltaEncodedRead` indicates whether the stored delta was computed from the reverse complement. During decompression, the reconstructed sequence is reverse-complemented back if this flag is set.

## Decompression

Delta decoding reconstructs reads from the consensus and deltas:

```rust
fn reconstruct_from_delta(delta: &DeltaEncodedRead, consensus: &[u8]) -> Vec<u8> {
    let mut result = vec![b'N'; delta.read_length as usize];

    // 1. Fill with consensus at aligned positions
    for i in read_start..read_length {
        let cons_pos = cons_start + (i - read_start);
        if cons_pos < consensus.len() {
            result[i] = consensus[cons_pos];
        }
    }

    // 2. Apply mismatches
    for (j, &pos) in delta.mismatch_positions.iter().enumerate() {
        if pos < read_start {
            result[pos] = delta.mismatch_chars[j];  // Raw base
        } else {
            let cons_pos = cons_start + (pos - read_start);
            result[pos] = decode_noise(consensus[cons_pos], delta.mismatch_chars[j]);
        }
    }

    // 3. Reverse complement if needed
    if delta.is_rc {
        reverse_complement(&result)
    } else {
        result
    }
}
```

## Configuration

| Parameter | Default | Description |
|-----------|---------|-------------|
| `max_shift` | 32 | Maximum alignment shift in bases |
| `consensus_hamming_threshold` | 16 | Maximum Hamming distance for alignment |
| `zstd_level` | 3 | Zstd level for serializing contigs |

The zstd level for the ABC payload is derived from the user's compression level:

| User Level | ABC Zstd Level |
|------------|----------------|
| 1-2 | 1 |
| 3-4 | 3 |
| 5-6 | 5 |
| 7-8 | 9 |
| 9 | 15 |

## Performance Characteristics

### Compression Ratio

For typical Illumina short-read data (150bp, 30x coverage):
- ABC achieves **4-8x** compression on sequence data
- This is **20-30% better** than plain Zstd for the same data

### Time Complexity

| Phase | Complexity | Notes |
|-------|------------|-------|
| Contig building | O(n² × L) in worst case | Greedy, n reads, L read length |
| Consensus update | O(L) per read | Base counting |
| Alignment search | O(max_shift × L) per candidate | Bounded Hamming distance |
| Delta encoding | O(L) per read | Single pass |
| Zstd serialization | O(S) | S = serialized size |

In practice, the greedy algorithm runs much faster than O(n²) because most reads align to early contigs.

### Memory Usage

| Component | Memory | Notes |
|-----------|--------|-------|
| Consensus base counts | O(consensus_length × 8 bytes) | 2 bytes × 4 bases per position |
| Delta reads | O(reads_per_contig × avg_mismatches × 8 bytes) | Sparse representation |
| Zstd buffer | O(serialized_contig_size) | Temporary |

## When ABC is Used

ABC is selected when:
- `ReadLengthClass == Short` (all reads < 511bp, median < 1,024bp)
- The block codec is `AbcV1` (0x10)

For medium and long reads, `ZstdPlain` is used instead.

## Limitations

1. **Maximum effectiveness**: ABC works best when reads have high coverage and low diversity (e.g., resequencing). For highly diverse data (metagenomics, high-error-rate reads), fewer reads will align to each contig, reducing compression efficiency.

2. **No structural variant detection**: ABC does not detect large insertions, deletions, or structural variants. Reads with large structural differences will form separate contigs.

3. **Greedy algorithm**: The greedy contig building is not optimal. A read assigned to an early contig might have been a better fit for a later one. However, the greedy approach is much faster and produces good results in practice.

## Related Documents

- [Strategy Selection](./strategy-selection.md)
- [Zstd Integration](./zstd.md)
- [Source Module Overview](../architecture/modules.md)
- [Block Format](../architecture/block-format.md)
- [Compression Algorithms RFC](../../specs/rfc/0002-compression-algorithms.md)
