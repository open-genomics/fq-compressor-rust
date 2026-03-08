# CLI Reference

## Global Options

| Option | Description |
|--------|-------------|
| `--version` | Print version |
| `--help` | Print help |

## compress

Compress a FASTQ file to FQC format.

```
fqc compress [OPTIONS] -i <INPUT> -o <OUTPUT>
```

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--input` | `-i` | required | Input FASTQ file (or `-` for stdin) |
| `--output` | `-o` | required | Output FQC file |
| `--input2` | `-2` | | Second input file (PE separate files) |
| `--level` | `-l` | `3` | Zstd compression level (1-19) |
| `--threads` | `-t` | all cores | Number of threads |
| `--block-size` | | auto | Reads per block |
| `--pipeline` | | false | Enable 3-stage pipeline mode |
| `--streaming` | | false | Streaming mode (no global reorder) |
| `--interleaved` | | false | Input is interleaved paired-end |
| `--lossy-quality` | | lossless | Quality mode: `lossless`, `illumina8`, `discard` |
| `--id-mode` | | `strip` | ID mode: `exact`, `strip`, `discard` |
| `--long-read-mode` | | auto | Force: `short`, `medium`, `long` |
| `--memory-limit` | | auto | Memory limit in MB |

## decompress

Decompress an FQC file to FASTQ format.

```
fqc decompress [OPTIONS] -i <INPUT> -o <OUTPUT>
```

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--input` | `-i` | required | Input FQC file |
| `--output` | `-o` | required | Output FASTQ file (or `-` for stdout) |
| `--threads` | `-t` | all cores | Number of threads |
| `--pipeline` | | false | Enable 3-stage pipeline mode |
| `--range` | | | Extract read range (1-based, e.g., `1:1000`) |
| `--header-only` | | false | Output headers only |

## info

Display archive information.

```
fqc info [OPTIONS] -i <INPUT>
```

| Option | Short | Description |
|--------|-------|-------------|
| `--input` | `-i` | Input FQC file |
| `--json` | | Output as JSON |
| `--detailed` | | Show block index details |

## verify

Verify archive integrity.

```
fqc verify [OPTIONS] -i <INPUT>
```

| Option | Short | Description |
|--------|-------|-------------|
| `--input` | `-i` | Input FQC file |
| `--verbose` | `-v` | Verbose output |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | I/O error |
| 3 | Format error |
| 4 | Checksum mismatch |
| 5 | Argument error |
