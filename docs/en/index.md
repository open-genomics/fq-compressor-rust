---
layout: home
---

<div class="academic-badge">
  <span class="academic-badge-item highlight">Technical Whitepaper</span>
  <span class="academic-badge-item">v0.1.1</span>
  <span class="academic-badge-item">Rust 1.75+</span>
  <span class="academic-badge-item">Zero unsafe</span>
</div>

<div class="home-header">
  <div class="home-header-left">
    <div class="home-logo">FQ</div>
    <div>
      <span class="home-title">fqc</span>
      <span class="home-subtitle">Block-Indexed FASTQ Compression</span>
    </div>
  </div>
  <div class="home-nav">
    <a href="./whitepaper">Whitepaper</a>
    <a href="./architecture/">Architecture</a>
    <a href="./algorithms/">Algorithms</a>
    <a href="./benchmarks/performance-report">Benchmarks</a>
    <a href="https://github.com/LessUp/fq-compressor-rust">GitHub</a>
    <a href="../zh/">中文</a>
  </div>
</div>

<div class="abstract-section">
  <div class="abstract-title">Abstract</div>
  <div class="abstract-content">
    <strong>fqc</strong> is a domain-aware FASTQ compression engine that achieves <mark>2.4&times; compression ratio</mark> through block-indexed archiving, component-specific encoding (ABC consensus/delta, SCM quality, Zstd sequence), and <mark>O(log N) random access</mark>. Written in pure Rust with zero unsafe code, it introduces three execution modes &mdash; Archive, Streaming, and Pipeline &mdash; enabling flexible memory-throughput trade-offs without format fragmentation. Designed for bioinformatics workflows requiring both compression efficiency and operational tooling.
  </div>
</div>

<div class="stats-bar">
  <div class="stat-item">
    <span class="stat-value">2.4&times;</span>
    <span class="stat-label">Compression Ratio</span>
  </div>
  <div class="stat-item">
    <span class="stat-value">O(log N)</span>
    <span class="stat-label">Random Access</span>
  </div>
  <div class="stat-item">
    <span class="stat-value">3</span>
    <span class="stat-label">Execution Modes</span>
  </div>
  <div class="stat-item">
    <span class="stat-value">0</span>
    <span class="stat-label">unsafe Code</span>
  </div>
</div>

## Core Architecture

<div class="core-architecture">

```mermaid
flowchart LR
    subgraph Input
        A[FASTQ File]
    end
    
    subgraph Parser
        B[Record Reader]
        C[Block Grouper]
    end
    
    subgraph Encoder
        D{Read Length?}
        E[ABC Codec<br/>Short Reads]
        F[Zstd Codec<br/>Medium]
        G[Zstd Large<br/>Long Reads]
    end
    
    subgraph Archive
        H[.fqc Container]
        I[Block Index]
    end
    
    A --> B --> C --> D
    D -->|&le;511 bp| E
    D -->|512bp-10KB| F
    D -->|&gt;10KB| G
    E --> H
    F --> H
    G --> H
    H --> I
```

</div>

## Why fqc?

<div class="feature-map">
  <div class="feature-card">
    <div class="feature-card-title">Block-Indexed Random Access</div>
    <div class="feature-card-desc">
      Unlike stream-based compressors (gzip, DSRC), fqc's footer-embedded block index enables O(log N) range queries without full archive decompression.
    </div>
    <div class="feature-tags">
      <a href="./reference/format-spec" class="feature-tag">Format Spec</a>
      <a href="./architecture/" class="feature-tag">Architecture</a>
    </div>
  </div>

  <div class="feature-card">
    <div class="feature-card-title">Component-Specific Encoding</div>
    <div class="feature-card-desc">
      Each FASTQ component (ID, sequence, quality, aux) uses an independently tuned codec. Sequence gets ABC or Zstd; quality gets lossless, Illumina8 binning, QVZ, or discard.
    </div>
    <div class="feature-tags">
      <a href="./algorithms/" class="feature-tag">Algorithms</a>
      <a href="./algorithms/abc-deep-dive" class="feature-tag">ABC Deep Dive</a>
    </div>
  </div>

  <div class="feature-card">
    <div class="feature-card-title">Three Modes, One Format</div>
    <div class="feature-card-desc">
      Archive, Streaming, and Pipeline modes all produce identical .fqc output. Trade memory for ratio or throughput without creating format fragmentation.
    </div>
    <div class="feature-tags">
      <a href="./guide/modes" class="feature-tag">Modes Guide</a>
      <a href="./architecture/decisions/002-three-execution-modes" class="feature-tag">ADR-002</a>
    </div>
  </div>

  <div class="feature-card">
    <div class="feature-card-title">Operational Tooling</div>
    <div class="feature-card-desc">
      A single binary provides compress, decompress, info, and verify commands. No sidecar scripts needed. CI/CD friendly with structured exit codes.
    </div>
    <div class="feature-tags">
      <a href="./guide/cli" class="feature-tag">CLI Docs</a>
      <a href="./guide/quick-start" class="feature-tag">Quick Start</a>
    </div>
  </div>

  <div class="feature-card">
    <div class="feature-card-title">Domain-Aware ABC Algorithm</div>
    <div class="feature-card-desc">
      Anchor-Based Compression exploits short-read similarity through contig building and delta encoding, outperforming generic LZ on biological sequences.
    </div>
    <div class="feature-tags">
      <a href="./theory" class="feature-tag">Theory</a>
      <a href="./algorithms/abc-deep-dive" class="feature-tag">Deep Dive</a>
    </div>
  </div>

  <div class="feature-card">
    <div class="feature-card-title">Pure Rust, Zero unsafe</div>
    <div class="feature-card-desc">
      Memory-safe by construction. No undefined behavior surface. MSRV 1.75.0. Single static binary with no runtime dependencies beyond libc.
    </div>
    <div class="feature-tags">
      <a href="./comparison" class="feature-tag">Comparison</a>
    </div>
  </div>
</div>

## Quick Start

<div class="terminal-block">
  <div class="terminal-header">
    <span class="terminal-dot red"></span>
    <span class="terminal-dot yellow"></span>
    <span class="terminal-dot green"></span>
    <span class="terminal-title">bash</span>
  </div>
  <div class="terminal-body">
    <span class="terminal-line"><span class="terminal-prompt">$</span> <span class="terminal-comment"># Build from source</span></span>
    <span class="terminal-line"><span class="terminal-prompt">$</span> <span class="terminal-command">cargo build --release</span></span>
    <span class="terminal-line"></span>
    <span class="terminal-line"><span class="terminal-prompt">$</span> <span class="terminal-comment"># Compress a FASTQ file</span></span>
    <span class="terminal-line"><span class="terminal-prompt">$</span> <span class="terminal-command">./target/release/fqc compress -i reads.fq -o reads.fqc</span></span>
    <span class="terminal-line"></span>
    <span class="terminal-line"><span class="terminal-prompt">$</span> <span class="terminal-comment"># Decompress</span></span>
    <span class="terminal-line"><span class="terminal-prompt">$</span> <span class="terminal-command">./target/release/fqc decompress -i reads.fqc -o reads.fq</span></span>
    <span class="terminal-line"></span>
    <span class="terminal-line"><span class="terminal-prompt">$</span> <span class="terminal-comment"># Inspect archive</span></span>
    <span class="terminal-line"><span class="terminal-prompt">$</span> <span class="terminal-command">./target/release/fqc info -i reads.fqc</span></span>
  </div>
</div>

See the [Quick Start guide](./guide/quick-start) for installation options and first compression steps.

## Research Context

<div class="highlight-box">

<strong>fqc</strong> is positioned at the intersection of domain-specific compression and modern systems programming. It draws on decades of research in DNA compression (LZ variants, reference-based encoding) while applying contemporary software engineering practices: memory safety, structured concurrency, and format stability.

For a survey of the field and how fqc compares to prior art, see [References & Related Work](./references/) and [Competitive Analysis](./comparison).

</div>

## Resources

<div class="resources-section">
  <a href="./whitepaper" class="resource-item">
    <span class="resource-icon">&#128221;</span>
    <span>Technical Whitepaper</span>
  </a>
  <a href="./architecture/" class="resource-item">
    <span class="resource-icon">&#128202;</span>
    <span>Architecture Deep Dive</span>
  </a>
  <a href="./architecture/decisions/" class="resource-item">
    <span class="resource-icon">&#128203;</span>
    <span>3 Architecture Decision Records</span>
  </a>
  <a href="./reference/format-spec" class="resource-item">
    <span class="resource-icon">&#128196;</span>
    <span>Binary Format Specification</span>
  </a>
  <a href="./theory" class="resource-item">
    <span class="resource-icon">&#128300;</span>
    <span>Algorithmic Theory</span>
  </a>
  <a href="./comparison" class="resource-item">
    <span class="resource-icon">&#128200;</span>
    <span>Competitive Analysis</span>
  </a>
  <a href="./references/" class="resource-item">
    <span class="resource-icon">&#128218;</span>
    <span>References & Related Work</span>
  </a>
</div>
