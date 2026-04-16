# Changelog

All notable changes to the fqc project.

## [v0.1.1](https://github.com/LessUp/fq-compressor-rust/releases/tag/v0.1.1) (2024-04-16)

### Documentation
- New SECURITY.md with vulnerability reporting
- GitBook glossary with terminology
- Enhanced navigation plugins

### CI/CD
- Trivy container scanning
- PR preview for documentation
- SHA512 checksums
- Performance metrics tracking

### Fixed
- Default compression level corrected (3 → 6)
- Workflow permission fixes

---

## [v0.1.0](https://github.com/LessUp/fq-compressor-rust/releases/tag/v0.1.0) (2024-03-07)

### Highlights

First stable release! Complete Rust implementation with feature parity to C++ version.

### Features

- **ABC Algorithm** for short reads
- **SCM Compression** for quality scores
- **Zstd Codec** for medium/long reads
- **Global Reordering** with minimizers
- **Random Access** block-indexed format
- **Pipeline Mode** 3-stage processing
- **Streaming Mode** low-memory option
- **Paired-End** support (interleaved/separate)
- **131 Tests** comprehensive coverage

### Platforms

- Linux (x64, ARM64)
- macOS (Intel, Apple Silicon)
- Windows (x64)

---

[View all releases on GitHub](https://github.com/LessUp/fq-compressor-rust/releases)
