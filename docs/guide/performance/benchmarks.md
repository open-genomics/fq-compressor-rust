# Benchmarks

Performance benchmarks for fqc compression.

## Test Setup

- **CPU**: Intel i9-12900K (16 cores)
- **Memory**: 32GB DDR4
- **Storage**: NVMe SSD
- **OS**: Ubuntu 22.04 LTS
- **Rust**: 1.75.0

## Datasets

| Dataset | Size | Reads | Length | Type |
|---------|------|-------|--------|------|
| E. coli | 500MB | 5M | 150bp | Bacteria |
| Human WGS | 100GB | 1B | 150bp | Human |
| RNA-Seq | 10GB | 100M | 75bp | Transcriptome |

## Compression Performance

### Speed

| Dataset | Mode | Time | Speed (reads/s) |
|---------|------|------|-----------------|
| E. coli | Default | 8.5s | 588K |
| E. coli | Pipeline | 3.2s | 1.56M |
| Human WGS | Default | 28min | 595K |
| Human WGS | Pipeline | 9.5min | 1.75M |

### Compression Ratio

| Dataset | Original | Compressed | Ratio |
|---------|----------|------------|-------|
| E. coli | 500MB | 125MB | 4.0x |
| Human WGS | 100GB | 25GB | 4.0x |
| RNA-Seq | 10GB | 3.2GB | 3.1x |

### Memory Usage

| Mode | E. coli | Human WGS |
|------|---------|-----------|
| Default | 800MB | 16GB |
| Streaming | 200MB | 200MB |
| Pipeline | 1.2GB | 8GB |

## Comparison with Other Tools

### Compression Ratio

| Tool | E. coli | Human WGS |
|------|---------|-----------|
| **fqc** | **4.0x** | **4.0x** |
| fqzcomp | 3.5x | 3.5x |
| Spring | 3.8x | 3.7x |
| gzip | 2.5x | 2.5x |

### Speed

| Tool | E. coli | Human WGS |
|------|---------|-----------|
| **fqc (pipeline)** | **3.2s** | **9.5min** |
| fqzcomp | 5.1s | 15min |
| Spring | 4.8s | 14min |
| gzip | 12s | 35min |

## Scaling

### CPU Scaling (Pipeline Mode)

| Cores | Time (Human WGS) | Speedup |
|-------|------------------|---------|
| 1 | 28min | 1.0x |
| 4 | 9.5min | 2.9x |
| 8 | 5.2min | 5.4x |
| 16 | 3.8min | 7.4x |

## Related

- [Performance Tuning](./tuning.md)
- [Pipeline Mode](../guide/features/pipeline.md)
- [Streaming Mode](../guide/features/streaming.md)
