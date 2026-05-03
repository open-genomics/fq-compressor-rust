// =============================================================================
// Block Compressor - Orchestration Layer
// =============================================================================
//! Block-level compression orchestration for FASTQ reads.
//!
//! This module coordinates compression of reads within a block, delegating to:
//! - [`AbcCompressor`] for short reads (ABC algorithm)
//! - Zstd for medium/long reads
//! - [`QualityCompressor`] for quality scores
//! - [`compress_ids`](crate::algo::id_compressor::compress_ids) for read IDs

use crate::algo::abc::{AbcCompressor, AbcConfig, SHORT_READ_ABC_MAX_READS};
use crate::algo::quality_compressor::{ContextOrder, QualityCompressor, QualityCompressorConfig};
use crate::archive_traits::BlockData;
use crate::error::{FqcError, Result};
use crate::types::*;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Cursor, Read};
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

    /// Create ABC configuration from this config.
    pub fn to_abc_config(&self) -> AbcConfig {
        AbcConfig {
            max_shift: self.max_shift,
            hamming_threshold: self.consensus_hamming_threshold,
            zstd_level: self.zstd_level,
        }
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
            let abc = AbcCompressor::new(self.config.to_abc_config());
            abc.compress(reads)?.data
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

    /// Decompress a block from raw `BlockData`.
    ///
    /// This is a convenience method that unpacks `BlockData` fields internally.
    pub fn decompress_block(&self, block: &BlockData) -> Result<DecompressedBlockData> {
        let bh = &block.header;
        self.decompress_raw(
            bh.block_id,
            bh.uncompressed_count,
            bh.uniform_read_length,
            bh.codec_seq,
            bh.codec_qual,
            &block.ids_data,
            &block.seq_data,
            &block.qual_data,
            &block.aux_data,
        )
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
            let abc = AbcCompressor::new(self.config.to_abc_config());
            abc.decompress(seq_stream, read_count)?
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
