# Streaming Mode

Streaming mode enables low-memory compression by processing data in a continuous flow.

## Overview

Streaming mode is ideal when:
- Memory is limited (< 2GB available)
- Processing very large files
- Working with stdin/stdout pipes
- Compressing data from other processes

## Usage

Enable streaming mode with `--streaming`:

```bash
fqc compress input.fastq --streaming -o output.fqc
```

## How It Works

### Default Mode (Block-Based)

```
Load all reads → Process blocks → Write archive
```

- Loads entire dataset into memory
- Optimizes block boundaries
- Higher compression ratios
- Memory: O(n) where n = file size

### Streaming Mode

```
Read chunk → Process → Write → Repeat
```

- Processes one chunk at a time
- Constant memory usage
- Slightly lower compression ratios
- Memory: O(1) - constant

## Memory Usage Comparison

| Mode | 1GB FASTQ | 10GB FASTQ | 100GB FASTQ |
|------|-----------|------------|-------------|
| Default | 1.2GB | 12GB | 120GB |
| Streaming | 200MB | 200MB | 200MB |

## Trade-offs

### Advantages
- ✅ Constant, predictable memory usage
- ✅ Works with arbitrarily large files
- ✅ Supports stdin/stdout
- ✅ Good for pipeline integration

### Disadvantages
- ⚠️ No block reordering optimization
- ⚠️ Slightly lower compression ratios (5-10% larger)
- ⚠️ Cannot use parallel processing

## Examples

### Pipe from stdin

```bash
cat input.fastq | fqc compress --streaming -o output.fqc
```

### Pipe to stdout

```bash
fqc decompress --streaming archive.fqc | cat > output.fastq
```

### Full Pipeline

```bash
zcat input.fastq.gz | fqc compress --streaming -o output.fqc
```

## When to Use Streaming

| Scenario | Recommended Mode |
|----------|------------------|
| Small files (< 1GB) | Default |
| Large files (> 10GB) | Streaming |
| Limited memory (< 4GB) | Streaming |
| Maximum compression | Default |
| Pipeline processing | Streaming |
| stdin/stdout | Streaming |

## Related

- [Pipeline Mode](./pipeline.md)
- [Performance Tuning](../performance/tuning.md)
