---
layout: home
---

<div class="home-header">
  <div class="home-header-left">
    <div class="home-logo">FQ</div>
    <div>
      <span class="home-title">fqc</span>
      <span class="home-subtitle">块索引 FASTQ 压缩工具</span>
    </div>
  </div>
  <div class="home-nav">
    <a href="./architecture/">架构</a>
    <a href="./algorithms/">算法</a>
    <a href="./benchmarks/performance-report">基准测试</a>
    <a href="https://github.com/LessUp/fq-compressor-rust">GitHub</a>
    <a href="../en/">English</a>
  </div>
</div>

<div class="home-intro-row">
  <div class="home-intro">
    fqc 是一款用 Rust 编写的高性能 FASTQ 压缩工具。采用自定义块索引 <code>.fqc</code> 归档格式，支持组件级流编码、随机访问、并行压缩和灵活的内存控制。专为需要压缩效率与运维工具兼备的生物信息学工作流而设计。
  </div>
  <div class="home-stats">
    <span><strong>Rust</strong> 原生</span>
    <span><strong>2.4x+</strong> 压缩比</span>
    <span><strong>块</strong>索引</span>
    <span><strong>3</strong> 种模式</span>
  </div>
</div>

## 技术亮点

<div class="feature-map">
  <div class="feature-card">
    <div class="feature-card-title">架构与设计</div>
    <div class="feature-card-desc">
      分层系统架构，包含 CLI、I/O、格式、算法和流水线五层。三份架构决策记录记录了关键设计选择。
    </div>
    <div class="feature-tags">
      <a href="./architecture/" class="feature-tag">概述</a>
      <a href="./architecture/decisions/" class="feature-tag">ADR</a>
      <a href="./architecture/performance-roadmap" class="feature-tag">路线图</a>
    </div>
  </div>

  <div class="feature-card">
    <div class="feature-card-title">算法</div>
    <div class="feature-card-desc">
      ABC（锚定压缩）用于短读段，含 contig 构建和 delta 编码。SCM 算术编码用于质量值。按组件自适应选择编解码器。
    </div>
    <div class="feature-tags">
      <a href="./algorithms/" class="feature-tag">概述</a>
      <a href="./algorithms/abc-deep-dive" class="feature-tag">ABC 深度解析</a>
    </div>
  </div>

  <div class="feature-card">
    <div class="feature-card-title">二进制格式</div>
    <div class="feature-card-desc">
      自定义 <code>.fqc</code> 容器，含 magic header、块级校验和（xxHash64）、尾部索引实现 O(log N) 随机访问，以及前向兼容版本控制。
    </div>
    <div class="feature-tags">
      <a href="./reference/format-spec" class="feature-tag">格式规范</a>
    </div>
  </div>

  <div class="feature-card">
    <div class="feature-card-title">执行模式</div>
    <div class="feature-card-desc">
      Archive 模式全局重排获取最佳压缩比，Streaming 模式限制内存使用，Pipeline 模式分阶段吞吐。三种模式输出相同的 <code>.fqc</code> 文件。
    </div>
    <div class="feature-tags">
      <a href="./guide/modes" class="feature-tag">模式指南</a>
      <a href="./architecture/performance-roadmap" class="feature-tag">性能</a>
    </div>
  </div>

  <div class="feature-card">
    <div class="feature-card-title">基准测试</div>
    <div class="feature-card-desc">
      测试数据 2.39x 压缩比，亚 100ms 操作。基于 Criterion 的解析器吞吐量和完整归档工作流基准测试。
    </div>
    <div class="feature-tags">
      <a href="./benchmarks/performance-report" class="feature-tag">报告</a>
    </div>
  </div>

  <div class="feature-card">
    <div class="feature-card-title">CLI 参考</div>
    <div class="feature-card-desc">
      四个命令：compress、decompress、info、verify。支持双端测序、压缩输入检测、内存预算管理。
    </div>
    <div class="feature-tags">
      <a href="./guide/cli" class="feature-tag">CLI 文档</a>
      <a href="./guide/quick-start" class="feature-tag">快速开始</a>
    </div>
  </div>
</div>

## 快速开始

<div class="quick-start">
  <div class="quick-start-title">安装并压缩</div>
  <div class="quick-start-content">
    <div class="command-block">
      <code>cargo build --release && ./target/release/fqc compress -i reads.fastq -o reads.fqc</code>
    </div>
    查看<a href="./guide/quick-start">快速开始指南</a>了解安装选项和首次压缩步骤。
  </div>
</div>

## 架构深度解析

<div class="architecture-teaser">
  <div class="architecture-teaser-title">探索架构设计</div>
  <div class="architecture-teaser-desc">
    fqc 采用分层架构，支持组件级流编码、块级索引和三种执行模式。文档包含 29 个交互式 Mermaid 图表，覆盖从 CLI 到二进制格式的每一层。
  </div>
  <a href="./architecture/" class="architecture-teaser-link">查看架构文档 &rarr;</a>
</div>
