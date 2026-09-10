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

use crate::algo::compressor_traits::SequenceCompressor;
use crate::algo::dna::{is_valid_base, reverse_complement, BASE_TO_INDEX, INDEX_TO_BASE};
use crate::error::{FqcError, Result};
use crate::types::{encode_codec, CodecFamily, ReadRecord, SPRING_MAX_READ_LENGTH};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Cursor, Read};

// =============================================================================
// ABC Format Version
// =============================================================================

/// ABC format version 1: u16 lengths, ASCII '0'-'3' substitution codes.
const ABC_FORMAT_V1: u8 = 0x01;
/// ABC format version 2: u32 lengths, ASCII '0'-'3' substitution codes.
const ABC_FORMAT_V2: u8 = 0x02;
/// ABC format version 3: lossless mismatch chars — substitution codes are
/// bytes 0x00-0x03, ANY other byte is the raw read base verbatim (IUPAC
/// codes, digits, case differences).
const ABC_FORMAT_V3: u8 = 0x03;
/// Current ABC format version
const ABC_CURRENT_VERSION: u8 = ABC_FORMAT_V3;

/// Normalize legacy (V1/V2) mismatch chars — ASCII '0'-'3' — into the V3
/// code space (0x00-0x03). Applied after reading mismatch_chars so the
/// reconstruction path has a single uniform rule.
#[inline]
fn normalize_legacy_noise_char(c: u8) -> u8 {
    if (b'0'..=b'3').contains(&c) {
        c - b'0'
    } else {
        c
    }
}

/// Short-read ABC packing becomes quadratic in block size, so larger blocks fall back to Zstd.
pub const SHORT_READ_ABC_MAX_READS: usize = 4_096;
const MAX_ABC_DECODED_BYTES_PER_READ: usize = 4_096;

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

/// Substitution codes are bytes 0x00-0x03 (V3 format). Any other byte in
/// `mismatch_chars` is the raw read base verbatim.
#[inline]
fn decode_as_substitution(c: u8) -> Option<usize> {
    if c <= 3 {
        Some(c as usize)
    } else {
        None
    }
}

#[inline]
fn can_encode_substitution_losslessly(read_base: u8) -> bool {
    matches!(read_base, b'A' | b'C' | b'G' | b'T' | b'N')
}

fn can_reverse_complement_losslessly(seq: &[u8]) -> bool {
    seq.iter().all(|&base| {
        matches!(
            base | 32,
            b'a' | b'c' | b'g' | b't' | b'n' | b'r' | b'y' | b's' | b'w' | b'k' | b'm' | b'b' | b'v' | b'd' | b'h'
        )
    })
}

/// Encode a mismatch as a noise character.
///
/// Maps (reference_base, read_base) → 0..=3 to compactly represent substitutions.
/// Bases whose exact byte cannot be recovered by a substitution code (IUPAC
/// codes, digits, case-only differences handled by the caller) are returned
/// verbatim; decode passes such bytes through unchanged.
#[inline]
fn encode_noise(ref_base: u8, read_base: u8) -> u8 {
    if !can_encode_substitution_losslessly(read_base) {
        return read_base;
    }
    match (ref_base | 32, read_base | 32) {
        (b'a', b'c') => 0,
        (b'a', b'g') => 1,
        (b'a', b't') => 2,
        (b'a', _) => 3,
        (b'c', b'a') => 0,
        (b'c', b'g') => 1,
        (b'c', b't') => 2,
        (b'c', _) => 3,
        (b'g', b't') => 0,
        (b'g', b'a') => 1,
        (b'g', b'c') => 2,
        (b'g', _) => 3,
        (b't', b'g') => 0,
        (b't', b'c') => 1,
        (b't', b'a') => 2,
        (b't', _) => 3,
        (b'n', b'a') => 0,
        (b'n', b'g') => 1,
        (b'n', b'c') => 2,
        // Unreachable via the is_lossless_representable guard above; kept for
        // exhaustiveness.
        _ => read_base,
    }
}

/// Decode a noise character back to the original base.
/// Bytes outside the substitution-code space carry the raw read base verbatim.
#[inline]
fn decode_noise(ref_base: u8, noise_char: u8) -> u8 {
    let idx = match decode_as_substitution(noise_char) {
        Some(idx) => idx,
        None => return noise_char,
    };
    const DECODE: [[u8; 4]; 5] = [
        *b"CGTN", // A
        *b"AGTN", // C
        *b"TACN", // G
        *b"GCAN", // T
        *b"AGCT", // N
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
    // No useful alignment can start farther than the two sequences together;
    // clamp the public configuration before entering the shift loop so a
    // forged/accidental `usize::MAX` setting cannot turn compression into an
    // effectively unbounded CPU walk.
    let max_shift = max_shift
        .min(reference.len().saturating_add(read.len()))
        .min(i32::MAX as usize) as i32;

    let try_align = |read_seq: &[u8], is_rc: bool, best_dist: &mut usize, best_sh: &mut i32, best_rc: &mut bool| {
        for shift in -max_shift..=max_shift {
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

    if can_reverse_complement_losslessly(read) {
        let rc_read = reverse_complement(read);
        try_align(&rc_read, true, &mut best_distance, &mut best_shift, &mut best_is_rc);
    }

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
            if is_valid_base(b) {
                let idx = BASE_TO_INDEX[b as usize] as usize;
                // Only count valid bases (A=0, C=1, G=2, T=3). N=4 is ignored.
                if idx < 4 {
                    base_counts[i][idx] = 1;
                }
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
            if pos < self.base_counts.len() && is_valid_base(b) {
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
        } else if aligned[i] != consensus[cons_pos] {
            mismatch_positions.push(i as u32);
            if (aligned[i] | 32) == (consensus[cons_pos] | 32) {
                // Case-only difference: not a substitution; store raw byte.
                mismatch_chars.push(aligned[i]);
            } else {
                mismatch_chars.push(encode_noise(consensus[cons_pos], aligned[i]));
            }
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

fn decode_contig(
    cur: &mut Cursor<&[u8]>,
    buf: &[u8],
    abc_version: u8,
    read_count: usize,
    sequences: &mut [String],
    seen: &mut [bool],
    decoded_reads: &mut usize,
) -> Result<()> {
    let cons_len = if abc_version >= ABC_FORMAT_V2 {
        cur.read_u32::<LittleEndian>()
            .map_err(|e| FqcError::Format(format!("Truncated ABC consensus: {e}")))? as usize
    } else {
        cur.read_u16::<LittleEndian>()
            .map_err(|e| FqcError::Format(format!("Truncated ABC consensus: {e}")))? as usize
    };
    if cons_len > SPRING_MAX_READ_LENGTH {
        return Err(FqcError::Format(format!(
            "ABC consensus length {cons_len} exceeds short-read limit {SPRING_MAX_READ_LENGTH}"
        )));
    }
    let consensus_start = cur.position() as usize;
    let consensus_end = consensus_start
        .checked_add(cons_len)
        .ok_or_else(|| FqcError::Format("ABC consensus extent overflows".to_string()))?;
    let consensus = buf
        .get(consensus_start..consensus_end)
        .ok_or_else(|| FqcError::Format("Truncated ABC consensus bytes".to_string()))?;
    if consensus.iter().any(|&byte| byte <= 3) {
        return Err(FqcError::Format(
            "ABC consensus contains reserved control bytes 0x00-0x03".to_string(),
        ));
    }
    cur.set_position(consensus_end as u64);

    let num_deltas = cur
        .read_u32::<LittleEndian>()
        .map_err(|e| FqcError::Format(format!("Truncated ABC deltas: {e}")))? as usize;
    if num_deltas == 0 || num_deltas > read_count - *decoded_reads {
        return Err(FqcError::Format(format!(
            "ABC contig declares invalid delta count {num_deltas}"
        )));
    }

    for _ in 0..num_deltas {
        let original_order = cur
            .read_u32::<LittleEndian>()
            .map_err(|e| FqcError::Format(format!("Truncated ABC original_order: {e}")))?;
        let original_index = original_order as usize;
        if original_index >= sequences.len() || seen[original_index] {
            return Err(FqcError::Format(format!(
                "ABC has duplicate or out-of-range read index {original_order}"
            )));
        }

        let position_offset = if abc_version >= ABC_FORMAT_V2 {
            cur.read_i32::<LittleEndian>()
                .map_err(|e| FqcError::Format(format!("Truncated ABC position_offset: {e}")))?
        } else {
            i32::from(
                cur.read_i16::<LittleEndian>()
                    .map_err(|e| FqcError::Format(format!("Truncated ABC position_offset: {e}")))?,
            )
        };
        let mut flags = [0u8; 1];
        cur.read_exact(&mut flags)
            .map_err(|e| FqcError::Format(format!("Truncated ABC flags: {e}")))?;
        if flags[0] & !1 != 0 {
            return Err(FqcError::Format(format!(
                "ABC flags 0x{:02x} contain reserved bits",
                flags[0]
            )));
        }
        let read_length = if abc_version >= ABC_FORMAT_V2 {
            cur.read_u32::<LittleEndian>()
                .map_err(|e| FqcError::Format(format!("Truncated ABC read_length: {e}")))?
        } else {
            u32::from(
                cur.read_u16::<LittleEndian>()
                    .map_err(|e| FqcError::Format(format!("Truncated ABC read_length: {e}")))?,
            )
        };
        if read_length as usize > SPRING_MAX_READ_LENGTH {
            return Err(FqcError::Format(format!(
                "ABC read length {read_length} exceeds short-read limit {SPRING_MAX_READ_LENGTH}"
            )));
        }
        if (read_length == 0 && position_offset != 0)
            || (read_length > 0 && position_offset.unsigned_abs() as usize >= read_length as usize)
        {
            return Err(FqcError::Format(format!(
                "ABC alignment offset {position_offset} has no read overlap"
            )));
        }

        let num_mismatches = if abc_version >= ABC_FORMAT_V2 {
            cur.read_u32::<LittleEndian>()
                .map_err(|e| FqcError::Format(format!("Truncated ABC num_mismatches: {e}")))?
        } else {
            u32::from(
                cur.read_u16::<LittleEndian>()
                    .map_err(|e| FqcError::Format(format!("Truncated ABC num_mismatches: {e}")))?,
            )
        } as usize;
        if num_mismatches > read_length as usize {
            return Err(FqcError::Format(format!(
                "ABC declares {num_mismatches} mismatches for a {read_length}-byte read"
            )));
        }
        let position_width = if abc_version >= ABC_FORMAT_V2 { 4 } else { 2 };
        let remaining = buf.len().saturating_sub(cur.position() as usize);
        let required = num_mismatches
            .checked_mul(position_width)
            .and_then(|n| n.checked_add(num_mismatches))
            .ok_or_else(|| FqcError::Format("ABC mismatch extent overflows".to_string()))?;
        if required > remaining {
            return Err(FqcError::Format("Truncated ABC mismatch data".to_string()));
        }

        let mut mismatch_positions = Vec::with_capacity(num_mismatches);
        let mut previous_position = None;
        for _ in 0..num_mismatches {
            let position = if abc_version >= ABC_FORMAT_V2 {
                cur.read_u32::<LittleEndian>()
                    .map_err(|e| FqcError::Format(format!("Truncated ABC mismatch_pos: {e}")))?
            } else {
                u32::from(
                    cur.read_u16::<LittleEndian>()
                        .map_err(|e| FqcError::Format(format!("Truncated ABC mismatch_pos: {e}")))?,
                )
            };
            if position >= read_length || previous_position.is_some_and(|previous| position <= previous) {
                return Err(FqcError::Format(format!(
                    "ABC mismatch position {position} is invalid for read length {read_length}"
                )));
            }
            previous_position = Some(position);
            mismatch_positions.push(position);
        }

        let mut mismatch_chars = vec![0u8; num_mismatches];
        cur.read_exact(&mut mismatch_chars)
            .map_err(|e| FqcError::Format(format!("Truncated ABC mismatch_chars: {e}")))?;
        if abc_version < ABC_FORMAT_V3 {
            for c in &mut mismatch_chars {
                *c = normalize_legacy_noise_char(*c);
            }
        }

        let delta = DeltaEncodedRead {
            original_order,
            position_offset,
            is_rc: flags[0] & 1 != 0,
            read_length,
            mismatch_positions,
            mismatch_chars,
        };
        let reconstructed = reconstruct_from_delta(&delta, consensus);
        sequences[original_index] = String::from_utf8(reconstructed)
            .map_err(|e| FqcError::Format(format!("Invalid UTF-8 in ABC sequence: {e}")))?;
        seen[original_index] = true;
        *decoded_reads += 1;
    }

    Ok(())
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
        if let Some(read) = reads.iter().find(|read| read.sequence.len() > SPRING_MAX_READ_LENGTH) {
            return Err(FqcError::InvalidArgument(format!(
                "ABC only supports reads up to {SPRING_MAX_READ_LENGTH} bytes; got {}",
                read.sequence.len()
            )));
        }
        if reads.iter().any(|read| read.sequence.bytes().any(|byte| byte <= 3)) {
            return Err(FqcError::InvalidArgument(
                "ABC cannot represent sequence control bytes 0x00-0x03".to_string(),
            ));
        }

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
        self.decompress_internal(data, read_count, None)
    }

    pub(crate) fn decompress_with_codec_revision(
        &self,
        data: &[u8],
        read_count: u32,
        codec_revision: u8,
    ) -> Result<Vec<String>> {
        self.decompress_internal(data, read_count, Some(codec_revision))
    }

    fn decompress_internal(
        &self,
        data: &[u8],
        read_count: u32,
        expected_codec_revision: Option<u8>,
    ) -> Result<Vec<String>> {
        if data.is_empty() {
            return if read_count == 0 {
                Ok(Vec::new())
            } else {
                Err(FqcError::Format(
                    "ABC stream is empty for a non-empty block".to_string(),
                ))
            };
        }

        let max_out = (read_count as usize)
            .saturating_mul(MAX_ABC_DECODED_BYTES_PER_READ)
            .saturating_add(4096)
            .max(256);
        let buf = crate::memory_budget::zstd_decompress_bounded(data, max_out, "abc sequences")?;
        let mut sequences = vec![String::new(); read_count as usize];
        let mut seen = vec![false; read_count as usize];
        let mut decoded_reads = 0usize;
        let mut cur = Cursor::new(buf.as_slice());

        let mut version_byte = [0u8; 1];
        cur.read_exact(&mut version_byte)
            .map_err(|e| FqcError::Format(format!("Truncated ABC version: {e}")))?;
        let version = version_byte[0];
        let (abc_version, num_contigs) = if version == ABC_FORMAT_V2 || version == ABC_FORMAT_V3 {
            let n = cur
                .read_u32::<LittleEndian>()
                .map_err(|e| FqcError::Format(format!("Truncated ABC data: {e}")))?;
            (version, n)
        } else {
            let rest = cur
                .read_u32::<LittleEndian>()
                .map_err(|e| FqcError::Format(format!("Truncated ABC data: {e}")))?;
            ((ABC_FORMAT_V1), (rest << 8) | u32::from(version))
        };

        if num_contigs as usize > read_count as usize {
            return Err(FqcError::Format(format!(
                "ABC declares {num_contigs} contigs for {read_count} reads"
            )));
        }
        if let Some(codec_revision) = expected_codec_revision {
            let payload_revision = u8::from(abc_version == ABC_FORMAT_V3);
            if codec_revision != payload_revision {
                return Err(FqcError::Format(format!(
                    "ABC codec revision {codec_revision} contradicts payload version {abc_version}"
                )));
            }
        }

        for _ in 0..num_contigs {
            decode_contig(
                &mut cur,
                &buf,
                abc_version,
                read_count as usize,
                &mut sequences,
                &mut seen,
                &mut decoded_reads,
            )?;
        }

        if decoded_reads != read_count as usize || seen.iter().any(|seen| !seen) {
            return Err(FqcError::Format(format!(
                "ABC decoded {decoded_reads} records for declared count {read_count}"
            )));
        }
        if cur.position() as usize != buf.len() {
            return Err(FqcError::Format("ABC stream has trailing bytes".to_string()));
        }

        Ok(sequences)
    }
}

// =============================================================================
// SequenceCompressor Trait Implementation
// =============================================================================

impl SequenceCompressor for AbcCompressor {
    fn compress(&self, reads: &[ReadRecord]) -> Result<Vec<u8>> {
        AbcCompressor::compress(self, reads).map(|encoded| encoded.data)
    }

    fn decompress(&self, data: &[u8], read_count: u32, _uniform_length: u32, _lengths: &[u32]) -> Result<Vec<String>> {
        AbcCompressor::decompress(self, data, read_count)
    }

    fn codec_id(&self) -> u8 {
        encode_codec(CodecFamily::AbcV1, 1)
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
        let bases = *b"ACGTN";
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
        // r2 is the true reverse complement of r1 (non-palindromic, so the
        // forward alignment cannot win and the is_rc delta path must execute).
        let r1 = "GATTACA";
        let r2 = "TGTAATC"; // reverse complement of GATTACA
        assert_ne!(r1, r2);
        let reads = vec![make_read("r1", r1, "IIIIIII"), make_read("r2", r2, "IIIIIII")];

        let encoded = compressor.compress(&reads).unwrap();
        let decoded = compressor.decompress(&encoded.data, 2).unwrap();

        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0], r1);
        assert_eq!(decoded[1], r2);
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
        // Empty input must still produce a valid (non-empty) versioned stream.
        assert!(
            !encoded.data.is_empty(),
            "empty input should produce a valid versioned stream"
        );

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
