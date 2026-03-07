# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

## [0.1.0] - 2026-03-07

### Features

- ABC (Alignment-Based Compression) algorithm for short reads
- Zstd compression for medium/long reads
- SCM quality compression with arithmetic coding
- Global minimizer-based read reordering
- Block-indexed archive format with random access
- 3-stage compression/decompression pipeline (`--pipeline`)
- Async I/O with write-behind buffering
- Streaming mode for stdin input
- Compressed input support (gz, bz2, xz, zst)
- Paired-end support (interleaved and separate files)
- Memory budget with system memory detection
- ExitCode mapping (0-5) for all CLI commands

### Testing

- 97 tests across 6 test suites
- E2E round-trip, format, parser, reorder map, types tests
