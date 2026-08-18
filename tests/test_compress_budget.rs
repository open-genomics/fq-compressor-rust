//! Archive compress resource budget tests (FQCR-LIMIT-002).

use fqc::commands::compress::{CompressCommand, CompressOptions};
use fqc::error::FqcError;
use fqc::memory_budget::{
    check_archive_ingest, estimate_archive_ingest_bytes, resolve_compress_limit_mb, HARD_MAX_COMPRESS_MEMORY_MB,
    MIN_COMPRESS_MEMORY_MB,
};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn write_reads(path: &Path, n_reads: usize, seq_len: usize) {
    let seq = "A".repeat(seq_len);
    let qual = "I".repeat(seq_len);
    let mut out = String::with_capacity(n_reads * (seq_len * 2 + 32));
    for i in 0..n_reads {
        write!(out, "@r{i}\n{seq}\n+\n{qual}\n").unwrap();
    }
    fs::write(path, out).unwrap();
}

fn compress(input: &Path, output: &Path, memory_limit_mb: usize, streaming: bool) -> i32 {
    compress_with(input, output, memory_limit_mb, streaming, false)
}

fn compress_with(input: &Path, output: &Path, memory_limit_mb: usize, streaming: bool, pipeline: bool) -> i32 {
    CompressCommand::new(CompressOptions {
        input_path: input.to_string_lossy().into_owned(),
        output_path: output.to_string_lossy().into_owned(),
        memory_limit_mb,
        streaming_mode: streaming,
        use_pipeline: pipeline,
        enable_reorder: !streaming,
        force_overwrite: true,
        show_progress: false,
        ..Default::default()
    })
    .execute()
}

#[test]
fn resolve_compress_limit_mb_zero_is_finite() {
    let mb = resolve_compress_limit_mb(0);
    assert!(mb >= MIN_COMPRESS_MEMORY_MB);
    assert!(mb <= HARD_MAX_COMPRESS_MEMORY_MB);
}

#[test]
fn check_archive_ingest_rejects_peak_over_explicit_limit() {
    let err = check_archive_ingest(2_000_000, 150, MIN_COMPRESS_MEMORY_MB).unwrap_err();
    match err {
        FqcError::ResourceLimit {
            location,
            declared,
            allowed,
        } => {
            assert!(location.contains("archive ingest"));
            assert!(declared > allowed);
        }
        other => panic!("expected ResourceLimit, got {other}"),
    }
}

#[test]
fn check_archive_ingest_allows_tiny_under_auto() {
    check_archive_ingest(20, 150, 0).unwrap();
}

#[test]
fn estimate_is_monotonic_in_reads_and_length() {
    let a = estimate_archive_ingest_bytes(100, 100);
    let b = estimate_archive_ingest_bytes(200, 100);
    let c = estimate_archive_ingest_bytes(100, 200);
    assert!(b > a);
    assert!(c > a);
}

#[test]
fn archive_compress_accepts_tiny_fixture_under_auto_and_min() {
    let input = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/test_se.fastq");
    let dir = tempfile::tempdir().unwrap();

    let auto_out = dir.path().join("auto.fqc");
    assert_eq!(compress(&input, &auto_out, 0, false), 0);
    assert!(auto_out.exists());

    let min_out = dir.path().join("min.fqc");
    assert_eq!(compress(&input, &min_out, MIN_COMPRESS_MEMORY_MB, false), 0);
    assert!(min_out.exists());
}

#[test]
fn archive_compress_rejects_over_budget_before_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("big.fastq");
    let output = dir.path().join("out.fqc");
    // Peak factor 2: 600 reads × ~20 KiB record ≈ 12 MiB held → ~24 MiB peak > 16 MiB.
    write_reads(&input, 600, 10_000);

    let code = compress(&input, &output, MIN_COMPRESS_MEMORY_MB, false);
    assert_ne!(code, 0);
    assert!(!output.exists());
}

#[test]
fn streaming_compress_accepts_input_that_archive_rejects() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("big.fastq");
    let output = dir.path().join("out.fqc");
    write_reads(&input, 600, 10_000);

    assert_ne!(compress(&input, &output, MIN_COMPRESS_MEMORY_MB, false), 0);
    let code = compress(&input, &output, MIN_COMPRESS_MEMORY_MB, true);
    assert_eq!(code, 0);
    assert!(output.exists());
}

#[test]
fn pipeline_compress_rejects_over_budget_before_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("big.fastq");
    let output = dir.path().join("out.fqc");
    write_reads(&input, 600, 10_000);

    let code = compress_with(&input, &output, MIN_COMPRESS_MEMORY_MB, false, true);
    assert_ne!(code, 0);
    assert!(!output.exists());
}

#[test]
fn pipeline_compress_accepts_tiny_fixture_under_min() {
    let input = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/test_se.fastq");
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("out.fqc");
    assert_eq!(compress_with(&input, &output, MIN_COMPRESS_MEMORY_MB, false, true), 0);
    assert!(output.exists());
}
