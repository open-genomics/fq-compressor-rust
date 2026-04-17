# CLI Commands Specification

**Status**: ✅ Implemented  
**Version**: 1.0

## Overview

fqc provides four main commands for FASTQ compression workflows.

## Command Structure

```
fqc <COMMAND> [OPTIONS]
```

## Commands

### 1. `compress`

Compress FASTQ files to FQC format.

#### Synopsis

```bash
fqc compress -i INPUT -o OUTPUT [OPTIONS]
```

#### Required Arguments

| Argument | Type | Description |
|----------|------|-------------|
| `-i, --input <FILE>` | Path | Input FASTQ file (or `-` for stdin) |
| `-o, --output <FILE>` | Path | Output FQC file |

#### Optional Arguments

| Argument | Type | Default | Description |
|----------|------|---------|-------------|
| `-l, --level <N>` | 1-9 | 3 | Zstd compression level |
| `--quality-mode <MODE>` | lossless/illumina8/discard | lossless | Quality score handling |
| `--id-mode <MODE>` | exact/strip-comment/discard | exact | ID preservation |
| `--block-size <N>` | u64 | auto | Reads per block |
| `--pipeline` | bool | false | Enable 3-stage parallel pipeline |
| `--streaming` | bool | false | Low-memory streaming mode |
| `--interleaved` | bool | false | Input is interleaved paired-end |
| `-2, --input2 <FILE>` | Path | - | Second input for paired-end |
| `--long-read-mode <MODE>` | auto/short/medium/long | auto | Read length class override |
| `-f, --force` | bool | false | Overwrite existing output file |

#### Exit Codes

| Code | Meaning |
|------|--------|
| 0 | Success |
| 1 | Input file error |
| 2 | Invalid FASTQ format |
| 3 | Compression error |
| 4 | I/O error |
| 5 | Usage error |

#### Examples

```bash
# Basic compression
fqc compress -i reads.fastq -o reads.fqc

# Maximum compression
fqc compress -i reads.fastq -o reads.fqc -l 9

# Streaming from stdin
cat reads.fastq | fqc compress --streaming -i - -o reads.fqc

# Pipeline mode (parallel)
fqc compress -i reads.fastq -o reads.fqc --pipeline

# Lossy quality (discard quality scores)
fqc compress -i reads.fastq -o reads.fqc --quality-mode discard

# Paired-end (separate files)
fqc compress -i reads_R1.fastq -2 reads_R2.fastq -o paired.fqc
```

---

### 2. `decompress`

Decompress FQC files to FASTQ format.

#### Synopsis

```bash
fqc decompress -i INPUT -o OUTPUT [OPTIONS]
```

#### Required Arguments

| Argument | Type | Description |
|----------|------|-------------|
| `-i, --input <FILE>` | Path | Input FQC file |
| `-o, --output <FILE>` | Path | Output FASTQ file (or `-` for stdout) |

#### Optional Arguments

| Argument | Type | Default | Description |
|----------|------|---------|-------------|
| `--range <RANGE>` | string | - | Extract range (1-based, inclusive) |
| `--header-only` | bool | false | Output headers only (no sequence/quality) |
| `--original-order` | bool | false | Restore original read order |
| `--split-pe` | bool | false | Split paired-end to separate files |
| `--pipeline` | bool | false | Enable parallel decompression |
| `--skip-corrupted` | bool | false | Skip corrupted blocks instead of failing |

#### Range Format

- `1:1000` - Extract reads 1 through 1000
- `100:` - Extract from read 100 to end
- `:500` - Extract from start to read 500

#### Exit Codes

Same as `compress` command.

#### Examples

```bash
# Full decompression
fqc decompress -i reads.fqc -o reads.fastq

# Extract range
fqc decompress -i reads.fqc -o subset.fastq --range 1:1000

# Output to stdout
fqc decompress -i reads.fqc -o -

# Headers only
fqc decompress -i reads.fqc -o headers.txt --header-only

# Split paired-end
fqc decompress -i paired.fqc -o output.fastq --split-pe
```

---

### 3. `info`

Display archive information.

#### Synopsis

```bash
fqc info -i INPUT [OPTIONS]
```

#### Required Arguments

| Argument | Type | Description |
|----------|------|-------------|
| `-i, --input <FILE>` | Path | Input FQC file |

#### Optional Arguments

| Argument | Type | Default | Description |
|----------|------|---------|-------------|
| `--json` | bool | false | Output in JSON format |
| `--detailed` | bool | false | Show detailed block index |
| `--show-codecs` | bool | false | Show codec information per block |

#### Output Format

**Human-readable** (default):
```
File: reads.fqc
Version: 1
Total reads: 2,270,000
Quality mode: Lossless
ID mode: Exact
Length class: Short
Has reorder map: Yes
Block count: 100
```

**JSON** (`--json`):
```json
{
  "filename": "reads.fqc",
  "version": 1,
  "total_reads": 2270000,
  "quality_mode": "lossless",
  "id_mode": "exact",
  "length_class": "short",
  "has_reorder_map": true,
  "block_count": 100
}
```

#### Examples

```bash
# Basic info
fqc info -i reads.fqc

# JSON output
fqc info -i reads.fqc --json

# Detailed block index
fqc info -i reads.fqc --detailed --show-codecs
```

---

### 4. `verify`

Verify archive integrity.

#### Synopsis

```bash
fqc verify -i INPUT [OPTIONS]
```

#### Required Arguments

| Argument | Type | Description |
|----------|------|-------------|
| `-i, --input <FILE>` | Path | Input FQC file |

#### Optional Arguments

| Argument | Type | Default | Description |
|----------|------|---------|-------------|
| `--verbose` | bool | false | Show per-block progress |
| `--quick` | bool | false | Check header + footer only |

#### Output

**Success**:
```
Verification successful: reads.fqc
  Total blocks: 100
  Total reads: 2,270,000
  Checksum: valid
```

**Failure**:
```
Verification FAILED: reads.fqc
  Block 42: checksum mismatch
  Verified 42/100 blocks before failure
```

#### Examples

```bash
# Full verification
fqc verify -i reads.fqc

# Verbose (per-block progress)
fqc verify -i reads.fqc --verbose

# Quick check
fqc verify -i reads.fqc --quick
```

---

## Global Options

These options apply to all commands:

| Argument | Description |
|----------|-------------|
| `--version` | Print version information |
| `--help, -h` | Print help information |
| `-v, --verbose` | Increase verbosity (can be repeated: `-vv`, `-vvv`) |
| `-q, --quiet` | Suppress all output except errors |

## Related Documents

- [Core Compression Spec](./core-compression.md)
- [Development Guide](../../docs/guide/development.md)
