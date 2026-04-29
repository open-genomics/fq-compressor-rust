// =============================================================================
// ABC Format Version (for backward compatibility)
// =============================================================================

/// ABC format version 1: uses u16 for lengths (max 65535)
const ABC_FORMAT_V1: u8 = 0x01;
/// ABC format version 2: uses u32 for lengths (supports long reads)
const ABC_FORMAT_V2: u8 = 0x02;
/// Current ABC format version
const ABC_CURRENT_VERSION: u8 = ABC_FORMAT_V2;

/// Short-read ABC packing becomes quadratic in block size, so larger blocks fall back to Zstd.
const SHORT_READ_ABC_MAX_READS: usize = 4_096;

// =============================================================================
// Block Compressor (ABC Algorithm + Zstd)
// =============================================================================

use crate::algo::dna::{reverse_complement, BASE_TO_INDEX, INDEX_TO_BASE};
use crate::algo::quality_compressor::{ContextOrder, QualityCompressor, QualityCompressorConfig};
use crate::error::{FqcError, Result};
use crate::types::*;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Cursor, Read};
use xxhash_rust::xxh64::Xxh64;

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

            // Penalty for non-overlapping regions (matches C++ implementation)
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

        // cons_start: where in consensus the overlap begins
        // align_start: where in the aligned read the overlap begins
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
                // Only count valid bases (A=0, C=1, G=2, T=3). N=4 is ignored.
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
                // No valid bases at this position, keep 'N'
                continue;
            }
            let max_idx = counts
                .iter()
                .enumerate()
                .max_by_key(|(_, &c)| c)
                .map(|(idx, _)| idx)
                .unwrap_or(0);
            // INDEX_TO_BASE[0..4] = [A, C, G, T]
            self.sequence[i] = INDEX_TO_BASE[max_idx];
        }
    }
}

// =============================================================================
// Delta Encoded Read
// =============================================================================

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

struct Contig {
    consensus: ConsensusSequence,
    deltas: Vec<DeltaEncodedRead>,
}

// =============================================================================
// BlockCompressorConfig
// =============================================================================

#[derive(Debug, Clone)]
pub struct BlockCompressorConfig {
    pub read_length_class: ReadLengthClass,
    pub compression_level: CompressionLevel,
    pub quality_mode: QualityMode,
    pub id_mode: IdMode,
    pub max_shift: usize,
    pub consensus_hamming_threshold: usize,
    pub zstd_level: i32,
    /// ID prefix for discard mode reconstruction (e.g., "read" → @read1, @read2, ...)
    pub id_prefix: String,
}

impl Default for BlockCompressorConfig {
    fn default() -> Self {
        Self {
            read_length_class: ReadLengthClass::Short,
            compression_level: DEFAULT_COMPRESSION_LEVEL,
            quality_mode: QualityMode::Lossless,
            id_mode: IdMode::Exact,
            max_shift: 32,
            consensus_hamming_threshold: 16,
            zstd_level: 3,
            id_prefix: String::from("read"),
        }
    }
}

impl BlockCompressorConfig {
    pub fn zstd_level_for_compression_level(level: CompressionLevel) -> i32 {
        match level {
            1..=2 => 1,
            3..=4 => 3,
            5..=6 => 5,
            7..=8 => 9,
            _ => 15,
        }
    }

    pub fn get_sequence_codec(&self) -> u8 {
        match self.read_length_class {
            ReadLengthClass::Short => encode_codec(CodecFamily::AbcV1, 0),
            _ => encode_codec(CodecFamily::ZstdPlain, 0),
        }
    }

    pub fn use_short_read_abc(&self, read_count: usize) -> bool {
        self.read_length_class == ReadLengthClass::Short && read_count <= SHORT_READ_ABC_MAX_READS
    }

    pub fn get_quality_codec(&self) -> u8 {
        if self.quality_mode == QualityMode::Discard {
            return encode_codec(CodecFamily::Raw, 0);
        }
        match self.read_length_class {
            ReadLengthClass::Long => encode_codec(CodecFamily::ScmOrder1, 0),
            _ => encode_codec(CodecFamily::ScmV1, 0),
        }
    }

    pub fn get_id_codec(&self) -> u8 {
        if self.id_mode == IdMode::Discard {
            return encode_codec(CodecFamily::Raw, 0);
        }
        encode_codec(CodecFamily::DeltaZstd, 0)
    }

    pub fn get_aux_codec(&self) -> u8 {
        encode_codec(CodecFamily::DeltaVarint, 0)
    }
}

// =============================================================================
// CompressedBlockData
// =============================================================================

#[derive(Debug, Default)]
pub struct CompressedBlockData {
    pub block_id: BlockId,
    pub read_count: u32,
    pub uniform_read_length: u32,
    pub block_checksum: u64,
    pub codec_ids: u8,
    pub codec_seq: u8,
    pub codec_qual: u8,
    pub codec_aux: u8,
    pub id_stream: Vec<u8>,
    pub seq_stream: Vec<u8>,
    pub qual_stream: Vec<u8>,
    pub aux_stream: Vec<u8>,
}

impl CompressedBlockData {
    pub fn total_compressed_size(&self) -> usize {
        self.id_stream.len() + self.seq_stream.len() + self.qual_stream.len() + self.aux_stream.len()
    }
}

// =============================================================================
// DecompressedBlockData
// =============================================================================

#[derive(Debug, Default)]
pub struct DecompressedBlockData {
    pub block_id: BlockId,
    pub reads: Vec<ReadRecord>,
}

// =============================================================================
// BlockCompressor
// =============================================================================

pub struct BlockCompressor {
    config: BlockCompressorConfig,
}

impl BlockCompressor {
    pub fn new(config: BlockCompressorConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &BlockCompressorConfig {
        &self.config
    }

    pub fn compress(&self, reads: &[ReadRecord], block_id: BlockId) -> Result<CompressedBlockData> {
        let mut result = CompressedBlockData {
            block_id,
            read_count: reads.len() as u32,
            ..Default::default()
        };

        if reads.is_empty() {
            return Ok(result);
        }

        // Compress sequences
        result.seq_stream = if self.config.use_short_read_abc(reads.len()) {
            result.codec_seq = encode_codec(CodecFamily::AbcV1, 0);
            compress_sequences_abc(
                reads,
                self.config.zstd_level,
                self.config.max_shift,
                self.config.consensus_hamming_threshold,
            )?
        } else {
            result.codec_seq = encode_codec(CodecFamily::ZstdPlain, 0);
            compress_sequences_zstd(reads, self.config.zstd_level)?
        };

        // Compress quality
        result.qual_stream = compress_quality(reads, &self.config)?;
        result.codec_qual = self.config.get_quality_codec();

        // Compress IDs
        result.id_stream = compress_ids(reads, &self.config, self.config.zstd_level)?;
        result.codec_ids = self.config.get_id_codec();

        // Compress aux (lengths)
        result.aux_stream = compress_aux(reads, self.config.zstd_level, &mut result.uniform_read_length)?;
        result.codec_aux = self.config.get_aux_codec();

        // Compute block checksum
        result.block_checksum = compute_block_checksum(reads);

        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn decompress_raw(
        &self,
        block_id: BlockId,
        read_count: u32,
        uniform_read_length: u32,
        codec_seq: u8,
        _codec_qual: u8,
        id_stream: &[u8],
        seq_stream: &[u8],
        qual_stream: &[u8],
        aux_stream: &[u8],
    ) -> Result<DecompressedBlockData> {
        let mut result = DecompressedBlockData {
            block_id,
            reads: vec![ReadRecord::default(); read_count as usize],
        };

        if read_count == 0 {
            return Ok(result);
        }

        // Decompress aux (lengths) first
        let lengths: Vec<u32> = if !aux_stream.is_empty() {
            decompress_aux(aux_stream, read_count)?
        } else {
            vec![]
        };

        // Decompress sequences
        let seq_codec_family = decode_codec_family(codec_seq);
        let sequences = if seq_codec_family == CodecFamily::AbcV1 {
            decompress_sequences_abc(seq_stream, read_count)?
        } else {
            decompress_sequences_zstd(seq_stream, read_count, uniform_read_length, &lengths)?
        };

        // Decompress quality
        let qualities = decompress_quality(qual_stream, read_count, uniform_read_length, &lengths, &self.config)?;

        // Decompress IDs
        let ids = decompress_ids(id_stream, read_count, &self.config)?;

        // Assemble reads (split decompressed full header into id + comment)
        for i in 0..read_count as usize {
            let full_header = ids.get(i).cloned().unwrap_or_default();
            if let Some(space_pos) = full_header.find(' ') {
                result.reads[i].id = full_header[..space_pos].to_string();
                result.reads[i].comment = full_header[space_pos + 1..].to_string();
            } else {
                result.reads[i].id = full_header;
            }
            result.reads[i].sequence = sequences.get(i).cloned().unwrap_or_default();
            result.reads[i].quality = qualities.get(i).cloned().unwrap_or_default();
        }

        Ok(result)
    }
}

// =============================================================================
// Sequence Compression (ABC)
// =============================================================================

fn compress_sequences_abc(
    reads: &[ReadRecord],
    zstd_level: i32,
    max_shift: usize,
    hamming_threshold: usize,
) -> Result<Vec<u8>> {
    let contigs = build_contigs(reads, max_shift, hamming_threshold);

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

    zstd::bulk::compress(&buf, zstd_level).map_err(|e| FqcError::Compression(format!("ABC Zstd compress failed: {e}")))
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

fn decompress_sequences_abc(data: &[u8], read_count: u32) -> Result<Vec<String>> {
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
    // Old format: first byte is part of num_contigs (little-endian u32), typically 0x01-0x10
    // New format: first byte is version (0x02), then num_contigs
    let (abc_version, num_contigs) = if version == ABC_FORMAT_V2 {
        // New format with version prefix
        let n = cur
            .read_u32::<LittleEndian>()
            .map_err(|e| FqcError::Format(format!("Truncated ABC data: {e}")))?;
        (ABC_FORMAT_V2, n)
    } else {
        // Old format: version byte is actually the first byte of num_contigs
        // Reconstruct num_contigs: version is the low byte
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
                    .map_err(|e| FqcError::Format(format!("Truncated ABC read_length: {e}")))? as u32
            };

            let num_mismatches = if abc_version >= ABC_FORMAT_V2 {
                cur.read_u32::<LittleEndian>()
                    .map_err(|e| FqcError::Format(format!("Truncated ABC num_mismatches: {e}")))?
            } else {
                cur.read_u16::<LittleEndian>()
                    .map_err(|e| FqcError::Format(format!("Truncated ABC num_mismatches: {e}")))? as u32
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

// =============================================================================
// Sequence Compression (Zstd - medium/long reads)
// =============================================================================

fn compress_sequences_zstd(reads: &[ReadRecord], zstd_level: i32) -> Result<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::with_capacity(reads.len() * 200);

    for read in reads {
        buf.write_u32::<LittleEndian>(read.sequence.len() as u32)?;
        buf.extend_from_slice(read.sequence.as_bytes());
    }

    zstd::bulk::compress(&buf, zstd_level)
        .map_err(|e| FqcError::Compression(format!("Zstd sequence compress failed: {e}")))
}

fn decompress_sequences_zstd(
    data: &[u8],
    read_count: u32,
    _uniform_read_length: u32,
    _lengths: &[u32],
) -> Result<Vec<String>> {
    if data.is_empty() {
        return Ok(vec![String::new(); read_count as usize]);
    }

    let buf = zstd::stream::decode_all(data)
        .map_err(|e| FqcError::Decompression(format!("Zstd sequence decompress failed: {e}")))?;

    let mut sequences = Vec::with_capacity(read_count as usize);
    let mut cur = Cursor::new(&buf);

    for _ in 0..read_count {
        let len = cur
            .read_u32::<LittleEndian>()
            .map_err(|e| FqcError::Format(format!("Truncated sequence data: {e}")))?;
        let mut seq = vec![0u8; len as usize];
        cur.read_exact(&mut seq)
            .map_err(|e| FqcError::Format(format!("Truncated sequence bytes: {e}")))?;
        sequences.push(String::from_utf8_lossy(&seq).into_owned());
    }

    Ok(sequences)
}

// =============================================================================
// Quality Compression
// =============================================================================

fn compress_quality(reads: &[ReadRecord], config: &BlockCompressorConfig) -> Result<Vec<u8>> {
    if config.quality_mode == QualityMode::Discard {
        return Ok(Vec::new());
    }

    let qual_config = QualityCompressorConfig {
        quality_mode: config.quality_mode,
        context_order: if config.read_length_class == ReadLengthClass::Long {
            ContextOrder::Order1
        } else {
            ContextOrder::Order2
        },
        num_position_bins: 8,
    };

    let mut compressor = QualityCompressor::new(qual_config);
    let qualities: Vec<&str> = reads.iter().map(|r| r.quality.as_str()).collect();
    compressor.compress(&qualities)
}

fn decompress_quality(
    data: &[u8],
    read_count: u32,
    uniform_read_length: u32,
    lengths: &[u32],
    config: &BlockCompressorConfig,
) -> Result<Vec<String>> {
    let qual_config = QualityCompressorConfig {
        quality_mode: config.quality_mode,
        context_order: if config.read_length_class == ReadLengthClass::Long {
            ContextOrder::Order1
        } else {
            ContextOrder::Order2
        },
        num_position_bins: 8,
    };

    let mut compressor = QualityCompressor::new(qual_config);

    let length_vec: Vec<u32> = if uniform_read_length > 0 {
        vec![uniform_read_length; read_count as usize]
    } else if !lengths.is_empty() {
        lengths.to_vec()
    } else {
        return Err(FqcError::Format(
            "Missing length info for quality decompress".to_string(),
        ));
    };

    compressor.decompress(data, &length_vec)
}

// =============================================================================
// ID Compression
// =============================================================================

fn compress_ids(reads: &[ReadRecord], config: &BlockCompressorConfig, zstd_level: i32) -> Result<Vec<u8>> {
    // Combine id + comment into full header line for compression
    let full_headers: Vec<String> = reads
        .iter()
        .map(|r| {
            if r.comment.is_empty() {
                r.id.clone()
            } else {
                format!("{} {}", r.id, r.comment)
            }
        })
        .collect();
    let id_refs: Vec<&str> = full_headers.iter().map(|s| s.as_str()).collect();
    crate::algo::id_compressor::compress_ids(&id_refs, zstd_level, config.id_mode == IdMode::Discard)
}

fn decompress_ids(data: &[u8], read_count: u32, config: &BlockCompressorConfig) -> Result<Vec<String>> {
    crate::algo::id_compressor::decompress_ids(data, read_count, &config.id_prefix)
}

// =============================================================================
// Aux Compression (Read Lengths)
// =============================================================================

fn compress_aux(reads: &[ReadRecord], zstd_level: i32, uniform_length: &mut u32) -> Result<Vec<u8>> {
    if reads.is_empty() {
        *uniform_length = 0;
        return Ok(Vec::new());
    }

    let first_len = reads[0].sequence.len();
    let is_uniform = reads.iter().all(|r| r.sequence.len() == first_len);

    if is_uniform {
        *uniform_length = first_len as u32;
        return Ok(Vec::new());
    }

    *uniform_length = 0;

    let mut buf: Vec<u8> = Vec::with_capacity(reads.len() * 4);
    let mut prev_len = 0i32;

    for read in reads {
        let len = read.sequence.len() as i32;
        let delta = len - prev_len;
        prev_len = len;

        let mut zigzag = ((delta << 1) ^ (delta >> 31)) as u32;
        loop {
            let byte = (zigzag & 0x7F) as u8;
            zigzag >>= 7;
            if zigzag != 0 {
                buf.push(byte | 0x80);
            } else {
                buf.push(byte);
                break;
            }
        }
    }

    zstd::bulk::compress(&buf, zstd_level).map_err(|e| FqcError::Compression(format!("Aux Zstd compress failed: {e}")))
}

fn decompress_aux(data: &[u8], read_count: u32) -> Result<Vec<u32>> {
    if data.is_empty() {
        return Ok(Vec::new());
    }

    let buf = zstd::stream::decode_all(data)
        .map_err(|e| FqcError::Decompression(format!("Aux Zstd decompress failed: {e}")))?;

    let mut lengths = Vec::with_capacity(read_count as usize);
    let mut i = 0usize;
    let mut prev_len = 0i32;

    while i < buf.len() && lengths.len() < read_count as usize {
        let mut zigzag = 0u32;
        let mut shift = 0u32;

        for _ in 0..5 {
            if i >= buf.len() {
                break;
            }
            let byte = buf[i];
            i += 1;
            zigzag |= ((byte & 0x7F) as u32) << shift;
            shift += 7;
            if (byte & 0x80) == 0 {
                break;
            }
        }

        let delta = ((zigzag >> 1) as i32) ^ (-((zigzag & 1) as i32));
        let len = prev_len + delta;
        prev_len = len;
        lengths.push(len as u32);
    }

    Ok(lengths)
}

// =============================================================================
// Checksum
// =============================================================================

pub fn compute_block_checksum(reads: &[ReadRecord]) -> u64 {
    let mut hasher = Xxh64::new(0);
    // Hash the full header line (id + " " + comment) to match the ID stream content.
    // This ensures backward compatibility: old archives stored the full header as id.
    for r in reads {
        hasher.update(r.id.as_bytes());
        if !r.comment.is_empty() {
            hasher.update(b" ");
            hasher.update(r.comment.as_bytes());
        }
    }
    for r in reads {
        hasher.update(r.sequence.as_bytes());
    }
    for r in reads {
        hasher.update(r.quality.as_bytes());
    }
    for r in reads {
        let len = r.sequence.len() as u32;
        hasher.update(&len.to_le_bytes());
    }
    hasher.digest()
}

// =============================================================================
// Varint helpers for reorder map (used by fqc_writer/fqc_reader)
// =============================================================================

pub fn encode_varint(mut value: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            buf.push(byte | 0x80);
        } else {
            buf.push(byte);
            break;
        }
    }
    buf
}

pub fn delta_encode_ids(ids: &[u64]) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut prev = 0i64;
    for &id in ids {
        let delta = id as i64 - prev;
        prev = id as i64;
        let zigzag = ((delta << 1) ^ (delta >> 63)) as u64;
        buf.extend_from_slice(&encode_varint(zigzag));
    }
    buf
}

pub fn delta_decode_ids(data: &[u8], count: u64) -> Result<Vec<u64>> {
    let mut ids = Vec::with_capacity(count as usize);
    let mut i = 0usize;
    let mut prev = 0i64;

    while i < data.len() && ids.len() < count as usize {
        let mut zigzag = 0u64;
        let mut shift = 0u32;
        for _ in 0..10 {
            if i >= data.len() {
                break;
            }
            let byte = data[i];
            i += 1;
            zigzag |= ((byte & 0x7F) as u64) << shift;
            shift += 7;
            if (byte & 0x80) == 0 {
                break;
            }
        }
        let delta = ((zigzag >> 1) as i64) ^ (-((zigzag & 1) as i64));
        prev += delta;
        ids.push(prev as u64);
    }

    Ok(ids)
}
