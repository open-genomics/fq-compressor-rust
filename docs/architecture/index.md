# Architecture Overview

fqc uses a **block-based compression architecture** that enables parallel processing and random access.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         COMPRESSION FLOW                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  FASTQ Input                                                     │
│      │                                                           │
│      ▼                                                           │
│  ┌──────────────┐                                               │
│  │ FASTQ Parser │──┐                                            │
│  └──────────────┘  │                                            │
│                    ▼                                            │
│         ┌──────────────────┐                                    │
│         │ Global Analyzer  │  Minimizer extraction & sorting   │
│         └────────┬─────────┘                                    │
│                  │                                              │
│                  ▼                                              │
│         ┌──────────────────┐                                    │
│         │ Block Partition  │  Split into chunks                │
│         └────────┬─────────┘                                    │
│                  │                                              │
│    ┌─────────────┼─────────────┐                               │
│    │             │             │                                │
│    ▼             ▼             ▼                                │
│ ┌──────┐    ┌──────┐    ┌──────┐  (Parallel compression)       │
│ │Block0│    │Block1│    │BlockN│                               │
│ └──┬───┘    └──┬───┘    └──┬───┘                                │
│    │           │           │                                    │
│    └───────────┴───────────┘                                    │
│                  │                                              │
│                  ▼                                              │
│         ┌──────────────────┐                                    │
│         │   FQC Writer     │  Write header + blocks + index     │
│         └──────────────────┘                                    │
│                  │                                              │
│                  ▼                                              │
│              .fqc File                                          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Data Flow

### Compression Pipeline

1. **Parser** validates and parses FASTQ records
2. **Global Analyzer** extracts minimizers and sorts reads
3. **Block Partition** splits data into chunks
4. **Block Compression** processes blocks in parallel (Rayon)
5. **Writer** assembles final archive with index

### Decompression Pipeline

1. **Reader** parses header and block index
2. **Block Selection** for random access or sequential read
3. **Block Decompression** parallel processing
4. **Reorder Restoration** (optional) to original order
5. **Output** validated FASTQ

## Key Design Decisions

### Block Independence

Each block contains 4 compressed streams:
- **IDs** - Read identifiers
- **Sequences** - DNA sequences
- **Qualities** - Quality scores
- **Auxiliary** - Metadata

Blocks can be decompressed independently, enabling:
- **Parallel processing** during compression/decompression
- **Random access** to specific read ranges
- **Partial extraction** without full decompression

### Dual-Path Strategy

Different algorithms for different read lengths:

| Read Type | Sequence Codec | Quality Codec | Reorder |
|-----------|---------------|---------------|---------|
| Short (< 300bp) | ABC (consensus+Δ) | SCM Order-2 | Yes |
| Medium (300bp-10kbp) | Zstd | SCM Order-2 | No |
| Long (> 10kbp) | Zstd | SCM Order-1 | No |

This maximizes compression ratio for short reads while maintaining speed for long reads.

## Module Structure

```
src/
├── main.rs              # CLI entry point
├── lib.rs               # Library exports
├── error.rs             # Error handling
├── types.rs             # Core types
├── format.rs            # FQC format structures
├── fqc_reader.rs        # Archive reader
├── fqc_writer.rs        # Archive writer
├── reorder_map.rs       # Bidirectional mapping
├── algo/                # Compression algorithms
├── commands/            # CLI commands
├── fastq/               # FASTQ parser
├── io/                  # I/O utilities
└── pipeline/            # Pipeline processing
```

## Performance Characteristics

- **Throughput**: ~10 MB/s compression, ~60 MB/s decompression
- **Memory**: Configurable block size, streaming mode available
- **Scalability**: Linear scaling with CPU cores up to ~8 threads
- **I/O**: Async I/O with prefetch and write-behind buffering
