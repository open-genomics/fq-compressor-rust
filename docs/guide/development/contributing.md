# Contributing

Thank you for your interest in contributing to fqc!

## Code of Conduct

Please read our [Code of Conduct](../../CODE_OF_CONDUCT.md) before participating.

## How to Contribute

### Reporting Bugs

Before creating bug reports, please check existing issues. When creating a bug report, include:

- **Description**: Clear description of the issue
- **Steps to Reproduce**: Exact commands and input files
- **Expected behavior**: What should happen
- **Actual behavior**: What actually happened
- **Environment**: OS, Rust version, fqc version

**Example:**

```markdown
**Describe the bug**
fqc crashes when compressing FASTQ with very long reads (>1000bp)

**To Reproduce**
```bash
fqc compress long_reads.fastq -o output.fqc
```

**Expected behavior**
Should compress successfully or provide clear error message

**Environment**
- OS: Ubuntu 22.04
- Rust: 1.75.0
- fqc: v0.1.1
```

### Suggesting Features

Feature suggestions should include:

- **Use case**: Why this feature is needed
- **Proposed solution**: How it should work
- **Alternatives**: Other approaches considered

### Pull Requests

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make your changes
4. Run tests: `cargo test --lib --tests`
5. Run clippy: `cargo clippy --all-targets`
6. Format code: `cargo fmt --all`
7. Commit with clear message: `git commit -m "feat: add my feature"`
8. Push: `git push origin feature/my-feature`
9. Open a Pull Request

### Pull Request Guidelines

- **Describe changes**: What and why
- **Add tests**: New features need tests
- **Update docs**: If changing user-facing behavior
- **Pass CI**: All checks must pass
- **One change per PR**: Keep PRs focused

## Development Setup

### Prerequisites

- Rust 1.75+
- cargo-clippy
- cargo-fmt

### Setup

```bash
git clone https://github.com/LessUp/fq-compressor-rust.git
cd fq-compressor-rust
cargo build
cargo test --lib --tests
```

### Test Data

Test data is in `tests/data/`:

```bash
ls tests/data/
# test_se.fastq  test_R1.fastq  test_R2.fastq  test_interleaved.fastq
```

## Coding Standards

### Rust Style

Follow standard Rust conventions:
- 4-space indentation
- Max line width: 120 characters
- Use `snake_case` for functions/variables
- Use `PascalCase` for types
- Use `SCREAMING_SNAKE_CASE` for constants

### No Unsafe

`unsafe_code = "deny"` in Cargo.toml. No unsafe code allowed.

### Error Handling

Use `thiserror` and `?` operator:

```rust
// Good
pub fn my_function() -> Result<()> {
    let data = std::fs::read(path)?;
    Ok(())
}

// Bad
pub fn my_function() -> Result<()> {
    let data = std::fs::read(path).unwrap(); // Don't do this
}
```

### Logging

Use `log` crate, never `println!` for status:

```rust
// Good
log::info!("Processing {} reads", count);
log::warn!("Skipping corrupted block");
log::debug!("Block size: {}", size);

// Bad
println!("Processing reads");
```

### Testing

- All code must have tests
- Test pattern: compress → decompress → compare
- Use helpers from `test_e2e.rs`

**Example:**

```rust
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

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `test:` Test changes
- `refactor:` Code refactoring
- `perf:` Performance improvements
- `ci:` CI/CD changes

**Examples:**

```
feat: add streaming mode support
fix: correct ABC consensus for long reads
docs: update installation guide
test: add round-trip test for PE data
refactor: simplify block compressor interface
perf: optimize quality score encoding
ci: add Trivy security scanning
```

## Code Review

All PRs are reviewed by at least one maintainer. Review focuses on:

1. **Correctness**: Does it work as intended?
2. **Safety**: No unsafe code, proper error handling?
3. **Performance**: Efficient algorithms, no regressions?
4. **Testing**: Adequate test coverage?
5. **Documentation**: Updated docs for user-facing changes?

## Architecture Overview

See [Architecture](../architecture/index.md) for technical details.

## Questions?

- **GitHub Discussions**: General questions
- **GitHub Issues**: Bug reports and feature requests
- **Email**: Contact maintainers directly

Thank you for contributing! 🎉
