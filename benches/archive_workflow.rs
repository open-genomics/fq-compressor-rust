use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use fqc::algo::block_compressor::compute_block_checksum;
use fqc::types::ReadRecord;

mod support;

const INPUT_REPEAT_COUNT: usize = 8;
const INPUT_FIXTURE_NAME: &str = "archive-benchmark-input.fastq";
const ARCHIVE_FIXTURE_NAME: &str = "archive-benchmark-input.fqc";

static INPUT_RECORDS: OnceLock<Vec<ReadRecord>> = OnceLock::new();
static ARCHIVE_PATH: OnceLock<PathBuf> = OnceLock::new();

fn archive_workflow(c: &mut Criterion) {
    let mut group = c.benchmark_group("archive_workflow");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));
    group.throughput(Throughput::Bytes(input_size_bytes()));
    group.bench_function("compress_roundtrip", |b| {
        b.iter(|| black_box(bench_compress_roundtrip("compress-roundtrip.fqc")));
    });

    group.throughput(Throughput::Bytes(archive_size_bytes()));
    group.bench_function("verify_archive", |b| {
        b.iter(|| black_box(bench_verify_archive()));
    });
    group.finish();
}

criterion_group!(benches, archive_workflow);
criterion_main!(benches);

fn input_size_bytes() -> u64 {
    fs::metadata(bench_input_path()).unwrap().len()
}

fn archive_size_bytes() -> u64 {
    fs::metadata(bench_archive_path()).unwrap().len()
}

fn bench_compress_roundtrip(output_name: &str) -> u64 {
    let records = input_records();
    let output_path = bench_data_dir().join(output_name);
    support::write_archive(records, &output_path, INPUT_FIXTURE_NAME);
    let restored = support::decompress_archive(&output_path);

    assert_eq!(records.len(), restored.len());
    assert_eq!(compute_block_checksum(records), compute_block_checksum(&restored));

    restored.len() as u64
}

fn bench_verify_archive() -> u64 {
    support::verify_archive(bench_archive_path())
}

fn input_records() -> &'static [ReadRecord] {
    INPUT_RECORDS
        .get_or_init(|| support::load_fastq_records(&bench_input_path()))
        .as_slice()
}

fn bench_input_path() -> PathBuf {
    support::ensure_repeated_fixture(&support::bench_data_dir().join(INPUT_FIXTURE_NAME), INPUT_REPEAT_COUNT)
}

fn bench_archive_path() -> &'static Path {
    ARCHIVE_PATH
        .get_or_init(|| {
            let archive_path = support::bench_data_dir().join(ARCHIVE_FIXTURE_NAME);
            if !archive_path.exists() {
                support::write_archive(input_records(), &archive_path, INPUT_FIXTURE_NAME);
            }
            archive_path
        })
        .as_path()
}

fn bench_data_dir() -> PathBuf {
    support::bench_data_dir()
}
