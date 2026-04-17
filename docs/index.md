---
layout: home
hero:
  name: 'fqc'
  text: 'High-Performance FASTQ Compressor'
  tagline: Compress genomic sequencing data with ABC algorithm and Zstd
  image:
    src: /logo.svg
    alt: fqc logo
  actions:
    - theme: brand
      text: Get Started
      link: /guide/what-is-fqc
    - theme: alt
      text: View on GitHub
      link: https://github.com/LessUp/fq-compressor-rust

features:
  - icon: 🧬
    title: ABC Algorithm
    details: Consensus-based delta encoding optimized for short reads (< 300bp)
  - icon: ⚡
    title: High Performance
    details: Parallel processing with 3-stage pipeline for maximum throughput
  - icon: 📦
    title: Flexible Modes
    details: Streaming, batch, and pipeline modes with configurable quality settings
  - icon: 🔍
    title: Random Access
    details: Block-indexed format enables efficient partial decompression
---
