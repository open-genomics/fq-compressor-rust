// =============================================================================
// fqc-rust - End-to-End (E2E) Tests
// =============================================================================
// Tests the full CLI compress → decompress → verify round-trip using the
// library API (not spawning subprocesses).
// =============================================================================

use std::io::BufReader;

use fqc::algo::block_compressor::{BlockCompressor, BlockCompressorConfig};
use fqc::error::ExitCode;
use fqc::fastq::parser::{FastqParser, ParserOptions};
use fqc::format::*;
use fqc::fqc_reader::FqcReader;
use fqc::fqc_writer::FqcWriter;
use fqc::io::compressed_stream::*;
use fqc::reorder_map::*;
use fqc::types::*;

// =============================================================================
// Helpers
// =============================================================================

fn test_data_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
}

/// RAII guard that removes the file on drop, ensuring test cleanup.
struct TempFile(String);

impl TempFile {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join("fqc_e2e_tests");
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir.join(name).to_string_lossy().to_string())
    }
    fn path(&self) -> &str {
        &self.0
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn assert_roundtrip_match(original: &[ReadRecord], restored: &[ReadRecord]) {
    assert_eq!(
        original.len(),
        restored.len(),
        "Read count mismatch: {} vs {}",
        original.len(),
        restored.len()
    );
    for (i, (orig, rest)) in original.iter().zip(restored.iter()).enumerate() {
        assert_eq!(orig.id, rest.id, "ID mismatch at read {i}");
        assert_eq!(orig.sequence, rest.sequence, "Sequence mismatch at read {i}");
        assert_eq!(orig.quality, rest.quality, "Quality mismatch at read {i}");
    }
}

fn read_fastq_records(path: &str) -> Vec<ReadRecord> {
    let file = std::fs::File::open(path).unwrap();
    let reader = BufReader::new(file);
    let mut parser = FastqParser::new(reader);
    parser.collect_all().unwrap()
}

fn compress_file(
    input_path: &str,
    output_path: &str,
    quality_mode: QualityMode,
    id_mode: IdMode,
    enable_reorder: bool,
) {
    let records = read_fastq_records(input_path);
    assert!(!records.is_empty(), "Input file is empty");

    let length_class = ReadLengthClass::Short;
    let block_size = recommended_block_size(length_class);

    let flags = build_flags(
        false,
        !enable_reorder,
        quality_mode,
        id_mode,
        false,
        PeLayout::Interleaved,
        length_class,
        false,
    );

    let mut writer = FqcWriter::create(output_path).unwrap();
    let gh = GlobalHeader::new(flags, records.len() as u64, "test.fastq", 0);
    writer.write_global_header(&gh).unwrap();

    let config = BlockCompressorConfig {
        read_length_class: length_class,
        quality_mode,
        id_mode,
        zstd_level: 3,
        ..Default::default()
    };
    let compressor = BlockCompressor::new(config);

    for (i, chunk) in records.chunks(block_size).enumerate() {
        let compressed = compressor.compress(chunk, i as u32).unwrap();
        writer.write_block(&compressed).unwrap();
    }

    writer.finalize().unwrap();
}

fn decompress_file(input_path: &str, output_path: &str) {
    let mut reader = FqcReader::open(input_path).unwrap();
    let block_count = reader.block_count();

    let f = reader.global_header.flags;
    let config = BlockCompressorConfig {
        read_length_class: get_read_length_class(f),
        quality_mode: get_quality_mode(f),
        id_mode: get_id_mode(f),
        ..Default::default()
    };
    let compressor = BlockCompressor::new(config);

    let mut out = std::io::BufWriter::new(std::fs::File::create(output_path).unwrap());
    for block_id in 0..block_count {
        let bd = reader.read_block(block_id as u32).unwrap();
        let bh = &bd.header;
        let dec = compressor
            .decompress_raw(
                bh.block_id,
                bh.uncompressed_count,
                bh.uniform_read_length,
                bh.codec_seq,
                bh.codec_qual,
                &bd.ids_data,
                &bd.seq_data,
                &bd.qual_data,
                &bd.aux_data,
            )
            .unwrap();
        for read in &dec.reads {
            fqc::fastq::parser::write_record(&mut out, read).unwrap();
        }
    }
}

// =============================================================================
// E2E: Single-End Lossless Round-Trip
// =============================================================================

#[test]
fn test_e2e_se_lossless_roundtrip() {
    let input = test_data_dir().join("test_se.fastq").to_string_lossy().to_string();
    let compressed = TempFile::new("e2e_se_lossless.fqc");
    let decompressed = TempFile::new("e2e_se_lossless.fastq");

    compress_file(&input, compressed.path(), QualityMode::Lossless, IdMode::Exact, false);
    decompress_file(compressed.path(), decompressed.path());

    let original = read_fastq_records(&input);
    let restored = read_fastq_records(decompressed.path());
    assert_roundtrip_match(&original, &restored);
}

// =============================================================================
// E2E: Quality Discard Mode
// =============================================================================

#[test]
fn test_e2e_se_quality_discard() {
    let input = test_data_dir().join("test_se.fastq").to_string_lossy().to_string();
    let compressed = TempFile::new("e2e_se_qdiscard.fqc");
    let decompressed = TempFile::new("e2e_se_qdiscard.fastq");

    compress_file(&input, compressed.path(), QualityMode::Discard, IdMode::Exact, false);
    decompress_file(compressed.path(), decompressed.path());

    let original = read_fastq_records(&input);
    let restored = read_fastq_records(decompressed.path());

    assert_eq!(original.len(), restored.len());
    for (i, (orig, rest)) in original.iter().zip(restored.iter()).enumerate() {
        assert_eq!(orig.id, rest.id, "ID mismatch at read {i}");
        assert_eq!(orig.sequence, rest.sequence, "Sequence mismatch at read {i}");
        let first_char = rest.quality.chars().next().unwrap_or('!');
        assert!(
            rest.quality.chars().all(|c| c == first_char),
            "Quality should be uniform placeholder at read {i}"
        );
    }
}

// =============================================================================
// E2E: ID Discard Mode
// =============================================================================

#[test]
fn test_e2e_se_id_discard() {
    let input = test_data_dir().join("test_se.fastq").to_string_lossy().to_string();
    let compressed = TempFile::new("e2e_se_iddiscard.fqc");
    let decompressed = TempFile::new("e2e_se_iddiscard.fastq");

    compress_file(&input, compressed.path(), QualityMode::Lossless, IdMode::Discard, false);
    decompress_file(compressed.path(), decompressed.path());

    let original = read_fastq_records(&input);
    let restored = read_fastq_records(decompressed.path());

    assert_eq!(original.len(), restored.len());
    for (i, (orig, rest)) in original.iter().zip(restored.iter()).enumerate() {
        assert!(!rest.id.is_empty(), "ID should not be empty at read {i}");
        assert_eq!(orig.sequence, rest.sequence, "Sequence mismatch at read {i}");
        assert_eq!(orig.quality, rest.quality, "Quality mismatch at read {i}");
    }
}

// =============================================================================
// E2E: Archive Info (verify metadata)
// =============================================================================

#[test]
fn test_e2e_archive_info() {
    let input = test_data_dir().join("test_se.fastq").to_string_lossy().to_string();
    let compressed = TempFile::new("e2e_info.fqc");

    compress_file(&input, compressed.path(), QualityMode::Lossless, IdMode::Exact, false);

    let reader = FqcReader::open(compressed.path()).unwrap();
    assert_eq!(reader.total_read_count(), 20);
    assert!(reader.block_count() > 0);

    let f = reader.global_header.flags;
    assert_eq!(get_quality_mode(f), QualityMode::Lossless);
    assert_eq!(get_id_mode(f), IdMode::Exact);
}

// =============================================================================
// E2E: Parser Statistics
// =============================================================================

#[test]
fn test_e2e_parser_stats() {
    let input = test_data_dir().join("test_se.fastq").to_string_lossy().to_string();
    let file = std::fs::File::open(&input).unwrap();
    let reader = BufReader::new(file);
    let opts = ParserOptions {
        collect_stats: true,
        validate_sequence: true,
        ..Default::default()
    };
    let mut parser = FastqParser::with_options(reader, opts);
    let records = parser.collect_all().unwrap();

    let stats = parser.stats();
    assert_eq!(stats.total_records, 20);
    assert_eq!(stats.total_records, records.len() as u64);
    assert!(stats.total_bases > 0);
    assert!(stats.min_length > 0);
    assert!(stats.max_length >= stats.min_length);
    assert!(stats.avg_length() > 0.0);
}

// =============================================================================
// E2E: Compressed Stream Detection
// =============================================================================

#[test]
fn test_e2e_compression_format_detection() {
    let input = test_data_dir().join("test_se.fastq").to_string_lossy().to_string();
    let fmt = detect_compression_format(&input);
    assert_eq!(fmt, CompressionFormat::Plain);
}

#[test]
fn test_e2e_compression_format_extension() {
    assert_eq!(detect_format_from_extension("file.fastq.gz"), CompressionFormat::Gzip);
    assert_eq!(detect_format_from_extension("file.fq.bz2"), CompressionFormat::Bzip2);
    assert_eq!(detect_format_from_extension("file.fastq.xz"), CompressionFormat::Xz);
    assert_eq!(detect_format_from_extension("file.fastq.zst"), CompressionFormat::Zstd);
    assert_eq!(detect_format_from_extension("file.fastq"), CompressionFormat::Plain);
}

#[test]
fn test_e2e_supported_formats() {
    let formats = supported_formats();
    assert!(formats.len() >= 2);
    for fmt in &formats {
        assert!(is_compression_supported(*fmt));
    }
}

// =============================================================================
// E2E: ReorderMap Round-Trip with Real Data
// =============================================================================

#[test]
fn test_e2e_reorder_map_roundtrip() {
    let n = 500;
    // Create a pseudo-random permutation
    let mut reverse: Vec<u64> = (0..n as u64).collect();
    // Simple shuffle using a deterministic seed
    for i in (1..n).rev() {
        let j = (i * 7 + 3) % (i + 1);
        reverse.swap(i, j);
    }

    let map = ReorderMapData::from_reverse_map(reverse);
    assert!(map.is_valid());

    let serialized = map.serialize().unwrap();
    let restored = ReorderMapData::deserialize(&serialized).unwrap();
    assert!(restored.is_valid());
    assert_eq!(restored.total_reads(), n as u64);

    for i in 0..n as u64 {
        assert_eq!(map.get_archive_id(i), restored.get_archive_id(i));
        assert_eq!(map.get_original_id(i), restored.get_original_id(i));
    }
}

// =============================================================================
// E2E: ExitCode Mapping
// =============================================================================

#[test]
fn test_e2e_exit_codes() {
    use fqc::error::FqcError;

    let io_err = FqcError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "test"));
    assert_eq!(io_err.exit_code(), ExitCode::IoError);
    assert_eq!(io_err.exit_code_num(), 2);

    let fmt_err = FqcError::Format("bad".to_string());
    assert_eq!(fmt_err.exit_code(), ExitCode::FormatError);
    assert_eq!(fmt_err.exit_code_num(), 3);

    let arg_err = FqcError::InvalidArgument("bad".to_string());
    assert_eq!(arg_err.exit_code(), ExitCode::Usage);
    assert_eq!(arg_err.exit_code_num(), 1);

    let chk_err = FqcError::ChecksumMismatch { expected: 0, actual: 1 };
    assert_eq!(chk_err.exit_code(), ExitCode::ChecksumError);
    assert_eq!(chk_err.exit_code_num(), 4);

    let ver_err = FqcError::UnsupportedVersion { major: 99 };
    assert_eq!(ver_err.exit_code(), ExitCode::UnsupportedError);
    assert_eq!(ver_err.exit_code_num(), 5);
}

// =============================================================================
// E2E: Memory Budget
// =============================================================================

#[test]
fn test_e2e_memory_budget() {
    use fqc::common::memory_budget::*;

    let avail = get_available_memory_mb();
    assert!(avail > 0, "System memory detection failed");

    let budget = auto_memory_budget(0);
    assert!(budget.max_total_mb > 0);

    let budget2 = auto_memory_budget(4096);
    assert_eq!(budget2.max_total_mb, 4096);
}

#[test]
fn test_e2e_chunking_strategy() {
    use fqc::common::memory_budget::ChunkingStrategy;

    // Small dataset: should not require chunking
    let strat = ChunkingStrategy::compute(100_000, 150, 50_000, 4, 8192);
    assert!(!strat.requires_chunking());
    assert_eq!(strat.num_chunks, 1);

    // Very large dataset with small memory limit: should chunk
    let strat2 = ChunkingStrategy::compute(500_000_000, 150, 50_000, 8, 512);
    assert!(strat2.requires_chunking());
    assert!(strat2.num_chunks > 1);
    assert!(strat2.reads_per_chunk < 500_000_000);
}

// =============================================================================
// E2E: Pipeline Compression Round-Trip
// =============================================================================

#[test]
fn test_e2e_pipeline_roundtrip() {
    use fqc::pipeline::compression::{CompressionPipeline, CompressionPipelineConfig};

    let input = test_data_dir().join("test_se.fastq").to_string_lossy().to_string();
    let compressed = TempFile::new("e2e_pipeline.fqc");
    let decompressed = TempFile::new("e2e_pipeline.fastq");

    let config = CompressionPipelineConfig {
        num_threads: 2,
        block_size: 100,
        read_length_class: ReadLengthClass::Short,
        quality_mode: QualityMode::Lossless,
        id_mode: IdMode::Exact,
        compression_level: 3,
        enable_reorder: false,
        save_reorder_map: false,
        streaming_mode: false,
        pe_layout: PeLayout::Interleaved,
        memory_limit_mb: 1024,
        ..Default::default()
    };

    let mut pipeline = CompressionPipeline::new(config);
    pipeline.run(&input, compressed.path(), "test_se.fastq", None).unwrap();

    let stats = pipeline.stats();
    assert_eq!(stats.total_reads, 20);
    assert!(stats.total_blocks >= 1);
    assert!(stats.output_bytes > 0);

    decompress_file(compressed.path(), decompressed.path());
    let original = read_fastq_records(&input);
    let restored = read_fastq_records(decompressed.path());
    assert_roundtrip_match(&original, &restored);
}

// =============================================================================
// E2E: Decompression Pipeline Round-Trip
// =============================================================================

#[test]
fn test_e2e_decompress_pipeline_roundtrip() {
    use fqc::pipeline::decompression::{DecompressionPipeline, DecompressionPipelineConfig};

    let input = test_data_dir().join("test_se.fastq").to_string_lossy().to_string();
    let compressed = TempFile::new("e2e_dec_pipeline.fqc");
    let decompressed = TempFile::new("e2e_dec_pipeline.fastq");

    compress_file(&input, compressed.path(), QualityMode::Lossless, IdMode::Exact, false);

    let config = DecompressionPipelineConfig {
        num_threads: 2,
        skip_corrupted: false,
        ..Default::default()
    };
    let mut pipeline = DecompressionPipeline::new(config);
    pipeline.run(compressed.path(), decompressed.path(), None).unwrap();

    let stats = pipeline.stats();
    assert_eq!(stats.total_reads, 20);
    assert!(stats.output_bytes > 0);

    let original = read_fastq_records(&input);
    let restored = read_fastq_records(decompressed.path());
    assert_roundtrip_match(&original, &restored);
}

// =============================================================================
// E2E: Multiple Blocks (enough reads to span >1 block)
// =============================================================================

#[test]
fn test_e2e_multiblock_roundtrip() {
    let compressed = TempFile::new("e2e_multiblock.fqc");
    let decompressed = TempFile::new("e2e_multiblock.fastq");

    // Generate 200 reads (block_size for short is typically 50000, so use smaller block)
    let records: Vec<ReadRecord> = (0..200)
        .map(|i| {
            let bases = b"ACGT";
            let seq: String = (0..100).map(|j| bases[(i + j) % 4] as char).collect();
            let qual = "I".repeat(100);
            ReadRecord::new(format!("read_{}", i), seq, qual)
        })
        .collect();

    let block_size = 50; // Force multiple blocks
    let flags = build_flags(
        false,
        true,
        QualityMode::Lossless,
        IdMode::Exact,
        false,
        PeLayout::Interleaved,
        ReadLengthClass::Short,
        false,
    );

    let mut writer = FqcWriter::create(compressed.path()).unwrap();
    let gh = GlobalHeader::new(flags, records.len() as u64, "gen.fastq", 0);
    writer.write_global_header(&gh).unwrap();

    let config = BlockCompressorConfig {
        read_length_class: ReadLengthClass::Short,
        quality_mode: QualityMode::Lossless,
        id_mode: IdMode::Exact,
        zstd_level: 3,
        ..Default::default()
    };
    let compressor = BlockCompressor::new(config);

    let mut blocks = 0u32;
    for chunk in records.chunks(block_size) {
        let compressed_block = compressor.compress(chunk, blocks).unwrap();
        writer.write_block(&compressed_block).unwrap();
        blocks += 1;
    }
    writer.finalize().unwrap();

    assert!(blocks >= 4, "Expected at least 4 blocks, got {}", blocks);

    decompress_file(compressed.path(), decompressed.path());
    let restored = read_fastq_records(decompressed.path());
    assert_roundtrip_match(&records, &restored);
}
