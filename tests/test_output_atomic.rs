//! Ordinary-file output transaction integration tests (FQCR-IO-001).

use fqc::commands::compress::{CompressCommand, CompressOptions};
use fqc::commands::decompress::{DecompressCommand, DecompressOptions};
use std::fs;
use std::path::PathBuf;

fn write_tiny_fastq(path: &PathBuf) {
    fs::write(
        path,
        "@r1\nACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTACGTAC\n+\nIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIIII\n",
    )
    .unwrap();
}

#[test]
fn compress_without_force_leaves_existing_archive() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.fastq");
    let output = dir.path().join("out.fqc");
    write_tiny_fastq(&input);
    fs::write(&output, b"PREEXISTING").unwrap();

    let code = CompressCommand::new(CompressOptions {
        input_path: input.to_string_lossy().into_owned(),
        output_path: output.to_string_lossy().into_owned(),
        force_overwrite: false,
        ..Default::default()
    })
    .execute();

    assert_ne!(code, 0);
    assert_eq!(fs::read(&output).unwrap(), b"PREEXISTING");
}

#[test]
fn compress_force_success_replaces_and_leaves_no_temps() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.fastq");
    let output = dir.path().join("out.fqc");
    write_tiny_fastq(&input);
    fs::write(&output, b"PREEXISTING").unwrap();

    let code = CompressCommand::new(CompressOptions {
        input_path: input.to_string_lossy().into_owned(),
        output_path: output.to_string_lossy().into_owned(),
        force_overwrite: true,
        ..Default::default()
    })
    .execute();

    assert_eq!(code, 0);
    let bytes = fs::read(&output).unwrap();
    assert_ne!(bytes, b"PREEXISTING");
    assert!(bytes.starts_with(&[0x89, b'F', b'Q', b'C']));
    let leftovers: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with(".fqc-tmp-"))
        .collect();
    assert!(leftovers.is_empty(), "leftover temps: {leftovers:?}");
}

#[test]
fn decompress_force_failure_keeps_old_fastq() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.fastq");
    let archive = dir.path().join("good.fqc");
    let bad = dir.path().join("bad.fqc");
    let output = dir.path().join("out.fastq");
    write_tiny_fastq(&input);

    assert_eq!(
        CompressCommand::new(CompressOptions {
            input_path: input.to_string_lossy().into_owned(),
            output_path: archive.to_string_lossy().into_owned(),
            force_overwrite: true,
            ..Default::default()
        })
        .execute(),
        0
    );

    // Truncate a valid archive so decode fails after the writer has opened.
    let mut bytes = fs::read(&archive).unwrap();
    bytes.truncate(bytes.len() / 2);
    fs::write(&bad, &bytes).unwrap();
    fs::write(&output, b"KEEP-ME").unwrap();

    let code = DecompressCommand::new(DecompressOptions {
        input_path: bad.to_string_lossy().into_owned(),
        output_path: output.to_string_lossy().into_owned(),
        force_overwrite: true,
        ..Default::default()
    })
    .execute();

    assert_ne!(code, 0);
    assert_eq!(fs::read(&output).unwrap(), b"KEEP-ME");
    let leftovers: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with(".fqc-tmp-"))
        .collect();
    assert!(leftovers.is_empty(), "leftover temps: {leftovers:?}");
}
