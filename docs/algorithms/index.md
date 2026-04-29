# Algorithms overview

`fqc` uses different strategies for different FASTQ components and read-length classes.

## Sequence path

- **Short-read path**: `fqc` prefers an ABC-style consensus/delta representation for smaller short-read blocks and falls back to Zstd-backed block storage when larger blocks would make that path too expensive.
- **Medium and long reads**: sequence payloads are stored with a Zstd-backed path instead of the short-read consensus model.

The current implementation classifies reads using observed lengths rather than a single CLI preset.

## Quality path

Quality strings are compressed separately from sequences:

- `none` keeps quality scores lossless
- `illumina8` bins qualities
- `qvz` is exposed in the type surface
- `discard` replaces qualities with placeholders on decode

## Reordering

For short single-end archives in non-streaming mode, `fqc` can reorder reads to improve locality and compression efficiency. The archive stores reorder metadata when needed so original-order decompression remains possible.

## Paired-end layout

Paired-end input can be ingested from separate files or interleaved input and stored with either interleaved or consecutive archive layout metadata.
