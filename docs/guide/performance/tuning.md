# Performance Tuning

Optimize fqc for your specific use case.

## Compression Speed vs Ratio

### Maximum Speed

```bash
fqc compress input.fastq --pipeline --threads 16 --level 3 -o fast.fqc
```

- Level 3: Fast compression
- Pipeline: Parallel processing
- Threads: Use all cores

### Balanced

```bash
fqc compress input.fastq --pipeline --level 6 -o balanced.fqc
```

- Level 6: Default (balanced)
- Good ratio with reasonable speed

### Maximum Compression

```bash
fqc compress input.fastq --level 9 -o small.fqc
```

- Level 9: Best compression
- Slower but smallest output

## Memory Optimization

### Low Memory (< 2GB)

```bash
fqc compress input.fastq --streaming --buffer-size 500 -o lowmem.fqc
```

### Moderate Memory (2-8GB)

```bash
fqc compress input.fastq --pipeline --buffer-size 1000 -o output.fqc
```

### High Memory (> 8GB)

```bash
fqc compress input.fastq --pipeline --buffer-size 2000 -o output.fqc
```

## Block Size Tuning

### Short Reads (< 150bp)

```bash
fqc compress input.fastq --block-size 50000 -o output.fqc
```

Larger blocks for short reads improve compression.

### Long Reads (> 300bp)

```bash
fqc compress input.fastq --block-size 10000 -o output.fqc
```

Smaller blocks for long reads reduce memory.

### Mixed Length

```bash
fqc compress input.fastq --block-size auto -o output.fqc
```

Auto-detect optimal block size.

## Thread Count

### Determine Core Count

```bash
nproc  # Linux
sysctl -n hw.ncpu  # macOS
```

### Set Thread Count

```bash
fqc compress input.fastq --pipeline --threads 8 -o output.fqc
```

Rule of thumb: Use physical cores, not hyperthreads.

## I/O Optimization

### Fast Storage

Use NVMe SSD for best performance:
- Read: 3-7 GB/s
- Write: 2-5 GB/s

### Parallel I/O

```bash
# Compress to fast storage
fqc compress /mnt/nvme/input.fastq -o /mnt/nvme/output.fqc

# Then copy to archive
cp /mnt/nvme/output.fqc /mnt/archive/
```

## Quality Score Optimization

### Discard Quality (Smallest)

```bash
fqc compress input.fastq --quality-mode discard -o smallest.fqc
```

30-40% size reduction.

### Lossy Quality

```bash
fqc compress input.fastq --quality-mode lossy -o smaller.fqc
```

10-20% size reduction.

## Pipeline Configuration

### Buffer Size

Control inter-stage buffering:

```bash
fqc compress input.fastq \
  --pipeline \
  --buffer-size 2000 \
  -o output.fqc
```

- Small (500): Low memory, more blocking
- Medium (1000): Balanced
- Large (2000): High throughput, more memory

## Real-World Examples

### Quick QC Check

```bash
fqc compress sample.fastq --streaming --level 3 -o quick.fqc
# Fast: 30 seconds for 10GB
```

### Production Archive

```bash
fqc compress sample.fastq --pipeline --level 9 --quality-mode lossy -o archive.fqc
# Best: 15 minutes for 10GB, small size
```

### Limited Environment

```bash
fqc compress sample.fastq --streaming --buffer-size 500 -o limited.fqc
# Safe: 200MB memory max
```

## Monitoring

### Verbose Output

```bash
fqc compress input.fastq --verbose -o output.fqc
```

Shows:
- Progress
- Current block
- Compression ratio per block
- Memory usage

### JSON Statistics

```bash
fqc compress input.fastq --json -o output.fqc 2>stats.json
```

## Troubleshooting

### Slow Compression

1. Check CPU usage: `top` or `htop`
2. Increase threads: `--threads 16`
3. Use pipeline mode: `--pipeline`
4. Lower compression level: `--level 3`

### High Memory Usage

1. Use streaming mode: `--streaming`
2. Reduce buffer size: `--buffer-size 500`
3. Reduce block size: `--block-size 10000`

### Large Output

1. Increase compression level: `--level 9`
2. Use lossy quality: `--quality-mode lossy`
3. Check if using ABC for short reads

## Related

- [Benchmarks](./benchmarks.md)
- [Pipeline Mode](../guide/features/pipeline.md)
- [Streaming Mode](../guide/features/streaming.md)
