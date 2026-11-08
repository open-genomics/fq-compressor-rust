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
use crate::block::{BlockMeta, DecodedBlock, EncodedBlock};
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
    /// Upper bound for the decoded bytes of one block:
    /// `read_count * size_of::<ReadRecord>() + 2 * sum(read lengths)`.
    /// `u64::MAX` disables the check (compression path). Decompress/verify
    /// set this from `DecodeBudget::max_alloc_bytes`. Both terms come from
    /// the block header/aux stream, so without this cap a forged archive can
    /// drive `vec![ReadRecord; read_count]` and per-read sequence/quality
    /// allocations past `--memory-limit`.
    pub max_block_decoded_bytes: u64,
    /// Hard per-read length ceiling enforced while decoding, in bytes.
    ///
    /// Archive-managed decode paths set this from the read-length class in the
    /// (untrusted) header, so a crafted archive that declares a smaller class
    /// cannot use it to under-estimate the decode budget. `None` (library /
    /// test use) keeps the historical behaviour of decoding whatever the block
    /// declares, as long as `max_block_decoded_bytes` allows it.
    pub max_read_length_bytes: Option<u32>,
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
            max_block_decoded_bytes: u64::MAX,
            max_read_length_bytes: None,
        }
    }
}

/// Codec parameters that do not depend on the detected read-length class.
///
/// The pipeline cannot know the class until it has ingested the whole input,
/// so its configuration carries these settings and resolves the class once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockCodecSettings {
    pub quality_mode: QualityMode,
    pub id_mode: IdMode,
    pub zstd_level: i32,
}

impl Default for BlockCodecSettings {
    fn default() -> Self {
        Self {
            quality_mode: QualityMode::Lossless,
            id_mode: IdMode::Exact,
            zstd_level: 3,
        }
    }
}

impl BlockCodecSettings {
    pub fn from_compression_level(quality_mode: QualityMode, id_mode: IdMode, level: CompressionLevel) -> Self {
        Self {
            quality_mode,
            id_mode,
            zstd_level: BlockCompressorConfig::zstd_level_for_compression_level(level),
        }
    }

    pub fn with_read_length_class(self, read_length_class: ReadLengthClass) -> BlockCompressorConfig {
        BlockCompressorConfig {
            read_length_class,
            quality_mode: self.quality_mode,
            id_mode: self.id_mode,
            zstd_level: self.zstd_level,
            ..Default::default()
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

    /// Conservative peak estimate for one parallel codec worker.
    ///
    /// The quality context model dominates order-2 short-read blocks; the
    /// record and Zstd buffers are included so memory-limited runs reduce
    /// worker count instead of silently exceeding `--memory-limit`.
    pub fn estimated_worker_bytes(&self, reads: usize, avg_length: usize) -> usize {
        let model = self.to_quality_config().estimated_model_bytes();
        usize::try_from(crate::plan::block_plan::estimated_worker_bytes(
            model, reads, avg_length,
        ))
        .unwrap_or(usize::MAX)
    }
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

    pub fn compress(&mut self, reads: &[ReadRecord], block_id: BlockId) -> Result<EncodedBlock> {
        let mut result = EncodedBlock {
            meta: BlockMeta {
                block_id,
                read_count: reads.len() as u32,
                ..Default::default()
            },
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
            result.meta.codec_seq = encode_codec(CodecFamily::AbcV1, 1);
        } else {
            // Use Zstd for large blocks (even if read_length_class is Short)
            // or for medium/long reads
            let zstd = ZstdSequenceCompressor::new(self.config.zstd_level);
            result.seq_stream = zstd.compress(reads)?;
            result.meta.codec_seq = zstd.codec_id();
        }

        // Compress quality
        result.qual_stream = self.quality.compress(reads)?;
        result.meta.codec_qual = self.quality.codec_id();

        // Compress IDs
        result.id_stream = self.id.compress(reads)?;
        result.meta.codec_ids = self.id.codec_id();

        // Compress aux (lengths)
        let (aux_data, uniform_len) = self.aux.compress(reads)?;
        result.aux_stream = aux_data;
        result.meta.uniform_read_length = uniform_len;
        result.meta.codec_aux = self.aux.codec_id();

        // Preserve the historical v2 logical checksum only when every field
        // round-trips exactly. Lossy/discard blocks use the archive-wide
        // compressed-stream checksum and leave this optional field at zero.
        if self.config.quality_mode == QualityMode::Lossless && self.config.id_mode != IdMode::Discard {
            result.meta.block_checksum = compute_block_checksum(reads);
        }

        Ok(result)
    }

    /// Decompress a neutral encoded block.
    pub fn decompress_block(&mut self, block: &EncodedBlock) -> Result<DecodedBlock> {
        let meta = &block.meta;
        let decompressed = self.decompress_raw(
            meta.block_id,
            meta.read_count,
            meta.uniform_read_length,
            meta.codec_ids,
            meta.codec_seq,
            meta.codec_qual,
            meta.codec_aux,
            &block.id_stream,
            &block.seq_stream,
            &block.qual_stream,
            &block.aux_stream,
        )?;
        self.verify_block_checksum(meta.block_checksum, &decompressed.reads)?;
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
    ) -> Result<DecodedBlock> {
        // read_count comes from the block header. It is cross-checked against
        // the block index, but not against the memory budget, so a forged
        // archive can drive `vec![ReadRecord; read_count]` (96 B/read) before
        // any per-block check. Reject oversized skeletons first.
        let skeleton_bytes = u64::from(read_count).saturating_mul(std::mem::size_of::<ReadRecord>() as u64);
        if skeleton_bytes > self.config.max_block_decoded_bytes {
            return Err(FqcError::ResourceLimit {
                location: format!("block {block_id} read_record_skeleton"),
                declared: skeleton_bytes,
                allowed: self.config.max_block_decoded_bytes,
            });
        }
        let mut result = DecodedBlock {
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

        // 单条读长的硬上限：归档的读长分类来自头部（不可信），而解压预算
        // （worker 数、--original-order 峰值）按分类上限推导。这里拒掉超过
        // 分类上限的读长，保证那些估算永远是上界而不是猜测。
        if let Some(max_read_len) = self.config.max_read_length_bytes {
            if let Some(&bad) = lengths.iter().find(|&&l| l > max_read_len) {
                return Err(FqcError::Format(format!(
                    "block {block_id} read length {bad} exceeds the {} class limit {max_read_len}",
                    self.config.read_length_class.as_str()
                )));
            }
        }

        // Lengths are untrusted (header uniform value or aux stream). Each
        // read allocates about `len` bytes for sequence and `len` for
        // quality, so bound their sum by the same per-block decode budget
        // before any decoder sees them.
        let total_bases: u64 = lengths.iter().map(|&l| u64::from(l)).fold(0u64, u64::saturating_add);
        let decoded_bytes = skeleton_bytes.saturating_add(total_bases.saturating_mul(2));
        if decoded_bytes > self.config.max_block_decoded_bytes {
            return Err(FqcError::ResourceLimit {
                location: format!("block {block_id} decoded_bytes"),
                declared: decoded_bytes,
                allowed: self.config.max_block_decoded_bytes,
            });
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
        // Raw 质量流表示 Discard 模式：解压只产出占位质量串，不需要
        // order-2 上下文模型（约 13 MiB）。避免为每个 discard block 白建一次模型。
        if family == CodecFamily::Raw {
            if !data.is_empty() {
                return Err(FqcError::Format(
                    "Discarded quality stream must not contain payload bytes".to_string(),
                ));
            }
            let length_vec: Vec<u32> = if uniform_read_length > 0 {
                vec![uniform_read_length; read_count as usize]
            } else {
                lengths.to_vec()
            };
            return Ok(length_vec.iter().map(|&len| "!".repeat(len as usize)).collect());
        }

        let mut qcfg = self.config.to_quality_config();
        match family {
            CodecFamily::Raw => unreachable!("Raw quality stream handled above"),
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
