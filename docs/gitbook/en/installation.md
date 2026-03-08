# Installation

## Prerequisites

- **Rust 1.75+** (see [rustup.rs](https://rustup.rs/))
- **Git**
- System dependencies for compressed input support:
  - Debian/Ubuntu: `sudo apt install libbz2-dev liblzma-dev pkg-config`
  - macOS: `brew install xz`

## From Source

```bash
git clone https://github.com/lessup/fq-compressor-rust.git
cd fq-compressor-rust
cargo build --release
```

The binary will be at `target/release/fqc` (or `fqc.exe` on Windows).

## Native CPU Build

For maximum performance on your specific CPU:

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

## Minimal Build

Disable optional compressed input formats to reduce binary size:

```bash
# Zstd only (no gz/bz2/xz input support)
cargo build --release --no-default-features

# Only gzip input support
cargo build --release --no-default-features --features gz
```

## Docker

```bash
# Pull from GitHub Container Registry
docker pull ghcr.io/lessup/fq-compressor-rust:latest

# Or build locally
docker build -t fqc .

# Run
docker run --rm -v $(pwd):/data fqc compress -i /data/reads.fastq -o /data/reads.fqc
```

## Verify Installation

```bash
fqc --version
fqc --help
```
