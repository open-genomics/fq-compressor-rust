# Source Module Overview

This document provides a comprehensive overview of all source modules in the fqc codebase, their responsibilities, and how they interact.

## Module Map

```
src/
├── lib.rs                      # Library root, re-exports all modules
├── main.rs                     # CLI entry point (clap derive)
├── error.rs                    # FqcError (11 variants) → ExitCode (0-5)
├── types.rs                    # Core types: ReadRecord, QualityMode, IdMode, PeLayout, ReadLengthClass
├── format.rs                   # FQC binary format: magic, GlobalHeader, BlockHeader, Footer
├── fqc_reader.rs              # Block-indexed archive reader
├── fqc_writer.rs              # Archive writer with finalize
├── reorder_map.rs              # ZigZag delta + varint encoded bidirectional map
├── algo/                       # Compression algorithms
│   ├── mod.rs                  # Algorithm module index
│   ├── block_compressor.rs     # ABC (consensus + delta) / Zstd
│   ├── dna.rs                  # Shared DNA encoding tables + reverse complement
│   ├── global_analyzer.rs      # Minimizer reordering
│   ├── quality_compressor.rs   # SCM arithmetic coding
│   ├── id_compressor.rs        # ID tokenization + delta encoding
│   └── pe_optimizer.rs         # Paired-end optimization
├── commands/                   # CLI commands
│   ├── compress.rs             # default / streaming / pipeline modes
│   └── decompress.rs           # sequential / parallel / reorder / pipeline
├── common/
│   └── memory_budget.rs        # System memory detection, chunking
├── fastq/
│   └── parser.rs               # FASTQ parser, validation, PE, stats
├── io/
│   ├── async_io.rs             # Async read/write with buffer pool
│   └── compressed_stream.rs    # Feature-gated gz/bz2/xz/zst
└── pipeline/
    ├── mod.rs                  # Shared types (PipelineControl, PipelineStats)
    ├── compression.rs          # 3-stage compression pipeline
    └── decompression.rs        # 3-stage decompression pipeline
```

---

## Core Modules

### `lib.rs` — Library Root

The library crate root that re-exports all public modules. fqc is structured as both a library (for integration tests and reuse) and a binary (CLI tool).

```rust
pub mod algo;
pub mod commands;
pub mod common;
pub mod error;
pub mod fastq;
pub mod format;
pub mod fqc_reader;
pub mod fqc_writer;
pub mod io;
pub mod pipeline;
pub mod reorder_map;
pub mod types;
```

**Key responsibility**: Organize the public API surface. All modules are `pub` for integration testing and downstream consumption.

---

### `main.rs` — CLI Entry Point

The binary entry point built with `clap` derive macros. Parses command-line arguments, constructs option structs, and dispatches to the appropriate command handler.

**Commands supported**:

| Command | Handler | Description |
|---------|---------|-------------|
| `fqc <file.fastq>` | `compress::CompressCommand` | Compress FASTQ to .fqc (default) |
| `fqc decompress` | `decompress::DecompressCommand` | Decompress .fqc to FASTQ |
| `fqc info` | `info::InfoCommand` | Display archive metadata |
| `fqc verify` | `verify::VerifyCommand` | Validate archive integrity |

**Option flow**: CLI args → `CompressOptions` / `DecompressOptions` → command `execute()` → `ExitCode`.

---

### `error.rs` — Error Types

Defines the unified error type `FqcError` with 11 variants and maps them to CLI exit codes (0-5).

```rust
pub enum FqcError {
    Io,              // std::io::Error
    Format,          // Invalid binary format
    Compression,     // Compression library errors
    Decompression,   // Decompression library errors
    InvalidArgument, // Bad CLI arguments
    ChecksumMismatch,// Integrity failures
    CorruptedBlock,  // Block-level corruption
    UnsupportedVersion, // Format version mismatch
    Parse,           // FASTQ parsing errors
    OutOfRange,      // Index/range errors
    UnsupportedFormat, // Unsupported features
}
```

**Exit code mapping**:

| Exit Code | Category | Error Variants |
|-----------|----------|----------------|
| 0 | Success | — |
| 1 | Usage | `InvalidArgument`, `OutOfRange` |
| 2 | I/O | `Io`, `Compression` |
| 3 | Format | `Format`, `Decompression`, `Parse` |
| 4 | Checksum | `ChecksumMismatch`, `CorruptedBlock` |
| 5 | Unsupported | `UnsupportedVersion`, `UnsupportedFormat` |

Also provides `ErrorContext` for attaching file path, block ID, read ID, and byte offset to errors.

**Related**: [Binary Format Specification](./format-spec.md)

---

### `types.rs` — Core Type Definitions

Defines the foundational types used throughout the codebase:

**Enumerations**:

| Type | Variants | Purpose |
|------|----------|---------|
| `QualityMode` | `Lossless` (0), `Illumina8` (1), `Qvz` (2), `Discard` (3) | Quality score handling |
| `IdMode` | `Exact` (0), `Tokenize` (1), `Discard` (2) | Read ID preservation |
| `ReadLengthClass` | `Short` (0), `Medium` (1), `Long` (2) | Algorithm selection |
| `PeLayout` | `Interleaved` (0), `Consecutive` (1) | Paired-end layout |
| `ChecksumType` | `XxHash64` (0) | Checksum algorithm |
| `CodecFamily` | `Raw`, `AbcV1`, `ScmV1`, `DeltaZstd`, `ZstdPlain`, etc. | Per-stream codec identification |

**Core structs**:

- `ReadRecord` — Single FASTQ record (`id`, `comment`, `sequence`, `quality`)
- `CompressOptions` — All compression parameters with `Default` impl
- `DecompressOptions` — All decompression parameters with `Default` impl

**Constants**:

| Constant | Value | Purpose |
|----------|-------|---------|
| `DEFAULT_COMPRESSION_LEVEL` | 5 | Default zstd level |
| `DEFAULT_BLOCK_SIZE_SHORT` | 100,000 | Reads per block for short reads |
| `DEFAULT_BLOCK_SIZE_MEDIUM` | 50,000 | Reads per block for medium reads |
| `DEFAULT_BLOCK_SIZE_LONG` | 10,000 | Reads per block for long reads |
| `SPRING_MAX_READ_LENGTH` | 511 | ABC algorithm max read length |
| `LONG_READ_THRESHOLD` | 10,240 | Bytes, long-read classification |

**Key functions**:
- `classify_read_length(median, max)` → `ReadLengthClass`
- `recommended_block_size(class)` → `usize`
- `encode_codec(family, version)` → `u8` (4-bit family + 4-bit version)
- `decode_codec(codec_byte)` → `CodecFamily`

**Related**: [Strategy Selection](../algorithms/strategy-selection.md), [Block Format](./block-format.md)

---

### `format.rs` — FQC Binary Format

Defines the complete `.fqc` binary format specification. This module is the single source of truth for the on-disk format.

**File layout**:

```
+----------------+
|  Magic Header  |  (9 bytes: 8 magic + 1 version)
+----------------+
| Global Header  |  (Variable length, min 34 bytes)
+----------------+
|    Block 0     |  (104-byte header + payload streams)
+----------------+
|    Block 1     |
+----------------+
|      ...       |
+----------------+
|    Block N     |
+----------------+
| Reorder Map    |  (Optional, variable length)
+----------------+
|   Block Index  |  (16-byte header + 28-byte entries)
+----------------+
|  File Footer   |  (32 bytes)
+----------------+
```

**Structs**:

| Struct | Size | Purpose |
|--------|------|---------|
| `GlobalHeader` | Variable (min 34 bytes) | Archive-level metadata, flags, filename |
| `BlockHeader` | 104 bytes fixed | Per-block metadata, codec IDs, stream offsets |
| `IndexEntry` | 28 bytes | Block index for random access |
| `BlockIndex` | Variable | Header + array of IndexEntry |
| `ReorderMapHeader` | 32 bytes | Reorder map metadata |
| `FileFooter` | 32 bytes | Index offset, reorder offset, global checksum, EOF magic |

**Flag bit definitions** (64-bit flags in `GlobalHeader`):

| Bit | Name | Description |
|-----|------|-------------|
| 0 | `IS_PAIRED` | Paired-end data |
| 1 | `PRESERVE_ORDER` | No reordering performed |
| 2 | `LEGACY_LONG_READ_MODE` | Legacy long-read mode |
| 3-4 | `QUALITY_MODE_MASK` | Quality mode (2 bits) |
| 5-6 | `ID_MODE_MASK` | ID mode (2 bits) |
| 7 | `HAS_REORDER_MAP` | Reorder map present |
| 8-9 | `PE_LAYOUT_MASK` | PE layout (2 bits) |
| 10-11 | `READ_LENGTH_CLASS_MASK` | Read length class (2 bits) |
| 12 | `STREAMING_MODE` | Written in streaming mode |

**Magic constants**:
- `MAGIC_BYTES`: `[0x89, 'F', 'Q', 'C', 0x0D, 0x0A, 0x1A, 0x0A]`
- `MAGIC_END`: `['F', 'Q', 'C', '_', 'E', 'O', 'F', 0x00]`
- Format version: major 2, minor 0

**Related**: [Block Format](./block-format.md), [Full Format Spec](./format-spec.md)

---

### `fqc_reader.rs` — Archive Reader

Block-indexed reader for `.fqc` archives. Provides random access to individual blocks via the block index stored at the end of the file.

**Key struct**: `FqcReader`

```rust
pub struct FqcReader {
    path: String,
    reader: BufReader<File>,
    pub global_header: GlobalHeader,
    pub footer: FileFooter,
    pub block_index: BlockIndex,
    pub file_size: u64,
    pub reorder_forward: Option<Vec<u64>>,
    pub reorder_reverse: Option<Vec<u64>>,
}
```

**Operations**:

| Method | Description |
|--------|-------------|
| `open(path)` | Open and validate an archive (magic, version) |
| `read_block(block_id)` | Read a single block with all streams |
| `read_block_header(block_id)` | Read only block header (no payload) |
| `load_reorder_map()` | Load and decode reorder maps |
| `lookup_original_id(archive_id)` | Map archive → original read ID |

**Open sequence**:
1. Read magic bytes (8) + version (1)
2. Seek to end, read footer (32 bytes)
3. Seek to after magic, read `GlobalHeader`
4. Seek to `footer.index_offset`, read `BlockIndex`

**Related**: [Block Format](./block-format.md)

---

### `fqc_writer.rs` — Archive Writer

Sequential writer for `.fqc` archives. Handles magic writing, block serialization, reorder map writing, and finalization with index and footer.

**Key struct**: `FqcWriter`

```rust
pub struct FqcWriter {
    writer: BufWriter<File>,
    current_offset: u64,
    index_entries: Vec<IndexEntry>,
    reorder_map_offset: u64,
    global_hasher: Xxh64,
    block_count: u64,
}
```

**Operations**:

| Method | Description |
|--------|-------------|
| `create(path)` | Create file, write magic + version |
| `write_global_header(header)` | Write `GlobalHeader` |
| `write_block(compressed)` | Write a `CompressedBlockData` |
| `write_block_with_id(compressed, start)` | Write block with explicit archive ID range |
| `write_reorder_map(fwd, rev)` | Write compressed reorder maps |
| `finalize()` | Write block index + footer, flush |
| `patch_total_read_count(n)` | Patch total read count in header (streaming mode) |

**Block writing flow**:
1. Write `BlockHeader` (104 bytes)
2. Write four payload streams in order: IDs, sequences, quality, aux
3. Record `IndexEntry` with offset and size
4. Update `global_hasher` with payload bytes

**Related**: [Full Format Spec](./format-spec.md)

---

### `reorder_map.rs` — Reorder Map

Implements the bidirectional read reordering map with ZigZag delta + varint compression.

**Key struct**: `ReorderMapData`

```rust
pub struct ReorderMapData {
    forward_map: Vec<ReadId>,  // original_id → archive_id
    reverse_map: Vec<ReadId>,  // archive_id → original_id
}
```

**Encoding**: Both maps are delta-encoded, then ZigZag varint compressed, then zstd-compressed.

**Key operations**:

| Method | Description |
|--------|-------------|
| `from_reverse_map(reverse)` | Build from archive order |
| `identity(n)` | Identity map (no reordering) |
| `get_archive_id(original_id)` | Forward lookup |
| `get_original_id(archive_id)` | Reverse lookup |
| `serialize()` | Delta + varint + zstd compress |
| `deserialize(data)` | Decompress and decode |
| `combine_chunks(chunks, sizes)` | Merge chunk maps in divide-and-conquer mode |
| `compression_stats()` | Report bytes/read and compression ratio |

**Validation**:
- `verify_map_consistency()` — Checks forward/reverse inverse relationship
- `validate_permutation()` — Checks map is a valid permutation

**Related**: [Reorder Map Architecture](./reorder-map.md)

---

## Algorithm Modules (`src/algo/`)

### `algo/dna.rs` — DNA Utilities

Shared DNA encoding tables and reverse complement function used by `block_compressor`, `global_analyzer`, and `pe_optimizer`.

**Tables**:

| Constant | Description |
|----------|-------------|
| `BASE_TO_INDEX[256]` | ASCII → 2-bit index (A=0, C=1, G=2, T=3, N=4) |
| `INDEX_TO_BASE[5]` | 2-bit index → ASCII base |
| `COMPLEMENT[256]` | ASCII → complement base (A↔T, C↔G, N→N) |

**Functions**:
- `reverse_complement(seq)` — Computes reverse complement of a DNA sequence
- `is_valid_base(c)` / `is_valid_base_strict(c)` — Base validation
- `validate_sequence(seq)` — Full sequence validation

**Related**: [ABC Algorithm](../algorithms/abc.md), [Minimizer](../algorithms/minimizer.md)

---

### `algo/block_compressor.rs` — Block Compressor

Implements the ABC algorithm (consensus + delta encoding) for short reads and Zstd passthrough for medium/long reads.

**Key structs**:

| Struct | Purpose |
|--------|---------|
| `BlockCompressor` | Main compressor with config |
| `BlockCompressorConfig` | Compression parameters |
| `CompressedBlockData` | Output: four compressed streams |
| `DecompressedBlockData` | Output: reconstructed `ReadRecord`s |
| `ConsensusSequence` | Running consensus with base counts |
| `DeltaEncodedRead` | Delta representation of a read |
| `Contig` | Consensus + its delta-encoded reads |

**Four output streams per block**:

| Stream | Codec (short reads) | Codec (long reads) |
|--------|---------------------|--------------------|
| IDs | `DeltaZstd` (tokenize + delta) | `DeltaZstd` |
| Sequences | `AbcV1` (consensus + delta) | `ZstdPlain` |
| Quality | `ScmV1` (SCM arithmetic) | `ScmOrder1` |
| Aux (lengths) | `DeltaVarint` | `DeltaVarint` |

**ABC compression process**:
1. `build_contigs()` — Group reads into contigs via alignment
2. For each contig: build consensus, encode all reads as deltas
3. Serialize contigs (format v2: u32 lengths for long-read support)
4. Apply zstd compression on the serialized buffer

**Related**: [ABC Algorithm](../algorithms/abc.md), [Zstd](../algorithms/zstd.md), [Strategy Selection](../algorithms/strategy-selection.md)

---

### `algo/global_analyzer.rs` — Global Analyzer

Minimizer-based read reordering to improve compression locality. Analyzes all reads before compression to determine optimal ordering.

**Key structs**:

| Struct | Purpose |
|--------|---------|
| `GlobalAnalyzer` | Main analyzer with config |
| `GlobalAnalyzerConfig` | k-mer size, window size, memory limits |
| `GlobalAnalysisResult` | Reordering maps, block boundaries |
| `BlockBoundary` | Archive ID ranges per block |
| `Minimizer` | Hash + position + reverse-complement flag |

**Analysis workflow**:
1. Extract minimizers from all sequences (parallel via `rayon`)
2. Build `HashMap<hash, Vec<read_id>>` index
3. Greedy reordering: start from read 0, find nearest neighbor by shared minimizers and length similarity
4. Compute block boundaries based on `ReadLengthClass`

**Configuration defaults**:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `minimizer_k` | 15 | k-mer length |
| `minimizer_w` | 10 | Window size |
| `max_search_reorder` | 64 | Candidates searched per step |
| `reads_per_block` | 100,000 | Default block size |

**Reordering is skipped** when:
- `enable_reorder` is `false`
- `ReadLengthClass` is not `Short`
- Input is paired-end

**Related**: [Minimizer Algorithm](../algorithms/minimizer.md), [Reorder Map](./reorder-map.md)

---

### `algo/quality_compressor.rs` — Quality Compressor

Statistical Context Model (SCM) with arithmetic coding for quality score compression.

**Key structs**:

| Struct | Purpose |
|--------|---------|
| `QualityCompressor` | Main compressor with context model |
| `QualityCompressorConfig` | Quality mode, context order, position bins |
| `QualityContextModel` | Collection of adaptive models per context |
| `AdaptiveModel` | Frequency table with rescaling |
| `ArithmeticEncoder` | Bit-level arithmetic encoder |
| `ArithmeticDecoder` | Bit-level arithmetic decoder |

**Context model hierarchy**:

| Context Order | Context dimensions | Models count |
|---------------|-------------------|--------------|
| Order-0 | position bin | 8 |
| Order-1 | prev_qual × position bin | 94 × 8 = 752 |
| Order-2 | prev2_qual × prev1_qual × position bin | 94² × 8 = 70,688 |

**Quality modes**:
- `Lossless` — Full 94-symbol alphabet (Phred 0-93)
- `Illumina8` — Quantized to 8 bins with representative values
- `Discard` — Empty output; placeholder `!` generated on decompression

**Illumina 8-bin mapping**:

| Bin | Range | Representative |
|-----|-------|----------------|
| 0 | Q < 2 | 2 |
| 1 | 2 ≤ Q < 10 | 6 |
| 2 | 10 ≤ Q < 20 | 15 |
| 3 | 20 ≤ Q < 25 | 22 |
| 4 | 25 ≤ Q < 30 | 27 |
| 5 | 30 ≤ Q < 35 | 33 |
| 6 | 35 ≤ Q < 40 | 37 |
| 7 | Q ≥ 40 | 40 |

**Codec selection**: Short/medium reads use `ScmV1` (Order-2); long reads use `ScmOrder1` (Order-1).

**Related**: [SCM Algorithm](../algorithms/scm.md)

---

### `algo/id_compressor.rs` — ID Compressor

Tokenization and delta encoding for read identifiers. Supports three modes: exact, tokenize, and discard.

**Token types**:

| Type | Code | Description |
|------|------|-------------|
| `Static` | 0 | Unchanging across reads (e.g., `HWI-ST1234`) |
| `DynamicInt` | 1 | Integer that varies (delta-encoded) |
| `DynamicString` | 2 | String that varies |
| `Delimiter` | 3 | Separator characters (`:/_|\t `) |

**Modes**:

| Mode | Magic | Description |
|------|-------|-------------|
| Exact | `0x01` | Length-prefixed IDs, zstd compressed |
| Tokenize | `0x02` | Pattern-based column storage, delta-varint for ints |
| Discard | `0x03` | Empty payload; regenerated as `{prefix}1`, `{prefix}2`, ... |

**Pattern detection**: Requires >= 95% of IDs to match the same token structure. Integer columns are delta-varint encoded; string columns are length-prefixed.

**Example tokenization**:

```
@HWI-ST1234:1:1101:12345:67890 1:N:0:ATGC
 → Static("HWI-ST1234"), Delimiter(":"), DynamicInt(1), Delimiter(":"),
   DynamicInt(1101), Delimiter(":"), DynamicInt(12345), Delimiter(":"),
   DynamicInt(67890), Delimiter(" "), DynamicInt(1), Delimiter(":"),
   Static("N"), Delimiter(":"), DynamicInt(0), Delimiter(":"),
   DynamicString("ATGC")
```

**Related**: [Core Compression Spec](../../specs/product/core-compression.md)

---

### `algo/pe_optimizer.rs` — Paired-End Optimizer

Exploits complementarity between paired-end reads. R2 is stored as a differential from R1's reverse complement.

**Key structs**:

| Struct | Purpose |
|--------|---------|
| `PEOptimizer` | Main optimizer with stats |
| `PEOptimizerConfig` | Thresholds and enable flags |
| `PEEncodedPair` | Encoded pair with diff data |
| `PEOptimizerStats` | Compression statistics |

**Complementarity check**:
1. Compute R1 reverse complement
2. Count Hamming distance between R1-RC and R2
3. If diff count <= threshold (default 15) and overlap >= 20bp, use complementarity

**Encoding when beneficial**:
- Store diff positions (u16), diff bases (u8), and quality deltas (i8)
- Serialize with varint-encoded diff counts and delta-encoded positions

**R2 ID generation**: Handles `/1`→`/2`, `.1`→`.2`, and `1:...`→`2:...` conventions.

**Related**: [PE Optimization Algorithm](../algorithms/pe-optimization.md)

---

## Command Modules (`src/commands/`)

### `commands/compress.rs` — Compress Command

Main compression command with three execution modes.

**Execution modes**:

| Mode | Flag | Description |
|------|------|-------------|
| Default | (none) | Load all records → global analysis → parallel block compression → sequential write |
| Streaming | `--streaming` | Read block-by-block, compress incrementally, no global reordering |
| Pipeline | `--pipeline` | 3-stage parallel pipeline (Reader → Compressor → Writer) |

**Phases (default mode)**:
1. Read all FASTQ records
2. Detect `ReadLengthClass`
3. Run `GlobalAnalyzer` (reordering + block boundaries)
4. Write `GlobalHeader`
5. Parallel block compression via `rayon`
6. Sequential block writing (file I/O must be ordered)
7. Write reorder map if applicable
8. Finalize archive

**Related**: [Pipeline Architecture](../../specs/rfc/0003-pipeline-architecture.md)

---

### `commands/decompress.rs` — Decompress Command

Decompression with multiple output modes.

**Features**:
- Sequential decompression (block-by-block)
- Parallel decompression (pipeline mode)
- Original order restoration via reorder map
- Range-based extraction (`--range`)
- Header-only mode (FASTA output, no quality)
- Corrupted block skipping

**Related**: [Pipeline Architecture](../../specs/rfc/0003-pipeline-architecture.md)

---

## I/O and Pipeline Modules

### `io/async_io.rs` — Async I/O

Asynchronous read/write with buffer pooling for high-throughput I/O operations.

### `io/compressed_stream.rs` — Compressed Stream

Feature-gated support for compressed input formats:

| Feature | Extension | Library |
|---------|-----------|---------|
| (always) | `.zst` | `zstd` |
| `gz` | `.gz`, `.fastq.gz` | `flate2` |
| `bz2` | `.bz2` | `bzip2` |
| `xz` | `.xz`, `.lzma` | `xz2` |

Auto-detection by file extension and magic bytes.

### `pipeline/mod.rs` — Pipeline Shared Types

Shared infrastructure for the 3-stage pipeline architecture.

**Key types**:

| Type | Purpose |
|------|---------|
| `PipelineControl` | Cancellation and progress tracking (atomic counters) |
| `PipelineStats` | Collected statistics (reads, blocks, bytes, time, memory) |
| `ProgressInfo` | Progress snapshot for callbacks |
| `ReadChunk` | Data unit passed between pipeline stages |
| `ProgressCallback` | `Fn(&ProgressInfo) -> bool` for progress reporting |

**Constants**:

| Constant | Value | Purpose |
|----------|-------|---------|
| `DEFAULT_MAX_IN_FLIGHT_BLOCKS` | 8 | Channel backpressure limit |
| `DEFAULT_INPUT_BUFFER_SIZE` | 64 MB | Input buffer |
| `DEFAULT_OUTPUT_BUFFER_SIZE` | 32 MB | Output buffer |
| `MIN_BLOCK_SIZE` | 100 | Minimum reads per block |
| `MAX_BLOCK_SIZE` | 1,000,000 | Maximum reads per block |

### `pipeline/compression.rs` — Compression Pipeline

3-stage pipeline following the pigz model:

```
[Reader Stage] → [Compressor Stage (parallel)] → [Writer Stage]
   (serial)            (rayon threads)             (serial)
```

1. **Reader**: Reads FASTQ, emits `ReadChunk` items
2. **Compressor**: Multiple threads compress chunks independently
3. **Writer**: Serial writer ensures block ordering

### `pipeline/decompression.rs` — Decompression Pipeline

3-stage decompression pipeline:

```
[Reader Stage] → [Decompressor Stage (parallel)] → [Writer Stage]
   (serial)             (rayon threads)              (serial)
```

---

### `fastq/parser.rs` — FASTQ Parser

Complete FASTQ parser with optional validation and statistics collection.

**Structs**:

| Struct | Purpose |
|--------|---------|
| `FastqParser<R>` | Main parser over any `BufRead` |
| `ParserOptions` | Validation flags |
| `ParserStats` | Collected statistics |
| `PairedFastqReader<R1, R2>` | Two-file paired-end reader |
| `InterleavedPeParser<R>` | Single-file interleaved PE reader |

**Validation**:
- `validate_sequence()` — Checks for valid DNA bases (A/C/G/T/N)
- `validate_quality_string()` — Checks Phred+33 range (ASCII 33-126)
- `validate_pe_pair_ids()` — Checks R1/R2 ID pairing conventions

**Statistics** (`ParserStats`): total records, bases, min/max length, N count, bytes read.

### `common/memory_budget.rs` — Memory Budget

System memory detection and chunking calculations. Uses `tikv-jemallocator` for musl static builds. Platform-specific memory queries via FFI on Windows.

---

## Cross-Reference Index

| Topic | Primary Module | Related Modules |
|-------|----------------|-----------------|
| Binary format | `format.rs` | `fqc_writer.rs`, `fqc_reader.rs` |
| ABC compression | `algo/block_compressor.rs` | `algo/dna.rs` |
| Quality compression | `algo/quality_compressor.rs` | `types.rs` |
| ID compression | `algo/id_compressor.rs` | `types.rs` |
| Read reordering | `algo/global_analyzer.rs` | `reorder_map.rs` |
| PE optimization | `algo/pe_optimizer.rs` | `algo/dna.rs` |
| CLI compress | `commands/compress.rs` | `pipeline/compression.rs` |
| CLI decompress | `commands/decompress.rs` | `pipeline/decompression.rs` |
| Error handling | `error.rs` | All modules |
| Core types | `types.rs` | All modules |

---

## Related Documents

- [Block Format Specification](./block-format.md)
- [Reorder Map Architecture](./reorder-map.md)
- [Full FQC Format Specification](./format-spec.md)
- [Strategy Selection](../algorithms/strategy-selection.md)
- [Core Compression Spec](../../specs/product/core-compression.md)
- [Compression Algorithms RFC](../../specs/rfc/0002-compression-algorithms.md)
