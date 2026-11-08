// =============================================================================
// fqc-rust - Core Type Definitions
// =============================================================================

use std::collections::BTreeMap;

/// Block identifier type
pub type BlockId = u32;

/// Read identifier type (1-based indexing)
pub type ReadId = u64;

/// Compression level (1-9)
pub type CompressionLevel = u8;

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

/// Default maximum bases per auto-split long-read block.
/// Long reads are split in archive/pipeline modes; streaming keeps its
/// historical block sizing unless the user passes an explicit base limit.
pub const DEFAULT_LONG_READ_BLOCK_BASES: usize = 5_000_000;

/// Automatic per-block base ceiling used when `--max-block-bases` is not set.
/// Blocks are also sized by the number of reads; this cap keeps a block from
/// becoming a single multi-hundred-megabase unit on long-read data.
pub const AUTO_MAX_BLOCK_BASES: usize = 8 * 1024 * 1024;

/// Minimum read count per block used by adaptive sizing. These values keep
/// per-block model setup and Zstd frame overhead amortized while still
/// producing enough blocks to feed the worker pool.
pub const MIN_BLOCK_READS_SHORT: usize = 10_000;
pub const MIN_BLOCK_READS_MEDIUM: usize = 2_000;
pub const MIN_BLOCK_READS_LONG: usize = 500;

/// Default cap for auto-selected codec workers.
///
/// The order-2 quality model still carries a non-trivial per-worker table
/// (~14 MB after the flat Fenwick model), so using every core by default
/// on a large machine can be a poor memory/throughput trade. An
/// explicit `--threads` or `--memory-limit` opts out of this guard.
pub const DEFAULT_AUTO_CODEC_WORKERS: usize = 4;

/// Hard per-read length ceiling enforced while decoding (`Long` class).
///
/// The archive's read-length class lives in the header and is therefore
/// untrusted, while decode budgets (worker counts, `--original-order` peak
/// estimates) are derived from it. Decode rejects any read longer than the
/// class ceiling, which makes every class-derived estimate a true upper bound
/// instead of a heuristic. 4 MiB sits above real sequencing reads (the longest
/// ONT reads are ~2.5 Mbp), so it bounds hostile input without rejecting data
/// that a sequencer can actually produce.
pub const MAX_LONG_READ_LENGTH: usize = 4 * 1024 * 1024;

/// Spring ABC max read length
pub const SPRING_MAX_READ_LENGTH: usize = 511;

/// Medium read threshold (bytes)
pub const MEDIUM_READ_THRESHOLD: usize = 1_024;

/// Long read threshold (bytes)
pub const LONG_READ_THRESHOLD: usize = 10_240;

/// Default placeholder quality character for discard mode
pub const DEFAULT_PLACEHOLDER_QUAL: char = '!';

// =============================================================================
// LengthStats
// =============================================================================

/// Statistics about read lengths in a sample.
#[derive(Debug, Clone, Copy, Default)]
pub struct LengthStats {
    pub sample_size: usize,
    pub avg_length: usize,
    pub median_length: usize,
    pub max_length: usize,
}

impl LengthStats {
    /// Compute length statistics from a slice of lengths.
    pub fn from_lengths(lengths: &[usize]) -> Self {
        Self::from_lengths_iter(lengths.iter().copied())
    }

    /// Compute length statistics without materializing every length.
    ///
    /// Counting distinct lengths keeps `--scan-all-lengths` memory proportional
    /// to the number of distinct read lengths instead of the read count.
    pub fn from_lengths_iter(lengths: impl IntoIterator<Item = usize>) -> Self {
        let mut counts: BTreeMap<usize, u64> = BTreeMap::new();
        for length in lengths {
            *counts.entry(length).or_insert(0) += 1;
        }
        Self::from_length_counts(&counts)
    }

    /// Compute statistics from an already-built length histogram.
    pub fn from_length_counts(counts: &BTreeMap<usize, u64>) -> Self {
        let total = counts.values().fold(0u64, |acc, &count| acc.saturating_add(count));
        if total == 0 {
            return Self::default();
        }

        let max_length = *counts.keys().next_back().unwrap_or(&0);
        let median_rank = total / 2;
        let mut cumulative = 0u64;
        let mut median_length = max_length;
        for (&length, &count) in counts {
            cumulative = cumulative.saturating_add(count);
            if cumulative > median_rank {
                median_length = length;
                break;
            }
        }

        let sum = counts.iter().fold(0u64, |acc, (&length, &count)| {
            acc.saturating_add((length as u64).saturating_mul(count))
        });
        Self {
            sample_size: usize::try_from(total).unwrap_or(usize::MAX),
            avg_length: usize::try_from(sum / total).unwrap_or(usize::MAX),
            median_length,
            max_length,
        }
    }
}

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

    pub fn as_str(self) -> &'static str {
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

    pub fn as_str(self) -> &'static str {
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

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Short => "short",
            Self::Medium => "medium",
            Self::Long => "long",
        }
    }

    /// Hard ceiling on a single read length for this class.
    ///
    /// `classify_read_length` guarantees `Short` reads are at most
    /// `SPRING_MAX_READ_LENGTH` and `Medium` reads stay below
    /// `LONG_READ_THRESHOLD`; `Long` has no natural ceiling, so it uses
    /// `MAX_LONG_READ_LENGTH`. The block-header validator and the block
    /// decoder both enforce this value, which is what lets
    /// `memory_estimate_read_length` be a real upper bound.
    pub fn max_read_length(self) -> usize {
        match self {
            Self::Short => SPRING_MAX_READ_LENGTH,
            Self::Medium => LONG_READ_THRESHOLD - 1,
            Self::Long => MAX_LONG_READ_LENGTH,
        }
    }

    /// Upper-bound read length for memory estimates when only the archive's
    /// length class is known.
    ///
    /// This is `max_read_length` (a ceiling enforced at decode), not an
    /// average: estimates built on it may reject work that would fit, but
    /// never under-estimate a forged archive that declares a smaller class.
    /// Callers that can read a block header still prefer its uniform length.
    pub fn memory_estimate_read_length(self) -> usize {
        self.max_read_length()
    }
}

/// Classify read length based on statistics
pub fn classify_read_length(median_length: usize, max_length: usize) -> ReadLengthClass {
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

/// Choose a block size from read count, base budget, and available workers.
///
/// `requested` is an explicit user override and wins. Otherwise the target is
/// two blocks per codec worker (clamped to 4..=64 blocks), with a class-specific
/// minimum read count and a base ceiling. Callers pass the memory-limited worker
/// count, not the requested thread count, so block sizing follows the pool that
/// will actually run. This keeps short-read blocks large enough for compression
/// locality while preventing long reads from falling into one serial block.
pub fn adaptive_block_size(
    requested: usize,
    class: ReadLengthClass,
    avg_length: usize,
    max_block_bases: usize,
    threads: usize,
    total_reads: Option<u64>,
    total_bases: Option<u64>,
) -> usize {
    // Preserve the archive/pipeline auto-split policy for existing callers.
    adaptive_block_size_with_policy(
        requested,
        class,
        avg_length,
        max_block_bases,
        threads,
        total_reads,
        total_bases,
        true,
    )
}

/// Adaptive block sizing with an explicit long-read auto-split policy.
///
/// `auto_split_long_reads = false` is used by the bounded streaming path: it
/// must not silently impose the archive/pipeline base ceiling. Explicit
/// `max_block_bases` still applies in both cases.
pub fn adaptive_block_size_with_policy(
    requested: usize,
    class: ReadLengthClass,
    avg_length: usize,
    max_block_bases: usize,
    threads: usize,
    total_reads: Option<u64>,
    total_bases: Option<u64>,
    auto_split_long_reads: bool,
) -> usize {
    if requested > 0 {
        return requested;
    }

    let effective_threads = if threads == 0 {
        std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
    } else {
        threads
    };
    let target_blocks = effective_threads.saturating_mul(2).clamp(4, 64);

    // Prefer exact totals when the caller has already ingested the input.
    // Sampled length stats are only a fallback for streaming, where totals are
    // not known yet; a divergent length distribution can otherwise mis-size
    // the base ceiling.
    let avg_length = total_bases
        .zip(total_reads)
        .filter(|(_, reads)| *reads > 0)
        .map(|(bases, reads)| usize::try_from(bases / reads).unwrap_or(usize::MAX).max(1))
        .unwrap_or_else(|| avg_length.max(1));

    let min_reads = match class {
        ReadLengthClass::Short => MIN_BLOCK_READS_SHORT,
        ReadLengthClass::Medium => MIN_BLOCK_READS_MEDIUM,
        ReadLengthClass::Long => MIN_BLOCK_READS_LONG,
    };

    let by_count = total_reads
        .map(|reads| usize::try_from(reads).unwrap_or(usize::MAX).div_ceil(target_blocks))
        .unwrap_or_else(|| recommended_block_size(class))
        .max(min_reads);

    let base_ceiling = if max_block_bases > 0 {
        Some(max_block_bases)
    } else if auto_split_long_reads {
        Some(if class == ReadLengthClass::Long {
            DEFAULT_LONG_READ_BLOCK_BASES
        } else {
            AUTO_MAX_BLOCK_BASES
        })
    } else {
        None
    };
    let by_bases = base_ceiling
        .map(|ceiling| (ceiling / avg_length).max(1))
        .unwrap_or(usize::MAX);

    by_count.min(by_bases).max(1)
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

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Interleaved => "interleaved",
            Self::Consecutive => "consecutive",
        }
    }

    /// Arrange paired-end reads according to this layout.
    ///
    /// - `Interleaved`: R1, R2, R1, R2, ...
    /// - `Consecutive`: All R1s followed by all R2s
    pub fn arrange(self, r1: Vec<ReadRecord>, r2: Vec<ReadRecord>) -> Vec<ReadRecord> {
        match self {
            Self::Interleaved => {
                let mut result = Vec::with_capacity(r1.len() + r2.len());
                let mut r1_iter = r1.into_iter();
                let mut r2_iter = r2.into_iter();
                loop {
                    match (r1_iter.next(), r2_iter.next()) {
                        (Some(a), Some(b)) => {
                            result.push(a);
                            result.push(b);
                        }
                        (Some(a), None) => result.push(a),
                        (None, Some(b)) => result.push(b),
                        (None, None) => break,
                    }
                }
                result
            }
            Self::Consecutive => {
                let mut result = r1;
                result.extend(r2);
                result
            }
        }
    }

    /// Split interleaved or consecutive reads into R1 and R2 pairs.
    ///
    /// - `Interleaved`: Expects R1, R2, R1, R2, ... order
    /// - `Consecutive`: Expects all R1s followed by all R2s
    ///
    /// Returns `(r1, r2)` vectors.
    pub fn split(self, reads: Vec<ReadRecord>) -> (Vec<ReadRecord>, Vec<ReadRecord>) {
        match self {
            Self::Interleaved => {
                let mut r1 = Vec::with_capacity(reads.len() / 2);
                let mut r2 = Vec::with_capacity(reads.len() / 2);
                for (i, read) in reads.into_iter().enumerate() {
                    if i % 2 == 0 {
                        r1.push(read);
                    } else {
                        r2.push(read);
                    }
                }
                (r1, r2)
            }
            Self::Consecutive => {
                let mid = reads.len() / 2;
                let mut iter = reads.into_iter();
                let r1: Vec<_> = iter.by_ref().take(mid).collect();
                let r2: Vec<_> = iter.collect();
                (r1, r2)
            }
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

    /// Strict parse for archive decode: unknown and Reserved are rejected.
    pub fn try_from_nibble(v: u8) -> Option<Self> {
        match v {
            0x0 => Some(Self::Raw),
            0x1 => Some(Self::AbcV1),
            0x2 => Some(Self::ScmV1),
            0x3 => Some(Self::DeltaLzma),
            0x4 => Some(Self::DeltaZstd),
            0x5 => Some(Self::DeltaVarint),
            0x6 => Some(Self::OverlapV1),
            0x7 => Some(Self::ZstdPlain),
            0x8 => Some(Self::ScmOrder1),
            0xE => Some(Self::External),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Raw => "Raw",
            Self::AbcV1 => "AbcV1",
            Self::ScmV1 => "ScmV1",
            Self::DeltaLzma => "DeltaLzma",
            Self::DeltaZstd => "DeltaZstd",
            Self::DeltaVarint => "DeltaVarint",
            Self::OverlapV1 => "OverlapV1",
            Self::ZstdPlain => "ZstdPlain",
            Self::ScmOrder1 => "ScmOrder1",
            Self::External => "External",
            Self::Reserved => "Reserved",
        }
    }
}

/// Encode codec as (family:4bit, version:4bit)
pub fn encode_codec(family: CodecFamily, version: u8) -> u8 {
    ((family as u8) << 4) | (version & 0x0F)
}

/// Decode codec family from codec byte (lenient; maps unknown to Reserved).
pub fn decode_codec_family(codec: u8) -> CodecFamily {
    CodecFamily::from_u8(codec >> 4)
}

/// Decode codec version nibble (low 4 bits).
pub fn decode_codec_version(codec: u8) -> u8 {
    codec & 0x0F
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
        Self {
            id,
            comment: String::new(),
            sequence,
            quality,
        }
    }

    /// Construct with all fields including comment
    pub fn with_comment(id: String, comment: String, sequence: String, quality: String) -> Self {
        Self {
            id,
            comment,
            sequence,
            quality,
        }
    }

    /// Structural validity: sequence and quality lengths must match.
    ///
    /// Empty records are valid here — the parser accepts them and the codec
    /// round-trips them faithfully, so verify must not flag archives
    /// containing them. Callers wanting stricter checks should validate
    /// during parsing (`ParserOptions::validate_sequence`).
    pub fn is_valid(&self) -> bool {
        self.sequence.len() == self.quality.len()
    }

    pub fn len(&self) -> usize {
        self.sequence.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sequence.is_empty()
    }
}
