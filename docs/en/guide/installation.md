# Installation

## Release binaries

Prebuilt binaries are published on the
[GitHub Releases](https://github.com/LessUp/fq-compressor-rust/releases) page.

Current release automation targets:

- Linux x86_64 (`gnu` and `musl`)
- macOS Intel
- macOS Apple Silicon
- Windows x86_64

## Build from source

Requirements:

- Rust **1.75.0**
- Git

```bash
git clone https://github.com/LessUp/fq-compressor-rust.git
cd fq-compressor-rust
cargo build --release
./target/release/fqc --help
```

## Local install

```bash
cargo install --path .
```

## Container image

The repository includes a `Dockerfile` for local or CI builds:

```bash
docker build -t fqc .
docker run --rm -v "$(pwd):/data" fqc --help
```
