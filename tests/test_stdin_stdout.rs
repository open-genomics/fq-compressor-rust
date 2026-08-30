// =============================================================================
// fqc-rust - Stdin / Stdout Data Path Tests
// =============================================================================
// The stdin/stdout data paths were never exercised: only the request
// normalization (`to_request`) was tested. `open_fastq_stdin` reads the
// process stdin directly, so under `cargo test` stdin is typically /dev/null
// (EOF). These tests pin the real behavior:
//   - `input_path = "-"` reads from stdin (empty here) and fails gracefully
//     with the expected error, proving the stdin path executes end to end
//   - `output_path = "-"` is rejected by the output transaction layer
//   - `open_fastq_stdin` + parser handle empty stdin without panicking
// =============================================================================

use fqc::commands::compress::{CompressCommand, CompressOptions};
use fqc::commands::decompress::{DecompressCommand, DecompressOptions};
use fqc::error::ExitCode;
use fqc::types::*;

// =============================================================================
// Stdin input
// =============================================================================

#[test]
fn stdin_input_empty_fails_gracefully() {
    // Under cargo test stdin is usually /dev/null → EOF → no records.
    // Whatever the harness provides, the command must terminate cleanly with
    // the expected "no FASTQ records" usage error rather than panicking.
    let dir = tempfile::tempdir().unwrap();
    let opts = CompressOptions {
        input_path: "-".into(),
        output_path: dir.path().join("out.fqc").to_string_lossy().to_string(),
        show_progress: false,
        force_overwrite: true,
        ..Default::default()
    };
    let code = CompressCommand::new(opts).execute();
    assert_eq!(
        code,
        ExitCode::Usage as i32,
        "empty stdin must produce a usage error, got {code}"
    );
    // No partial output must be left behind.
    assert!(!dir.path().join("out.fqc").exists());
}

#[test]
fn stdin_topology_maps_to_stdin_input() {
    // Stdin input maps to the Stdin topology regardless of mode flags.
    let opts = CompressOptions {
        input_path: "-".into(),
        output_path: "unused.fqc".into(),
        streaming_mode: false,
        use_pipeline: false,
        show_progress: false,
        ..Default::default()
    };
    let request = opts.to_request();
    assert!(matches!(
        request.input,
        fqc::engine::compression_request::CompressionInputTopology::Stdin { archive_layout: None }
    ));
    assert_eq!(
        request.mode,
        fqc::engine::compression_request::CompressionExecutionMode::Archive,
        "stdin input defaults to archive mode unless streaming/pipeline requested"
    );
}

#[test]
fn open_fastq_stdin_empty_input_parses_to_zero_records() {
    // Direct unit-level check of the stdin reader path: whatever the process
    // stdin holds (usually empty in CI), the parser must not panic and must
    // return a well-formed (possibly empty) collection.
    let mut parser = fqc::fastq::parser::open_fastq_stdin();
    let records = parser.collect_all();
    assert!(records.is_ok(), "stdin parser must not error on empty input");
}

// =============================================================================
// Stdout output
// =============================================================================

#[test]
fn compress_stdout_sentinel_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let fastq = dir.path().join("in.fastq");
    let records = vec![ReadRecord::new("r1".into(), "ACGTACGT".into(), "IIIIIIII".into())];
    let mut out = std::io::BufWriter::new(std::fs::File::create(&fastq).unwrap());
    for record in &records {
        fqc::fastq::parser::write_record(&mut out, record).unwrap();
    }
    drop(out);

    // stdout sentinel is refused by the output transaction layer.
    let opts = CompressOptions {
        input_path: fastq.to_string_lossy().to_string(),
        output_path: "-".into(),
        show_progress: false,
        ..Default::default()
    };
    let code = CompressCommand::new(opts).execute();
    assert_eq!(
        code,
        ExitCode::Usage as i32,
        "stdout sentinel must be rejected with a usage error, got {code}"
    );
}

#[test]
fn decompress_to_stdout_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let fastq = dir.path().join("in.fastq");
    let archive = dir.path().join("in.fqc");
    let records = vec![ReadRecord::new("r1".into(), "ACGTACGT".into(), "IIIIIIII".into())];
    let mut out = std::io::BufWriter::new(std::fs::File::create(&fastq).unwrap());
    for record in &records {
        fqc::fastq::parser::write_record(&mut out, record).unwrap();
    }
    drop(out);

    let opts = CompressOptions {
        input_path: fastq.to_string_lossy().to_string(),
        output_path: archive.to_string_lossy().to_string(),
        show_progress: false,
        force_overwrite: true,
        ..Default::default()
    };
    assert_eq!(CompressCommand::new(opts).execute(), 0);

    // Decompress to stdout is a supported path (writes raw FASTQ to stdout).
    // In the test harness the bytes go to the test process stdout, which is
    // fine: the assertion is that the command completes successfully.
    let d_opts = DecompressOptions {
        input_path: archive.to_string_lossy().to_string(),
        output_path: "-".into(),
        range_start: 0,
        range_end: 0,
        header_only: false,
        original_order: false,
        skip_corrupted: false,
        corrupted_placeholder: None,
        split_pe: false,
        threads: 1,
        show_progress: false,
        force_overwrite: false,
        use_pipeline: false,
        memory_limit_mb: 0,
    };
    let code = DecompressCommand::new(d_opts).execute();
    assert_eq!(code, 0, "decompress to stdout must succeed, got {code}");
}

#[test]
fn decompress_split_pe_to_stdout_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let fastq = dir.path().join("in.fastq");
    let archive = dir.path().join("in.fqc");
    let records = vec![ReadRecord::new("r1".into(), "ACGTACGT".into(), "IIIIIIII".into())];
    let mut out = std::io::BufWriter::new(std::fs::File::create(&fastq).unwrap());
    for record in &records {
        fqc::fastq::parser::write_record(&mut out, record).unwrap();
    }
    drop(out);

    let opts = CompressOptions {
        input_path: fastq.to_string_lossy().to_string(),
        output_path: archive.to_string_lossy().to_string(),
        show_progress: false,
        force_overwrite: true,
        ..Default::default()
    };
    assert_eq!(CompressCommand::new(opts).execute(), 0);

    let d_opts = DecompressOptions {
        input_path: archive.to_string_lossy().to_string(),
        output_path: "-".into(),
        range_start: 0,
        range_end: 0,
        header_only: false,
        original_order: false,
        skip_corrupted: false,
        corrupted_placeholder: None,
        split_pe: true,
        threads: 1,
        show_progress: false,
        force_overwrite: false,
        use_pipeline: false,
        memory_limit_mb: 0,
    };
    let code = DecompressCommand::new(d_opts).execute();
    assert_eq!(
        code,
        ExitCode::Usage as i32,
        "--split-pe with stdout must be rejected, got {code}"
    );
}
