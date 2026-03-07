// =============================================================================
// fqc-rust - Paired-End Optimizer
// =============================================================================
// Implements PE complementarity encoding: R2 stored as diff from R1-RC.
// =============================================================================

use crate::algo::block_compressor::reverse_complement;
use crate::types::ReadRecord;

// =============================================================================
// Constants
// =============================================================================

const COMPLEMENTARITY_THRESHOLD: usize = 15;
const MIN_COMPLEMENTARITY_OVERLAP: usize = 20;

// =============================================================================
// PEEncodedPair
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct PEEncodedPair {
    pub id1: String,
    pub seq1: String,
    pub qual1: String,
    pub id2: String,
    pub seq2: String,
    pub qual2: String,
    pub use_complementarity: bool,
    pub diff_positions: Vec<u16>,
    pub diff_bases: Vec<u8>,
    pub qual_delta: Vec<i8>,
}

impl PEEncodedPair {
    /// Decode R2 sequence from R1 sequence + diffs
    pub fn decode_r2_sequence(&self, r1_seq: &str) -> String {
        if !self.use_complementarity {
            return self.seq2.clone();
        }

        // Start with R1 reverse complement
        let r1_rc = reverse_complement(r1_seq.as_bytes());
        let mut result: Vec<u8> = r1_rc;

        // Apply differences
        for (i, &pos) in self.diff_positions.iter().enumerate() {
            if (pos as usize) < result.len() && i < self.diff_bases.len() {
                result[pos as usize] = self.diff_bases[i];
            }
        }

        String::from_utf8_lossy(&result).into_owned()
    }

    /// Decode R2 quality from R1 quality + deltas
    pub fn decode_r2_quality(&self, r1_qual: &str) -> String {
        if !self.use_complementarity {
            return self.qual2.clone();
        }

        // Start with reversed R1 quality
        let mut result: Vec<u8> = r1_qual.as_bytes().iter().rev().copied().collect();

        // Apply quality deltas at diff positions
        for (i, &pos) in self.diff_positions.iter().enumerate() {
            if (pos as usize) < result.len() && i < self.qual_delta.len() {
                let new_qual = (result[pos as usize] as i16) + (self.qual_delta[i] as i16);
                result[pos as usize] = new_qual.clamp(33, 126) as u8;
            }
        }

        String::from_utf8_lossy(&result).into_owned()
    }
}

// =============================================================================
// PEOptimizer
// =============================================================================

#[derive(Debug, Clone)]
pub struct PEOptimizerConfig {
    pub enable_complementarity: bool,
    pub complementarity_threshold: usize,
    pub min_overlap: usize,
}

impl Default for PEOptimizerConfig {
    fn default() -> Self {
        Self {
            enable_complementarity: true,
            complementarity_threshold: COMPLEMENTARITY_THRESHOLD,
            min_overlap: MIN_COMPLEMENTARITY_OVERLAP,
        }
    }
}

#[derive(Debug, Default)]
pub struct PEOptimizerStats {
    pub total_pairs: u64,
    pub complementarity_used: u64,
    pub bytes_saved: u64,
}

pub struct PEOptimizer {
    config: PEOptimizerConfig,
    stats: PEOptimizerStats,
}

impl PEOptimizer {
    pub fn new(config: PEOptimizerConfig) -> Self {
        Self { config, stats: PEOptimizerStats::default() }
    }

    pub fn stats(&self) -> &PEOptimizerStats {
        &self.stats
    }

    /// Check if R2 is approximately R1's reverse complement.
    /// Returns (beneficial, diff_count).
    pub fn check_complementarity(&self, r1_seq: &[u8], r2_seq: &[u8]) -> (bool, usize) {
        if !self.config.enable_complementarity {
            return (false, 0);
        }

        let min_len = r1_seq.len().min(r2_seq.len());
        if min_len < self.config.min_overlap {
            return (false, 0);
        }

        let r1_rc = reverse_complement(r1_seq);

        let mut diff_count = 0usize;
        for i in 0..min_len {
            if r1_rc[i] != r2_seq[i] {
                diff_count += 1;
                if diff_count > self.config.complementarity_threshold {
                    return (false, diff_count);
                }
            }
        }

        // Add length difference
        diff_count += r1_seq.len().abs_diff(r2_seq.len());

        let beneficial = diff_count <= self.config.complementarity_threshold;
        (beneficial, diff_count)
    }

    /// Compute diff positions and bases between two sequences.
    fn compute_diff(seq1: &[u8], seq2: &[u8]) -> (Vec<u16>, Vec<u8>) {
        let min_len = seq1.len().min(seq2.len());
        let mut positions = Vec::new();
        let mut bases = Vec::new();

        for i in 0..min_len {
            if seq1[i] != seq2[i] {
                positions.push(i as u16);
                bases.push(seq2[i]);
            }
        }

        // Handle length differences (extra bases in seq2)
        for (i, &b) in seq2.iter().enumerate().skip(min_len) {
            positions.push(i as u16);
            bases.push(b);
        }

        (positions, bases)
    }

    /// Encode a paired-end read pair, optionally using complementarity.
    pub fn encode_pair(&mut self, r1: &ReadRecord, r2: &ReadRecord) -> PEEncodedPair {
        let mut encoded = PEEncodedPair {
            id1: r1.id.clone(),
            seq1: r1.sequence.clone(),
            qual1: r1.quality.clone(),
            id2: r2.id.clone(),
            ..Default::default()
        };

        let (beneficial, _diff_count) = self.check_complementarity(
            r1.sequence.as_bytes(), r2.sequence.as_bytes()
        );

        if beneficial {
            encoded.use_complementarity = true;

            let r1_rc = reverse_complement(r1.sequence.as_bytes());
            let (positions, bases) = Self::compute_diff(&r1_rc, r2.sequence.as_bytes());

            // Compute quality deltas
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

            encoded.diff_positions = positions;
            encoded.diff_bases = bases;
            encoded.qual_delta = qual_delta;

            self.stats.complementarity_used += 1;
            let saved = r2.sequence.len().saturating_sub(encoded.diff_positions.len() * 3);
            self.stats.bytes_saved += saved as u64;
        } else {
            encoded.use_complementarity = false;
            encoded.seq2 = r2.sequence.clone();
            encoded.qual2 = r2.quality.clone();
        }

        self.stats.total_pairs += 1;
        encoded
    }

    /// Decode a pair back to two ReadRecords.
    pub fn decode_pair(&self, encoded: &PEEncodedPair) -> (ReadRecord, ReadRecord) {
        let r1 = ReadRecord {
            id: encoded.id1.clone(),
            sequence: encoded.seq1.clone(),
            quality: encoded.qual1.clone(),
        };

        let r2 = ReadRecord {
            id: encoded.id2.clone(),
            sequence: encoded.decode_r2_sequence(&encoded.seq1),
            quality: encoded.decode_r2_quality(&encoded.qual1),
        };

        (r1, r2)
    }
}

// =============================================================================
// Utility Functions
// =============================================================================

/// Generate R2 ID from R1 ID.
pub fn generate_r2_id(r1_id: &str) -> String {
    let bytes = r1_id.as_bytes();
    let len = bytes.len();

    // Check for /1 or .1 suffix
    if len >= 2 && bytes[len - 1] == b'1' && (bytes[len - 2] == b'/' || bytes[len - 2] == b'.') {
        let mut id = r1_id.to_string();
        unsafe { id.as_bytes_mut()[len - 1] = b'2'; }
        return id;
    }

    // Check for space-separated format: "id 1:..." -> "id 2:..."
    if let Some(space_pos) = r1_id.find(' ') {
        if space_pos + 1 < len && bytes[space_pos + 1] == b'1' {
            let mut id = r1_id.to_string();
            unsafe { id.as_bytes_mut()[space_pos + 1] = b'2'; }
            return id;
        }
    }

    // Default: append /2
    format!("{}/2", r1_id)
}

// =============================================================================
// Serialization
// =============================================================================

/// Serialize a PEEncodedPair to bytes.
pub fn serialize_encoded_pair(pair: &PEEncodedPair) -> Vec<u8> {
    let mut buf = Vec::with_capacity(256);

    // Flags byte
    let flags: u8 = if pair.use_complementarity { 1 } else { 0 };
    buf.push(flags);

    // ID2 (empty if same-pattern as ID1)
    let id2_bytes = pair.id2.as_bytes();
    push_varint(&mut buf, id2_bytes.len() as u64);
    buf.extend_from_slice(id2_bytes);

    if pair.use_complementarity {
        // Diff count
        push_varint(&mut buf, pair.diff_positions.len() as u64);

        // Diff positions (delta-encoded)
        let mut prev: u16 = 0;
        for &pos in &pair.diff_positions {
            push_varint(&mut buf, (pos - prev) as u64);
            prev = pos;
        }

        // Diff bases
        buf.extend_from_slice(&pair.diff_bases);

        // Quality deltas
        for &d in &pair.qual_delta {
            buf.push(d as u8);
        }
    } else {
        // Full seq2 + qual2
        let seq2 = pair.seq2.as_bytes();
        push_varint(&mut buf, seq2.len() as u64);
        buf.extend_from_slice(seq2);

        let qual2 = pair.qual2.as_bytes();
        push_varint(&mut buf, qual2.len() as u64);
        buf.extend_from_slice(qual2);
    }

    buf
}

/// Deserialize a PEEncodedPair from bytes.
pub fn deserialize_encoded_pair(data: &[u8], pos: &mut usize) -> Option<PEEncodedPair> {
    if *pos >= data.len() { return None; }

    let flags = data[*pos];
    *pos += 1;
    let use_comp = (flags & 1) != 0;

    // ID2
    let id2_len = read_varint(data, pos) as usize;
    if *pos + id2_len > data.len() { return None; }
    let id2 = String::from_utf8_lossy(&data[*pos..*pos + id2_len]).into_owned();
    *pos += id2_len;

    let mut pair = PEEncodedPair {
        use_complementarity: use_comp,
        id2,
        ..Default::default()
    };

    if use_comp {
        let diff_count = read_varint(data, pos) as usize;

        // Diff positions (delta-decoded)
        let mut positions = Vec::with_capacity(diff_count);
        let mut prev: u16 = 0;
        for _ in 0..diff_count {
            let delta = read_varint(data, pos) as u16;
            prev += delta;
            positions.push(prev);
        }
        pair.diff_positions = positions;

        // Diff bases
        if *pos + diff_count > data.len() { return None; }
        pair.diff_bases = data[*pos..*pos + diff_count].to_vec();
        *pos += diff_count;

        // Quality deltas
        if *pos + diff_count > data.len() { return None; }
        pair.qual_delta = data[*pos..*pos + diff_count].iter().map(|&b| b as i8).collect();
        *pos += diff_count;
    } else {
        let seq2_len = read_varint(data, pos) as usize;
        if *pos + seq2_len > data.len() { return None; }
        pair.seq2 = String::from_utf8_lossy(&data[*pos..*pos + seq2_len]).into_owned();
        *pos += seq2_len;

        let qual2_len = read_varint(data, pos) as usize;
        if *pos + qual2_len > data.len() { return None; }
        pair.qual2 = String::from_utf8_lossy(&data[*pos..*pos + qual2_len]).into_owned();
        *pos += qual2_len;
    }

    Some(pair)
}

fn push_varint(buf: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        buf.push((v as u8 & 0x7F) | 0x80);
        v >>= 7;
    }
    buf.push(v as u8);
}

fn read_varint(data: &[u8], pos: &mut usize) -> u64 {
    let mut result: u64 = 0;
    let mut shift = 0u32;
    for _ in 0..10 {
        if *pos >= data.len() { break; }
        let b = data[*pos];
        *pos += 1;
        result |= ((b & 0x7F) as u64) << shift;
        if b & 0x80 == 0 { return result; }
        shift += 7;
    }
    result
}
