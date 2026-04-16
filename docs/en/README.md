# fqc Documentation

Welcome to the fqc (fq-compressor-rust) technical documentation.

## Documentation Guide

| Document | Description | Target Audience |
|----------|-------------|-----------------|
| [architecture.md](architecture.md) | Project architecture, module responsibilities, data flow | New contributors, code reviewers |
| [format-spec.md](format-spec.md) | FQC binary format specification (v1.0) | Format implementers, interoperability developers |
| [algorithms.md](algorithms.md) | Compression algorithms in detail (ABC / SCM / Reorder) | Algorithm researchers, performance optimizers |
| [development.md](development.md) | Development guide, testing, CI/CD, release process | Project contributors |
| [performance.md](performance.md) | Performance tuning, profiling, benchmarks | DevOps, performance engineers |

## Quick Navigation

- **Looking for project structure?** → [architecture.md](architecture.md)
- **Want to implement FQC readers/writers?** → [format-spec.md](format-spec.md)
- **Want to understand ABC compression?** → [algorithms.md](algorithms.md)
- **Want to contribute?** → [development.md](development.md)
- **Want to optimize compression?** → [performance.md](performance.md)

## Related Files

- [README.md](../../README.md) — Project overview and usage
- [CHANGELOG.md](../../CHANGELOG.md) — Version history
- [Cargo.toml](../../Cargo.toml) — Dependencies and build configuration
