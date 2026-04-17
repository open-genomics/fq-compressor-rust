# API Specifications

This directory contains API interface definitions for fqc.

## Overview

API specs define the interfaces between components and external consumers. They serve as contracts for CLI usage and library integration.

## Interface Types

### CLI API

The command-line interface is the primary user-facing API.

**Implementation**: `src/main.rs` (clap derive)

**Commands**:

| Command | Description | Spec |
|---------|-------------|------|
| `fqc compress` | Compress FASTQ files | [cli-commands.md](../product/cli-commands.md#1-compress) |
| `fqc decompress` | Decompress FQC files | [cli-commands.md](../product/cli-commands.md#2-decompress) |
| `fqc info` | Display archive information | [cli-commands.md](../product/cli-commands.md#3-info) |
| `fqc verify` | Verify archive integrity | [cli-commands.md](../product/cli-commands.md#4-verify) |

### Library API

Public Rust API for programmatic usage.

**Implementation**: `src/lib.rs`

**Core Types**:

| Type | Module | Purpose |
|------|--------|---------|
| `ReadRecord` | `types.rs` | Single FASTQ record |
| `FqcError` | `error.rs` | Error type with exit codes |
| `FqcReader` | `fqc_reader.rs` | Archive reader |
| `FqcWriter` | `fqc_writer.rs` | Archive writer |
| `QualityMode` | `types.rs` | Quality handling mode |
| `IdMode` | `types.rs` | ID preservation mode |
| `PeLayout` | `types.rs` | Paired-end layout |

**Exit Codes**:

| Code | Name | Description |
|------|------|-------------|
| 0 | Success | Operation completed successfully |
| 1 | Input | Input file error |
| 2 | Format | Invalid FASTQ format |
| 3 | Compression | Compression/decompression error |
| 4 | Io | I/O error |
| 5 | Usage | Command-line usage error |

## API Stability

- **Stable**: CLI commands and options (backward compatible)
- **Stable**: Public library types
- **Unstable**: Internal modules (may change without notice)

## Versioning

API follows [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking changes to CLI or library API
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, backward compatible

## Related Documents

- [CLI Commands Spec](../product/cli-commands.md)
- [Core Architecture](../rfc/0001-core-architecture.md)
