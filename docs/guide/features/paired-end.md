# Paired-End Support

fqc provides comprehensive support for paired-end (PE) FASTQ data.

## Overview

Paired-end sequencing generates two reads from each DNA fragment:
- **Read 1 (R1)**: Forward read
- **Read 2 (R2)**: Reverse read

fqc can handle PE data in two formats:
1. **Interleaved**: R1 and R2 alternating in single file
2. **Separate**: R1 and R2 in different files

## Input Formats

### Interleaved Format

```
@read1/1
ATCG...
+
IIII...
@read1/2
GCTA...
+
IIII...
@read2/1
...
```

### Separate Files

`R1.fastq`:
```
@read1
ATCG...
+
IIII...
```

`R2.fastq`:
```
@read1
GCTA...
+
IIII...
```

## Compression

### Compress Interleaved PE

```bash
fqc compress interleaved.fastq -o pe.fqc
```

fqc auto-detects interleaved format from read identifiers.

### Compress Separate PE Files

Use the `--pe-layout` option:

```bash
fqc compress R1.fastq --pe-layout separate --pe-pair R2.fastq -o pe.fqc
```

## PE Optimization

fqc applies special optimizations for paired-end data:

1. **Cross-read correlation**: Exploits R1/R2 relationship
2. **Joint compression**: Compresses pairs together
3. **Shared context**: Uses R1 info for R2 compression

These optimizations typically achieve **10-20% better compression** for PE data.

## Decompression

### Extract Combined

```bash
fqc decompress pe.fqc -o output.fastq
```

### Split into Separate Files

Use `--split-pe`:

```bash
fqc decompress pe.fqc --split-pe -o output.fastq
```

Creates:
- `output_R1.fastq`
- `output_R2.fastq`

## PE Layout Options

| Layout | Description | Use Case |
|--------|-------------|----------|
| `interleaved` | R1/R2 in single file | Standard PE format |
| `separate` | R1/R2 in different files | Split PE files |
| `auto` | Auto-detect (default) | Most cases |

## Examples

### Compress Interleaved PE

```bash
fqc compress pe_interleaved.fastq -o compressed.fqc
```

### Compress Separate PE

```bash
fqc compress sample_R1.fastq \
  --pe-layout separate \
  --pe-pair sample_R2.fastq \
  -o sample.fqc
```

### Decompress and Split

```bash
fqc decompress sample.fqc --split-pe -o sample.fastq
ls sample_*_R*.fastq
# sample_R1.fastq  sample_R2.fastq
```

## Validation

fqc validates PE integrity during compression:
- Matching read counts
- Proper read pairing
- Consistent identifiers

## Related

- [Quick Start](../quick-start.md)
- [Compress Command](../cli/compress.md)
- [Decompress Command](../cli/decompress.md)
