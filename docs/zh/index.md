---
layout: home
---

<div class="academic-badge">
  <span class="academic-badge-item highlight">技术白皮书</span>
  <span class="academic-badge-item">v0.1.1</span>
  <span class="academic-badge-item">Rust 1.75+</span>
</div>

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

<div class="abstract-section">
  <div class="abstract-title">摘要</div>
  <div class="abstract-content">
    fqc 是一款领域感知的 FASTQ 压缩工具，通过块索引归档、组件级编码（ABC、SCM、Zstd）实现 <mark>2.4× 压缩比</mark>，支持 <mark>O(log N) 随机访问</mark>。采用纯 Rust 编写，零 unsafe 代码，支持三种执行模式（Archive、Streaming、Pipeline）灵活权衡内存与吞吐量。专为需要压缩效率与运维工具兼备的生物信息学工作流而设计。
  </div>
</div>

<div class="stats-bar">
  <div class="stat-item">
    <span class="stat-value">2.4×</span>
    <span class="stat-label">压缩比</span>
  </div>
  <div class="stat-item">
    <span class="stat-value">O(log N)</span>
    <span class="stat-label">随机访问</span>
  </div>
  <div class="stat-item">
    <span class="stat-value">3</span>
    <span class="stat-label">执行模式</span>
  </div>
  <div class="stat-item">
    <span class="stat-value">0</span>
    <span class="stat-label">unsafe 代码</span>
  </div>
</div>

## 核心架构

<div class="core-architecture">

```mermaid
flowchart LR
    subgraph Input[输入]
        A[FASTQ 文件]
    end
    
    subgraph Parser[解析器]
        B[记录读取器]
        C[块分组器]
    end
    
    subgraph Encoder[编码器]
        D{读长判断}
        E[ABC 编解码器<br/>短读长]
        F[Zstd 编解码器<br/>中等]
        G[Zstd Large<br/>长读长]
    end
    
    subgraph Archive[归档]
        H[.fqc 容器]
        I[块索引]
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

<div class="terminal-block">
  <div class="terminal-header">
    <span class="terminal-dot red"></span>
    <span class="terminal-dot yellow"></span>
    <span class="terminal-dot green"></span>
    <span class="terminal-title">bash</span>
  </div>
  <div class="terminal-body">
    <span class="terminal-line"><span class="terminal-prompt">$</span> <span class="terminal-comment"># 从源码构建</span></span>
    <span class="terminal-line"><span class="terminal-prompt">$</span> <span class="terminal-command">cargo build --release</span></span>
    <span class="terminal-line"></span>
    <span class="terminal-line"><span class="terminal-prompt">$</span> <span class="terminal-comment"># 压缩 FASTQ 文件</span></span>
    <span class="terminal-line"><span class="terminal-prompt">$</span> <span class="terminal-command">./target/release/fqc compress -i reads.fq -o reads.fqc</span></span>
    <span class="terminal-line"></span>
    <span class="terminal-line"><span class="terminal-prompt">$</span> <span class="terminal-comment"># 解压</span></span>
    <span class="terminal-line"><span class="terminal-prompt">$</span> <span class="terminal-command">./target/release/fqc decompress -i reads.fqc -o reads.fq</span></span>
  </div>
</div>

查看<a href="./guide/quick-start">快速开始指南</a>了解安装选项和首次压缩步骤。

## 资源

<div class="resources-section">
  <a href="./architecture/" class="resource-item">
    <span class="resource-icon">📊</span>
    <span>29 个交互式图表</span>
  </a>
  <a href="./architecture/decisions/" class="resource-item">
    <span class="resource-icon">📋</span>
    <span>3 份架构决策记录</span>
  </a>
  <a href="./reference/format-spec" class="resource-item">
    <span class="resource-icon">📄</span>
    <span>二进制格式规范</span>
  </a>
  <a href="./references/" class="resource-item">
    <span class="resource-icon">📚</span>
    <span>参考文献与相关工作</span>
  </a>
</div>
