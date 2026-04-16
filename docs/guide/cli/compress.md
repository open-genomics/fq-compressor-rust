# fqc compress

Compress FASTQ files to FQC format.

## Usage

```bash
fqc compress [OPTIONS] -i <INPUT> -o <OUTPUT>
```

## Options

### Required

| Option | Short | Description |
|--------|-------|-------------|
| `--input` | `-i` | Input FASTQ file (use `-` for stdin) |
| `--output` | `-o` | Output FQC file |

### Optional

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--input2` | `-2` | - | Second input file for paired-end |
| `--interleaved` | - | false | Input is interleaved paired-end |
| `--level` | `-l` | 3 | Compression level (1-9) |
| `--block-size` | - | auto | Reads per block |
| `--threads` | `-t` | all | Number of worker threads |
| `--streaming` | - | false | Streaming mode (low memory) |
| `--pipeline` | - | false | Pipeline mode (max throughput) |
| `--lossy-quality` | - | lossless | Quality mode: lossless/illumina8bin/discard |
| `--force` | `-f` | false | Overwrite output if exists |

## Examples

```bash
# Basic compression
fqc compress -i reads.fastq -o reads.fqc

# High compression
fqc compress -i reads.fastq -o reads.fqc -l 9

# Paired-end separate files
fqc compress -i R1.fastq -2 R2.fastq -o paired.fqc

# Paired-end interleaved
fqc compress -i interleaved.fastq -o paired.fqc --interleaved

# Compressed input (auto-detected)
fqc compress -i reads.fastq.gz -o reads.fqc

# Discard quality for max compression
fqc compress -i reads.fastq -o reads.fqc --lossy-quality discard

# Force long-read mode
fqc compress -i long_reads.fastq -o reads.fqc --long-read-mode long

# Pipeline mode for speed
fqc compress -i reads.fastq -o reads.fqc --pipeline
```

## Compression Levels

| Level | Speed | Ratio | Use Case |
|-------|-------|-------|----------|
| 1 | Fastest | Lower | Quick archives |
| 3 | Balanced | Good | **Default** |
| 6 | Slower | Better | Production |
| 9 | Slowest | Best | Distribution |

## Quality Modes

| Mode | Description | Size Impact |
|------|-------------|-------------|
| `lossless` | Exact quality preservation | Baseline |
| `illumina8bin` | Quantized to 8 bins | ~30% smaller |
| `discard` | All qualities set to '!' | ~50% smaller |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Invalid arguments |
| 2 | I/O error |
| 3 | Format error |
| 5 | Unsupported feature |
