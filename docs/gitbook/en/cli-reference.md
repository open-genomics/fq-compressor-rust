# CLI Reference

## Global Options

| Option | Description |
|--------|-------------|
| `--version` | Print version |
| `--help` | Print help |
| `-v, --verbose` | Increase verbosity (-v info, -vv debug) |
| `-q, --quiet` | Suppress non-error output |
| `-t, --threads <N>` | Number of threads (0 = auto-detect) |
| `--memory-limit <MB>` | Memory limit in MB (0 = auto) |
| `--no-progress` | Disable progress display |

## compress

Compress a FASTQ file to FQC format.

```
fqc compress [OPTIONS] -i <INPUT> -o <OUTPUT>
```

### Required Options

| Option | Short | Description |
|--------|-------|-------------|
| `--input <FILE>` | `-i` | Input FASTQ file (use `-` for stdin) |
| `--output <FILE>` | `-o` | Output FQC file |

### Compression Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--level <N>` | `-l` | `6` | Compression level (1-9) |
| `--lossy-quality <MODE>` | | `none` | Quality mode: `none` (lossless), `illumina8`, `qvz`, `discard` |
| `--long-read-mode <MODE>` | | `auto` | Force read type: `auto`, `short`, `medium`, `long` |

### Processing Options

| Option | Default | Description |
|--------|---------|-------------|
| `--pipeline` | false | Enable 3-stage pipeline mode |
| `--streaming` | false | Streaming mode (no global reorder, for stdin) |
| `--reorder` | true | Enable global read reordering (short reads only) |

### Paired-End Options

| Option | Description |
|--------|-------------|
| `--input2 <FILE>` | `-2` | Second input file (PE separate files) |
| `--interleaved` | Input is interleaved paired-end |
| `--pe-layout <LAYOUT>` | PE storage: `interleaved` (default), `consecutive` |

### Advanced Options

| Option | Default | Description |
|--------|---------|-------------|
| `--max-block-bases <N>` | `0` | Max bases per block (0 = auto) |
| `--scan-all-lengths` | false | Scan all reads for length detection (slower but accurate) |
| `--force` | `-f` | Overwrite existing output file |

### Examples

```bash
# Basic compression
fqc compress -i reads.fastq -o reads.fqc

# Maximum compression
fqc compress -i reads.fastq -o reads.fqc -l 9

# Streaming from stdin
cat reads.fastq | fqc compress --streaming -i - -o reads.fqc

# Pipeline mode (best throughput)
fqc compress -i reads.fastq -o reads.fqc --pipeline

# Paired-end (separate files)
fqc compress -i R1.fastq -2 R2.fastq -o paired.fqc

# Discard quality for smallest output
fqc compress -i reads.fastq -o reads.fqc --lossy-quality discard

# Compressed input (auto-detected)
fqc compress -i reads.fastq.gz -o reads.fqc
```

## decompress

Decompress an FQC file to FASTQ format.

```
fqc decompress [OPTIONS] -i <INPUT> -o <OUTPUT>
```

### Required Options

| Option | Short | Description |
|--------|-------|-------------|
| `--input <FILE>` | `-i` | Input FQC file |
| `--output <FILE>` | `-o` | Output FASTQ file (use `-` for stdout) |

### Extraction Options

| Option | Description |
|--------|-------------|
| `--range <START:END>` | Extract read range (1-based, e.g., `1:1000`, `100:`) |
| `--header-only` | Output read headers only (IDs) |
| `--original-order` | Output reads in original order (requires reorder map) |

### Processing Options

| Option | Default | Description |
|--------|---------|-------------|
| `--pipeline` | false | Enable 3-stage pipeline mode |
| `--skip-corrupted` | false | Skip corrupted blocks instead of failing |
| `--split-pe` | false | Split paired-end output to R1/R2 files |

### Other Options

| Option | Description |
|--------|-------------|
| `--corrupted-placeholder <SEQ>` | Placeholder sequence for corrupted reads |
| `--force` | `-f` | Overwrite existing output file |

### Examples

```bash
# Full decompression
fqc decompress -i reads.fqc -o reads.fastq

# Extract first 1000 reads
fqc decompress -i reads.fqc -o subset.fastq --range 1:1000

# Output to stdout
fqc decompress -i reads.fqc -o -

# Restore original order
fqc decompress -i reads.fqc -o reads.fastq --original-order

# Split paired-end
fqc decompress -i paired.fqc -o output.fastq --split-pe
# Creates output_R1.fastq and output_R2.fastq

# Pipeline mode
fqc decompress -i reads.fqc -o reads.fastq --pipeline
```

## info

Display archive information.

```
fqc info [OPTIONS] -i <INPUT>
```

| Option | Short | Description |
|--------|-------|-------------|
| `--input <FILE>` | `-i` | Input FQC file |
| `--json` | | Output as JSON |
| `--detailed` | | Show block index details |
| `--show-codecs` | | Show codec information for each block |

### Example Output

```
File:              reads.fqc
File size:         12345678 bytes
Total reads:       1000000
Num blocks:        10
Original filename: reads.fastq
Is paired-end:     false
Has reorder map:   true
Preserve order:    false
Streaming mode:    false
Quality mode:      lossless
ID mode:           exact
PE layout:         interleaved
Read length class: short
```

## verify

Verify archive integrity.

```
fqc verify [OPTIONS] -i <INPUT>
```

| Option | Short | Description |
|--------|-------|-------------|
| `--input <FILE>` | `-i` | Input FQC file |
| `--verbose` | `-v` | Verbose output (per-block progress) |
| `--fail-fast` | | Stop on first error |
| `--quick` | | Quick mode: only check header + footer |

### Example

```bash
# Verify integrity
fqc verify -i reads.fqc

# Verbose output
fqc verify -i reads.fqc --verbose

# Quick check only
fqc verify -i reads.fqc --quick
```

## Exit Codes

All commands return standardized exit codes:

| Code | Name | Description |
|------|------|-------------|
| 0 | Success | Operation completed successfully |
| 1 | Usage | Invalid arguments or missing files |
| 2 | IoError | I/O error (file not found, permission denied, disk full) |
| 3 | FormatError | Invalid magic, bad header, corrupted data |
| 4 | ChecksumError | Checksum mismatch or integrity violation |
| 5 | Unsupported | Unsupported codec or version |
