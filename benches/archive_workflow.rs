#[path = "support/archive.rs"]
mod support;

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use std::time::Duration;

fn archive_workflow(c: &mut Criterion) {
    let mut group = c.benchmark_group("archive_workflow");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(3));
    group.throughput(Throughput::Bytes(support::input_size_bytes()));
    group.bench_function("compress_roundtrip", |b| {
        b.iter(|| black_box(support::bench_compress_roundtrip("compress-roundtrip.fqc")));
    });

    group.throughput(Throughput::Bytes(support::archive_size_bytes()));
    group.bench_function("verify_archive", |b| {
        b.iter(|| black_box(support::bench_verify_archive()));
    });
    group.finish();
}

criterion_group!(benches, archive_workflow);
criterion_main!(benches);
