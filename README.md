# fqc

[![CI](https://github.com/LessUp/fq-compressor-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/LessUp/fq-compressor-rust/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/LessUp/fq-compressor-rust?label=release)](https://github.com/LessUp/fq-compressor-rust/releases)
[![License](https://img.shields.io/badge/license-GPL--3.0-green)](https://www.gnu.org/licenses/gpl-3.0.en.html)
[![Docs](https://img.shields.io/badge/docs-live-4f46e5)](https://lessup.github.io/fq-compressor-rust/)

`fqc` is a Rust FASTQ compressor built around a block-indexed `.fqc` archive format.
It combines a short-read ABC-style path, Zstd-backed medium/long-read compression, and quality-score coding into a single CLI for compression, decompression, inspection, and verification.

## Why use it

- **FASTQ-aware archive format** instead of a generic compressed blob
- **Block-level metadata** for inspection, verification, and partial workflows
- **Single binary CLI** with `compress`, `decompress`, `info`, and `verify`
- **Memory-safe Rust implementation** with a pinned MSRV of **1.75.0**

## Quick start

```bash
git clone https://github.com/LessUp/fq-compressor-rust.git
cd fq-compressor-rust
cargo build --release

./target/release/fqc compress -i tests/data/test_se.fastq -o sample.fqc
./target/release/fqc info -i sample.fqc
./target/release/fqc verify -i sample.fqc
./target/release/fqc decompress -i sample.fqc -o sample.fastq
```

## Common commands

```bash
fqc compress -i reads.fastq -o reads.fqc
fqc compress -i reads.fastq -o reads.fqc --pipeline
fqc compress -i reads.fastq -o reads.fqc --streaming
fqc compress -i reads_R1.fastq -2 reads_R2.fastq -o paired.fqc

fqc decompress -i reads.fqc -o reads.fastq
fqc decompress -i reads.fqc -o subset.fastq --range 1:1000
fqc decompress -i reads.fqc -o reads.fastq --original-order
fqc decompress -i paired.fqc -o paired.fastq --split-pe

fqc info -i reads.fqc --detailed --show-codecs
fqc verify -i reads.fqc
fqc verify -i reads.fqc --quick
```

## Documentation

- **Project site:** <https://lessup.github.io/fq-compressor-rust/>
- **Quick start:** [docs/guide/quick-start.md](docs/guide/quick-start.md)
- **CLI reference:** [docs/guide/cli.md](docs/guide/cli.md)
- **Architecture:** [docs/architecture/index.md](docs/architecture/index.md)
- **Algorithms:** [docs/algorithms/index.md](docs/algorithms/index.md)

## Development

This repository uses **OpenSpec** as its planning and change-management layer.

- living specs: [`openspec/specs/`](openspec/specs/)
- active change folders: [`openspec/changes/`](openspec/changes/)
- AI contributor guide: [`AGENTS.md`](AGENTS.md)

Validation commands:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib --tests
cargo doc --no-deps
npm run docs:build
```

To enable local Git hooks:

```bash
bash scripts/setup-hooks.sh
```
