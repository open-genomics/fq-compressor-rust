//! Cross-family magic recognition (FQC-FAMILY-001 / recognize-sequential-fqc-family).

use fqc::archive::format::{classify_magic, MagicFamily, MAGIC_BYTES, SEQUENTIAL_MAGIC_BYTES};
use fqc::archive::reader::FqcReader;
use fqc::commands::decompress::{DecompressCommand, DecompressOptions};
use fqc::commands::info::{InfoCommand, InfoOptions};
use fqc::commands::verify::{VerifyCommand, VerifyOptions};
use fqc::error::{ExitCode, FqcError};
use std::path::{Path, PathBuf};

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures")
}

fn sequential_fixture() -> PathBuf {
    fixture_dir().join("foreign-sequential-v2").join("frozen_se.fqc")
}

fn indexed_fixture() -> PathBuf {
    fixture_dir().join("indexed-v2").join("frozen.fqc")
}

fn assert_sequential_family_reject(err: FqcError) {
    match err {
        FqcError::UnsupportedFormat(msg) => {
            assert!(
                msg.contains("unsupported FQC format family")
                    && msg.contains("fqc-sequential/v2")
                    && msg.contains("open-genomics/fq-compressor"),
                "unexpected message: {msg}"
            );
        }
        other => panic!("expected UnsupportedFormat family reject, got {other}"),
    }
}

#[test]
fn classify_magic_distinguishes_families() {
    assert_eq!(classify_magic(&MAGIC_BYTES), MagicFamily::Indexed);
    assert_eq!(classify_magic(&SEQUENTIAL_MAGIC_BYTES), MagicFamily::Sequential);
    assert_eq!(classify_magic(&[0u8; 8]), MagicFamily::Unknown);
}

#[test]
fn accepts_own_indexed_frozen_fixture() {
    FqcReader::open(indexed_fixture().to_str().unwrap()).unwrap();
}

#[test]
fn rejects_sequential_frozen_fixture_as_known_family() {
    let err = match FqcReader::open(sequential_fixture().to_str().unwrap()) {
        Ok(_) => panic!("sequential archive must not open"),
        Err(e) => e,
    };
    assert_sequential_family_reject(err);
}

#[test]
fn rejects_unknown_magic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("unknown.fqc");
    std::fs::write(&path, b"NOTAFQC!\x00\x00\x00\x00").unwrap();
    let err = match FqcReader::open(path.to_str().unwrap()) {
        Ok(_) => panic!("unknown magic must not open"),
        Err(e) => e,
    };
    match err {
        FqcError::Format(msg) => {
            assert!(msg.contains("unknown FQC magic"), "unexpected message: {msg}");
        }
        other => panic!("expected Format unknown magic, got {other}"),
    }
}

#[test]
fn rejects_truncated_magic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("trunc.fqc");
    std::fs::write(&path, &MAGIC_BYTES[..5]).unwrap();
    let err = match FqcReader::open(path.to_str().unwrap()) {
        Ok(_) => panic!("truncated magic must not open"),
        Err(e) => e,
    };
    match err {
        FqcError::Format(msg) => {
            assert!(msg.contains("truncated"), "unexpected message: {msg}");
        }
        other => panic!("expected Format truncated, got {other}"),
    }
}

#[test]
fn info_verify_decompress_reject_sequential_without_output() {
    let seq = sequential_fixture().to_string_lossy().into_owned();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out.fastq");

    let info_code = InfoCommand::new(InfoOptions {
        input_path: seq.clone(),
        ..Default::default()
    })
    .execute();
    assert_eq!(info_code, ExitCode::UnsupportedError as i32);

    let verify_code = VerifyCommand::new(VerifyOptions {
        input_path: seq.clone(),
        ..Default::default()
    })
    .execute();
    assert_eq!(verify_code, ExitCode::UnsupportedError as i32);

    let decompress_code = DecompressCommand::new(DecompressOptions {
        input_path: seq,
        output_path: out.to_string_lossy().into_owned(),
        force_overwrite: true,
        show_progress: false,
        ..Default::default()
    })
    .execute();
    assert_eq!(decompress_code, ExitCode::UnsupportedError as i32);
    assert!(!out.exists());
}
