# Performance Tuning Guide

> See also: [algorithms.md](algorithms.md) (algorithm details), [architecture.md](architecture.md) (Pipeline architecture)

## Build Optimization

### Release Build (Default)

```bash
cargo build --release
```

Release profile configuration in `Cargo.toml`:

| Option | Value | Description |
|--------|-------|-------------|
| `opt-level` | 3 | Maximum optimization |
| `lto` | "fat" | Full link-time optimization |
| `codegen-units` | 1 | Single codegen unit, better optimization |
| `panic` | "abort" | Smaller binary, no unwind overhead |
| `strip` | "symbols" | Strip debug symbols |

### Native CPU Build

Maximum performance for current CPU architecture:

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

Enables CPU-specific SIMD instructions (AVX2, SSE4.2, etc.), improving compression/decompression throughput.

### Release with Debug Info

For profiling (keeps symbol table):

```bash
cargo build --profile release-with-debug
```

### Minimal Binary

Disable unwanted input formats to reduce binary size and compile time:

```bash
# Zstd only (no gz/bz2/xz input support)
cargo build --release --no-default-features

# Gzip input support only
cargo build --release --no-default-features --features gz
```

---

## Runtime Tuning

### Thread Count

fqc uses Rayon for parallel block processing:

```bash
# Specify 4 threads
fqc compress -i reads.fastq -o reads.fqc -t 4

# Use all available cores (default)
fqc compress -i reads.fastq -o reads.fqc
```

### Pipeline Mode

`--pipeline` enables 3-stage pipeline (Reader → Compressor → Writer) with crossbeam bounded channels for backpressure:

```bash
fqc compress -i reads.fastq -o reads.fqc --pipeline
fqc decompress -i reads.fqc -o reads.fastq --pipeline
```

Pipeline Mode advantages:
- I/O and computation execute in parallel
- AsyncWriter provides write-behind buffering (4MB buffer, depth 4)
- Higher throughput on NVMe/SSD storage

### Memory Control

#### Memory Budget (Automatic)

fqc has built-in memory budget system (`src/common/memory_budget.rs`):

1. **System memory detection** — Automatically gets available physical memory
2. **ChunkingStrategy** — Dynamically calculates optimal chunk size based on memory budget and read size
3. **auto_memory_budget** — Default uses 75% of available system memory

#### Manual Limit

```bash
# Limit to 4 GB
fqc compress -i large.fastq -o large.fqc --memory-limit 4096
```

### Compression Level

Higher level = better compression ratio but slower:

```bash
fqc compress -i reads.fastq -o reads.fqc -l 1   # Fast
fqc compress -i reads.fastq -o reads.fqc -l 3   # Default
fqc compress -i reads.fastq -o reads.fqc -l 9   # Maximum compression
```

### Block Size

Larger blocks improve compression ratio but increase memory usage:

```bash
fqc compress -i reads.fastq -o reads.fqc --block-size 50000
```

Default block sizes (by read length classification):

| Read Type | Default Block Size | Description |
|-----------|-------------------|-------------|
| Short (< 300bp) | 10,000 reads | ABC needs enough samples for consensus |
| Medium (300bp – 10kbp) | 1,000 reads | Balance memory and compression ratio |
| Long (> 10kbp) | 100 reads | Prevent memory explosion |

---

## Profiling

### CPU Profiling (Linux)

```bash
# Build with symbols
cargo build --profile release-with-debug

# perf sampling
perf record -g ./target/release-with-debug/fqc compress -i reads.fastq -o reads.fqc
perf report

# Flamegraph
cargo install flamegraph
flamegraph -- ./target/release-with-debug/fqc compress -i reads.fastq -o reads.fqc
```

### CPU Profiling (macOS)

```bash
cargo build --profile release-with-debug

# Instruments
xcrun xctrace record --template "Time Profiler" --launch -- \
  ./target/release-with-debug/fqc compress -i reads.fastq -o reads.fqc
```

### Memory Profiling

```bash
# Valgrind (Linux)
valgrind --tool=massif ./target/release-with-debug/fqc compress -i reads.fastq -o reads.fqc
ms_print massif.out.*

# DHAT (heap analysis)
cargo install dhat
```

---

## Benchmarking

### Quick Benchmark

```bash
# Time compression
time fqc compress -i reads.fastq -o reads.fqc

# Compare pipeline vs default mode
time fqc compress -i reads.fastq -o reads_default.fqc
time fqc compress -i reads.fastq -o reads_pipeline.fqc --pipeline

# View compression ratio
fqc info -i reads.fqc --detailed
```

### Bench Profile

`Cargo.toml` includes bench profile configuration:

```toml
[profile.bench]
inherits = "release"
debug = true
lto = "thin"
```

### Comparison with Other Tools

```bash
# Spring
time spring -c -i reads.fastq -o reads.spring

# fqzcomp
time fqzcomp reads.fastq reads.fqz

# fqc
time fqc compress -i reads.fastq -o reads.fqc

# Compare file sizes
ls -lh reads.spring reads.fqz reads.fqc
```

---

## Performance Tips

1. **Short reads** — Enable reorder (default on), significantly improves ABC compression ratio
2. **Large files** — Use `--pipeline` mode for parallel I/O and computation
3. **Memory constrained** — Reduce `--block-size` or set `--memory-limit`
4. **Maximum throughput** — Native CPU build + `--pipeline` + sufficient threads
5. **Maximum compression** — `-l 9` + `--lossy-quality illumina8` + large block size
6. **Streaming** — `--streaming` mode disables global ordering, suitable for stdin/pipes
