---
layout: home

hero:
  name: "fqc"
  text: "高性能\nFASTQ 压缩器"
  tagline: 使用 Rust 编写，采用 ABC 算法。3.9倍压缩比，~60 MB/s 解压速度。
  image:
    src: /logo.svg
    alt: fqc logo
  actions:
    - theme: brand
      text: 开始使用
      link: /zh/guide/what-is-fqc
    - theme: alt
      text: 安装指南
      link: /zh/guide/installation
    - theme: alt
      text: GitHub 仓库
      link: https://github.com/LessUp/fq-compressor-rust

features:
  - icon: 🧬
    title: ABC 算法
    details: 基于对齐的压缩，使用共识序列 + 增量编码处理短读段（<300bp）。Illumina 数据可达 3.9 倍压缩比。
  
  - icon: ⚡
    title: 高性能
    details: 基于 Rayon 的并行处理。压缩速度 ~10 MB/s，解压速度 ~60 MB/s。3 阶段流水线模式提供最大吞吐。
  
  - icon: 📦
    title: SCM 质量压缩
    details: 统计上下文模型配合一阶/二阶算术编码。支持无损、Illumina8Bin 或丢弃模式。
  
  - icon: 🔀
    title: 全局重排序
    details: 基于 minimizer 的读段重排序将相似序列聚集在一起，显著提升压缩比。
  
  - icon: 🎯
    title: 随机访问
    details: 块索引归档格式支持高效的部分解压和读段范围提取。
  
  - icon: 🔧
    title: 生产就绪
    details: 131 个测试，CI/CD，多平台二进制文件（Linux、macOS、Windows），Docker 支持。
---

<style>
:root {
  --vp-home-hero-name-color: transparent;
  --vp-home-hero-name-background: -webkit-linear-gradient(120deg, #646cff 30%, #2dd4bf);
  --vp-home-hero-image-background-image: linear-gradient(-45deg, #646cff 50%, #2dd4bf 50%);
  --vp-home-hero-image-filter: blur(44px);
}
</style>

## 快速开始

::: code-group

```bash [安装]
# 从源码构建
git clone https://github.com/LessUp/fq-compressor-rust.git
cd fq-compressor-rust
cargo build --release

# 或下载预编译二进制
# https://github.com/LessUp/fq-compressor-rust/releases
```

```bash [压缩]
# 基本压缩
fqc compress -i reads.fastq -o reads.fqc

# 流水线模式提速
fqc compress -i reads.fastq -o reads.fqc --pipeline

# 配对端
fqc compress -i R1.fastq -2 R2.fastq -o paired.fqc
```

```bash [解压]
# 完整解压
fqc decompress -i reads.fqc -o reads.fastq

# 提取范围
fqc decompress -i reads.fqc -o subset.fastq --range 1:1000
```

:::

## 压缩比

| 读段类型 | 原始大小 | fqc | 压缩比 |
|-----------|----------|-----|-------|
| Illumina 配对端 (2.27M reads) | 511 MB | 131 MB | **3.9x** |
| Nanopore (10kbp+) | 1.2 GB | 380 MB | **3.2x** |
| PacBio HiFi | 890 MB | 245 MB | **3.6x** |

*测试环境：Intel Core i7-9700 @ 3.0GHz*

## 资源

- [架构设计](/zh/architecture/) - 系统设计与数据流
- [算法详解](/zh/algorithms/) - ABC、SCM 与压缩策略
- [API 参考](/zh/guide/cli/compress) - CLI 文档
- [贡献指南](https://github.com/LessUp/fq-compressor-rust/blob/master/CONTRIBUTING.md)
