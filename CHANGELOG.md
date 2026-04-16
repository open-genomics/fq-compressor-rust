# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

For release notes in Chinese, see [RELEASE_zh.md](RELEASE_zh.md).

---

## [Unreleased]

### Added

- Nothing yet

---

## [0.1.1] - 2026-04-16

### Added

- **SECURITY.md** — Security policy with vulnerability reporting guidelines
- **GitBook Glossary** — `docs/gitbook/GLOSSARY.md` with terminology definitions
- **GitBook plugins** — Added `anchors` and `search-pro` for better navigation
- **PR Preview** — Documentation preview for pull requests in `pages.yml`
- **Trivy scanning** — Docker image vulnerability scanning in `docker.yml`
- **SHA512 checksums** — Added to release artifacts alongside SHA256
- **CI Summary job** — Consolidated CI status reporting in `ci.yml`
- **Documentation check** — Added `doc-check` job in `quality.yml`

### Fixed

- **Performance docs** — Corrected default compression level from 3 to 6 in en/zh performance.md
- **Docker workflow** — Added `security-events` permission for Trivy scanning
- **Pages workflow** — Fixed missing `configure-pages` step and added build validation

### Changed

- **GitBook config** — Enhanced `book.json` with sidebar links and PDF settings
- **package.json** — Added `docs:clean` and `docs:check` scripts, Node.js engine requirement
- **CI workflow** — Added job summaries and test log artifacts on failure
- **Quality workflow** — Added documentation check and quality gate summary

---

## [0.1.0] - 2026-03-07

### Highlights

First stable release of **fqc** — a high-performance FASTQ compressor in Rust. This is a complete port of the [C++ fq-compressor](https://github.com/LessUp/fq-compressor) with feature parity, sharing the same `.fqc` archive format.

### Added

#### Compression Algorithms

- **ABC Algorithm** — Alignment-Based Compression with consensus + delta encoding for short reads (< 300bp)
- **Zstd Compression** — Length-prefixed encoding for medium/long reads (≥ 300bp)
- **SCM Quality Compression** — Statistical Context Model with order-1/order-2 arithmetic coding
- **ID Compression** — Tokenization + delta encoding with exact/strip/discard modes

#### Processing Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| Default | Batch processing with global minimizer-based reordering | Standard compression |
| Streaming | Low-memory stdin compression without global reordering | Pipes, memory-constrained |
| Pipeline | 3-stage Reader→Compressor→Writer with backpressure | Maximum throughput |

#### I/O Features

- **Async I/O** — Background prefetch and write-behind buffering
- **Compressed Input** — Transparent decompression of `.gz`, `.bz2`, `.xz`, `.zst` files
- **Random Access** — Block-indexed archive format for partial decompression
- **Range Extraction** — Extract specific read ranges (e.g., `--range 1:1000`)

#### Paired-End Support

- Separate file input (`-i R1.fastq -2 R2.fastq`)
- Interleaved file input (`--interleaved`)
- PE layout options (interleaved/consecutive storage)
- Split output on decompress (`--split-pe`)

#### Quality Modes

| Mode | Description | Compression Impact |
|------|-------------|-------------------|
| Lossless | Exact quality score preservation | Baseline |
| Illumina8Bin | 8-bin quantization | ~30% improvement |
| Discard | Replace all with `!` (Phred 0) | Maximum compression |

#### Memory & Performance

- **Memory Budget** — Auto-detect system memory with dynamic chunking
- **Parallel Processing** — Rayon-based parallel block compression/decompression
- **System Memory Detection** — Windows, Linux, macOS support

#### CLI Commands

| Command | Description |
|---------|-------------|
| `fqc compress` | Compress FASTQ to FQC format |
| `fqc decompress` | Decompress FQC to FASTQ |
| `fqc info` | Display archive information (text/JSON) |
| `fqc verify` | Verify archive integrity |

#### Exit Codes

| Code | Name | Description |
|------|------|-------------|
| 0 | Success | Operation completed successfully |
| 1 | Usage | Invalid arguments or missing files |
| 2 | IoError | I/O error (file not found, permission denied) |
| 3 | FormatError | Invalid magic, bad header, corrupted data |
| 4 | ChecksumError | Checksum mismatch or integrity violation |
| 5 | Unsupported | Unsupported codec or version |

### Testing

- **131 tests** across 8 test suites
- Algorithm tests (ID/quality compressor, PE optimizer)
- DNA utility tests (encoding tables, reverse complement)
- End-to-end tests
- Binary format tests
- FASTQ parser tests
- Reorder map tests
- Round-trip compression tests
- Type definition tests

### Platform Support

Pre-built binaries available for:

| Platform | Architecture | Type |
|----------|-------------|------|
| Linux | x64 | glibc, musl (static) |
| Linux | ARM64 | glibc, musl (static) |
| macOS | x64 | Intel Mac |
| macOS | ARM64 | Apple Silicon |
| Windows | x64 | MSVC |

### Docker

- Official image: `ghcr.io/lessup/fq-compressor-rust:latest`
- Multi-stage build with Debian Bookworm

---

## Internal Changes

Development and infrastructure changes that don't affect end users.

### 2026-03-10 - Workflow Deep Standardization

- Pages workflow renamed: `docs-pages.yml` → `pages.yml`
- CI workflow unified `permissions: contents: read` and `concurrency` configuration
- Pages workflow added `actions/configure-pages@v5` step
- Pages workflow added `paths` trigger filter to reduce unnecessary builds

---

## Version Summary

| Version | Date | Type | Description |
|---------|------|------|-------------|
| 0.1.1 | 2026-04-16 | Patch | Documentation and workflow improvements |
| 0.1.0 | 2026-03-07 | Major | Initial release |
