---
layout: home
---

<div class="academic-badge">
  <span class="academic-badge-item highlight">Technical Whitepaper</span>
  <span class="academic-badge-item">v0.1.1</span>
  <span class="academic-badge-item">Rust 1.75+</span>
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
    fqc is a domain-aware FASTQ compression tool that achieves <mark>2.4× compression ratio</mark> through block-indexed archiving, component-specific encoding (ABC, SCM, Zstd), and <mark>O(log N) random access</mark>. Written in pure Rust with zero unsafe code, it supports three execution modes (Archive, Streaming, Pipeline) for flexible memory-throughput trade-offs. Designed for bioinformatics workflows requiring both compression efficiency and operational tooling.
  </div>
</div>

<div class="stats-bar">
  <div class="stat-item">
    <span class="stat-value">2.4×</span>
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
    D -->|≤511 bp| E
    D -->|512bp-10KB| F
    D -->|>10KB| G
    E --> H
    F --> H
    G --> H
    H --> I
```

</div>

## Technical Highlights

<div class="feature-map">
  <div class="feature-card">
    <div class="feature-card-title">Architecture & Design</div>
    <div class="feature-card-desc">
      Layered system architecture with CLI, I/O, Format, Algorithm, and Pipeline layers. Three Architecture Decision Records document key design choices.
    </div>
    <div class="feature-tags">
      <a href="./architecture/" class="feature-tag">Overview</a>
      <a href="./architecture/decisions/" class="feature-tag">ADRs</a>
      <a href="./architecture/performance-roadmap" class="feature-tag">Roadmap</a>
    </div>
  </div>

  <div class="feature-card">
    <div class="feature-card-title">Algorithms</div>
    <div class="feature-card-desc">
      ABC (Anchor-Based Compression) for short reads with contig building and delta encoding. SCM arithmetic coding for quality scores. Adaptive codec selection per component.
    </div>
    <div class="feature-tags">
      <a href="./algorithms/" class="feature-tag">Overview</a>
      <a href="./algorithms/abc-deep-dive" class="feature-tag">ABC Deep Dive</a>
    </div>
  </div>

  <div class="feature-card">
    <div class="feature-card-title">Binary Format</div>
    <div class="feature-card-desc">
      Custom <code>.fqc</code> container with magic header, block-level checksums (xxHash64), footer index for O(log N) random access, and forward-compatible versioning.
    </div>
    <div class="feature-tags">
      <a href="./reference/format-spec" class="feature-tag">Format Spec</a>
    </div>
  </div>

  <div class="feature-card">
    <div class="feature-card-title">Execution Modes</div>
    <div class="feature-card-desc">
      Archive mode for best ratio with global reordering, Streaming for bounded memory, Pipeline for staged throughput. All produce identical <code>.fqc</code> output.
    </div>
    <div class="feature-tags">
      <a href="./guide/modes" class="feature-tag">Modes Guide</a>
      <a href="./architecture/performance-roadmap" class="feature-tag">Performance</a>
    </div>
  </div>

  <div class="feature-card">
    <div class="feature-card-title">Benchmarks</div>
    <div class="feature-card-desc">
      2.39x compression ratio on test data, sub-100ms operations. Criterion-based benchmarks for parser throughput and full archive workflows.
    </div>
    <div class="feature-tags">
      <a href="./benchmarks/performance-report" class="feature-tag">Report</a>
    </div>
  </div>

  <div class="feature-card">
    <div class="feature-card-title">CLI Reference</div>
    <div class="feature-card-desc">
      Four commands: compress, decompress, info, verify. Paired-end support, compressed input detection, memory budget management.
    </div>
    <div class="feature-tags">
      <a href="./guide/cli" class="feature-tag">CLI Docs</a>
      <a href="./guide/quick-start" class="feature-tag">Quick Start</a>
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
  </div>
</div>

See the [Quick Start guide](./guide/quick-start) for installation options and first compression steps.

## Resources

<div class="resources-section">
  <a href="./architecture/" class="resource-item">
    <span class="resource-icon">📊</span>
    <span>29 Interactive Diagrams</span>
  </a>
  <a href="./architecture/decisions/" class="resource-item">
    <span class="resource-icon">📋</span>
    <span>3 Architecture Decision Records</span>
  </a>
  <a href="./reference/format-spec" class="resource-item">
    <span class="resource-icon">📄</span>
    <span>Binary Format Specification</span>
  </a>
  <a href="./references/" class="resource-item">
    <span class="resource-icon">📚</span>
    <span>References & Related Work</span>
  </a>
</div>
