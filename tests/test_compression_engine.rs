// =============================================================================
// fqc-rust - Compression Engine Interface Tests
// =============================================================================

use fqc::commands::compress::CompressOptions;
use fqc::engine::compression_engine::{CompressionEngine, CompressionOutcome};
use fqc::engine::compression_request::{CompressionExecutionMode, CompressionInputTopology, CompressionRequest};
use fqc::fastq::parser::open_fastq;
use fqc::types::{IdMode, PeLayout, QualityMode, ReadLengthClass};

// =============================================================================
// Task 1: CompressionRequest and CompressionEngine interface tests
// =============================================================================

#[test]
fn request_keeps_pipeline_and_streaming_distinct() {
    let request = CompressionRequest {
        mode: CompressionExecutionMode::Pipeline,
        input: CompressionInputTopology::SingleFile {
            input_path: "tests/data/test_se.fastq".into(),
        },
        quality_mode: QualityMode::Lossless,
        id_mode: IdMode::Exact,
        requested_read_length_class: None,
        force_overwrite: true,
        ..CompressionRequest::for_tests()
    };

    assert_eq!(request.mode, CompressionExecutionMode::Pipeline);
}

#[test]
fn outcome_exposes_mode_length_class_and_reorder_metadata() {
    let outcome = CompressionOutcome::new_for_tests(CompressionExecutionMode::Archive, ReadLengthClass::Short, true, 3);

    assert_eq!(outcome.mode, CompressionExecutionMode::Archive);
    assert_eq!(outcome.detected_read_length_class, ReadLengthClass::Short);
    assert!(outcome.reorder_map_written);
    assert_eq!(outcome.blocks_written, 3);
}

#[test]
fn request_carries_interleaved_input_topology() {
    let request = CompressionRequest {
        mode: CompressionExecutionMode::Streaming,
        input: CompressionInputTopology::InterleavedFile {
            input_path: "tests/data/test_se.fastq".into(),
            archive_layout: PeLayout::Consecutive,
        },
        quality_mode: QualityMode::Lossless,
        id_mode: IdMode::Exact,
        force_overwrite: true,
        ..CompressionRequest::for_tests()
    };

    assert!(matches!(
        request.input,
        CompressionInputTopology::InterleavedFile {
            archive_layout: PeLayout::Consecutive,
            ..
        }
    ));
}

// =============================================================================
// Task 2: Normalize CompressOptions into CompressionRequest
// =============================================================================

#[test]
fn compress_options_normalize_pipeline_without_enabling_streaming() {
    let opts = CompressOptions {
        input_path: "tests/data/test_se.fastq".into(),
        output_path: "target/test-output.fqc".into(),
        use_pipeline: true,
        streaming_mode: false,
        force_overwrite: true,
        show_progress: false,
        ..CompressOptions::default()
    };

    let request = opts.to_request();

    assert_eq!(request.mode, CompressionExecutionMode::Pipeline);
}

#[test]
fn stdin_normalization_stays_streaming() {
    let opts = CompressOptions {
        input_path: "-".into(),
        output_path: "target/test-output.fqc".into(),
        use_pipeline: true,
        streaming_mode: true,
        force_overwrite: true,
        show_progress: false,
        ..CompressOptions::default()
    };

    let request = opts.to_request();

    assert_eq!(request.mode, CompressionExecutionMode::Streaming);
    assert!(matches!(
        request.input,
        CompressionInputTopology::Stdin { archive_layout: None }
    ));
}

// =============================================================================
// Task 3: Route archive mode through the new orchestration module
// =============================================================================

#[test]
fn archive_execution_returns_outcome_metadata() {
    use fqc::archive::reader::FqcReader;
    use fqc::engine::compression_engine::CompressionEngine;

    let output = tempfile::NamedTempFile::new().unwrap();
    let opts = CompressOptions {
        input_path: "tests/data/test_se.fastq".into(),
        output_path: output.path().to_string_lossy().to_string(),
        force_overwrite: true,
        show_progress: false,
        ..CompressOptions::default()
    };

    // Test that CompressionEngine::run takes CompressionRequest by value
    let outcome = CompressionEngine::new().run(opts.to_request()).unwrap();

    assert_eq!(outcome.mode, CompressionExecutionMode::Archive);
    assert!(outcome.blocks_written >= 1);

    // Test that outcome carries ProcessingStats
    assert_eq!(outcome.stats.total_reads, outcome.reads_compressed);
    assert_eq!(outcome.stats.total_bases, outcome.bytes_read);
    assert_eq!(outcome.stats.output_bytes, outcome.bytes_written);
    assert_eq!(outcome.stats.blocks_written, outcome.blocks_written as u64);

    let reader = FqcReader::open(output.path().to_str().unwrap()).unwrap();
    assert_eq!(reader.block_count() as u64, outcome.blocks_written as u64);
}

// =============================================================================
// Task 4: Mode-specific behavior tests
// =============================================================================

#[test]
fn pipeline_request_reports_pipeline_mode_in_outcome() {
    let output = tempfile::NamedTempFile::new().unwrap();
    let opts = CompressOptions {
        input_path: "tests/data/test_se.fastq".into(),
        output_path: output.path().to_string_lossy().to_string(),
        use_pipeline: true,
        force_overwrite: true,
        show_progress: false,
        ..CompressOptions::default()
    };

    let outcome = CompressionEngine::new().run(opts.to_request()).unwrap();
    assert_eq!(outcome.mode, CompressionExecutionMode::Pipeline);
}

#[test]
fn streaming_request_reports_streaming_mode_in_outcome() {
    let output = tempfile::NamedTempFile::new().unwrap();
    let opts = CompressOptions {
        input_path: "tests/data/test_se.fastq".into(),
        output_path: output.path().to_string_lossy().to_string(),
        streaming_mode: true,
        force_overwrite: true,
        show_progress: false,
        ..CompressOptions::default()
    };

    let outcome = CompressionEngine::new().run(opts.to_request()).unwrap();
    assert_eq!(outcome.mode, CompressionExecutionMode::Streaming);
}

#[test]
fn pipeline_outcome_tracks_total_bases_for_summary_metrics() {
    let output = tempfile::NamedTempFile::new().unwrap();
    let opts = CompressOptions {
        input_path: "tests/data/test_se.fastq".into(),
        output_path: output.path().to_string_lossy().to_string(),
        use_pipeline: true,
        force_overwrite: true,
        show_progress: false,
        ..CompressOptions::default()
    };

    let outcome = CompressionEngine::new().run(opts.to_request()).unwrap();

    let mut parser = open_fastq("tests/data/test_se.fastq").unwrap();
    let expected_total_bases: u64 = parser
        .collect_all()
        .unwrap()
        .iter()
        .map(|record| record.sequence.len() as u64)
        .sum();

    assert_eq!(outcome.stats.total_bases, expected_total_bases);
    assert!(outcome.stats.bits_per_base() > 0.0);
}

#[test]
fn pipeline_outcome_reports_written_reorder_map() {
    let output = tempfile::NamedTempFile::new().unwrap();
    let opts = CompressOptions {
        input_path: "tests/data/test_se.fastq".into(),
        output_path: output.path().to_string_lossy().to_string(),
        use_pipeline: true,
        force_overwrite: true,
        show_progress: false,
        ..CompressOptions::default()
    };

    let outcome = CompressionEngine::new().run(opts.to_request()).unwrap();

    assert!(outcome.reorder_map_written);
}

#[cfg(unix)]
#[test]
fn archive_execution_accepts_non_utf8_output_path() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let dir = tempfile::tempdir().unwrap();
    let output_path = dir.path().join(OsString::from_vec(b"nonutf8-\xff.fqc".to_vec()));
    let request = CompressionRequest {
        mode: CompressionExecutionMode::Archive,
        input: CompressionInputTopology::SingleFile {
            input_path: "tests/data/test_se.fastq".into(),
        },
        output_path: output_path.clone(),
        quality_mode: QualityMode::Lossless,
        id_mode: IdMode::Exact,
        force_overwrite: true,
        ..CompressionRequest::for_tests()
    };

    CompressionEngine::new().run(request).unwrap();

    let bytes = std::fs::read(&output_path).unwrap();
    assert!(bytes.starts_with(&fqc::archive::format::MAGIC_BYTES));
}
