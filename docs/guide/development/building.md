# Building from Source

Instructions for building fqc from source code.

## Prerequisites

### Required

- **Rust**: 1.75.0 or later
- **Cargo**: Included with Rust
- **Git**: For cloning repository

### Optional

- **Docker**: For containerized builds
- **musl-tools**: For static Linux builds
- **cross**: For cross-compilation

## Quick Build

### Clone Repository

```bash
git clone https://github.com/LessUp/fq-compressor-rust.git
cd fq-compressor-rust
```

### Build Debug

```bash
cargo build
```

Output: `target/debug/fqc`

### Build Release

```bash
cargo build --release
```

Output: `target/release/fqc`

Release builds are optimized for performance and are 2-5x faster than debug builds.

## Build Options

### Default Features

```bash
cargo build --release
```

Includes:
- zstd compression
- ABC algorithm
- SCM quality compression

### With Optional Features

```bash
# Enable gzip support
cargo build --release --features gz

# Enable bzip2 support
cargo build --release --features bz2

# Enable xz support
cargo build --release --features xz

# Enable all optional features
cargo build --release --features gz,bz2,xz
```

### Minimal Build

Disable default features:

```bash
cargo build --release --no-default-features
```

This builds only core functionality without zstd, ABC, or SCM.

## Static Builds

### Linux Static (musl)

```bash
# Install musl tools
sudo apt install musl-tools

# Build static binary
cargo build --release --target x86_64-unknown-linux-musl
```

Output: `target/x86_64-unknown-linux-musl/release/fqc`

### Using Docker

```bash
docker build -t fqc-builder .
docker run --rm -v $(pwd)/target:/target fqc-builder
```

## Cross-Compilation

### Install cross

```bash
cargo install cross
```

### Build for Different Targets

```bash
# Windows
cross build --release --target x86_64-pc-windows-gnu

# macOS
cross build --release --target x86_64-apple-darwin

# ARM64 Linux
cross build --release --target aarch64-unknown-linux-gnu
```

## Testing

### Run All Tests

```bash
cargo test --lib --tests
```

Expected: 131 tests, 0 failures

### Run Specific Tests

```bash
# Run algorithm tests
cargo test --test test_algo

# Run round-trip tests
cargo test --test test_roundtrip

# Run single test
cargo test test_abc_compression
```

### Run Clippy

```bash
cargo clippy --all-targets
```

Must pass with 0 warnings.

### Run Format Check

```bash
cargo fmt --all -- --check
```

## Benchmarking

```bash
cargo bench
```

Runs performance benchmarks using criterion.

## Documentation

### Build Documentation

```bash
cargo doc --no-deps --open
```

Opens generated documentation in browser.

### Build VitePress Docs

```bash
cd docs
npm install
npm run docs:build
```

Output: `docs/.vitepress/dist/`

## Common Build Errors

### Missing zstd Library

```
error: could not find system library 'zstd'
```

**Fix:**

```bash
# Ubuntu/Debian
sudo apt install libzstd-dev

# macOS
brew install zstd

# Fedora
sudo dnf install zstd-devel
```

### Outdated Rust

```
error[E0xxx]: ...
```

**Fix:**

```bash
rustup update
```

### Permission Denied

```
error: failed to create `target/...`
```

**Fix:**

```bash
chmod -R 755 target/
# Or
sudo chown -R $USER target/
```

## CI Build Matrix

The project CI builds for:

| OS | Target | Notes |
|----|--------|-------|
| Ubuntu 22.04 | x86_64-unknown-linux-gnu | Default |
| Ubuntu 22.04 | x86_64-unknown-linux-musl | Static |
| macOS 13 | x86_64-apple-darwin | Intel |
| macOS 13 | aarch64-apple-darwin | Apple Silicon |
| Windows 2022 | x86_64-pc-windows-msvc | MSVC |

## Related

- [Contributing Guide](./contributing.md)
- [Installation Guide](../installation.md)
- [Performance Tuning](../performance/tuning.md)
