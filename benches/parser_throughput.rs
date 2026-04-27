#[path = "support/parser.rs"]
mod support;

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use fqc::fastq::parser::FastqParser;
use std::time::Duration;

fn parser_throughput(c: &mut Criterion) {
    let input = support::load_input_bytes();
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
