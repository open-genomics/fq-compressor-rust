//! Stable facade tests: the embedding surface must validate once and keep the
//! same on-disk behavior as the CLI paths.

use fqc::api::{
    self, CompressRequest, CompressSettings, CompressionExecutionMode, CompressionInputTopology, DecompressRequest,
    DecompressSettings, ReorderMode,
};
use std::fs;

fn write_fastq(path: &std::path::Path, n: usize) {
    use std::fmt::Write as _;
    let mut out = String::new();
    for i in 0..n {
        writeln!(out, "@read{i}\nACGTACGTACGTACGTACGT\n+\nIIIIIIIIIIIIIIIIIIII").unwrap();
    }
    fs::write(path, out).unwrap();
}

#[test]
fn facade_round_trip_info_and_verify() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.fastq");
    let archive = dir.path().join("out.fqc");
    let restored = dir.path().join("restored.fastq");
    write_fastq(&input, 40);

    let request = CompressRequest::new(
        CompressionExecutionMode::Archive,
        CompressionInputTopology::SingleFile {
            input_path: input.clone(),
        },
        &archive,
    )
    .with_settings(CompressSettings {
        reorder: ReorderMode::Never,
        force_overwrite: true,
        ..Default::default()
    });
    let outcome = api::compress(request).unwrap();
    assert_eq!(outcome.stats.total_reads, 40);
    assert_eq!(outcome.stats.output_bytes, fs::metadata(&archive).unwrap().len());

    let info = api::info(&archive).unwrap();
    assert_eq!(info.total_reads, 40);
    assert!(info.num_blocks >= 1);

    let report = api::verify(&api::VerifyOptions {
        input_path: archive.clone(),
        ..Default::default()
    })
    .unwrap();
    assert!(report.passed);

    let stats = api::decompress(
        DecompressRequest::new(&archive, &restored).with_settings(DecompressSettings {
            force_overwrite: true,
            ..Default::default()
        }),
    )
    .unwrap();
    assert_eq!(stats.total_reads, 40);
    assert_eq!(stats.output_bytes, fs::metadata(&restored).unwrap().len());
    assert_eq!(fs::read(&input).unwrap(), fs::read(&restored).unwrap());
}

#[test]
fn facade_rejects_invalid_request_before_writing() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.fastq");
    let archive = dir.path().join("out.fqc");
    write_fastq(&input, 4);

    let request = CompressRequest::new(
        CompressionExecutionMode::Archive,
        CompressionInputTopology::SingleFile {
            input_path: input.clone(),
        },
        &archive,
    )
    .with_settings(CompressSettings {
        level: 0,
        force_overwrite: true,
        ..Default::default()
    });
    assert!(api::compress(request).is_err());
    assert!(!archive.exists(), "validation must fail before creating output");
}

#[test]
fn facade_rejects_split_pe_to_stdout() {
    let request = DecompressRequest::new("in.fqc", "-").with_settings(DecompressSettings {
        split_pe: true,
        ..Default::default()
    });
    assert!(api::decompress(request).is_err());
}

#[cfg(unix)]
#[test]
fn facade_preserves_non_utf8_paths() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join(OsString::from_vec(b"in\xff.fastq".to_vec()));
    let archive = dir.path().join(OsString::from_vec(b"out\xff.fqc".to_vec()));
    let restored = dir.path().join(OsString::from_vec(b"restored\xff.fastq".to_vec()));
    write_fastq(&input, 40);

    let outcome = api::compress(
        CompressRequest::new(
            CompressionExecutionMode::Archive,
            CompressionInputTopology::SingleFile {
                input_path: input.clone(),
            },
            archive.clone(),
        )
        .with_settings(CompressSettings {
            reorder: ReorderMode::Never,
            force_overwrite: true,
            ..Default::default()
        }),
    )
    .unwrap();
    assert_eq!(outcome.stats.input_bytes, fs::metadata(&input).unwrap().len());

    let info = api::info(&archive).unwrap();
    assert_eq!(info.file_path, archive, "facade must not lossy-convert paths");

    let stats = api::decompress(
        DecompressRequest::new(archive, restored.clone()).with_settings(DecompressSettings {
            use_pipeline: true,
            force_overwrite: true,
            ..Default::default()
        }),
    )
    .unwrap();
    assert_eq!(stats.output_bytes, fs::metadata(&restored).unwrap().len());
    assert_eq!(fs::read(&input).unwrap(), fs::read(&restored).unwrap());
}
