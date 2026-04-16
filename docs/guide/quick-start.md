# Quick Start

Get started with fqc in 5 minutes.

## Compress Your First File

```bash
# Basic compression (auto-detects read length)
fqc compress -i sample.fastq -o sample.fqc

# Verify output
fqc info -i sample.fqc
```

## Common Use Cases

### 1. Paired-End Data

```bash
# Separate files
fqc compress -i R1.fastq -2 R2.fastq -o paired.fqc

# Interleaved
fqc compress -i interleaved.fastq -o paired.fqc --interleaved
```

### 2. Streaming from stdin

```bash
# Low-memory mode for large files
cat huge.fastq | fqc compress --streaming -i - -o output.fqc
```

### 3. Pipeline Mode (Maximum Speed)

```bash
# 3-stage parallel processing
fqc compress -i reads.fastq -o reads.fqc --pipeline
```

## Decompression

```bash
# Extract all reads
fqc decompress -i sample.fqc -o output.fastq

# Extract range (1-based)
fqc decompress -i sample.fqc -o subset.fastq --range 1:1000
```

## Next Steps

- [CLI Reference](./cli/compress.md) - All command options
- [Performance Tuning](./performance/tuning.md) - Optimize for your workload
