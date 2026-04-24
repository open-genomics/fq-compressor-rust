---
layout: home

hero:
  name: fqc
  text: "FASTQ compression in Rust"
  tagline: "A focused `.fqc` compressor with compression, decompression, inspection, and verification in one CLI."
  image:
    src: /logo.svg
    alt: fqc logo
  actions:
    - theme: brand
      text: Quick Start
      link: /guide/quick-start
    - theme: alt
      text: View on GitHub
      link: https://github.com/LessUp/fq-compressor-rust

features:
  - icon: 🧬
    title: "FASTQ-aware compression"
    details: "Short-read data uses an ABC-style path while medium and long reads use Zstd-backed compression."
  - icon: 📦
    title: "Block-indexed archives"
    details: "`.fqc` stores archive metadata per block so inspection and partial workflows stay practical."
  - icon: 🔍
    title: "Integrity tooling included"
    details: "`fqc info` and `fqc verify` are first-class commands rather than afterthought scripts."
  - icon: ⚙️
    title: "Lean project surface"
    details: "The repository is documented and automated around the current release line instead of speculative future scope."
---
