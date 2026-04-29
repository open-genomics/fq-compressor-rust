#[path = "../benches/support/mod.rs"]
mod support;

use fqc::algo::block_compressor::compute_block_checksum;
use std::fs;
use std::path::{Path, PathBuf};

fn unique_path(file_name: &str) -> PathBuf {
    support::bench_data_dir().join(format!("test-{}-{}", std::process::id(), file_name))
}

fn remove_if_exists(path: &Path) {
    if path.exists() {
        fs::remove_file(path).unwrap();
    }
}

#[test]
fn test_ensure_repeated_fixture_copies_seed_bytes() {
    let fixture_path = unique_path("repeated.fastq");
    remove_if_exists(&fixture_path);

    let seed = fs::read(support::test_data_dir().join("test_se.fastq")).unwrap();
    let repeated = support::ensure_repeated_fixture(&fixture_path, 3);

    assert_eq!(repeated, fixture_path);
    assert_eq!(fs::read(&repeated).unwrap(), seed.repeat(3));

    remove_if_exists(&repeated);
}

#[test]
fn test_archive_helpers_roundtrip_and_verify_fixture_reads() {
    let fixture_path = unique_path("archive.fastq");
    let archive_path = unique_path("archive.fqc");
    remove_if_exists(&fixture_path);
    remove_if_exists(&archive_path);

    let fixture = support::ensure_repeated_fixture(&fixture_path, 2);
    let records = support::load_fastq_records(&fixture);
    support::write_archive(&records, &archive_path, "archive.fastq");
    let restored = support::decompress_archive(&archive_path);

    assert_eq!(records.len(), restored.len());
    assert_eq!(compute_block_checksum(&records), compute_block_checksum(&restored));
    assert_eq!(support::verify_archive(&archive_path), records.len() as u64);

    remove_if_exists(&fixture);
    remove_if_exists(&archive_path);
}
