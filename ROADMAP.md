# Project Roadmap

This document outlines the development roadmap for fqc.

## Current Status: v0.1.1 ✅

- Core compression algorithms implemented
- Full CLI with 4 commands
- 131 integration tests passing
- Cross-platform support (Linux, macOS, Windows)
- VitePress documentation site

## Short-term Goals (v0.2.x)

### Performance Improvements

- [ ] **Parallel Compression** — Multi-threaded block processing
- [ ] **SIMD Optimization** — Vectorized DNA encoding
- [ ] **Memory Pool** — Reusable buffers to reduce allocations
- [ ] **Streaming Performance** — Optimize async I/O paths

### Feature Enhancements

- [ ] **Lossless Quality Compression** — Optional quality-preserving mode
- [ ] **Comment Preservation** — Store FASTQ comments in archive
- [ ] **Batch Mode** — Compress multiple files in one command
- [ ] **Progress Reporting** — Real-time compression progress

### Code Quality

- [ ] **Property-Based Testing** — Use `proptest` for invariant testing
- [ ] **Benchmarking Suite** — Continuous performance tracking
- [ ] **Fuzz Testing** — Input validation with `cargo-fuzz`
- [ ] **Coverage Reports** — Tarpaulin integration

## Mid-term Goals (v0.3.x)

### Algorithm Improvements

- [ ] **Adaptive Block Sizing** — Dynamic block size based on read length distribution
- [ ] **Improved SCM** — Better quality score compression with deeper context
- [ ] **Reference-Based Compression** — Optional reference genome assistance
- [ ] **Multi-Codec Support** — Allow mixing ABC/Zstd/LZ4 within archive

### Ecosystem

- [ ] **Python Bindings** — `pyfqc` package for Python integration
- [ ] **Nextflow Integration** — nf-core pipeline compatibility
- [ ] **Cloud Storage** — Direct S3/GCS support
- [ ] **Streaming API** — Pipe support for stdin/stdout

### Developer Experience

- [ ] **API Documentation** — Generate docs with `cargo doc`
- [ ] **Examples Directory** — Usage examples for library API
- [ ] **Migration Guide** — C++ to Rust migration notes

## Long-term Goals (v1.0.0)

### Stability

- [ ] **API Freeze** — Stable 1.0 public API
- [ ] **Format Specification** — Formal FQC format specification
- [ ] **Interoperability Testing — Cross-validation with C++ version
- [ ] **Production Deployment** — Deploy in production genomics pipelines

### Advanced Features

- [ ] **GPU Acceleration** — CUDA support for ABC algorithm
- [ ] **Incremental Updates** — Append to existing archives
- [ ] **Selective Decompression** — Extract specific reads by ID
- [ ] **Compression Presets** — `--fast`, `--balanced`, `--best` profiles

### Community

- [ ] **Contributing Guidelines** — Detailed contribution guide
- [ ] **Code of Conduct** — Community standards
- [ ] **Plugin System** — Custom codec support
- [ ] **Third-party Tools** — Integration with SAMtools, BWA, etc.

## Release Schedule

| Version | Target Date | Focus |
|---------|-------------|-------|
| v0.1.x | Current | Bug fixes, documentation |
| v0.2.0 | Q2 2024 | Performance, features |
| v0.3.0 | Q3 2024 | Algorithms, ecosystem |
| v1.0.0 | Q4 2024 | Stability, production-ready |

## How to Contribute

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

**Priority Areas for Contributions:**
1. Benchmarking and profiling
2. SIMD optimizations
3. Python bindings
4. Documentation and examples

## Tracking Progress

- **GitHub Issues** — Feature requests and bugs
- **GitHub Projects** — Sprint tracking
- **Milestones** — Release planning

---

*Last updated: 2024-04-16*
