// =============================================================================
// fqc-rust - Core Type Definitions
// =============================================================================

/// Block identifier type
pub type BlockId = u32;

/// Read identifier type (1-based indexing)
pub type ReadId = u64;

/// Compression level (1-9)
pub type CompressionLevel = u8;

/// File offset type
pub type FileOffset = u64;

/// Checksum value type
pub type Checksum = u64;

/// Invalid block ID sentinel
pub const INVALID_BLOCK_ID: BlockId = u32::MAX;

/// Invalid read ID sentinel
pub const INVALID_READ_ID: ReadId = u64::MAX;

/// Default compression level
pub const DEFAULT_COMPRESSION_LEVEL: CompressionLevel = 5;

/// Minimum compression level
pub const MIN_COMPRESSION_LEVEL: CompressionLevel = 1;

/// Maximum compression level
pub const MAX_COMPRESSION_LEVEL: CompressionLevel = 9;

/// Default block size for short reads (reads per block)
pub const DEFAULT_BLOCK_SIZE_SHORT: usize = 100_000;

/// Default block size for medium reads
pub const DEFAULT_BLOCK_SIZE_MEDIUM: usize = 50_000;

/// Default block size for long reads
pub const DEFAULT_BLOCK_SIZE_LONG: usize = 10_000;

/// Spring ABC max read length
pub const SPRING_MAX_READ_LENGTH: usize = 511;

/// Medium read threshold (bytes)
pub const MEDIUM_READ_THRESHOLD: usize = 1_024;

/// Long read threshold (bytes)
pub const LONG_READ_THRESHOLD: usize = 10_240;

/// Ultra-long read threshold (bytes)
pub const ULTRA_LONG_READ_THRESHOLD: usize = 102_400;

/// Default placeholder quality character for discard mode
pub const DEFAULT_PLACEHOLDER_QUAL: char = '!';

// =============================================================================
// Quality Mode Enumeration
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum QualityMode {
    #[default]
    Lossless = 0,
    Illumina8 = 1,
    Qvz = 2,
    Discard = 3,
}

impl QualityMode {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Lossless,
            1 => Self::Illumina8,
            2 => Self::Qvz,
            3 => Self::Discard,
            _ => Self::Lossless,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Lossless => "lossless",
            Self::Illumina8 => "illumina8",
            Self::Qvz => "qvz",
            Self::Discard => "discard",
        }
    }
}

// =============================================================================
// ID Mode Enumeration
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum IdMode {
    #[default]
    Exact = 0,
    Tokenize = 1,
    Discard = 2,
}

impl IdMode {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Exact,
            1 => Self::Tokenize,
            2 => Self::Discard,
            _ => Self::Exact,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Tokenize => "tokenize",
            Self::Discard => "discard",
        }
    }
}

// =============================================================================
// Read Length Class Enumeration
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ReadLengthClass {
    #[default]
    Short = 0,
    Medium = 1,
    Long = 2,
}

impl ReadLengthClass {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Short,
            1 => Self::Medium,
            2 => Self::Long,
            _ => Self::Short,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Short => "short",
            Self::Medium => "medium",
            Self::Long => "long",
        }
    }
}

/// Classify read length based on statistics
pub fn classify_read_length(median_length: usize, max_length: usize) -> ReadLengthClass {
    if max_length >= ULTRA_LONG_READ_THRESHOLD {
        return ReadLengthClass::Long;
    }
    if max_length >= LONG_READ_THRESHOLD {
        return ReadLengthClass::Long;
    }
    if max_length > SPRING_MAX_READ_LENGTH {
        return ReadLengthClass::Medium;
    }
    if median_length >= MEDIUM_READ_THRESHOLD {
        return ReadLengthClass::Medium;
    }
    ReadLengthClass::Short
}

/// Get recommended block size for a read length class
pub fn recommended_block_size(class: ReadLengthClass) -> usize {
    match class {
        ReadLengthClass::Short => DEFAULT_BLOCK_SIZE_SHORT,
        ReadLengthClass::Medium => DEFAULT_BLOCK_SIZE_MEDIUM,
        ReadLengthClass::Long => DEFAULT_BLOCK_SIZE_LONG,
    }
}

// =============================================================================
// PE Layout Enumeration
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PeLayout {
    #[default]
    Interleaved = 0,
    Consecutive = 1,
}

impl PeLayout {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Interleaved,
            1 => Self::Consecutive,
            _ => Self::Interleaved,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Interleaved => "interleaved",
            Self::Consecutive => "consecutive",
        }
    }
}

// =============================================================================
// Checksum Type Enumeration
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum ChecksumType {
    #[default]
    XxHash64 = 0,
}

// =============================================================================
// Codec Family Enumeration
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CodecFamily {
    Raw = 0x0,
    AbcV1 = 0x1,
    ScmV1 = 0x2,
    DeltaLzma = 0x3,
    DeltaZstd = 0x4,
    DeltaVarint = 0x5,
    OverlapV1 = 0x6,
    ZstdPlain = 0x7,
    ScmOrder1 = 0x8,
    External = 0xE,
    Reserved = 0xF,
}

impl CodecFamily {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0x0 => Self::Raw,
            0x1 => Self::AbcV1,
            0x2 => Self::ScmV1,
            0x3 => Self::DeltaLzma,
            0x4 => Self::DeltaZstd,
            0x5 => Self::DeltaVarint,
            0x6 => Self::OverlapV1,
            0x7 => Self::ZstdPlain,
            0x8 => Self::ScmOrder1,
            0xE => Self::External,
            _ => Self::Reserved,
        }
    }
}

/// Encode codec as (family:4bit, version:4bit)
pub fn encode_codec(family: CodecFamily, version: u8) -> u8 {
    ((family as u8) << 4) | (version & 0x0F)
}

/// Decode codec family from codec byte
pub fn decode_codec_family(codec: u8) -> CodecFamily {
    CodecFamily::from_u8(codec >> 4)
}

// =============================================================================
// ReadRecord Structure
// =============================================================================

/// A single FASTQ read record
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReadRecord {
    pub id: String,
    /// Optional comment after ID (space-separated on the FASTQ header line)
    pub comment: String,
    pub sequence: String,
    pub quality: String,
}

impl ReadRecord {
    pub fn new(id: String, sequence: String, quality: String) -> Self {
        Self { id, comment: String::new(), sequence, quality }
    }

    /// Construct with all fields including comment
    pub fn with_comment(id: String, comment: String, sequence: String, quality: String) -> Self {
        Self { id, comment, sequence, quality }
    }

    pub fn is_valid(&self) -> bool {
        !self.sequence.is_empty() && self.sequence.len() == self.quality.len()
    }

    pub fn len(&self) -> usize {
        self.sequence.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sequence.is_empty()
    }
}

// =============================================================================
// Compression Options
// =============================================================================

#[derive(Debug, Clone)]
pub struct CompressOptions {
    pub level: CompressionLevel,
    pub quality_mode: QualityMode,
    pub id_mode: IdMode,
    pub enable_reorder: bool,
    pub save_reorder_map: bool,
    pub streaming_mode: bool,
    pub block_size: usize,
    pub memory_limit_mb: usize,
    pub threads: usize,
    pub pe_layout: PeLayout,
    pub read_length_class: Option<ReadLengthClass>,
}

impl Default for CompressOptions {
    fn default() -> Self {
        Self {
            level: DEFAULT_COMPRESSION_LEVEL,
            quality_mode: QualityMode::Lossless,
            id_mode: IdMode::Exact,
            enable_reorder: true,
            save_reorder_map: true,
            streaming_mode: false,
            block_size: DEFAULT_BLOCK_SIZE_SHORT,
            memory_limit_mb: 8192,
            threads: 0,
            pe_layout: PeLayout::Interleaved,
            read_length_class: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DecompressOptions {
    pub range_start: ReadId,
    pub range_end: ReadId,
    pub original_order: bool,
    pub header_only: bool,
    pub verify: bool,
    pub skip_corrupted: bool,
    pub placeholder_qual: char,
    /// ID prefix for discard mode reconstruction (e.g., "read" → @read1, @read2, ...)
    pub id_prefix: String,
    pub threads: usize,
}

impl Default for DecompressOptions {
    fn default() -> Self {
        Self {
            range_start: 1,
            range_end: 0,
            original_order: false,
            header_only: false,
            verify: true,
            skip_corrupted: false,
            placeholder_qual: DEFAULT_PLACEHOLDER_QUAL,
            id_prefix: String::from("read"),
            threads: 0,
        }
    }
}
