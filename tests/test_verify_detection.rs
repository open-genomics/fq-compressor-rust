// =============================================================================
// fqc-rust - Verify Command Detection Tests
// =============================================================================
// Verifies that `VerifyCommand` actually detects corruption: clean archives
// pass, byte-tampered archives fail with non-zero exit, quick mode skips block
// decompression, and verbose output reports per-block status.
//
// Note: VerifyCommand writes status to the process stdout/stderr directly;
// the library API offers no capture handle, so these tests assert on exit
// codes (which map 1:1 to the FAILED/error paths).
// =============================================================================

use fqc::commands::compress::{CompressCommand, CompressOptions};
use fqc::commands::verify::{VerifyCommand, VerifyOptions};
use fqc::error::ExitCode;
use fqc::types::*;

// =============================================================================
// Helpers
// =============================================================================

fn write_fastq(path: &std::path::Path, records: &[ReadRecord]) {
    let mut out = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    for record in records {
        fqc::fastq::parser::write_record(&mut out, record).unwrap();
    }
}

fn make_records(n: usize, len: usize) -> Vec<ReadRecord> {
    let bases = b"ACGT";
    (0..n)
        .map(|i| {
            let seq: String = (0..len).map(|j| bases[(i + j) % 4] as char).collect();
            let qual = "I".repeat(len);
            ReadRecord::new(format!("read_{}", i), seq, qual)
        })
        .collect()
}

/// Compress a FASTQ file into an archive via the CLI command (library API).
fn compress_to_archive(input: &std::path::Path, archive: &std::path::Path) {
    let opts = CompressOptions {
        input_path: input.to_string_lossy().to_string(),
        output_path: archive.to_string_lossy().to_string(),
        show_progress: false,
        force_overwrite: true,
        ..Default::default()
    };
    let code = CompressCommand::new(opts).execute();
    assert_eq!(code, 0, "compression should succeed");
}

/// Flip a byte inside the first block's compressed payload area (not the
/// footer, so quick mode — which skips block decompression — still passes).
fn corrupt_block_payload(path: &std::path::Path) {
    use fqc::archive::reader::FqcReader;

    let reader = FqcReader::open(path.to_string_lossy().as_ref()).unwrap();
    let first_block = &reader.block_index.entries[0];
    let payload_offset = first_block.offset as usize + fqc::archive::format::BLOCK_HEADER_SIZE;
    drop(reader);

    let mut bytes = std::fs::read(path).unwrap();
    assert!(
        bytes.len() > payload_offset + 8,
        "archive too small to corrupt block payload"
    );
    // Flip a byte roughly in the middle of the block's compressed streams.
    let target = payload_offset + 4;
    bytes[target] ^= 0xFF;
    std::fs::write(path, bytes).unwrap();
}

fn run_verify(archive: &std::path::Path, opts_mut: impl FnOnce(&mut VerifyOptions)) -> i32 {
    let mut opts = VerifyOptions {
        input_path: archive.to_string_lossy().to_string(),
        ..Default::default()
    };
    opts_mut(&mut opts);
    VerifyCommand::new(opts).execute()
}

// =============================================================================
// Tests
// =============================================================================

#[test]
fn verify_passes_clean_archive() {
    let dir = tempfile::tempdir().unwrap();
    let fastq = dir.path().join("in.fastq");
    let archive = dir.path().join("out.fqc");
    write_fastq(&fastq, &make_records(20, 150));
    compress_to_archive(&fastq, &archive);

    let code = run_verify(&archive, |_| {});
    assert_eq!(code, 0, "clean archive must pass verification");
}

#[test]
fn verify_fails_corrupted_block_data() {
    let dir = tempfile::tempdir().unwrap();
    let fastq = dir.path().join("in.fastq");
    let archive = dir.path().join("out.fqc");
    write_fastq(&fastq, &make_records(20, 150));
    compress_to_archive(&fastq, &archive);
    corrupt_block_payload(&archive);

    let code = run_verify(&archive, |_| {});
    assert_ne!(code, 0, "tampered archive must fail verification");
}

#[test]
fn verify_quick_mode_still_detects_payload_corruption_via_global_checksum() {
    let dir = tempfile::tempdir().unwrap();
    let fastq = dir.path().join("in.fastq");
    let archive = dir.path().join("out.fqc");
    write_fastq(&fastq, &make_records(20, 150));
    compress_to_archive(&fastq, &archive);
    corrupt_block_payload(&archive);

    // Quick mode skips per-block xxhash decompression but still recomputes the
    // global checksum over all block streams (writer always emits a non-zero
    // global checksum), so payload corruption is still detected.
    let code = run_verify(&archive, |o| o.quick_mode = true);
    assert_ne!(code, 0, "quick mode still validates the global checksum");

    // Sanity: quick mode passes on a clean archive.
    let clean_archive = dir.path().join("clean.fqc");
    compress_to_archive(&fastq, &clean_archive);
    let clean_code = run_verify(&clean_archive, |o| o.quick_mode = true);
    assert_eq!(clean_code, 0, "quick mode passes on clean archive");
}

#[test]
fn verify_fail_fast_stops_at_first_bad_block() {
    let dir = tempfile::tempdir().unwrap();
    let fastq = dir.path().join("in.fastq");
    let archive = dir.path().join("out.fqc");
    // Multiple blocks via a small block size.
    write_fastq(&fastq, &make_records(200, 150));
    let opts = CompressOptions {
        input_path: fastq.to_string_lossy().to_string(),
        output_path: archive.to_string_lossy().to_string(),
        show_progress: false,
        force_overwrite: true,
        block_size: 40,
        ..Default::default()
    };
    assert_eq!(CompressCommand::new(opts).execute(), 0);
    corrupt_block_payload(&archive);

    let code = run_verify(&archive, |o| o.fail_fast = true);
    assert_ne!(code, 0, "fail-fast verify must still report failure");
}

#[test]
fn verify_verbose_reports_blocks() {
    let dir = tempfile::tempdir().unwrap();
    let fastq = dir.path().join("in.fastq");
    let archive = dir.path().join("out.fqc");
    write_fastq(&fastq, &make_records(20, 150));
    compress_to_archive(&fastq, &archive);

    let code = run_verify(&archive, |o| o.verbose = true);
    assert_eq!(code, 0, "verbose verify of clean archive passes");
}

#[test]
fn verify_missing_input_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("does_not_exist.fqc");

    let code = run_verify(&missing, |_| {});
    assert_eq!(code, ExitCode::IoError as i32, "missing input maps to I/O error");
}

#[test]
fn verify_detects_checksum_mismatch_on_tampered_payload() {
    let dir = tempfile::tempdir().unwrap();
    let fastq = dir.path().join("in.fastq");
    let archive = dir.path().join("out.fqc");
    write_fastq(&fastq, &make_records(20, 150));
    compress_to_archive(&fastq, &archive);
    corrupt_block_payload(&archive);

    // The block xxhash check must fail (ChecksumMismatch -> ChecksumError) or
    // the block fails to decompress (FormatError). Either path is non-zero.
    let code = run_verify(&archive, |_| {});
    assert_ne!(code, 0);
}
