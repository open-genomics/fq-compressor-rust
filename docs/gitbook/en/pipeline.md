# Parallel Pipeline

## Overview

fqc provides an optional 3-stage pipeline mode (`--pipeline`) that overlaps I/O with computation using crossbeam bounded channels for backpressure.

## Architecture

```
┌────────┐  channel  ┌────────────┐  channel  ┌────────┐
│ Reader │──────────▶│ Compressor │──────────▶│ Writer │
│ (serial)│  bounded  │  (parallel) │  bounded  │ (serial)│
└────────┘           └────────────┘           └────────┘
```

### Stage 1: Reader (Serial)

- Reads FASTQ input sequentially
- Partitions reads into chunks (block-sized)
- Sends chunks through bounded channel

### Stage 2: Compressor (Parallel)

- Receives chunks from Reader
- Compresses each block using Rayon thread pool
- ABC for short reads, Zstd for medium/long reads
- Sends compressed blocks to Writer

### Stage 3: Writer (Serial)

- Receives compressed blocks
- Writes blocks sequentially to output FQC file
- Uses `AsyncWriter` with write-behind buffer (4MB, depth 4)
- Builds block index and writes footer

## Backpressure

Bounded channels ensure memory stays controlled:

- If the Compressor is slow, the Reader blocks (channel full)
- If the Writer is slow, the Compressor blocks (channel full)
- Channel capacity is configurable (default: 2× thread count)

## When to Use Pipeline Mode

| Scenario | Default | Pipeline |
|----------|---------|----------|
| Small files (< 100MB) | ✓ Simpler | Overhead not worth it |
| Large files (> 1GB) | Adequate | ✓ Better throughput |
| NVMe/SSD storage | Adequate | ✓ I/O overlap helps |
| HDD storage | Adequate | ✓ I/O overlap helps more |
| Streaming input | N/A | ✓ Natural fit |

## Usage

```bash
# Compression
fqc compress -i reads.fastq -o reads.fqc --pipeline

# Decompression
fqc decompress -i reads.fqc -o reads.fastq --pipeline
```

## Implementation

- **Compression pipeline**: `src/pipeline/compression.rs`
- **Decompression pipeline**: `src/pipeline/decompression.rs`
- **AsyncWriter**: `src/io/async_io.rs`
- **Channel**: crossbeam-channel bounded channels
