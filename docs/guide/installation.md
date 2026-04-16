# Installation

## System Requirements

- **Linux**: glibc 2.31+ or musl (x64, ARM64)
- **macOS**: 10.15+ (Intel or Apple Silicon)
- **Windows**: Windows 10/11 (x64)
- **Rust**: 1.75+ (for building from source)

## Pre-built Binaries

### Download from GitHub Releases

```bash
# Linux x64
curl -LO https://github.com/LessUp/fq-compressor-rust/releases/latest/download/fqc-x86_64-unknown-linux-gnu.tar.gz
tar -xzf fqc-x86_64-unknown-linux-gnu.tar.gz
sudo mv fqc /usr/local/bin/

# macOS (Apple Silicon)
curl -LO https://github.com/LessUp/fq-compressor-rust/releases/latest/download/fqc-aarch64-apple-darwin.tar.gz
tar -xzf fqc-aarch64-apple-darwin.tar.gz
sudo mv fqc /usr/local/bin/
```

Available targets:
- `x86_64-unknown-linux-gnu` - Linux x64 (glibc)
- `x86_64-unknown-linux-musl` - Linux x64 (static)
- `aarch64-unknown-linux-gnu` - Linux ARM64
- `aarch64-unknown-linux-musl` - Linux ARM64 (static)
- `x86_64-apple-darwin` - macOS Intel
- `aarch64-apple-darwin` - macOS Apple Silicon
- `x86_64-pc-windows-msvc` - Windows x64

## Docker

```bash
# Pull from GitHub Container Registry
docker pull ghcr.io/lessup/fq-compressor-rust:latest

# Run
docker run --rm -v $(pwd):/data ghcr.io/lessup/fq-compressor-rust:latest \
  compress -i /data/reads.fastq -o /data/reads.fqc
```

## Build from Source

### Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Clone and Build

```bash
git clone https://github.com/LessUp/fq-compressor-rust.git
cd fq-compressor-rust
cargo build --release

# Binary will be at ./target/release/fqc
./target/release/fqc --version
```

### Build Options

```bash
# Native CPU optimizations (AVX2, SSE4.2)
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Minimal binary (no gzip/bzip2/xz support)
cargo build --release --no-default-features

# With debug symbols (for profiling)
cargo build --profile release-with-debug
```

## Verify Installation

```bash
fqc --version
# fqc 0.1.1

fqc --help
# Shows all available commands
```

## Shell Completions

```bash
# Bash
fqc --help | grep -A 100 "COMMANDS:" > /dev/null  # Manual for now

# Coming soon: native shell completion generation
```

## Next Steps

- [Quick Start](./quick-start.md) - Compress your first file
- [CLI Reference](./cli/compress.md) - Full command documentation
