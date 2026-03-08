# Performance

> See also: [Algorithms](algorithms.md), [Pipeline](pipeline.md)

## Build Optimization

### Release Build

```bash
cargo build --release
```

Release profile settings in `Cargo.toml`:

| Option | Value | Description |
|--------|-------|-------------|
| `opt-level` | 3 | Maximum optimization |
| `lto` | "fat" | Full link-time optimization |
| `codegen-units` | 1 | Single codegen unit for better optimization |
| `panic` | "abort" | Smaller binary, no unwind overhead |
| `strip` | "symbols" | Strip debug symbols |

### Native CPU Build

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

Enables CPU-specific SIMD instructions (AVX2, SSE4.2, etc.) for improved throughput.

## Runtime Tuning

### Thread Count

```bash
fqc compress -i reads.fastq -o reads.fqc -t 4    # 4 threads
fqc compress -i reads.fastq -o reads.fqc          # all cores (default)
```

### Pipeline Mode

```bash
fqc compress -i reads.fastq -o reads.fqc --pipeline
```

Benefits: I/O and computation overlap, write-behind buffering, higher throughput on fast storage.

### Memory Control

fqc includes an automatic memory budget system:

1. **System memory detection** — auto-detect available physical memory
2. **ChunkingStrategy** — dynamically compute optimal block size based on memory budget and read size
3. **auto_memory_budget** — defaults to 75% of available system memory

```bash
# Manual limit (4 GB)
fqc compress -i large.fastq -o large.fqc --memory-limit 4096
```

### Compression Level

```bash
fqc compress -i reads.fastq -o reads.fqc -l 1   # fast
fqc compress -i reads.fastq -o reads.fqc -l 3   # default
fqc compress -i reads.fastq -o reads.fqc -l 9   # max compression
```

### Block Size

| Read Type | Default Block Size | Notes |
|-----------|-------------------|-------|
| Short (< 300bp) | 10,000 reads | ABC needs sufficient samples for consensus |
| Medium (300bp – 10kbp) | 1,000 reads | Balance memory and ratio |
| Long (> 10kbp) | 100 reads | Avoid memory spikes |

## Profiling

### CPU Profiling (Linux)

```bash
cargo build --profile release-with-debug
perf record -g ./target/release-with-debug/fqc compress -i reads.fastq -o reads.fqc
perf report

# Flame graph
cargo install flamegraph
flamegraph -- ./target/release-with-debug/fqc compress -i reads.fastq -o reads.fqc
```

### CPU Profiling (macOS)

```bash
cargo build --profile release-with-debug
xcrun xctrace record --template "Time Profiler" --launch -- \
  ./target/release-with-debug/fqc compress -i reads.fastq -o reads.fqc
```

## Tips

1. **Short reads** — Enable reorder (default), significantly improves ABC ratio
2. **Large files** — Use `--pipeline` mode for I/O overlap
3. **Memory constrained** — Reduce `--block-size` or set `--memory-limit`
4. **Max throughput** — Native CPU build + `--pipeline` + sufficient threads
5. **Max compression** — `-l 9` + `--lossy-quality illumina8` + large block size
6. **Streaming** — `--streaming` disables global reorder, suitable for stdin/pipes
