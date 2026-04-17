# RFC-0001: Core Architecture

**Status**: ✅ Accepted  
**Proposed**: 2024-01-15  
**Accepted**: 2024-02-01

## Summary

This RFC defines the core architecture of fqc, a high-performance FASTQ compressor written in Rust.

## Motivation

fqc is a Rust port of the C++ fq-compressor project, maintaining feature parity while leveraging Rust's safety guarantees and modern concurrency features.

## Design Details

### Module Organization

```
src/
├── main.rs              # CLI entry point (clap derive)
├── lib.rs               # Library re-exports
├── error.rs             # FqcError enum + ExitCode mapping
├── types.rs             # Core types (ReadRecord, modes, layouts)
├── format.rs            # Binary format structures
├── fqc_reader.rs        # Archive reader with block index
├── fqc_writer.rs        # Archive writer with finalize
├── reorder_map.rs       # Bidirectional reorder map
├── algo/                # Compression algorithms
├── commands/            # CLI command implementations
├── common/              # Shared utilities
├── fastq/               # FASTQ parsing
├── io/                  # I/O utilities
└── pipeline/            # Pipeline stage implementations
```

### Error Handling

- All errors use `FqcError` enum (defined with `thiserror`)
- Exit codes mapped in `FqcError::exit_code()`: 0-5
- No `unwrap()` in library code; use `?` operator

### Concurrency Model

- **Batch mode**: `rayon` for parallel block processing
- **Pipeline mode**: `crossbeam-channel` for 3-stage pipeline
- **Async I/O**: Background prefetch + write-behind

### Memory Management

- Auto-detect system memory via `memory_budget.rs`
- Dynamic chunking for large datasets
- Buffer pool for I/O operations

### Binary Format

Custom block-indexed format with:
- Magic header for corruption detection
- Global header with flags and metadata
- Per-block headers with codec information
- xxHash64 checksums for integrity
- Optional reorder map for random access
- Block index for efficient partial decompression

## Alternatives Considered

### Use Existing Formats (e.g., CRAM, BAM)
- **Rejected**: FQC format is optimized for FASTQ-specific compression
- CRAM/BAM require reference genomes; FQC is reference-free

### Single-threaded Only
- **Rejected**: Genomic datasets are large; parallelism is essential
- Both batch (rayon) and pipeline modes provided

## Constraints

- **MSRV**: Rust 1.75 (pinned in Cargo.toml and CI)
- **No unsafe code**: `unsafe_code = "deny"` except Windows FFI
- **Clippy pedantic**: 0 warnings required

## References

- [C++ fq-compressor](https://github.com/LessUp/fq-compressor)
- [Spring algorithm paper](https://github.com/shubhamchandak94/Spring)
- [FQC Format Spec](../product/file-format.md)
