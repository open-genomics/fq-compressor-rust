# Pipeline Mode

Pipeline mode enables parallel 3-stage processing for maximum performance.

## Overview

Pipeline mode uses a 3-stage parallel architecture:

```
Stage 1: Read & Parse
    ↓ (crossbeam channel)
Stage 2: Compress
    ↓ (crossbeam channel)
Stage 3: Write Output
```

Each stage runs in parallel on separate CPU cores.

## Usage

Enable pipeline mode with `--pipeline`:

```bash
fqc compress input.fastq --pipeline -o output.fqc
```

## Architecture

### Stage 1: Read & Parse
- Reads FASTQ data from input
- Parses records into memory
- Handles validation

### Stage 2: Compress
- Applies compression algorithms
- ABC for short reads
- Zstd for long reads
- SCM for quality scores

### Stage 3: Write
- Serializes compressed data
- Writes to output file
- Manages block boundaries

## Performance

### Speedup

Pipeline mode typically achieves **2-4x speedup** over sequential processing:

| File Size | Sequential | Pipeline | Speedup |
|-----------|------------|----------|---------|
| 1GB | 45s | 15s | 3.0x |
| 10GB | 7.5min | 2.5min | 3.0x |
| 100GB | 75min | 25min | 3.0x |

### CPU Utilization

Pipeline mode uses multiple cores:

| Cores | Utilization |
|-------|-------------|
| 1 | 100% (single stage) |
| 4 | ~350% (3 stages active) |
| 8 | ~350% (limited by 3 stages) |

## Configuration

### Buffer Sizes

Control channel buffer sizes:

```bash
fqc compress input.fastq --pipeline --buffer-size 1000 -o output.fqc
```

Larger buffers reduce blocking but use more memory.

### Thread Count

By default, pipeline mode uses all available cores. Limit threads:

```bash
fqc compress input.fastq --pipeline --threads 4 -o output.fqc
```

## When to Use Pipeline

| Scenario | Recommended Mode |
|----------|------------------|
| Small files (< 100MB) | Default |
| Medium files (1-10GB) | Pipeline |
| Large files (> 10GB) | Pipeline |
| Single-core machine | Default |
| Multi-core machine (4+ cores) | Pipeline |
| Maximum throughput | Pipeline |

## Comparison with Other Modes

| Feature | Default | Streaming | Pipeline |
|---------|---------|-----------|----------|
| Memory | High | Low | Medium |
| Speed | Medium | Slow | **Fast** |
| Parallelism | No | No | **Yes** |
| Best for | Small files | Memory limits | Large files |

## Examples

### Full Pipeline with Options

```bash
fqc compress large.fastq \
  --pipeline \
  --threads 8 \
  --buffer-size 2000 \
  -o compressed.fqc
```

### Monitor Progress

```bash
fqc compress input.fastq --pipeline --verbose -o output.fqc
```

## Related

- [Streaming Mode](./streaming.md)
- [Performance Tuning](../performance/tuning.md)
- [Benchmarks](../performance/benchmarks.md)
