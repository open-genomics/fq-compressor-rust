use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use fqc::fastq::parser::FastqParser;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

mod support;

const INPUT_REPEAT_COUNT: usize = 2_048;
const INPUT_FIXTURE_NAME: &str = "parser-benchmark-input.fastq";

static INPUT_BYTES: OnceLock<Vec<u8>> = OnceLock::new();

fn parser_throughput(c: &mut Criterion) {
    let input = load_input_bytes();
    let mut group = c.benchmark_group("parser");
    group.sample_size(20);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));
    group.throughput(Throughput::Bytes(input.len() as u64));
    group.bench_function("parse_repeated_test_fixture", |b| {
        b.iter(|| {
            let mut total_bases = 0usize;
            let mut parser = FastqParser::new(black_box(input));
            let total_reads = parser
                .for_each(|record| {
                    total_bases += record.sequence.len();
                    Ok::<(), fqc::error::FqcError>(())
                })
                .unwrap();
            black_box((total_reads, total_bases));
        });
    });
    group.finish();
}

criterion_group!(benches, parser_throughput);
criterion_main!(benches);

fn load_input_bytes() -> &'static [u8] {
    INPUT_BYTES
        .get_or_init(|| fs::read(bench_input_path()).unwrap())
        .as_slice()
}

fn bench_input_path() -> PathBuf {
    support::ensure_repeated_fixture(&bench_data_dir().join(INPUT_FIXTURE_NAME), INPUT_REPEAT_COUNT)
}

fn bench_data_dir() -> PathBuf {
    support::bench_data_dir()
}
