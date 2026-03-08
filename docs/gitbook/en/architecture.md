# Architecture

## Overview

fqc is a high-performance FASTQ compressor with a layered, modular architecture. The core design revolves around **block-level compression**: data is split into fixed-size blocks, each compressed independently, enabling random access.

## Data Flow

### Compression

```
FASTQ Input
    │
    ▼
┌─────────────┐     ┌──────────────────┐
│ FASTQ Parser │────▶│ Global Analyzer  │  (optional) Minimizer sorting
│  fastq/      │     │  global_analyzer │  generates ReorderMap
└─────────────┘     └────────┬─────────┘
                             │
                    ┌────────▼─────────┐
                    │  Block Partition  │  Split by block_size
                    └────────┬─────────┘
                             │ (parallel)
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
        ┌──────────┐  ┌──────────┐  ┌──────────┐
        │ Block 0  │  │ Block 1  │  │ Block N  │
        │ compress │  │ compress │  │ compress │
        └────┬─────┘  └────┬─────┘  └────┬─────┘
             │              │              │
             ▼              ▼              ▼
        ┌─────────────────────────────────────┐
        │           FQC Writer                │  header + blocks
        │  → Block Index + Footer + Checksum  │  + index + footer
        └─────────────────────────────────────┘
```

### Decompression

```
FQC File
    │
    ▼
┌──────────────┐
│  FQC Reader  │  Read header + block index
└──────┬───────┘
       │ (random access or sequential)
       ▼
┌──────────────┐     ┌─────────────────┐
│ Block Decomp │────▶│ Reorder Restore │  (optional) Restore original order
└──────────────┘     └────────┬────────┘
                              ▼
                        FASTQ Output
```

### Pipeline Mode

Pipeline mode uses a 3-stage pipeline with backpressure via crossbeam bounded channels:

```
┌────────┐  channel  ┌────────────┐  channel  ┌────────┐
│ Reader │──────────▶│ Compressor │──────────▶│ Writer │
│ (serial)│  bounded  │  (parallel) │  bounded  │ (serial)│
└────────┘           └────────────┘           └────────┘
```

## Module Structure

```
src/
├── main.rs                    # CLI entry (clap)
├── lib.rs                     # Library entry (pub mod exports)
│
├── algo/                      # Compression algorithms
│   ├── block_compressor.rs    # Block compress/decompress (ABC + Zstd dual path)
│   ├── dna.rs                 # Shared DNA encoding tables + reverse complement
│   ├── global_analyzer.rs     # Global read analysis + minimizer sorting
│   ├── id_compressor.rs       # Read ID compression (Exact/StripComment/Discard)
│   ├── pe_optimizer.rs        # Paired-end reverse complement optimization
│   └── quality_compressor.rs  # Quality score SCM arithmetic coding
│
├── commands/                  # CLI subcommand implementations
│   ├── compress.rs            # compress (default/streaming/pipeline)
│   ├── decompress.rs          # decompress (sequential/parallel/reorder)
│   ├── info.rs                # info (archive info display)
│   └── verify.rs              # verify (integrity check)
│
├── common/
│   └── memory_budget.rs       # System memory detection + dynamic chunking
│
├── fastq/
│   └── parser.rs              # FASTQ parser (SE/PE/interleaved/sampling/validation)
│
├── io/
│   ├── async_io.rs            # Async I/O (prefetch/write-behind buffer)
│   └── compressed_stream.rs   # Transparent decompression (.gz/.bz2/.xz/.zst)
│
├── pipeline/
│   ├── compression.rs         # 3-stage compression pipeline
│   └── decompression.rs       # 3-stage decompression pipeline
│
├── error.rs                   # FqcError enum + ExitCode mapping (0-5)
├── format.rs                  # FQC binary format structures (header/block/footer)
├── fqc_reader.rs              # FQC archive reader (random access)
├── fqc_writer.rs              # FQC archive writer (block index)
├── reorder_map.rs             # Bidirectional reorder map (ZigZag varint)
└── types.rs                   # Core types and constants
```

## Key Design Decisions

1. **Block Independence** — Each block can be compressed/decompressed independently, enabling random access and parallel processing
2. **Codec Separation** — Sequence/quality/ID use independent codecs and compression streams
3. **Dual-Path Strategy** — Short reads use ABC (high ratio), medium/long reads use Zstd (general purpose)
4. **Backpressure Pipeline** — Bounded channels prevent memory overflow, adapting to different I/O speeds
5. **No Unsafe** — `unsafe` code is globally denied (only exception: Windows FFI in `memory_budget.rs`)
