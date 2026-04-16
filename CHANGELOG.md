# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## Quick Links

- [中文版本](RELEASE_zh.md)
- [Release Notes](changelog/releases/)
  - [v0.1.1](changelog/releases/v0.1.1.md) — Documentation & CI improvements
  - [v0.1.0](changelog/releases/v0.1.0.md) — Initial stable release

---

## [Unreleased]

## [0.1.1] - 2026-04-16

### Added

#### Documentation
- SECURITY.md with vulnerability reporting policy
- GitBook glossary (GLOSSARY.md) with domain terminology
- GitBook anchors and search-pro plugins
- Enhanced book.json with sidebar links and PDF settings

#### Security
- Trivy container scanning in docker.yml
- SHA512 checksums alongside SHA256 in releases

#### CI/CD
- PR preview workflow for documentation changes
- CI summary job with consolidated status reporting
- Documentation check job in quality.yml
- Test log artifacts on failure for debugging

### Fixed
- Corrected default compression level from 3 to 6 in performance docs (EN & 中文)
- Docker workflow permissions for security scanning
- Pages workflow configure-pages step

### Changed
- Updated package.json with docs:clean and docs:check scripts
- Enhanced CI workflow with artifact collection
- Quality workflow with quality gate summary

---

## [0.1.0] - 2026-03-07

### Highlights

First stable release of **fqc** — a high-performance FASTQ compressor in Rust. Complete port of the C++ fq-compressor with feature parity, sharing the same `.fqc` archive format.

### Compression Algorithms

| Algorithm | Target | Method |
|-----------|--------|--------|
| ABC | Short reads (< 300bp) | Consensus + delta encoding |
| Zstd | Medium/Long reads (≥ 300bp) | Length-prefixed + Zstd |
| SCM | Quality scores | Order-1/2 arithmetic coding |

### Processing Modes

- **Default**: Batch with global minimizer-based reordering
- **Streaming**: Low-memory stdin without reordering
- **Pipeline**: 3-stage with backpressure for throughput

### Key Features

- Async I/O with background prefetch/write-behind
- Transparent decompression of `.gz/.bz2/.xz/.zst` inputs
- Block-indexed format for random access
- Paired-end support (separate/interleaved files)
- Three quality modes: Lossless / Illumina8Bin / Discard

### Platform Support

Pre-built binaries:
- Linux (x64, ARM64) — glibc and musl (static)
- macOS (Intel, Apple Silicon)
- Windows x64

### Docker

Official image: `ghcr.io/lessup/fq-compressor-rust:latest`

---

## Version History

| Version | Date | Type | Description |
|---------|------|------|-------------|
| [0.1.1](changelog/releases/v0.1.1.md) | 2026-04-16 | Patch | Documentation security and CI improvements |
| [0.1.0](changelog/releases/v0.1.0.md) | 2026-03-07 | Major | Initial stable release |

---

[Unreleased]: https://github.com/LessUp/fq-compressor-rust/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/LessUp/fq-compressor-rust/releases/tag/v0.1.1
[0.1.0]: https://github.com/LessUp/fq-compressor-rust/releases/tag/v0.1.0
