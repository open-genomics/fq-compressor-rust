# Project Architecture

## Overview

fqc is a high-performance FASTQ compressor with a layered, modular architecture. The core design revolves around **block-level compression**: data is partitioned into fixed-size blocks, each independently compressed, supporting random access.

## Data Flow

### Compression Pipeline

```
FASTQ Input
    │
    ▼
┌─────────────┐     ┌──────────────────┐
│ FASTQ Parser │────▶│ Global Analyzer  │  (Optional) Minimizer ordering
│  fastq/      │     │  global_analyzer │  Generates ReorderMap
└─────────────┘     └────────┬─────────┘
                             │
                    ┌────────▼─────────┐
                    │  Block Partition  │  Split by block_size
                    └────────┬─────────┘
                             │ (Parallel)
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
        ┌──────────┐  ┌──────────┐  ┌──────────┐
        │ Block 0  │  │ Block 1  │  │ Block N  │
        │ compress │  │ compress │  │ compress │
        └────┬─────┘  └────┬─────┘  └────┬─────┘
             │              │              │
             ▼              ▼              ▼
        ┌─────────────────────────────────────┐
        │           FQC Writer                │  Write header + blocks
        │  → Block Index + Footer + Checksum  │  + index + footer
        └─────────────────────────────────────┘
```

### Decompression Pipeline

```
FQC File
    │
    ▼
┌──────────────┐
│  FQC Reader  │  Read header + block index
└──────┬───────┘
       │ (Random access or sequential)
       ▼
┌──────────────┐     ┌─────────────────┐
│ Block Decomp │────▶│ Reorder Restore │  (Optional) Restore original order
└──────────────┘     └────────┬────────┘
                              ▼
                        FASTQ Output
```

### Pipeline Mode

Pipeline mode uses a 3-stage pipeline, implemented with crossbeam bounded channels for backpressure:

```
┌────────┐  channel  ┌────────────┐  channel  ┌────────┐
│ Reader │──────────▶│ Compressor │──────────▶│ Writer │
│(Serial)│  bounded  │ (Parallel) │  bounded  │(Serial)│
└────────┘           └────────────┘           └────────┘
```

## Module Structure

```
src/
├── main.rs                    # CLI entry (clap)
├── lib.rs                     # Library entry (pub mod exports)
│
├── algo/                      # Compression algorithms
│   ├── block_compressor.rs    # Block compression (ABC + Zstd dual-path)
│   ├── global_analyzer.rs     # Global read analysis + minimizer ordering
│   ├── id_compressor.rs       # Read ID compression (Exact/StripComment/Discard)
│   ├── pe_optimizer.rs        # Paired-end (PE) reverse complement optimization
│   └── quality_compressor.rs  # Quality score SCM arithmetic coding
│
├── commands/                  # CLI subcommand implementations
│   ├── compress.rs            # compress command (default/streaming/pipeline)
│   ├── decompress.rs          # decompress command (sequential/parallel/reorder)
│   ├── info.rs                # info command (archive info display)
│   └── verify.rs              # verify command (integrity verification)
│
├── common/
│   └── memory_budget.rs       # System memory detection + dynamic chunking
│
├── fastq/
│   └── parser.rs              # FASTQ parser (SE/PE/interleaved/sampling/validation)
│
├── io/
│   ├── async_io.rs            # Async I/O (prefetch/write-behind buffering)
│   └── compressed_stream.rs   # Transparent decompression (.gz/.bz2/.xz/.zst)
│
├── pipeline/
│   ├── compression.rs         # 3-stage compression pipeline
│   └── decompression.rs       # 3-stage decompression pipeline
│
├── error.rs                   # FqcError enum + ExitCode mapping (0-5)
├── format.rs                  # FQC binary format structs (header/block/footer)
├── fqc_reader.rs              # FQC archive reader (random access)
├── fqc_writer.rs              # FQC archive writer (block index)
├── reorder_map.rs             # Bidirectional reorder mapping (ZigZag varint)
└── types.rs                   # Core types and constants
```

## Core Module Responsibilities

### `algo/block_compressor.rs`

Core block-level compression/decompression logic. Different encoders selected based on read length classification:

- **Short reads (< 300bp)** → ABC algorithm: consensus building + delta encoding + Zstd
- **Medium reads (300bp – 10kbp)** → Direct Zstd compression (length-prefixed encoding)
- **Long reads (> 10kbp)** → Direct Zstd compression

Each block contains 4 independent compressed streams: IDs, Sequences, Quality, Auxiliary data.

### `algo/global_analyzer.rs`

Global read analyzer, performing minimizer ordering:

1. Extract canonical k-mer minimizer from each read
2. Sort by minimizer value, clustering similar reads
3. Generate `ReorderMap` (bidirectional mapping) stored in archive

### `algo/quality_compressor.rs`

Quality score compressor using Statistical Context Model (SCM) + arithmetic coding:

- Order-2 context (short/medium reads): 2 previous quality values as context
- Order-1 context (long reads): 1 previous quality value
- Adaptive frequency model + 32-bit precision arithmetic coding

### `pipeline/`

crossbeam-channel based 3-stage pipeline:

- **Reader** — Serial FASTQ read, send by chunk
- **Compressor** — Rayon parallel block compression
- **Writer** — Serial write, AsyncWriter provides write-behind buffering

Bounded channels implement backpressure, `PipelineControl` provides cancellation and progress tracking.

### `error.rs`

Unified error system:

| ExitCode | Meaning |
|----------|---------|
| 0 | Success |
| 1 | General error |
| 2 | I/O error |
| 3 | Format error |
| 4 | Checksum mismatch |
| 5 | Parameter error |

### `reorder_map.rs`

Bidirectional read reorder mapping:

- `forward_map[original_id] → archive_id`
- `reverse_map[archive_id] → original_id`
- Encoded with ZigZag delta + varint for compact storage

## Dependency Graph

```
main.rs
  └── commands/*
        ├── algo/*           # Compression algorithms
        ├── pipeline/*       # Pipeline (optional)
        ├── fastq/parser     # Input parser
        ├── io/*             # I/O layer
        ├── fqc_reader       # Archive reader
        ├── fqc_writer       # Archive writer
        └── reorder_map      # Reorder mapping
```

## Key Design Decisions

1. **Block Independence** — Each block can be independently compressed/decompressed, supporting random access and parallel processing
2. **Codec Separation** — Sequence/Quality/IDs use independent codecs and compression streams
3. **Dual-Path Strategy** — Short reads use ABC (high compression ratio), medium/long reads use Zstd (general purpose)
4. **Backpressure Pipeline** — Bounded channels prevent memory overflow, adapting to different I/O speeds
5. **unsafe deny** — Global unsafe code prohibition (Windows FFI exception only)
