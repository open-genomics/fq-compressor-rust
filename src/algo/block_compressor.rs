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
use crate::archive_traits::BlockData;
use crate::error::Result;
use crate::types::*;
use xxhash_rust::xxh64::Xxh64;

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
    pub block_id: BlockId,
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
    sequence: Box<dyn SequenceCompressor>,
    quality: Box<dyn QualityCompressor>,
    id: Box<dyn IdCompressor>,
    aux: Box<dyn AuxCompressor>,
    config: BlockCompressorConfig,
}

impl BlockCompressor {
    /// Create a compressor for short reads (uses ABC algorithm).
    pub fn for_short_reads(config: BlockCompressorConfig) -> Self {
        Self {
            sequence: Box::new(AbcCompressor::new(config.to_abc_config())),
            quality: Box::new(ScmQualityCompressor::new(config.to_quality_config())),
            id: Box::new(DeltaZstdIdCompressor::new(
                config.zstd_level,
                config.id_mode == IdMode::Discard,
                config.id_prefix.clone(),
            )),
            aux: Box::new(DeltaVarintAuxCompressor::new(config.zstd_level)),
            config,
        }
    }

    /// Create a compressor for medium/long reads (uses Zstd).
    pub fn for_long_reads(config: BlockCompressorConfig) -> Self {
        Self {
            sequence: Box::new(ZstdSequenceCompressor::new(config.zstd_level)),
            quality: Box::new(ScmQualityCompressor::new(config.to_quality_config())),
            id: Box::new(DeltaZstdIdCompressor::new(
                config.zstd_level,
                config.id_mode == IdMode::Discard,
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

    pub fn config(&self) -> &BlockCompressorConfig {
        &self.config
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
        if self.config.use_short_read_abc(reads.len()) {
            // Use ABC for small blocks of short reads
            let abc = AbcCompressor::new(self.config.to_abc_config());
            result.seq_stream = abc.compress(reads)?.data;
            result.codec_seq = encode_codec(CodecFamily::AbcV1, 0);
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

        // Compute block checksum
        result.block_checksum = compute_block_checksum(reads);

        Ok(result)
    }

    /// Decompress a block from raw `BlockData`.
    pub fn decompress_block(&mut self, block: &BlockData) -> Result<DecompressedBlockData> {
        let bh = &block.header;
        self.decompress_raw(
            bh.block_id,
            bh.uncompressed_count,
            bh.uniform_read_length,
            bh.codec_seq,
            &block.ids_data,
            &block.seq_data,
            &block.qual_data,
            &block.aux_data,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn decompress_raw(
        &mut self,
        block_id: BlockId,
        read_count: u32,
        uniform_read_length: u32,
        codec_seq: u8,
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
        let lengths = self.aux.decompress(aux_stream, read_count)?;

        // Decompress sequences - select based on codec
        let sequences = if decode_codec_family(codec_seq) == CodecFamily::AbcV1 {
            // Use ABC decompressor for ABC-encoded data
            let abc = AbcCompressor::new(self.config.to_abc_config());
            abc.decompress(seq_stream, read_count)?
        } else {
            // Use Zstd decompressor for Zstd-encoded data
            let zstd = ZstdSequenceCompressor::new(self.config.zstd_level);
            zstd.decompress(seq_stream, read_count, uniform_read_length, &lengths)?
        };

        // Decompress quality
        let qualities = self
            .quality
            .decompress(qual_stream, read_count, uniform_read_length, &lengths)?;

        // Decompress IDs
        let ids = self.id.decompress(id_stream, read_count)?;

        // Assemble reads
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
// Checksum
// =============================================================================

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
