# CLI reference

`fqc` exposes four top-level commands:

- `compress`
- `decompress`
- `info`
- `verify`

Global options:

| Option | Meaning |
| --- | --- |
| `-t, --threads` | thread count (`0` means auto) |
| `-v, --verbose` | increase log verbosity |
| `-q, --quiet` | suppress non-error output |
| `--memory-limit` | memory budget in MB (`0` means auto-detect) |
| `--no-progress` | disable progress summaries |

## `compress`

```bash
fqc compress -i INPUT -o OUTPUT [OPTIONS]
```

| Option | Meaning |
| --- | --- |
| `-2, --input2` | second FASTQ file for paired-end input |
| `-l, --level` | compression level `1..9` (default: `5`) |
| `--reorder <true|false>` | enable or disable global read reordering |
| `--streaming` | disable reordering and process input incrementally |
| `--lossy-quality` | `none`, `illumina8`, `qvz`, or `discard` |
| `--long-read-mode` | `auto`, `short`, `medium`, or `long` |
| `--interleaved` | treat input as interleaved paired-end FASTQ |
| `--max-block-bases` | cap block size for longer reads |
| `--scan-all-lengths` | inspect all reads for length detection |
| `--pipeline` | use the staged compression pipeline |
| `--pe-layout` | `interleaved` or `consecutive` metadata for paired-end archives |
| `-f, --force` | overwrite output if it exists |

## `decompress`

```bash
fqc decompress -i INPUT -o OUTPUT [OPTIONS]
```

| Option | Meaning |
| --- | --- |
| `--range` | extract a read range such as `1:1000` or `100:` |
| `--header-only` | write only read headers |
| `--original-order` | restore original read order if reorder metadata exists |
| `--skip-corrupted` | continue when a block fails integrity checks |
| `--corrupted-placeholder` | placeholder sequence for skipped blocks |
| `--split-pe` | write paired-end output to separate files |
| `--pipeline` | use the staged decompression pipeline |
| `-f, --force` | overwrite output if it exists |

## `info`

```bash
fqc info -i INPUT [--json] [--detailed] [--show-codecs]
```

- `--json` emits machine-readable archive metadata
- `--detailed` shows block index entries
- `--show-codecs` reports codec bytes per block

## `verify`

```bash
fqc verify -i INPUT [--quick] [--fail-fast] [--verbose]
```

- `--quick` checks archive framing and global checksum without block decompression
- `--fail-fast` stops at the first failing block
- `--verbose` prints per-block verification progress
