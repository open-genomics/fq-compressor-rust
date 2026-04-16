---
layout: home

hero:
  name: "fqc"
  text: "High-Performance\nFASTQ Compressor"
  tagline: Written in Rust with ABC algorithm. 3.9x compression ratio, ~60 MB/s decompression.
  image:
    src: /logo.svg
    alt: fqc logo
  actions:
    - theme: brand
      text: Get Started
      link: /guide/what-is-fqc
    - theme: alt
      text: Installation
      link: /guide/installation
    - theme: alt
      text: View on GitHub
      link: https://github.com/LessUp/fq-compressor-rust

features:
  - icon: 🧬
    title: ABC Algorithm
    details: Alignment-Based Compression with consensus + delta encoding for short reads (<300bp). Achieves 3.9x compression ratio on Illumina data.
  
  - icon: ⚡
    title: High Performance
    details: Parallel processing with Rayon. ~10 MB/s compression, ~60 MB/s decompression. 3-stage pipeline mode for maximum throughput.
  
  - icon: 📦
    title: SCM Quality Compression
    details: Statistical Context Model with Order-1/2 arithmetic coding. Lossless, Illumina8Bin, or Discard modes.
  
  - icon: 🔀
    title: Global Reordering
    details: Minimizer-based read reordering clusters similar sequences, significantly improving compression ratio.
  
  - icon: 🎯
    title: Random Access
    details: Block-indexed archive format enables efficient partial decompression and read range extraction.
  
  - icon: 🔧
    title: Production Ready
    details: 131 tests, CI/CD, multi-platform binaries (Linux, macOS, Windows), Docker support.
---

<style>
:root {
  --vp-home-hero-name-color: transparent;
  --vp-home-hero-name-background: -webkit-linear-gradient(120deg, #646cff 30%, #2dd4bf);
  --vp-home-hero-image-background-image: linear-gradient(-45deg, #646cff 50%, #2dd4bf 50%);
  --vp-home-hero-image-filter: blur(44px);
}
</style>

## Quick Start

::: code-group

```bash [Install]
# From source
git clone https://github.com/LessUp/fq-compressor-rust.git
cd fq-compressor-rust
cargo build --release

# Or download binary
# https://github.com/LessUp/fq-compressor-rust/releases
```

```bash [Compress]
# Basic compression
fqc compress -i reads.fastq -o reads.fqc

# Pipeline mode for speed
fqc compress -i reads.fastq -o reads.fqc --pipeline

# Paired-end
fqc compress -i R1.fastq -2 R2.fastq -o paired.fqc
```

```bash [Decompress]
# Full decompression
fqc decompress -i reads.fqc -o reads.fastq

# Extract range
fqc decompress -i reads.fqc -o subset.fastq --range 1:1000
```

:::

## Compression Ratio

| Read Type | Original | fqc | Ratio |
|-----------|----------|-----|-------|
| Illumina PE (2.27M reads) | 511 MB | 131 MB | **3.9x** |
| Nanopore (10kbp+) | 1.2 GB | 380 MB | **3.2x** |
| PacBio HiFi | 890 MB | 245 MB | **3.6x** |

*Tested on Intel Core i7-9700 @ 3.0GHz*

## Resources

- [Architecture](/architecture/) - System design and data flow
- [Algorithms](/algorithms/) - ABC, SCM, and compression strategies
- [API Reference](/guide/cli/compress) - CLI documentation
- [Contributing](https://github.com/LessUp/fq-compressor-rust/blob/master/CONTRIBUTING.md) - How to contribute
