use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use fqc::fastq::parser::FastqParser;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

const INPUT_REPEAT_COUNT: usize = 2_048;
const INPUT_FIXTURE_NAME: &str = "benchmark-input.fastq";

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
    let path = bench_data_dir().join(INPUT_FIXTURE_NAME);
    if path.exists() {
        return path;
    }

    let seed = fs::read(test_data_dir().join("test_se.fastq")).unwrap();
    let mut expanded = Vec::with_capacity(seed.len() * INPUT_REPEAT_COUNT);
    for _ in 0..INPUT_REPEAT_COUNT {
        expanded.extend_from_slice(&seed);
    }
    fs::write(&path, expanded).unwrap();
    path
}

fn bench_data_dir() -> PathBuf {
    let dir = repo_root().join("target").join("bench-data");
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn test_data_dir() -> PathBuf {
    repo_root().join("tests").join("data")
}
