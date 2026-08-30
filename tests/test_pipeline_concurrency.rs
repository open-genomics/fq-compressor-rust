// =============================================================================
// fqc-rust - Pipeline Concurrency Tests
// =============================================================================
// The pipeline's out-of-order writer reordering (BTreeMap + next_expected),
// multi-threaded block compression, and thread error propagation had no
// tests. These tests force many small blocks through several threads and
// verify byte-exact roundtrips plus stability across repeated runs.
// =============================================================================

use std::io::Write as _;

use fqc::algo::block_compressor::{BlockCompressor, BlockCompressorConfig};
use fqc::pipeline::compression::{CompressionPipeline, CompressionPipelineConfig};
use fqc::pipeline::{DEFAULT_MAX_IN_FLIGHT_BLOCKS, MIN_BLOCK_SIZE};
use fqc::types::*;

// =============================================================================
// Helpers
// =============================================================================

fn make_records(n: usize, len: usize) -> Vec<ReadRecord> {
    let bases = b"ACGT";
    (0..n)
        .map(|i| {
            let seq: String = (0..len).map(|j| bases[(i + j) % 4] as char).collect();
            let qual = "I".repeat(len);
            ReadRecord::new(format!("read_{:04}", i), seq, qual)
        })
        .collect()
}

fn write_fastq(path: &std::path::Path, records: &[ReadRecord]) {
    let mut out = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    for record in records {
        fqc::fastq::parser::write_record(&mut out, record).unwrap();
    }
    out.flush().unwrap();
}

fn read_fastq_records(path: &std::path::Path) -> Vec<ReadRecord> {
    let file = std::fs::File::open(path).unwrap();
    let mut parser = fqc::fastq::parser::FastqParser::new(std::io::BufReader::new(file));
    parser.collect_all().unwrap()
}

fn decompress_archive(archive: &std::path::Path) -> Vec<ReadRecord> {
    let dir = tempfile::tempdir().unwrap();
    let restored = dir.path().join("restored.fastq");

    let mut reader = fqc::archive::reader::FqcReader::open(archive.to_string_lossy().as_ref()).unwrap();
    let flags = reader.global_header.flags;
    let config = BlockCompressorConfig {
        read_length_class: fqc::archive::format::get_read_length_class(flags),
        quality_mode: fqc::archive::format::get_quality_mode(flags),
        id_mode: fqc::archive::format::get_id_mode(flags),
        ..Default::default()
    };
    let mut compressor = BlockCompressor::new(config);

    let mut out = std::io::BufWriter::new(std::fs::File::create(&restored).unwrap());
    for block_id in 0..reader.block_count() {
        let bd = reader.read_block(block_id as u32).unwrap();
        let bh = &bd.header;
        let dec = compressor
            .decompress_raw(
                bh.block_id,
                bh.uncompressed_count,
                bh.uniform_read_length,
                bh.codec_ids,
                bh.codec_seq,
                bh.codec_qual,
                bh.codec_aux,
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
    drop(out);

    read_fastq_records(&restored)
}

fn assert_records_match(expected: &[ReadRecord], actual: &[ReadRecord]) {
    assert_eq!(actual.len(), expected.len());
    for (i, (orig, rest)) in expected.iter().zip(actual.iter()).enumerate() {
        assert_eq!(orig.id, rest.id, "ID mismatch at read {i}");
        assert_eq!(orig.sequence, rest.sequence, "Sequence mismatch at read {i}");
        assert_eq!(orig.quality, rest.quality, "Quality mismatch at read {i}");
    }
}

fn run_pipeline_to_archive(
    records: &[ReadRecord],
    threads: usize,
    block_size: usize,
    max_in_flight: usize,
) -> (std::path::PathBuf, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let fastq = dir.path().join("in.fastq");
    let archive = dir.path().join("out.fqc");
    write_fastq(&fastq, records);

    let config = CompressionPipelineConfig {
        num_threads: threads,
        max_in_flight_blocks: max_in_flight,
        block_size,
        read_length_class: ReadLengthClass::Short,
        quality_mode: QualityMode::Lossless,
        id_mode: IdMode::Exact,
        enable_reorder: false,
        save_reorder_map: false,
        streaming_mode: false,
        force_overwrite: true,
        ..Default::default()
    };
    let mut pipeline = CompressionPipeline::new(config);
    pipeline.run(&fastq.to_string_lossy(), &archive, "in.fastq").unwrap();

    (archive, dir)
}

// =============================================================================
// Tests
// =============================================================================

#[test]
fn pipeline_many_small_blocks_roundtrip_with_few_threads() {
    // 40 blocks of MIN_BLOCK_SIZE reads through 2 threads: the writer's
    // BTreeMap + next_expected out-of-order path is heavily exercised.
    let records = make_records(40 * MIN_BLOCK_SIZE, 150);
    let (archive, _dir) = run_pipeline_to_archive(&records, 2, MIN_BLOCK_SIZE, 4);

    let reader = fqc::archive::reader::FqcReader::open(archive.to_string_lossy().as_ref()).unwrap();
    assert!(
        reader.block_count() >= 40,
        "expected >= 40 blocks, got {}",
        reader.block_count()
    );
    drop(reader);

    let restored = decompress_archive(&archive);
    assert_records_match(&records, &restored);
}

#[test]
fn pipeline_many_small_blocks_roundtrip_many_threads() {
    let records = make_records(40 * MIN_BLOCK_SIZE, 150);
    let (archive, _dir) = run_pipeline_to_archive(&records, 8, MIN_BLOCK_SIZE, DEFAULT_MAX_IN_FLIGHT_BLOCKS);

    let reader = fqc::archive::reader::FqcReader::open(archive.to_string_lossy().as_ref()).unwrap();
    assert!(reader.block_count() >= 40);
    drop(reader);

    let restored = decompress_archive(&archive);
    assert_records_match(&records, &restored);
}

#[test]
fn pipeline_repeated_runs_are_stable() {
    // Same input, three independent runs: the decompressed reads must be
    // identical (no data races or nondeterministic block ordering). Note the
    // archive bytes themselves legitimately differ because the global header
    // embeds a unix timestamp.
    let records = make_records(20 * MIN_BLOCK_SIZE, 150);

    let mut restored_first: Option<Vec<ReadRecord>> = None;
    let mut timestamps = Vec::new();
    for _ in 0..3 {
        let (archive, _dir) = run_pipeline_to_archive(&records, 4, MIN_BLOCK_SIZE, 4);

        let reader = fqc::archive::reader::FqcReader::open(archive.to_string_lossy().as_ref()).unwrap();
        timestamps.push(reader.global_header.timestamp);
        drop(reader);

        let restored = decompress_archive(&archive);
        if let Some(first) = &restored_first {
            assert_records_match(first, &restored);
        } else {
            restored_first = Some(restored);
        }
    }

    // Sanity: content is stable and the archive metadata really is the only
    // varying field (timestamps differ across runs).
    assert!(restored_first.is_some());
    assert!(
        timestamps.windows(2).any(|w| w[0] != w[1]) || timestamps.len() < 2,
        "timestamps should vary across runs (or there was only one run)"
    );
}

#[test]
fn pipeline_rejects_too_small_block_size() {
    let dir = tempfile::tempdir().unwrap();
    let fastq = dir.path().join("in.fastq");
    let archive = dir.path().join("out.fqc");
    write_fastq(&fastq, &make_records(100, 150));

    let config = CompressionPipelineConfig {
        num_threads: 2,
        block_size: MIN_BLOCK_SIZE - 1,
        enable_reorder: false,
        save_reorder_map: false,
        force_overwrite: true,
        ..Default::default()
    };
    let mut pipeline = CompressionPipeline::new(config);
    let result = pipeline.run(&fastq.to_string_lossy(), &archive, "in.fastq");
    assert!(result.is_err(), "block size below MIN_BLOCK_SIZE must be rejected");
    assert!(!archive.exists(), "no output must be left behind");
}

#[test]
fn pipeline_empty_input_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let fastq = dir.path().join("empty.fastq");
    let archive = dir.path().join("out.fqc");
    std::fs::write(&fastq, b"").unwrap();

    let config = CompressionPipelineConfig {
        num_threads: 2,
        block_size: MIN_BLOCK_SIZE,
        enable_reorder: false,
        save_reorder_map: false,
        force_overwrite: true,
        ..Default::default()
    };
    let mut pipeline = CompressionPipeline::new(config);
    let result = pipeline.run(&fastq.to_string_lossy(), &archive, "empty.fastq");
    assert!(result.is_err(), "empty input must be rejected");
    assert!(!archive.exists(), "no output must be left behind");
}
