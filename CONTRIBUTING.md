# Contributing to fqc

Thank you for your interest in contributing to **fqc** — a high-performance FASTQ compressor! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Code Style](#code-style)
- [Testing](#testing)
- [Commit Guidelines](#commit-guidelines)
- [Pull Request Process](#pull-request-process)
- [Release Process](#release-process)

## Code of Conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). By participating, you are expected to uphold this code. Please report unacceptable behavior to the project maintainers.

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/fq-compressor-rust.git
   cd fq-compressor-rust
   ```
3. Create a feature branch:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## Development Setup

### Prerequisites

- **Rust 1.75+** (MSRV pinned in `Cargo.toml`)
- **Git**
- **Docker** (optional, for containerized testing)

### Build Commands

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run all tests
cargo test --lib --tests

# Run specific test suite
cargo test --test test_e2e
cargo test --test test_roundtrip

# Check code style
cargo clippy --all-targets
cargo fmt --all -- --check
```

### Project Structure

```
src/
├── main.rs              # CLI entry point (clap derive)
├── lib.rs               # Library re-exports
├── error.rs             # FqcError enum + ExitCode mapping
├── types.rs             # Core types (ReadRecord, QualityMode, etc.)
├── format.rs            # FQC binary format structures
├── algo/                # Compression algorithms
├── commands/            # CLI command implementations
├── pipeline/            # 3-stage parallel pipelines
├── io/                  # Async I/O and compressed streams
└── fastq/               # FASTQ parser
```

## Code Style

### Must Follow

| Rule | Description |
|------|-------------|
| **MSRV 1.75** | Do not use APIs stabilized after Rust 1.75 |
| **No unsafe** | `unsafe_code = "deny"` — only allowed for Windows FFI in `memory_budget.rs` |
| **Clippy pedantic** | All code must pass `cargo clippy --all-targets` with 0 warnings |
| **Formatting** | Run `cargo fmt` before committing |
| **Logging** | Use `log::info!`/`warn!`/`debug!`, never `println!` for status messages |

### Code Organization

- Use section separators: `// ====...====` for major code blocks
- Group imports: std → external → crate
- Use `thiserror` for error types
- Feature-gate optional dependencies: `#[cfg(feature = "gz")]`

### Error Handling

```rust
// Good: Use ? operator with FqcError
pub fn my_function() -> Result<()> {
    let data = std::fs::read(path)?;
    // ...
    Ok(())
}

// Bad: unwrap() in library code
pub fn my_function() -> Result<()> {
    let data = std::fs::read(path).unwrap(); // Don't do this
}
```

## Testing

### Test Requirements

- **All 131 tests must pass** before submitting a PR
- New features require corresponding tests
- Bug fixes require regression tests

### Test Structure

| Test Suite | File | Count | Focus |
|------------|------|-------|-------|
| test_algo | `tests/test_algo.rs` | 19 | Algorithm unit tests |
| test_dna | `tests/test_dna.rs` | 15 | DNA utilities |
| test_e2e | `tests/test_e2e.rs` | 15 | End-to-end tests |
| test_format | `tests/test_format.rs` | 15 | Binary format |
| test_parser | `tests/test_parser.rs` | 19 | FASTQ parsing |
| test_reorder_map | `tests/test_reorder_map.rs` | 23 | Reorder map |
| test_roundtrip | `tests/test_roundtrip.rs` | 14 | Compress→Decompress |
| test_types | `tests/test_types.rs` | 11 | Type definitions |

### Writing Tests

Use the helper functions from `test_e2e.rs`:

```rust
use super::*;

#[test]
fn test_my_feature() -> Result<()> {
    let input = "tests/data/test_se.fastq";
    let output = TempFile::new(".fqc")?;
    
    compress_file(input, output.path(), Default::default())?;
    
    let records = decompress_file(output.path(), Default::default())?;
    let original = read_fastq_records(input)?;
    
    assert_roundtrip_match(&original, &records)?;
    Ok(())
}
```

## Commit Guidelines

### Commit Message Format

```
<type>: <subject>

<body>

<footer>
```

### Types

| Type | Description |
|------|-------------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `style` | Formatting, no code change |
| `refactor` | Code restructuring |
| `perf` | Performance improvement |
| `test` | Adding/updating tests |
| `chore` | Maintenance tasks |

### Examples

```
feat: add --split-pe option for decompress command

Allows splitting paired-end output into separate R1/R2 files.

Closes #42
```

```
fix: handle empty sequence in ABC encoder

Previously, empty sequences caused a panic. Now they are skipped
with a warning log.
```

## Pull Request Process

### Before Submitting

1. **Update from main**:
   ```bash
   git fetch origin
   git rebase origin/main
   ```

2. **Run all checks**:
   ```bash
   cargo test --lib --tests
   cargo clippy --all-targets
   cargo fmt --all -- --check
   ```

3. **Update documentation** if you changed:
   - CLI options → update `README.md` and `docs/gitbook/*/cli-reference.md`
   - File format → update `docs/gitbook/*/format-spec.md`
   - Architecture → update `CLAUDE.md` and `docs/gitbook/*/architecture.md`

### PR Checklist

- [ ] Code compiles without warnings
- [ ] All 131 tests pass
- [ ] New code has corresponding tests
- [ ] Documentation updated (if applicable)
- [ ] Commit messages follow the guidelines
- [ ] PR description explains the change and motivation

### Review Process

1. Automated CI checks must pass (tests on Linux/macOS/Windows, clippy, fmt)
2. At least one maintainer approval required
3. All conversations resolved
4. Squash and merge to `main`

## Release Process

Releases are automated via GitHub Actions:

1. Update version in `Cargo.toml`
2. Create a tag: `git tag v0.x.x`
3. Push tag: `git push origin v0.x.x`
4. CI builds binaries for 5 platforms and creates a GitHub Release

### Version Numbering

We follow [Semantic Versioning](https://semver.org/):
- **MAJOR**: Breaking changes to file format or CLI
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, backward compatible

## Questions?

- Open a [GitHub Issue](https://github.com/LessUp/fq-compressor-rust/issues) for bugs or feature requests
- Start a [Discussion](https://github.com/LessUp/fq-compressor-rust/discussions) for questions

Thank you for contributing! 🎉
