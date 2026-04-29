---
layout: home

hero:
  name: fqc
  text: "FASTQ compression in Rust"
  tagline: "A block-indexed `.fqc` archive tool for compressing, restoring, inspecting, and verifying FASTQ data without turning project maintenance into a platform."
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
    title: "FASTQ-aware by design"
    details: "Sequences, qualities, read IDs, and paired-end layout are encoded as separate archive concerns instead of being hidden inside a generic compressed stream."
  - icon: 📦
    title: "Block-indexed archives"
    details: "The `.fqc` container keeps per-block metadata, a footer, and an index so `info`, `verify`, and range-oriented workflows have structure to work with."
  - icon: 🔍
    title: "Operational commands included"
    details: "The same binary ships `compress`, `decompress`, `info`, and `verify`; users do not need sidecar scripts to validate an archive."
  - icon: ⚙️
    title: "Explicit memory modes"
    details: "Default archive mode optimizes globally, `--streaming` favors strict memory control, and `--memory-limit 0` means automatic memory selection."
---

## Pick the right path

| Need | Command shape |
| --- | --- |
| Standard single-end archive | `fqc compress -i reads.fastq -o reads.fqc` |
| Low-memory compression | `fqc compress -i reads.fastq -o reads.fqc --streaming --memory-limit 1024` |
| Paired-end input | `fqc compress -i reads_R1.fastq -2 reads_R2.fastq -o paired.fqc` |
| Check an archive before use | `fqc verify -i reads.fqc` |
| Inspect codecs and blocks | `fqc info -i reads.fqc --detailed --show-codecs` |

Start with the [Quick Start](/guide/quick-start) if you want the shortest path from FASTQ to `.fqc`, or jump to the [CLI reference](/guide/cli) for flags and mode details.
