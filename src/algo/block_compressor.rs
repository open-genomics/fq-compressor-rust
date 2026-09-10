// =============================================================================
// Block Compressor - Orchestration Layer
// =============================================================================
//! Block-level compression orchestration for FASTQ reads.
//!
//! This module coordinates compression of reads within a block, delegating to
//! trait implementations for each stream type:
//! - [`SequenceCompressor`] for DNA sequences
//! - [`QualityCompressor`] for quality scores
//! - [`IdCompressor`] for read IDs
//! - [`AuxCompressor`] for auxiliary data (lengths)

use crate::algo::abc::{AbcCompressor, AbcConfig, SHORT_READ_ABC_MAX_READS};
use crate::algo::aux_compressor::DeltaVarintAuxCompressor;
use crate::algo::compressor_traits::{AuxCompressor, IdCompressor, QualityCompressor, SequenceCompressor};
use crate::algo::id_compressor_impl::DeltaZstdIdCompressor;
use crate::algo::quality_compressor::{
    ContextOrder, QualityCompressor as ScmQualityCompressor, QualityCompressorConfig,
};
use crate::algo::zstd_sequence::ZstdSequenceCompressor;
use crate::archive::traits::BlockData;
use crate::error::{FqcError, Result};
use crate::types::*;
use xxhash_rust::xxh64::Xxh64;

fn unsupported_stream_codec(block_id: BlockId, stream: &str, codec: u8, reason: &str) -> FqcError {
    FqcError::UnsupportedFormat(format!("block {block_id} {stream} codec 0x{codec:02x}: {reason}"))
}

/// Parse a stream codec byte: known family, allowed for this stream, version 0 only.
fn parse_stream_codec(
    block_id: BlockId,
    stream: &str,
    codec: u8,
    allowed: &[CodecFamily],
) -> Result<(CodecFamily, u8)> {
    let family_nibble = codec >> 4;
    let version = decode_codec_version(codec);
    let family = CodecFamily::try_from_nibble(family_nibble)
        .ok_or_else(|| unsupported_stream_codec(block_id, stream, codec, "unknown or reserved codec family"))?;
    if !allowed.contains(&family) {
        return Err(unsupported_stream_codec(
            block_id,
            stream,
            codec,
            &format!(
                "family {} is not valid for this stream (allowed: {})",
                family.as_str(),
                allowed.iter().map(|f| f.as_str()).collect::<Vec<_>>().join(", ")
            ),
        ));
    }
    Ok((family, version))
}

// =============================================================================
// BlockCompressorConfig
// =============================================================================

#[derive(Debug, Clone)]
pub struct BlockCompressorConfig {
    pub read_length_class: ReadLengthClass,
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

    /// Create ABC configuration from this config.
    pub fn to_abc_config(&self) -> AbcConfig {
        AbcConfig {
            max_shift: self.max_shift,
            hamming_threshold: self.consensus_hamming_threshold,
            zstd_level: self.zstd_level,
        }
    }

    /// Create quality compressor configuration.
    pub fn to_quality_config(&self) -> QualityCompressorConfig {
        QualityCompressorConfig {
            quality_mode: self.quality_mode,
            context_order: if self.read_length_class == ReadLengthClass::Long {
                ContextOrder::Order1
            } else {
                ContextOrder::Order2
            },
            num_position_bins: 8,
        }
    }

    pub fn use_short_read_abc(&self, read_count: usize) -> bool {
        self.read_length_class == ReadLengthClass::Short && read_count <= SHORT_READ_ABC_MAX_READS
    }
}

// =============================================================================
// CompressedBlockData
// =============================================================================

#[derive(Debug, Default, Clone)]
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
    pub reads: Vec<ReadRecord>,
}

// =============================================================================
// BlockCompressor
// =============================================================================

/// Block compressor that delegates to trait implementations.
///
/// Use factory methods to create with appropriate compressors:
/// - [`BlockCompressor::for_short_reads`] — uses ABC for sequences
/// - [`BlockCompressor::for_long_reads`] — uses Zstd for sequences
pub struct BlockCompressor {
    quality: Box<dyn QualityCompressor>,
    id: Box<dyn IdCompressor>,
    aux: Box<dyn AuxCompressor>,
    config: BlockCompressorConfig,
}

impl BlockCompressor {
    /// Create a compressor for short reads (uses ABC algorithm).
    pub fn for_short_reads(config: BlockCompressorConfig) -> Self {
        Self {
            quality: Box::new(ScmQualityCompressor::new(config.to_quality_config())),
            id: Box::new(DeltaZstdIdCompressor::new(
                config.zstd_level,
                config.id_mode,
                config.id_prefix.clone(),
            )),
            aux: Box::new(DeltaVarintAuxCompressor::new(config.zstd_level)),
            config,
        }
    }

    /// Create a compressor for medium/long reads (uses Zstd).
    pub fn for_long_reads(config: BlockCompressorConfig) -> Self {
        Self {
            quality: Box::new(ScmQualityCompressor::new(config.to_quality_config())),
            id: Box::new(DeltaZstdIdCompressor::new(
                config.zstd_level,
                config.id_mode,
                config.id_prefix.clone(),
            )),
            aux: Box::new(DeltaVarintAuxCompressor::new(config.zstd_level)),
            config,
        }
    }

    /// Create a compressor based on read length class.
    pub fn new(config: BlockCompressorConfig) -> Self {
        match config.read_length_class {
            ReadLengthClass::Short => Self::for_short_reads(config),
            ReadLengthClass::Medium | ReadLengthClass::Long => Self::for_long_reads(config),
        }
    }

    pub fn compress(&mut self, reads: &[ReadRecord], block_id: BlockId) -> Result<CompressedBlockData> {
        let mut result = CompressedBlockData {
            block_id,
            read_count: reads.len() as u32,
            ..Default::default()
        };

        if reads.is_empty() {
            return Ok(result);
        }

        // For short reads, check if we should fall back to Zstd due to block size
        // ABC is O(n²) in block size, so large blocks use Zstd instead
        if self.config.use_short_read_abc(reads.len())
            && reads.iter().all(|read| read.sequence.len() <= SPRING_MAX_READ_LENGTH)
        {
            // Use ABC for small blocks of short reads
            let abc = AbcCompressor::new(self.config.to_abc_config());
            result.seq_stream = abc.compress(reads)?.data;
            result.codec_seq = encode_codec(CodecFamily::AbcV1, 1);
        } else {
            // Use Zstd for large blocks (even if read_length_class is Short)
            // or for medium/long reads
            let zstd = ZstdSequenceCompressor::new(self.config.zstd_level);
            result.seq_stream = zstd.compress(reads)?;
            result.codec_seq = zstd.codec_id();
        }

        // Compress quality
        result.qual_stream = self.quality.compress(reads)?;
        result.codec_qual = self.quality.codec_id();

        // Compress IDs
        result.id_stream = self.id.compress(reads)?;
        result.codec_ids = self.id.codec_id();

        // Compress aux (lengths)
        let (aux_data, uniform_len) = self.aux.compress(reads)?;
        result.aux_stream = aux_data;
        result.uniform_read_length = uniform_len;
        result.codec_aux = self.aux.codec_id();

        // Preserve the historical v2 logical checksum only when every field
        // round-trips exactly. Lossy/discard blocks use the archive-wide
        // compressed-stream checksum and leave this optional field at zero.
        if self.config.quality_mode == QualityMode::Lossless && self.config.id_mode != IdMode::Discard {
            result.block_checksum = compute_block_checksum(reads);
        }

        Ok(result)
    }

    /// Decompress a block from raw `BlockData`.
    pub fn decompress_block(&mut self, block: &BlockData) -> Result<DecompressedBlockData> {
        let bh = &block.header;
        let decompressed = self.decompress_raw(
            bh.block_id,
            bh.uncompressed_count,
            bh.uniform_read_length,
            bh.codec_ids,
            bh.codec_seq,
            bh.codec_qual,
            bh.codec_aux,
            &block.ids_data,
            &block.seq_data,
            &block.qual_data,
            &block.aux_data,
        )?;
        self.verify_block_checksum(bh.block_xxhash64, &decompressed.reads)?;
        Ok(decompressed)
    }

    /// Decompress streams using the four per-stream codec IDs from the block header.
    #[allow(clippy::too_many_arguments)]
    pub fn decompress_raw(
        &mut self,
        block_id: BlockId,
        read_count: u32,
        uniform_read_length: u32,
        codec_ids: u8,
        codec_seq: u8,
        codec_qual: u8,
        codec_aux: u8,
        id_stream: &[u8],
        seq_stream: &[u8],
        qual_stream: &[u8],
        aux_stream: &[u8],
    ) -> Result<DecompressedBlockData> {
        let mut result = DecompressedBlockData {
            reads: vec![ReadRecord::default(); read_count as usize],
        };

        if read_count == 0 {
            if !id_stream.is_empty() || !seq_stream.is_empty() || !qual_stream.is_empty() || !aux_stream.is_empty() {
                return Err(FqcError::Format(
                    "empty block must not contain compressed stream data".to_string(),
                ));
            }
            if uniform_read_length != 0 {
                return Err(FqcError::Format(
                    "empty block must not declare a uniform read length".to_string(),
                ));
            }
            return Ok(result);
        }

        let (id_family, id_version) =
            parse_stream_codec(block_id, "ids", codec_ids, &[CodecFamily::Raw, CodecFamily::DeltaZstd])?;
        let (seq_family, seq_version) = parse_stream_codec(
            block_id,
            "seq",
            codec_seq,
            &[CodecFamily::AbcV1, CodecFamily::ZstdPlain],
        )?;
        let (qual_family, qual_version) = parse_stream_codec(
            block_id,
            "qual",
            codec_qual,
            &[CodecFamily::Raw, CodecFamily::ScmV1, CodecFamily::ScmOrder1],
        )?;
        let (_aux_family, aux_version) = parse_stream_codec(block_id, "aux", codec_aux, &[CodecFamily::DeltaVarint])?;

        if id_version != 0 || qual_version != 0 || aux_version != 0 {
            return Err(unsupported_stream_codec(
                block_id,
                "ids/qual/aux",
                if id_version != 0 {
                    codec_ids
                } else if qual_version != 0 {
                    codec_qual
                } else {
                    codec_aux
                },
                "only codec revision 0 is implemented for this stream",
            ));
        }
        match seq_family {
            CodecFamily::AbcV1 if seq_version <= 1 => {}
            CodecFamily::ZstdPlain if seq_version == 0 => {}
            _ => {
                return Err(unsupported_stream_codec(
                    block_id,
                    "seq",
                    codec_seq,
                    "unsupported codec version/revision",
                ));
            }
        }

        self.check_flag_codec_consistency(block_id, id_family, qual_family, codec_ids, codec_qual)?;

        // Aux (lengths) — always DeltaVarint v0 after validation above.
        // An empty aux stream with uniform_read_length > 0 means all reads
        // share that length; materialize the lengths vector here so quality
        // and sequence decoders receive per-read lengths.
        let mut lengths = self.aux.decompress(aux_stream, read_count)?;
        if !aux_stream.is_empty() && uniform_read_length != 0 {
            return Err(FqcError::Format(format!(
                "block {block_id} declares uniform length alongside an auxiliary length stream"
            )));
        }
        if lengths.is_empty() && aux_stream.is_empty() {
            lengths = vec![uniform_read_length; read_count as usize];
        }
        if lengths.len() != read_count as usize {
            return Err(FqcError::Format(format!(
                "block {block_id} aux stream decoded {} lengths for {read_count} reads",
                lengths.len()
            )));
        }

        let sequences = match seq_family {
            CodecFamily::AbcV1 => {
                let abc = AbcCompressor::new(self.config.to_abc_config());
                abc.decompress_with_codec_revision(seq_stream, read_count, seq_version)?
            }
            CodecFamily::ZstdPlain => {
                let zstd = ZstdSequenceCompressor::new(self.config.zstd_level);
                zstd.decompress(seq_stream, read_count, uniform_read_length, &lengths)?
            }
            other => {
                return Err(unsupported_stream_codec(
                    block_id,
                    "seq",
                    codec_seq,
                    &format!("internal: unexpected family {}", other.as_str()),
                ));
            }
        };

        let qualities = self.decompress_quality_stream(
            block_id,
            qual_family,
            codec_qual,
            qual_stream,
            read_count,
            uniform_read_length,
            &lengths,
        )?;

        let ids = self.decompress_id_stream(block_id, id_family, codec_ids, id_stream, read_count)?;

        if sequences.len() != read_count as usize
            || qualities.len() != read_count as usize
            || ids.len() != read_count as usize
        {
            return Err(FqcError::Format(format!(
                "block {block_id} stream counts do not match declared read count {read_count}"
            )));
        }

        for i in 0..read_count as usize {
            let full_header = ids[i].clone();
            if let Some(space_pos) = full_header.find(' ') {
                result.reads[i].id = full_header[..space_pos].to_string();
                result.reads[i].comment = full_header[space_pos + 1..].to_string();
            } else {
                result.reads[i].id = full_header;
            }
            result.reads[i].sequence.clone_from(&sequences[i]);
            result.reads[i].quality.clone_from(&qualities[i]);
            if result.reads[i].sequence.len() != lengths[i] as usize
                || result.reads[i].quality.len() != lengths[i] as usize
            {
                return Err(FqcError::Format(format!(
                    "block {block_id} read {i} lengths disagree with aux metadata"
                )));
            }
        }

        Ok(result)
    }

    fn check_flag_codec_consistency(
        &self,
        block_id: BlockId,
        id_family: CodecFamily,
        qual_family: CodecFamily,
        codec_ids: u8,
        codec_qual: u8,
    ) -> Result<()> {
        let id_discard = id_family == CodecFamily::Raw;
        let flags_id_discard = self.config.id_mode == IdMode::Discard;
        if id_discard != flags_id_discard {
            return Err(FqcError::Format(format!(
                "block {block_id} ids codec 0x{codec_ids:02x} ({}) contradicts global id_mode {:?}",
                id_family.as_str(),
                self.config.id_mode
            )));
        }

        let qual_discard = qual_family == CodecFamily::Raw;
        let flags_qual_discard = self.config.quality_mode == QualityMode::Discard;
        if qual_discard != flags_qual_discard {
            return Err(FqcError::Format(format!(
                "block {block_id} qual codec 0x{codec_qual:02x} ({}) contradicts global quality_mode {:?}",
                qual_family.as_str(),
                self.config.quality_mode
            )));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn decompress_quality_stream(
        &self,
        block_id: BlockId,
        family: CodecFamily,
        codec: u8,
        data: &[u8],
        read_count: u32,
        uniform_read_length: u32,
        lengths: &[u32],
    ) -> Result<Vec<String>> {
        let mut qcfg = self.config.to_quality_config();
        match family {
            CodecFamily::Raw => {
                qcfg.quality_mode = QualityMode::Discard;
            }
            CodecFamily::ScmV1 => {
                qcfg.context_order = ContextOrder::Order2;
                if qcfg.quality_mode == QualityMode::Discard {
                    qcfg.quality_mode = QualityMode::Lossless;
                }
            }
            CodecFamily::ScmOrder1 => {
                qcfg.context_order = ContextOrder::Order1;
                if qcfg.quality_mode == QualityMode::Discard {
                    qcfg.quality_mode = QualityMode::Lossless;
                }
            }
            other => {
                return Err(unsupported_stream_codec(
                    block_id,
                    "qual",
                    codec,
                    &format!("internal: unexpected family {}", other.as_str()),
                ));
            }
        }
        let mut quality: Box<dyn QualityCompressor> = Box::new(ScmQualityCompressor::new(qcfg));
        quality.decompress(data, read_count, uniform_read_length, lengths)
    }

    fn decompress_id_stream(
        &self,
        block_id: BlockId,
        family: CodecFamily,
        codec: u8,
        data: &[u8],
        read_count: u32,
    ) -> Result<Vec<String>> {
        let id_mode = match family {
            CodecFamily::Raw => IdMode::Discard,
            CodecFamily::DeltaZstd => match self.config.id_mode {
                IdMode::Discard => IdMode::Exact,
                other => other,
            },
            other => {
                return Err(unsupported_stream_codec(
                    block_id,
                    "ids",
                    codec,
                    &format!("internal: unexpected family {}", other.as_str()),
                ));
            }
        };
        let id = DeltaZstdIdCompressor::new(self.config.zstd_level, id_mode, self.config.id_prefix.clone());
        id.decompress(data, read_count)
    }

    fn verify_block_checksum(&self, expected: u64, reads: &[ReadRecord]) -> Result<()> {
        if expected == 0 {
            return Ok(());
        }
        if self.config.quality_mode != QualityMode::Lossless || self.config.id_mode == IdMode::Discard {
            return Err(FqcError::Format(
                "lossy or discard block must not carry a logical block checksum".to_string(),
            ));
        }
        let actual = compute_block_checksum(reads);
        if actual != expected {
            return Err(FqcError::ChecksumMismatch { expected, actual });
        }
        Ok(())
    }
}

// =============================================================================
// Checksum
// =============================================================================

/// Historical v2 block checksum over a fully lossless logical record stream.
pub fn compute_block_checksum(reads: &[ReadRecord]) -> u64 {
    let mut hasher = Xxh64::new(0);
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
        hasher.update(&(r.sequence.len() as u32).to_le_bytes());
    }
    hasher.digest()
}

// =============================================================================
// Varint helpers for reorder map (used by archive::writer/archive::reader)
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
        // Perform the zig-zag transform in unsigned space. Shifting a
        // negative i64 directly can overflow in debug builds; the cast keeps
        // the two's-complement bit pattern while the u64 shift is defined.
        let zigzag = ((delta as u64) << 1) ^ ((delta >> 63) as u64);
        buf.extend_from_slice(&encode_varint(zigzag));
    }
    buf
}

pub fn delta_decode_ids(data: &[u8], count: u64) -> Result<Vec<u64>> {
    let expected = usize::try_from(count)
        .map_err(|_| FqcError::Format("delta_decode_ids count exceeds platform limits".to_string()))?;
    let mut ids = Vec::with_capacity(expected);
    let mut i = 0usize;
    let mut prev = 0i64;

    for id_index in 0..expected {
        let mut zigzag = 0u64;
        let mut terminated = false;
        for byte_index in 0..10 {
            let byte = *data
                .get(i)
                .ok_or_else(|| FqcError::Format(format!("delta_decode_ids: truncated varint at id {id_index}")))?;
            i += 1;
            let payload = u64::from(byte & 0x7F);
            if byte_index == 9 && payload > 1 {
                return Err(FqcError::Format(format!(
                    "delta_decode_ids: varint overflows u64 at id {id_index}"
                )));
            }
            zigzag |= payload << (byte_index * 7);
            if (byte & 0x80) == 0 {
                terminated = true;
                break;
            }
        }
        if !terminated {
            return Err(FqcError::Format(format!(
                "delta_decode_ids: unterminated varint at id {id_index}"
            )));
        }
        let delta = ((zigzag >> 1) as i64) ^ (-((zigzag & 1) as i64));
        prev = prev
            .checked_add(delta)
            .ok_or_else(|| FqcError::Format(format!("delta_decode_ids: delta overflows at id {id_index}")))?;
        if prev < 0 {
            return Err(FqcError::Format(format!(
                "delta_decode_ids: decoded negative id at position {id_index}"
            )));
        }
        ids.push(prev as u64);
    }

    if i != data.len() {
        return Err(FqcError::Format(format!(
            "delta_decode_ids: trailing bytes after {} ids",
            count,
        )));
    }

    Ok(ids)
}
