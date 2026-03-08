# Quick Start

## Compress a FASTQ File

```bash
fqc compress -i reads.fastq -o reads.fqc
```

This auto-detects read length and selects the optimal compression strategy.

## Decompress

```bash
fqc decompress -i reads.fqc -o reads.fastq
```

## View Archive Info

```bash
fqc info -i reads.fqc
```

## Verify Integrity

```bash
fqc verify -i reads.fqc
```

## Common Scenarios

### Compressed Input

fqc transparently handles compressed FASTQ files:

```bash
fqc compress -i reads.fastq.gz -o reads.fqc
fqc compress -i reads.fastq.bz2 -o reads.fqc
```

### Pipeline Mode

For large files, use 3-stage pipeline for better throughput:

```bash
fqc compress -i reads.fastq -o reads.fqc --pipeline
fqc decompress -i reads.fqc -o reads.fastq --pipeline
```

### Paired-End Data

```bash
# Separate files
fqc compress -i reads_R1.fastq -2 reads_R2.fastq -o reads.fqc

# Interleaved
fqc compress -i interleaved.fastq -o reads.fqc --interleaved
```

### Lossy Quality Compression

```bash
# Illumina 8-bin quantization (~30% better ratio)
fqc compress -i reads.fastq -o reads.fqc --lossy-quality illumina8

# Discard quality (maximum compression)
fqc compress -i reads.fastq -o reads.fqc --lossy-quality discard
```

### Random Access

```bash
# Extract reads 1-1000
fqc decompress -i reads.fqc -o subset.fastq --range 1:1000
```
