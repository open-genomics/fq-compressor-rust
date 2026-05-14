---
layout: home
---

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

<div class="home-intro-row">
  <div class="home-intro">
    fqc is a high-performance FASTQ compression tool written in Rust. It uses a custom block-indexed <code>.fqc</code> archive format with component-specific stream encoding, enabling random access, parallel compression, and flexible memory control. Designed for bioinformatics workflows that demand both compression efficiency and operational tooling.
  </div>
  <div class="home-stats">
    <span><strong>Rust</strong> native</span>
    <span><strong>2.4x+</strong> ratio</span>
    <span><strong>Block</strong> indexed</span>
    <span><strong>3</strong> modes</span>
  </div>
</div>

## Technical Highlights

<div class="feature-map">
  <div class="feature-card">
    <div class="feature-card-title">Architecture &amp; Design</div>
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

<div class="quick-start">
  <div class="quick-start-title">Install and compress</div>
  <div class="quick-start-content">
    <div class="command-block">
      <code>cargo build --release && ./target/release/fqc compress -i reads.fastq -o reads.fqc</code>
    </div>
    See the <a href="./guide/quick-start">Quick Start guide</a> for installation options and first compression steps.
  </div>
</div>

## Architecture Deep Dive

<div class="architecture-teaser">
  <div class="architecture-teaser-title">Explore the Architecture</div>
  <div class="architecture-teaser-desc">
    fqc uses a layered architecture with component-specific stream encoding, block-level indexing, and three execution modes. The documentation includes 29 interactive Mermaid diagrams covering every layer from CLI to binary format.
  </div>
  <a href="./architecture/" class="architecture-teaser-link">View Architecture Documentation &rarr;</a>
</div>
