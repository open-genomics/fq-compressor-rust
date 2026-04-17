# RFC-0003: Pipeline Architecture

**Status**: ✅ Accepted  
**Proposed**: 2024-01-20  
**Accepted**: 2024-02-10

## Summary

This RFC defines the 3-stage pipeline architecture for parallel compression and decompression in fqc.

## Motivation

Single-threaded batch processing leaves CPU cores underutilized. A pipeline architecture enables:
- Overlapping I/O and CPU-bound work
- Backpressure to prevent memory exhaustion
- Better throughput on multi-core systems

## Design Details

### Pipeline Stages

#### Compression Pipeline

```
┌──────────┐     ┌──────────────┐     ┌──────────┐
│  Stage 1  │────▶│   Stage 2    │────▶│  Stage 3  │
│  Reader   │     │  Compressor  │     │  Writer   │
└──────────┘     └──────────────┘     └──────────┘
   FASTQ              FQC block          FQC file
   parsing            compression        writing
```

**Stage 1: Reader**
- Parse FASTQ records from input
- Chunk into configurable block sizes
- Send `ReadChunk` to Stage 2 via channel

**Stage 2: Compressor**
- Receive `ReadChunk` from Stage 1
- Apply compression algorithms (ABC/Zstd/SCM)
- Send compressed block data to Stage 3

**Stage 3: Writer**
- Receive compressed blocks from Stage 2
- Write to FQC file with proper format
- Build block index and reorder map
- Finalize archive on completion

#### Decompression Pipeline

```
┌──────────┐     ┌──────────────┐     ┌──────────┐
│  Stage 1  │────▶│   Stage 2    │────▶│  Stage 3  │
│  Reader   │     │ Decompressor │     │  Writer   │
└──────────┘     └──────────────┘     └──────────┘
   FQC file           FQC block          FASTQ
   reading            decompression      output
```

### Channel Communication

**Implementation**: `crossbeam-channel`

- **Bounded channels**: Prevent unbounded memory growth
- **Backpressure**: Slow stage blocks fast producers
- **Pipeline control**: `PipelineControl` for graceful shutdown

```rust
pub struct PipelineControl {
    pub max_chunks_in_flight: usize,
    pub cancel_requested: AtomicBool,
}
```

### Parallelism Modes

#### Batch Mode (Default)

- Single thread processes all blocks sequentially
- Within each block, use `rayon` for parallel compression
- Simpler, lower memory overhead

#### Pipeline Mode (`--pipeline`)

- 3 stages run in parallel threads
- Each stage can have multiple workers
- Higher throughput, higher memory usage

```bash
# Enable pipeline mode
fqc compress -i input.fastq -o output.fqc --pipeline
fqc decompress -i input.fqc -o output.fastq --pipeline
```

### Memory Budget

**Auto-Detection**:
- Detect system memory via `memory_budget.rs`
- Calculate safe chunk sizes based on available memory
- Pipeline channel bounds derived from memory budget

**Chunking Strategy**:
- Small files (< 1GB): Single chunk
- Medium files (1-10GB): 4-8 chunks
- Large files (> 10GB): Dynamic chunking based on memory

### Error Handling

- Any stage can signal error via channel
- `PipelineControl::cancel_requested` stops all stages
- Partial output cleaned up on failure
- Error propagated to user with context

### Statistics

`PipelineStats` tracks:
- Reads processed per stage
- Bytes input/output
- Time per stage
- Compression ratio
- Throughput (MB/s)

Logged on completion for performance analysis.

## Performance Characteristics

| Mode | Compression | Decompression | Memory |
|------|-------------|---------------|--------|
| Batch | ~10 MB/s | ~50 MB/s | Low |
| Pipeline | ~12 MB/s | ~60 MB/s | Medium-High |

*Tested on Intel Core i7-9700 @ 3.00GHz (8 cores)*

## Alternatives Considered

### Tokio Async Runtime
- **Rejected**: Adds dependency complexity
- `crossbeam-channel` + threads sufficient for CPU-bound work
- No network I/O requiring async

### Single-Threaded Only
- **Rejected**: Leaves CPU cores idle on modern hardware
- Genomic datasets benefit from parallelism

### More Pipeline Stages
- **Rejected**: 3 stages optimal for I/O + CPU work split
- Additional stages increase complexity without proportional gain

## Implementation

- **Shared types**: `src/pipeline/mod.rs`
- **Compression pipeline**: `src/pipeline/compression.rs`
- **Decompression pipeline**: `src/pipeline/decompression.rs`
- **Async I/O**: `src/io/async_io.rs`

## References

- [Crossbeam channels](https://docs.rs/crossbeam-channel)
- [Pipeline pattern](https://en.wikipedia.org/wiki/Pipeline_(computing))
- [Backpressure](https://en.wikipedia.org/wiki/Backpressure_routing)
