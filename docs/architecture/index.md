# Architecture overview

`fqc` is a single-binary Rust CLI with a small number of well-defined layers.

## Main layers

| Layer | Key files | Responsibility |
| --- | --- | --- |
| CLI | `src/main.rs`, `src/commands/*` | Parse arguments and dispatch command behavior |
| FASTQ I/O | `src/fastq/parser.rs`, `src/io/*` | Read FASTQ input and compressed stream variants |
| Archive format | `src/format.rs`, `src/fqc_writer.rs`, `src/fqc_reader.rs` | Encode and decode the `.fqc` container |
| Compression logic | `src/algo/*` | Sequence, quality, ID, reorder, and paired-end logic |
| Pipelines | `src/pipeline/*` | Reader/compressor/writer parallel flow for pipeline mode |
| Shared types | `src/types.rs`, `src/error.rs` | Public types, defaults, and exit-code mapping |

## Archive model

An `.fqc` archive contains:

1. a global header with mode flags and archive metadata
2. one or more compressed blocks
3. an optional reorder map
4. a footer and block index

This layout is why `fqc info`, `fqc verify`, and range-based decompression can operate on archive structure rather than treating the file as an opaque blob.

## Execution modes

- **default mode**: full ingest with optional reordering
- **streaming mode**: lower-memory flow with reordering disabled
- **pipeline mode**: staged reader/compressor/writer execution

## Performance roadmap

For the maintained summary of current bottlenecks, the preferred optimization direction, and the active phase boundary, see [Performance roadmap](./performance-roadmap.md).
