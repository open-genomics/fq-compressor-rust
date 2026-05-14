---
layout: home
---

<div class="academic-badge">
  <span class="academic-badge-item highlight">技术白皮书</span>
  <span class="academic-badge-item">v0.1.1</span>
  <span class="academic-badge-item">Rust 1.75+</span>
  <span class="academic-badge-item">零 unsafe</span>
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
    <a href="./whitepaper">白皮书</a>
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
    <strong>fqc</strong> 是一款领域感知的 FASTQ 压缩引擎，通过块索引归档、组件级编码（ABC 共识/差分编码、SCM 质量压缩、Zstd 序列压缩）实现 <mark>2.4 倍压缩比</mark>，并支持 <mark>O(log N) 随机访问</mark>。采用纯 Rust 编写，零 unsafe 代码，引入三种执行模式 &mdash; Archive、Streaming 和 Pipeline &mdash; 在不产生格式碎片的前提下灵活权衡内存与吞吐量。专为需要压缩效率与运维工具兼备的生物信息学工作流而设计。
  </div>
</div>

<div class="stats-bar">
  <div class="stat-item">
    <span class="stat-value">2.4&times;</span>
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
    D -->|&le;511 bp| E
    D -->|512bp-10KB| F
    D -->|&gt;10KB| G
    E --> H
    F --> H
    G --> H
    H --> I
```

</div>

## 为什么选择 fqc

<div class="feature-map">
  <div class="feature-card">
    <div class="feature-card-title">块索引随机访问</div>
    <div class="feature-card-desc">
      与基于流的压缩器（gzip、DSRC）不同，fqc 的尾部嵌入式块索引支持 O(log N) 范围查询，无需完整解压归档。
    </div>
    <div class="feature-tags">
      <a href="./reference/format-spec" class="feature-tag">格式规范</a>
      <a href="./architecture/" class="feature-tag">架构</a>
    </div>
  </div>

  <div class="feature-card">
    <div class="feature-card-title">组件级编码</div>
    <div class="feature-card-desc">
      FASTQ 的每个组件（ID、序列、质量值、辅助信息）使用独立调优的编解码器。序列采用 ABC 或 Zstd；质量值支持无损、Illumina8 分级、QVZ 或丢弃。
    </div>
    <div class="feature-tags">
      <a href="./algorithms/" class="feature-tag">算法</a>
      <a href="./algorithms/abc-deep-dive" class="feature-tag">ABC 深度解析</a>
    </div>
  </div>

  <div class="feature-card">
    <div class="feature-card-title">三种模式，一种格式</div>
    <div class="feature-card-desc">
      Archive、Streaming 和 Pipeline 模式均生成相同的 .fqc 输出。在内存、压缩比和吞吐量之间灵活权衡，不产生格式碎片。
    </div>
    <div class="feature-tags">
      <a href="./guide/modes" class="feature-tag">模式指南</a>
      <a href="./architecture/decisions/002-three-execution-modes" class="feature-tag">ADR-002</a>
    </div>
  </div>

  <div class="feature-card">
    <div class="feature-card-title">运维级工具链</div>
    <div class="feature-card-desc">
      单一二进制文件提供 compress、decompress、info 和 verify 四个命令。无需额外脚本。支持结构化退出码，CI/CD 友好。
    </div>
    <div class="feature-tags">
      <a href="./guide/cli" class="feature-tag">CLI 文档</a>
      <a href="./guide/quick-start" class="feature-tag">快速开始</a>
    </div>
  </div>

  <div class="feature-card">
    <div class="feature-card-title">领域感知 ABC 算法</div>
    <div class="feature-card-desc">
      锚定压缩（Anchor-Based Compression）通过 contig 构建和差分编码利用短读段相似性，在生物序列上超越通用 LZ 算法。
    </div>
    <div class="feature-tags">
      <a href="./theory" class="feature-tag">理论基础</a>
      <a href="./algorithms/abc-deep-dive" class="feature-tag">深度解析</a>
    </div>
  </div>

  <div class="feature-card">
    <div class="feature-card-title">纯 Rust，零 unsafe</div>
    <div class="feature-card-desc">
      内存安全由语言保证。无未定义行为风险。MSRV 1.75.0。单静态二进制文件，除 libc 外无运行时依赖。
    </div>
    <div class="feature-tags">
      <a href="./comparison" class="feature-tag">竞品对比</a>
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
    <span class="terminal-line"></span>
    <span class="terminal-line"><span class="terminal-prompt">$</span> <span class="terminal-comment"># 查看归档信息</span></span>
    <span class="terminal-line"><span class="terminal-prompt">$</span> <span class="terminal-command">./target/release/fqc info -i reads.fqc</span></span>
  </div>
</div>

查看<a href="./guide/quick-start">快速开始指南</a>了解安装选项和首次压缩步骤。

## 研究背景

<div class="highlight-box">

<strong>fqc</strong> 位于领域特定压缩与现代系统编程的交叉点。它借鉴了数十年来 DNA 压缩的研究成果（LZ 变体、基于参考的编码），同时应用了当代软件工程实践：内存安全、结构化并发和格式稳定性。

如需了解该领域的概况以及 fqc 与先前工作的比较，请参阅<a href="./references/">参考文献与相关工作</a>和<a href="./comparison">竞品深度对比</a>。

</div>

## 资源导航

<div class="resources-section">
  <a href="./whitepaper" class="resource-item">
    <span class="resource-icon">&#128221;</span>
    <span>技术白皮书</span>
  </a>
  <a href="./architecture/" class="resource-item">
    <span class="resource-icon">&#128202;</span>
    <span>架构深度解析</span>
  </a>
  <a href="./architecture/decisions/" class="resource-item">
    <span class="resource-icon">&#128203;</span>
    <span>3 份架构决策记录</span>
  </a>
  <a href="./reference/format-spec" class="resource-item">
    <span class="resource-icon">&#128196;</span>
    <span>二进制格式规范</span>
  </a>
  <a href="./theory" class="resource-item">
    <span class="resource-icon">&#128300;</span>
    <span>算法理论基础</span>
  </a>
  <a href="./comparison" class="resource-item">
    <span class="resource-icon">&#128200;</span>
    <span>竞品深度对比</span>
  </a>
  <a href="./references/" class="resource-item">
    <span class="resource-icon">&#128218;</span>
    <span>参考文献与相关工作</span>
  </a>
</div>
