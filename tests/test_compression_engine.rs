// =============================================================================
// fqc-rust - Compression Engine Interface Tests
// =============================================================================

use fqc::commands::compress::{CompressCommand, CompressOptions};
use fqc::engine::compression_engine::{CompressionEngine, CompressionOutcome};
use fqc::fastq::parser::open_fastq;
use fqc::plan::compress::{CompressionInputTopology, CompressionRequest, CompressionSettings};
use fqc::plan::CompressionExecutionMode;
use fqc::types::{IdMode, PeLayout, QualityMode, ReadLengthClass, ReadRecord};

// =============================================================================
// Task 1: CompressionRequest and CompressionEngine interface tests
// =============================================================================

#[test]
fn request_keeps_pipeline_and_streaming_distinct() {
    let request = CompressionRequest::new(
        CompressionExecutionMode::Pipeline,
        CompressionInputTopology::SingleFile {
            input_path: "tests/data/test_se.fastq".into(),
        },
        "tests/output/test.fqc",
    )
    .with_settings(CompressionSettings {
        quality_mode: QualityMode::Lossless,
        id_mode: IdMode::Exact,
        requested_read_length_class: None,
        force_overwrite: true,
        ..Default::default()
    });

    assert_eq!(request.mode(), CompressionExecutionMode::Pipeline);
}

#[test]
fn outcome_exposes_mode_length_class_and_reorder_metadata() {
    let outcome = CompressionOutcome {
        mode: CompressionExecutionMode::Archive,
        detected_read_length_class: ReadLengthClass::Short,
        stats: fqc::stats::OperationStats {
            reorder_map_written: true,
            blocks: 3,
            ..Default::default()
        },
    };

    assert_eq!(outcome.mode, CompressionExecutionMode::Archive);
    assert_eq!(outcome.detected_read_length_class, ReadLengthClass::Short);
    assert!(outcome.stats.reorder_map_written);
    assert_eq!(outcome.stats.blocks, 3);
}

#[test]
fn request_carries_interleaved_input_topology() {
    let request = CompressionRequest::new(
        CompressionExecutionMode::Streaming,
        CompressionInputTopology::InterleavedFile {
            input_path: "tests/data/test_se.fastq".into(),
            archive_layout: PeLayout::Consecutive,
        },
        "tests/output/test.fqc",
    )
    .with_settings(CompressionSettings {
        quality_mode: QualityMode::Lossless,
        id_mode: IdMode::Exact,
        force_overwrite: true,
        ..Default::default()
    });

    assert!(matches!(
        request.input(),
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

    assert_eq!(request.mode(), CompressionExecutionMode::Pipeline);
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

    assert_eq!(request.mode(), CompressionExecutionMode::Streaming);
    assert!(matches!(
        request.input(),
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
    assert!(outcome.stats.blocks >= 1);

    let input_bytes = std::fs::metadata("tests/data/test_se.fastq").unwrap().len();
    let output_bytes = std::fs::metadata(output.path()).unwrap().len();
    assert_eq!(outcome.stats.input_bytes, input_bytes);
    assert_eq!(outcome.stats.output_bytes, output_bytes);
    assert!(outcome.stats.total_bases > 0);
    assert!(outcome.stats.compression_ratio() > 0.0);

    let reader = FqcReader::open(output.path().to_str().unwrap()).unwrap();
    assert_eq!(reader.block_count() as u64, outcome.stats.blocks);
}

// =============================================================================
// Task 4: Mode-specific behavior tests
// =============================================================================

#[test]
fn compress_command_rejects_oversized_block_reads() {
    let block_size = 1_000_001;
    let output = tempfile::NamedTempFile::new().unwrap();
    let code = CompressCommand::new(CompressOptions {
        input_path: "tests/data/test_se.fastq".into(),
        output_path: output.path().to_string_lossy().into_owned(),
        block_size,
        force_overwrite: true,
        show_progress: false,
        ..CompressOptions::default()
    })
    .execute();
    assert_eq!(code, 1, "block_size={block_size} should be rejected");
}

#[test]
fn archive_and_pipeline_share_auto_block_plan() {
    use fqc::archive::reader::FqcReader;
    use std::io::Write as _;

    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.fastq");
    let archive = dir.path().join("archive.fqc");
    let pipeline = dir.path().join("pipeline.fqc");

    let mut writer = std::io::BufWriter::new(std::fs::File::create(&input).unwrap());
    for i in 0..50_000 {
        let seq: String = (0..150).map(|j| b"ACGT"[(i * 3 + j * 7) % 4] as char).collect();
        fqc::fastq::parser::write_record(&mut writer, &ReadRecord::new(format!("read_{i}"), seq, "I".repeat(150)))
            .unwrap();
    }
    writer.flush().unwrap();

    let base = CompressOptions {
        input_path: input.to_string_lossy().into_owned(),
        threads: 4,
        reorder: fqc::plan::ReorderMode::Never,
        show_progress: false,
        force_overwrite: true,
        ..CompressOptions::default()
    };

    let archive_outcome = CompressionEngine::new()
        .run(
            CompressOptions {
                output_path: archive.to_string_lossy().into_owned(),
                ..base.clone()
            }
            .to_request(),
        )
        .unwrap();
    let pipeline_outcome = CompressionEngine::new()
        .run(
            CompressOptions {
                output_path: pipeline.to_string_lossy().into_owned(),
                use_pipeline: true,
                ..base
            }
            .to_request(),
        )
        .unwrap();

    assert_eq!(archive_outcome.stats.blocks, pipeline_outcome.stats.blocks);
    assert!(archive_outcome.stats.blocks > 1);
    let archive_reader = FqcReader::open(archive.to_str().unwrap()).unwrap();
    let pipeline_reader = FqcReader::open(pipeline.to_str().unwrap()).unwrap();
    assert_eq!(archive_reader.block_count() as u64, archive_outcome.stats.blocks);
    assert_eq!(pipeline_reader.block_count() as u64, pipeline_outcome.stats.blocks);
    assert_eq!(archive_reader.block_count(), pipeline_reader.block_count());
}

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
        reorder: fqc::plan::ReorderMode::Always,
        force_overwrite: true,
        show_progress: false,
        ..CompressOptions::default()
    };

    let outcome = CompressionEngine::new().run(opts.to_request()).unwrap();

    assert!(outcome.stats.reorder_map_written);
}

#[cfg(unix)]
#[test]
fn archive_execution_accepts_non_utf8_output_path() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let dir = tempfile::tempdir().unwrap();
    let output_path = dir.path().join(OsString::from_vec(b"nonutf8-\xff.fqc".to_vec()));
    let request = CompressionRequest::new(
        CompressionExecutionMode::Archive,
        CompressionInputTopology::SingleFile {
            input_path: "tests/data/test_se.fastq".into(),
        },
        output_path.clone(),
    )
    .with_settings(CompressionSettings {
        quality_mode: QualityMode::Lossless,
        id_mode: IdMode::Exact,
        force_overwrite: true,
        ..Default::default()
    });

    CompressionEngine::new().run(request).unwrap();

    let bytes = std::fs::read(&output_path).unwrap();
    assert!(bytes.starts_with(&fqc::archive::format::MAGIC_BYTES));
}

#[test]
fn archive_and_pipeline_classify_mixed_length_tail_identically() {
    use std::io::Write as _;

    // 4,100 short reads followed by 100 long reads: a prefix-only scan sees
    // only short reads, while an even sample across the file sees the long
    // tail. Both modes must classify from the complete ingest and agree on
    // the block plan.
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("mixed.fastq");
    let archive = dir.path().join("archive.fqc");
    let pipeline = dir.path().join("pipeline.fqc");

    let mut writer = std::io::BufWriter::new(std::fs::File::create(&input).unwrap());
    for i in 0..4_100 {
        let record = ReadRecord::new(format!("short_{i}"), "A".repeat(100), "I".repeat(100));
        fqc::fastq::parser::write_record(&mut writer, &record).unwrap();
    }
    for i in 0..100 {
        let record = ReadRecord::new(format!("long_{i}"), "ACGT".repeat(1_250), "I".repeat(5_000));
        fqc::fastq::parser::write_record(&mut writer, &record).unwrap();
    }
    writer.flush().unwrap();

    let base = CompressOptions {
        input_path: input.to_string_lossy().into_owned(),
        threads: 2,
        reorder: fqc::plan::ReorderMode::Never,
        show_progress: false,
        force_overwrite: true,
        ..CompressOptions::default()
    };

    let archive_outcome = CompressionEngine::new()
        .run(
            CompressOptions {
                output_path: archive.to_string_lossy().into_owned(),
                ..base.clone()
            }
            .to_request(),
        )
        .unwrap();
    let pipeline_outcome = CompressionEngine::new()
        .run(
            CompressOptions {
                output_path: pipeline.to_string_lossy().into_owned(),
                use_pipeline: true,
                ..base
            }
            .to_request(),
        )
        .unwrap();

    assert_eq!(
        archive_outcome.detected_read_length_class,
        ReadLengthClass::Medium,
        "the long tail must be visible to the archive classifier"
    );
    assert_eq!(
        pipeline_outcome.detected_read_length_class,
        archive_outcome.detected_read_length_class
    );
    assert_eq!(pipeline_outcome.stats.blocks, archive_outcome.stats.blocks);
    assert_eq!(
        std::fs::metadata(&pipeline).unwrap().len(),
        std::fs::metadata(&archive).unwrap().len(),
        "same class and settings must produce the same archive layout"
    );
}
