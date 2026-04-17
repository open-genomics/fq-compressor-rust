# Decompress Command

The `decompress` command extracts FASTQ records from FQC archives.

## Basic Usage

```bash
fqc decompress input.fqc -o output.fastq
```

## Options

| Option | Description | Default |
|--------|-------------|---------|
| `-i, --input <FILE>` | Input FQC file | **Required** |
| `-o, --output <FILE>` | Output FASTQ file | stdout |
| `--force` | Overwrite existing output file | false |
| `--range <START-END>` | Extract specific read range | All reads |
| `--header-only` | Output only headers, no sequences/qualities | false |
| `--original-order` | Restore original read order | false |
| `--split-pe` | Split paired-end into separate files | false |
| `--skip-corrupted` | Skip corrupted blocks instead of failing | false |
| `--json` | Output statistics as JSON | false |

## Examples

### Decompress Entire Archive

```bash
fqc decompress archive.fqc -o output.fastq
```

### Extract Read Range

Extract reads 100-200 (0-indexed):

```bash
fqc decompress archive.fqc --range 100-200 -o subset.fastq
```

### Header-Only Mode

Extract only read IDs and comments:

```bash
fqc decompress archive.fqc --header-only -o headers.fastq
```

### Restore Original Order

If archive was reordered during compression:

```bash
fqc decompress archive.fqc --original-order -o ordered.fastq
```

### Split Paired-End

Split interleaved PE archive into separate R1/R2 files:

```bash
fqc decompress archive.fqc --split-pe -o output.fastq
# Creates: output_R1.fastq and output_R2.fastq
```

### Skip Corrupted Blocks

Continue processing even if some blocks are corrupted:

```bash
fqc decompress archive.fqc --skip-corrupted -o output.fastq
```

## Output Statistics

By default, decompress prints statistics:

```
Decompression complete:
  Total reads: 1,000,000
  Blocks processed: 50
  Output size: 500 MB
  Time elapsed: 2.5s
```

Use `--json` for machine-readable output:

```bash
fqc decompress archive.fqc --json
```

## Exit Codes

| Code | Meaning |
|------|----------|
| 0 | Success |
| 1 | Input file not found |
| 2 | Invalid FQC format |
| 3 | Output file exists (use --force) |
| 4 | Decompression error |
| 5 | Checksum mismatch |

## Performance Tips

1. **Use `--original-order` only if needed** - Reordering adds overhead
2. **Extract subsets with `--range`** - Faster than full decompress
3. **Parallel mode** - Automatically uses multiple threads for large archives

## Related

- [Compress Command](./compress.md)
- [Info Command](./info.md)
- [Verify Command](./verify.md)
