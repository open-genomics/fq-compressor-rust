# Performance Tuning Guide

## Build Optimization

### Release Build (Default)

```bash
cargo build --release
```

Configured in `Cargo.toml`:
- `opt-level = 3` — maximum optimization
- `lto = "fat"` — full link-time optimization across all crates
- `codegen-units = 1` — single codegen unit for better optimization
- `panic = "abort"` — smaller binary, no unwinding overhead
- `strip = "symbols"` — remove debug symbols from binary

### Native CPU Build

For maximum performance on your specific CPU:

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

This enables CPU-specific SIMD instructions (AVX2, SSE4.2, etc.) that can improve compression/decompression throughput.

### Release with Debug Info

For profiling with symbols:

```bash
cargo build --profile release-with-debug
```

## Runtime Performance

### Thread Count

fqc uses `rayon` for parallel block processing. Control thread count:

```bash
# Use 4 threads
fqc compress -i reads.fastq -o reads.fqc -t 4

# Use all available cores (default)
fqc compress -i reads.fastq -o reads.fqc
```

### Pipeline Mode

The `--pipeline` flag uses a 3-stage pipeline (Reader → Compressor → Writer) with backpressure via crossbeam channels. This can improve throughput for I/O-bound workloads:

```bash
fqc compress -i reads.fastq -o reads.fqc --pipeline
fqc decompress -i reads.fqc -o reads.fastq --pipeline
```

Pipeline mode benefits:
- Overlaps I/O with computation
- AsyncWriter provides write-behind buffering (4MB buffer, depth 4)
- Better throughput on NVMe/SSD storage

### Memory Limit

Control memory usage for large datasets:

```bash
# Limit to 4 GB
fqc compress -i large.fastq -o large.fqc --memory-limit 4096
```

The memory budget system auto-detects available system memory and dynamically adjusts block sizes via `ChunkingStrategy`.

### Compression Level

Higher levels = better ratio but slower:

```bash
# Fast (level 1)
fqc compress -i reads.fastq -o reads.fqc -l 1

# Default (level 3)
fqc compress -i reads.fastq -o reads.fqc

# Maximum (level 9)
fqc compress -i reads.fastq -o reads.fqc -l 9
```

### Block Size

Larger blocks improve compression ratio but increase memory usage:

```bash
fqc compress -i reads.fastq -o reads.fqc --block-size 50000
```

Default block sizes by read length class:
- Short (< 300bp): 10,000 reads/block
- Medium (300bp – 10kbp): 1,000 reads/block
- Long (> 10kbp): 100 reads/block

## Profiling

### CPU Profiling (Linux)

```bash
# Build with debug info
cargo build --profile release-with-debug

# Profile with perf
perf record -g ./target/release-with-debug/fqc compress -i reads.fastq -o reads.fqc
perf report

# Generate flamegraph
cargo install flamegraph
flamegraph -- ./target/release-with-debug/fqc compress -i reads.fastq -o reads.fqc
```

### CPU Profiling (macOS)

```bash
cargo build --profile release-with-debug

# Use Instruments
xcrun xctrace record --template "Time Profiler" --launch -- \
  ./target/release-with-debug/fqc compress -i reads.fastq -o reads.fqc
```

### Memory Profiling

```bash
# Use DHAT (heap profiler)
cargo install dhat
# Or use Valgrind on Linux:
valgrind --tool=massif ./target/release-with-debug/fqc compress -i reads.fastq -o reads.fqc
```

## Benchmarking

A bench profile is configured in `Cargo.toml`:

```toml
[profile.bench]
inherits = "release"
debug = true
lto = "thin"
```

To benchmark:

```bash
# Time a compression run
time fqc compress -i reads.fastq -o reads.fqc

# Compare pipeline vs default mode
time fqc compress -i reads.fastq -o reads_default.fqc
time fqc compress -i reads.fastq -o reads_pipeline.fqc --pipeline

# Check compression ratio
fqc info -i reads.fqc
```

## Feature Flags

Disable unused compression formats to reduce binary size and compile time:

```bash
# Only Zstd (no gz/bz2/xz input support)
cargo build --release --no-default-features

# Only gzip input support
cargo build --release --no-default-features --features gz
```
