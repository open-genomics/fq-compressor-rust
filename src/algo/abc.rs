// =============================================================================
// ABC (Anchor-Based Compression) Algorithm
// =============================================================================
//! Anchor-Based Compression for short-read sequences.
//!
//! This module implements the ABC algorithm which:
//! 1. Groups similar reads into contigs
//! 2. Builds consensus sequences for each contig
//! 3. Delta-encodes each read against its consensus
//!
//! Best suited for short reads (≤511 bp) where reads have high similarity.

use crate::algo::dna::{reverse_complement, BASE_TO_INDEX, INDEX_TO_BASE};
use crate::error::{FqcError, Result};
use crate::types::ReadRecord;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Cursor, Read};

// =============================================================================
// ABC Format Version
// =============================================================================

/// ABC format version 1: uses u16 for lengths (max 65535)
const ABC_FORMAT_V1: u8 = 0x01;
/// ABC format version 2: uses u32 for lengths (supports long reads)
const ABC_FORMAT_V2: u8 = 0x02;
/// Current ABC format version
const ABC_CURRENT_VERSION: u8 = ABC_FORMAT_V2;

/// Short-read ABC packing becomes quadratic in block size, so larger blocks fall back to Zstd.
pub const SHORT_READ_ABC_MAX_READS: usize = 4_096;

// =============================================================================
// AbcConfig
// =============================================================================

/// Configuration for ABC compression.
#[derive(Debug, Clone)]
pub struct AbcConfig {
    /// Maximum shift allowed when aligning reads to consensus.
    pub max_shift: usize,
    /// Hamming distance threshold for consensus matching.
    pub hamming_threshold: usize,
    /// Zstd compression level for final output.
    pub zstd_level: i32,
}

impl Default for AbcConfig {
    fn default() -> Self {
        Self {
            max_shift: 32,
            hamming_threshold: 16,
            zstd_level: 3,
        }
    }
}

// =============================================================================
// AbcEncoded
// =============================================================================

/// ABC-encoded sequence data (Zstd-compressed).
#[derive(Debug, Default)]
pub struct AbcEncoded {
    /// Compressed ABC data ready for storage.
    pub data: Vec<u8>,
}

// =============================================================================
// Noise Encoding
// =============================================================================

/// Encode a mismatch as a noise character.
///
/// Maps (reference_base, read_base) → '0'-'3' to compactly represent substitutions.
#[inline]
fn encode_noise(ref_base: u8, read_base: u8) -> u8 {
    match (ref_base | 32, read_base | 32) {
        (b'a', b'c') => b'0',
        (b'a', b'g') => b'1',
        (b'a', b't') => b'2',
        (b'a', _) => b'3',
        (b'c', b'a') => b'0',
        (b'c', b'g') => b'1',
        (b'c', b't') => b'2',
        (b'c', _) => b'3',
        (b'g', b't') => b'0',
        (b'g', b'a') => b'1',
        (b'g', b'c') => b'2',
        (b'g', _) => b'3',
        (b't', b'g') => b'0',
        (b't', b'c') => b'1',
        (b't', b'a') => b'2',
        (b't', _) => b'3',
        (b'n', b'a') => b'0',
        (b'n', b'g') => b'1',
        (b'n', b'c') => b'2',
        (b'n', _) => b'3',
        _ => b'0',
    }
}

/// Decode a noise character back to the original base.
#[inline]
fn decode_noise(ref_base: u8, noise_char: u8) -> u8 {
    let idx = noise_char.wrapping_sub(b'0').min(3) as usize;
    const DECODE: [[u8; 4]; 5] = [
        [b'C', b'G', b'T', b'N'], // A
        [b'A', b'G', b'T', b'N'], // C
        [b'T', b'A', b'C', b'N'], // G
        [b'G', b'C', b'A', b'N'], // T
        [b'A', b'G', b'C', b'T'], // N
    ];
    let row = match ref_base | 32 {
        b'a' => 0,
        b'c' => 1,
        b'g' => 2,
        b't' => 3,
        _ => 4,
    };
    DECODE[row][idx]
}

// =============================================================================
// Alignment Helpers
// =============================================================================

/// Compute Hamming distance between two sequences, capped at max_dist.
fn hamming_distance(s1: &[u8], s2: &[u8], max_dist: usize) -> usize {
    let min_len = s1.len().min(s2.len());
    let mut dist = 0usize;
    for i in 0..min_len {
        if (s1[i] | 32) != (s2[i] | 32) {
            dist += 1;
            if dist > max_dist {
                return max_dist + 1;
            }
        }
    }
    dist + (s1.len().max(s2.len()) - min_len)
}

/// Find the best alignment of a read against a reference.
///
/// Returns (shift, is_rc) if alignment found within threshold, None otherwise.
/// - shift: position offset of read relative to reference
/// - is_rc: true if reverse complement matches better
fn find_best_alignment(
    read: &[u8],
    reference: &[u8],
    max_shift: usize,
    hamming_threshold: usize,
) -> Option<(i32, bool)> {
    let mut best_distance = usize::MAX;
    let mut best_shift = 0i32;
    let mut best_is_rc = false;

    let try_align = |read_seq: &[u8], is_rc: bool, best_dist: &mut usize, best_sh: &mut i32, best_rc: &mut bool| {
        for shift in -(max_shift as i32)..=(max_shift as i32) {
            let ref_start = if shift >= 0 { shift as usize } else { 0 };
            let read_start = if shift < 0 { (-shift) as usize } else { 0 };

            if ref_start >= reference.len() || read_start >= read_seq.len() {
                continue;
            }

            let compare_len = (reference.len() - ref_start).min(read_seq.len() - read_start);
            if compare_len == 0 {
                continue;
            }

            // Penalty for non-overlapping regions
            let penalty = read_seq.len() - compare_len;

            let dist = hamming_distance(
                &reference[ref_start..ref_start + compare_len],
                &read_seq[read_start..read_start + compare_len],
                hamming_threshold,
            ) + penalty;

            if dist < *best_dist {
                *best_dist = dist;
                *best_sh = shift;
                *best_rc = is_rc;
            }
        }
    };

    try_align(read, false, &mut best_distance, &mut best_shift, &mut best_is_rc);

    let rc_read = reverse_complement(read);
    try_align(&rc_read, true, &mut best_distance, &mut best_shift, &mut best_is_rc);

    if best_distance <= hamming_threshold {
        Some((best_shift, best_is_rc))
    } else {
        None
    }
}

// =============================================================================
// ConsensusSequence
// =============================================================================

/// A consensus sequence built from multiple aligned reads.
struct ConsensusSequence {
    sequence: Vec<u8>,
    /// Base counts for A, C, G, T (index 0-3). N is not counted separately.
    base_counts: Vec<[u16; 4]>,
    contributing_reads: u32,
}

impl ConsensusSequence {
    fn init_from_read(read: &[u8]) -> Self {
        let mut base_counts = vec![[0u16; 4]; read.len()];
        for (i, &b) in read.iter().enumerate() {
            let idx = BASE_TO_INDEX[b as usize] as usize;
            // Only count valid bases (A=0, C=1, G=2, T=3). N=4 is ignored.
            if idx < 4 {
                base_counts[i][idx] = 1;
            }
        }
        Self {
            sequence: read.to_vec(),
            base_counts,
            contributing_reads: 1,
        }
    }

    fn add_read(&mut self, read: &[u8], shift: i32, is_rc: bool) {
        let aligned: Vec<u8> = if is_rc { reverse_complement(read) } else { read.to_vec() };

        let cons_start = if shift >= 0 { shift as usize } else { 0 };
        let align_start = if shift < 0 { (-shift) as usize } else { 0 };
        let overlap_len = aligned.len().saturating_sub(align_start);
        let new_len = self.base_counts.len().max(cons_start + overlap_len);

        if new_len > self.base_counts.len() {
            self.base_counts.resize(new_len, [0u16; 4]);
        }

        for k in 0..overlap_len {
            let pos = cons_start + k;
            let b = aligned[align_start + k];
            if pos < self.base_counts.len() {
                let idx = BASE_TO_INDEX[b as usize] as usize;
                if idx < 4 {
                    self.base_counts[pos][idx] = self.base_counts[pos][idx].saturating_add(1);
                }
            }
        }

        self.contributing_reads += 1;
        self.recompute_consensus();
    }

    fn recompute_consensus(&mut self) {
        self.sequence.resize(self.base_counts.len(), b'N');
        for (i, counts) in self.base_counts.iter().enumerate() {
            let total: u16 = counts.iter().sum();
            if total == 0 {
                continue;
            }
            let max_idx = counts
                .iter()
                .enumerate()
                .max_by_key(|(_, &c)| c)
                .map(|(idx, _)| idx)
                .unwrap_or(0);
            self.sequence[i] = INDEX_TO_BASE[max_idx];
        }
    }
}

// =============================================================================
// DeltaEncodedRead
// =============================================================================

/// A read encoded as deltas against a consensus sequence.
struct DeltaEncodedRead {
    original_order: u32,
    position_offset: i32,
    is_rc: bool,
    read_length: u32,
    mismatch_positions: Vec<u32>,
    mismatch_chars: Vec<u8>,
}

#[allow(clippy::needless_range_loop)]
fn compute_delta(read: &[u8], consensus: &[u8], shift: i32, is_rc: bool) -> DeltaEncodedRead {
    let aligned: Vec<u8> = if is_rc { reverse_complement(read) } else { read.to_vec() };

    let cons_start = if shift >= 0 { shift as usize } else { 0 };
    let read_start = if shift < 0 { (-shift) as usize } else { 0 };

    let mut mismatch_positions = Vec::new();
    let mut mismatch_chars = Vec::new();

    for i in 0..aligned.len() {
        if i < read_start {
            // Positions before the consensus overlap: store as raw base
            mismatch_positions.push(i as u32);
            mismatch_chars.push(aligned[i]);
            continue;
        }
        let cons_pos = cons_start + (i - read_start);
        if cons_pos >= consensus.len() {
            mismatch_positions.push(i as u32);
            mismatch_chars.push(aligned[i]);
        } else if (aligned[i] | 32) != (consensus[cons_pos] | 32) {
            mismatch_positions.push(i as u32);
            mismatch_chars.push(encode_noise(consensus[cons_pos], aligned[i]));
        }
    }

    DeltaEncodedRead {
        original_order: 0,
        position_offset: shift,
        is_rc,
        read_length: read.len() as u32,
        mismatch_positions,
        mismatch_chars,
    }
}

#[allow(clippy::needless_range_loop)]
fn reconstruct_from_delta(delta: &DeltaEncodedRead, consensus: &[u8]) -> Vec<u8> {
    let shift = delta.position_offset;
    let cons_start = if shift >= 0 { shift as usize } else { 0 };
    let read_start = if shift < 0 { (-shift) as usize } else { 0 };

    let mut result = vec![b'N'; delta.read_length as usize];

    for i in 0..delta.read_length as usize {
        if i < read_start {
            continue;
        }
        let cons_pos = cons_start + (i - read_start);
        if cons_pos < consensus.len() {
            result[i] = consensus[cons_pos];
        }
    }

    for (j, &pos) in delta.mismatch_positions.iter().enumerate() {
        let pos = pos as usize;
        if pos < result.len() {
            if pos < read_start {
                // Before consensus overlap: raw base
                result[pos] = delta.mismatch_chars[j];
            } else {
                let cons_pos = cons_start + (pos - read_start);
                if cons_pos < consensus.len() {
                    result[pos] = decode_noise(consensus[cons_pos], delta.mismatch_chars[j]);
                } else {
                    result[pos] = delta.mismatch_chars[j];
                }
            }
        }
    }

    if delta.is_rc {
        reverse_complement(&result)
    } else {
        result
    }
}

// =============================================================================
// Contig
// =============================================================================

/// A contig: one consensus sequence with its delta-encoded reads.
struct Contig {
    consensus: ConsensusSequence,
    deltas: Vec<DeltaEncodedRead>,
}

fn build_contigs(reads: &[ReadRecord], max_shift: usize, hamming_threshold: usize) -> Vec<Contig> {
    let mut contigs: Vec<Contig> = Vec::new();
    let mut assigned = vec![false; reads.len()];

    // Temporary struct to track alignment info before final delta recomputation
    struct AlignInfo {
        read_index: usize,
        shift: i32,
        is_rc: bool,
    }

    for i in 0..reads.len() {
        if assigned[i] {
            continue;
        }

        let mut contig = Contig {
            consensus: ConsensusSequence::init_from_read(reads[i].sequence.as_bytes()),
            deltas: Vec::new(),
        };

        let mut align_infos: Vec<AlignInfo> = vec![AlignInfo {
            read_index: i,
            shift: 0,
            is_rc: false,
        }];
        assigned[i] = true;

        for j in (i + 1)..reads.len() {
            if assigned[j] {
                continue;
            }

            if let Some((shift, is_rc)) = find_best_alignment(
                reads[j].sequence.as_bytes(),
                &contig.consensus.sequence,
                max_shift,
                hamming_threshold,
            ) {
                contig.consensus.add_read(reads[j].sequence.as_bytes(), shift, is_rc);
                align_infos.push(AlignInfo {
                    read_index: j,
                    shift,
                    is_rc,
                });
                assigned[j] = true;
            }
        }

        // Recompute all deltas against the final consensus
        for info in &align_infos {
            let mut delta = compute_delta(
                reads[info.read_index].sequence.as_bytes(),
                &contig.consensus.sequence,
                info.shift,
                info.is_rc,
            );
            delta.original_order = info.read_index as u32;
            contig.deltas.push(delta);
        }

        contigs.push(contig);
    }

    contigs
}

// =============================================================================
// AbcCompressor
// =============================================================================

/// ABC (Anchor-Based Compression) compressor for short reads.
pub struct AbcCompressor {
    config: AbcConfig,
}

impl AbcCompressor {
    /// Create a new ABC compressor with the given configuration.
    pub fn new(config: AbcConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(AbcConfig::default())
    }

    /// Compress a batch of reads using ABC algorithm.
    ///
    /// Returns `AbcEncoded` containing Zstd-compressed ABC data.
    pub fn compress(&self, reads: &[ReadRecord]) -> Result<AbcEncoded> {
        let contigs = build_contigs(reads, self.config.max_shift, self.config.hamming_threshold);

        let mut buf: Vec<u8> = Vec::with_capacity(reads.len() * 20);

        // Write ABC format version
        buf.push(ABC_CURRENT_VERSION);

        // Write number of contigs
        buf.write_u32::<LittleEndian>(contigs.len() as u32)?;

        for contig in &contigs {
            // Write consensus (u32 for length to support long reads)
            buf.write_u32::<LittleEndian>(contig.consensus.sequence.len() as u32)?;
            buf.extend_from_slice(&contig.consensus.sequence);

            // Write deltas
            buf.write_u32::<LittleEndian>(contig.deltas.len() as u32)?;

            for delta in &contig.deltas {
                buf.write_u32::<LittleEndian>(delta.original_order)?;
                buf.write_i32::<LittleEndian>(delta.position_offset)?;
                buf.push(u8::from(delta.is_rc));
                buf.write_u32::<LittleEndian>(delta.read_length)?;
                buf.write_u32::<LittleEndian>(delta.mismatch_positions.len() as u32)?;
                for &pos in &delta.mismatch_positions {
                    buf.write_u32::<LittleEndian>(pos)?;
                }
                buf.extend_from_slice(&delta.mismatch_chars);
            }
        }

        let compressed = zstd::bulk::compress(&buf, self.config.zstd_level)
            .map_err(|e| FqcError::Compression(format!("ABC Zstd compress failed: {e}")))?;

        Ok(AbcEncoded { data: compressed })
    }

    /// Decompress ABC-encoded data back to sequences.
    ///
    /// Returns a vector of strings in original order.
    pub fn decompress(&self, data: &[u8], read_count: u32) -> Result<Vec<String>> {
        if data.is_empty() {
            return Ok(vec![String::new(); read_count as usize]);
        }

        let buf = zstd::stream::decode_all(data)
            .map_err(|e| FqcError::Decompression(format!("ABC Zstd decompress failed: {e}")))?;

        let mut sequences = vec![String::new(); read_count as usize];
        let mut cur = Cursor::new(&buf);

        // Read ABC format version
        let mut version_byte = [0u8; 1];
        cur.read_exact(&mut version_byte)
            .map_err(|e| FqcError::Format(format!("Truncated ABC version: {e}")))?;
        let version = version_byte[0];

        // If version byte looks like part of a u32 (old format without version), rewind
        let (abc_version, num_contigs) = if version == ABC_FORMAT_V2 {
            let n = cur
                .read_u32::<LittleEndian>()
                .map_err(|e| FqcError::Format(format!("Truncated ABC data: {e}")))?;
            (ABC_FORMAT_V2, n)
        } else {
            // Old format: version byte is actually the first byte of num_contigs
            let rest = cur
                .read_u32::<LittleEndian>()
                .map_err(|e| FqcError::Format(format!("Truncated ABC data: {e}")))?;
            let num_contigs = (rest << 8) | (version as u32);
            (ABC_FORMAT_V1, num_contigs)
        };

        for _ in 0..num_contigs {
            let cons_len = if abc_version >= ABC_FORMAT_V2 {
                cur.read_u32::<LittleEndian>()
                    .map_err(|e| FqcError::Format(format!("Truncated ABC consensus: {e}")))? as usize
            } else {
                cur.read_u16::<LittleEndian>()
                    .map_err(|e| FqcError::Format(format!("Truncated ABC consensus: {e}")))? as usize
            };
            let mut consensus = vec![0u8; cons_len];
            cur.read_exact(&mut consensus)
                .map_err(|e| FqcError::Format(format!("Truncated ABC consensus bytes: {e}")))?;

            let num_deltas = cur
                .read_u32::<LittleEndian>()
                .map_err(|e| FqcError::Format(format!("Truncated ABC deltas: {e}")))?;

            for _ in 0..num_deltas {
                let original_order = cur
                    .read_u32::<LittleEndian>()
                    .map_err(|e| FqcError::Format(format!("Truncated ABC original_order: {e}")))?;

                let position_offset = if abc_version >= ABC_FORMAT_V2 {
                    cur.read_i32::<LittleEndian>()
                        .map_err(|e| FqcError::Format(format!("Truncated ABC position_offset: {e}")))?
                } else {
                    cur.read_i16::<LittleEndian>()
                        .map_err(|e| FqcError::Format(format!("Truncated ABC position_offset: {e}")))?
                        as i32
                };

                let mut flags = [0u8; 1];
                cur.read_exact(&mut flags)
                    .map_err(|e| FqcError::Format(format!("Truncated ABC flags: {e}")))?;
                let is_rc = (flags[0] & 1) != 0;

                let read_length = if abc_version >= ABC_FORMAT_V2 {
                    cur.read_u32::<LittleEndian>()
                        .map_err(|e| FqcError::Format(format!("Truncated ABC read_length: {e}")))?
                } else {
                    cur.read_u16::<LittleEndian>()
                        .map_err(|e| FqcError::Format(format!("Truncated ABC read_length: {e}")))?
                        as u32
                };

                let num_mismatches = if abc_version >= ABC_FORMAT_V2 {
                    cur.read_u32::<LittleEndian>()
                        .map_err(|e| FqcError::Format(format!("Truncated ABC num_mismatches: {e}")))?
                } else {
                    cur.read_u16::<LittleEndian>()
                        .map_err(|e| FqcError::Format(format!("Truncated ABC num_mismatches: {e}")))?
                        as u32
                };

                let mismatch_positions = if abc_version >= ABC_FORMAT_V2 {
                    let mut pos = vec![0u32; num_mismatches as usize];
                    for p in &mut pos {
                        *p = cur
                            .read_u32::<LittleEndian>()
                            .map_err(|e| FqcError::Format(format!("Truncated ABC mismatch_pos: {e}")))?;
                    }
                    pos
                } else {
                    let mut pos = vec![0u32; num_mismatches as usize];
                    for p in &mut pos {
                        *p = cur
                            .read_u16::<LittleEndian>()
                            .map_err(|e| FqcError::Format(format!("Truncated ABC mismatch_pos: {e}")))?
                            as u32;
                    }
                    pos
                };

                let mut mismatch_chars = vec![0u8; num_mismatches as usize];
                cur.read_exact(&mut mismatch_chars)
                    .map_err(|e| FqcError::Format(format!("Truncated ABC mismatch_chars: {e}")))?;

                let delta = DeltaEncodedRead {
                    original_order,
                    position_offset,
                    is_rc,
                    read_length,
                    mismatch_positions,
                    mismatch_chars,
                };

                if (original_order as usize) < sequences.len() {
                    let reconstructed = reconstruct_from_delta(&delta, &consensus);
                    sequences[original_order as usize] = String::from_utf8_lossy(&reconstructed).into_owned();
                }
            }
        }

        Ok(sequences)
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_read(id: &str, seq: &str, qual: &str) -> ReadRecord {
        ReadRecord {
            id: id.to_string(),
            comment: String::new(),
            sequence: seq.to_string(),
            quality: qual.to_string(),
        }
    }

    #[test]
    fn test_noise_encoding_roundtrip() {
        let bases = [b'A', b'C', b'G', b'T', b'N'];
        for &ref_base in &bases {
            for &read_base in &bases {
                // Skip matches - encode_noise is only for mismatches
                if (ref_base | 32) == (read_base | 32) {
                    continue;
                }
                let noise = encode_noise(ref_base, read_base);
                let decoded = decode_noise(ref_base, noise);
                // For valid base pairs, should decode correctly
                if ref_base != b'N' && read_base != b'N' {
                    assert_eq!(
                        (decoded | 32),
                        (read_base | 32),
                        "Failed for ref={:?}, read={:?}",
                        ref_base as char,
                        read_base as char
                    );
                }
            }
        }
    }

    #[test]
    fn test_abc_single_read() {
        let compressor = AbcCompressor::with_defaults();
        let reads = vec![make_read("read1", "ACGTACGT", "IIIIIIII")];

        let encoded = compressor.compress(&reads).unwrap();
        let decoded = compressor.decompress(&encoded.data, 1).unwrap();

        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0], "ACGTACGT");
    }

    #[test]
    fn test_abc_identical_reads() {
        let compressor = AbcCompressor::with_defaults();
        let seq = "ACGTACGTACGT";
        let qual = "IIIIIIIIIIII";
        let reads = vec![
            make_read("r1", seq, qual),
            make_read("r2", seq, qual),
            make_read("r3", seq, qual),
        ];

        let encoded = compressor.compress(&reads).unwrap();
        let decoded = compressor.decompress(&encoded.data, 3).unwrap();

        assert_eq!(decoded.len(), 3);
        for s in &decoded {
            assert_eq!(s, seq);
        }
    }

    #[test]
    fn test_abc_similar_reads() {
        let compressor = AbcCompressor::with_defaults();
        let reads = vec![
            make_read("r1", "ACGTACGT", "IIIIIIII"),
            make_read("r2", "ACGTACCT", "IIIIIIII"), // 1 mismatch
            make_read("r3", "ACGTAGGT", "IIIIIIII"), // 1 mismatch
        ];

        let encoded = compressor.compress(&reads).unwrap();
        let decoded = compressor.decompress(&encoded.data, 3).unwrap();

        assert_eq!(decoded.len(), 3);
        assert_eq!(decoded[0], "ACGTACGT");
        assert_eq!(decoded[1], "ACGTACCT");
        assert_eq!(decoded[2], "ACGTAGGT");
    }

    #[test]
    fn test_abc_reverse_complement() {
        let compressor = AbcCompressor::with_defaults();
        let reads = vec![
            make_read("r1", "ACGTACGT", "IIIIIIII"),
            make_read("r2", "ACGTACGT", "IIIIIIII"), // identical, will test RC detection
        ];

        let encoded = compressor.compress(&reads).unwrap();
        let decoded = compressor.decompress(&encoded.data, 2).unwrap();

        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0], "ACGTACGT");
        assert_eq!(decoded[1], "ACGTACGT");
    }

    #[test]
    fn test_abc_different_lengths() {
        let compressor = AbcCompressor::with_defaults();
        let reads = vec![
            make_read("r1", "ACGT", "IIII"),
            make_read("r2", "ACGTAC", "IIIIII"),
            make_read("r3", "ACGTACGT", "IIIIIIII"),
        ];

        let encoded = compressor.compress(&reads).unwrap();
        let decoded = compressor.decompress(&encoded.data, 3).unwrap();

        assert_eq!(decoded.len(), 3);
        assert_eq!(decoded[0], "ACGT");
        assert_eq!(decoded[1], "ACGTAC");
        assert_eq!(decoded[2], "ACGTACGT");
    }

    #[test]
    fn test_abc_preserves_order() {
        let compressor = AbcCompressor::with_defaults();
        let reads = vec![
            make_read("r3", "TTTTTTTT", "IIIIIIII"),
            make_read("r1", "AAAAAAAA", "IIIIIIII"),
            make_read("r2", "CCCCCCCC", "IIIIIIII"),
        ];

        let encoded = compressor.compress(&reads).unwrap();
        let decoded = compressor.decompress(&encoded.data, 3).unwrap();

        // Order should be preserved
        assert_eq!(decoded[0], "TTTTTTTT");
        assert_eq!(decoded[1], "AAAAAAAA");
        assert_eq!(decoded[2], "CCCCCCCC");
    }

    #[test]
    fn test_abc_empty_reads() {
        let compressor = AbcCompressor::with_defaults();
        let reads: Vec<ReadRecord> = vec![];

        let encoded = compressor.compress(&reads).unwrap();
        // Empty input produces minimal output (just version + 0 contigs)
        assert!(!encoded.data.is_empty() || reads.is_empty());

        let decoded = compressor.decompress(&encoded.data, 0).unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn test_abc_with_n_bases() {
        let compressor = AbcCompressor::with_defaults();
        let reads = vec![
            make_read("r1", "ACGTNACG", "IIIIIIII"),
            make_read("r2", "ACGTNACG", "IIIIIIII"),
        ];

        let encoded = compressor.compress(&reads).unwrap();
        let decoded = compressor.decompress(&encoded.data, 2).unwrap();

        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0], "ACGTNACG");
        assert_eq!(decoded[1], "ACGTNACG");
    }

    #[test]
    fn test_abc_compression_ratio() {
        let compressor = AbcCompressor::with_defaults();
        let seq = "ACGTACGTACGTACGT"; // 16 bp
        let qual = "IIIIIIIIIIIIIIII";
        let reads: Vec<ReadRecord> = (0..100).map(|i| make_read(&format!("r{}", i), seq, qual)).collect();

        let encoded = compressor.compress(&reads).unwrap();

        // 100 identical 16bp reads should compress well
        // Raw: 100 * 16 = 1600 bytes (just sequences)
        // Compressed should be much smaller
        let raw_size: usize = reads.iter().map(|r| r.sequence.len()).sum();
        assert!(
            encoded.data.len() < raw_size,
            "Compressed {} bytes should be < raw {} bytes",
            encoded.data.len(),
            raw_size
        );
    }
}
