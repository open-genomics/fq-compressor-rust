# 快速开始

使用 fqc 压缩 FASTQ 文件的基础指南。

## 安装

### 使用预编译二进制
从 [GitHub Releases](https://github.com/LessUp/fq-compressor-rust/releases) 下载适合您平台的版本。

### 从源码编译
```bash
git clone https://github.com/LessUp/fq-compressor-rust.git
cd fq-compressor-rust
cargo build --release
```

## 基本用法

### 压缩 FASTQ 文件
```bash
fqc compress input.fastq -o output.fqc
```

### 解压 FQC 文件
```bash
fqc decompress input.fqc -o output.fastq
```

### 查看归档信息
```bash
fqc info archive.fqc
```

### 验证完整性
```bash
fqc verify archive.fqc
```

## 压缩模式

| 模式 | 命令 | 说明 |
|------|------|------|
| 默认 | `fqc compress` | 自动选择最佳算法 |
| 流式 | `--streaming` | 低内存使用 |
| 管道 | `--pipeline` | 高性能，多线程 |

## 下一步

- 查看 [安装指南](installation.md) 获取详细安装说明
- 了解 [CLI 命令](cli/compress.md) 的完整选项
- 查看 [算法文档](../algorithms/) 了解压缩原理

## 相关文档

- [什么是 FQC?](what-is-fqc.md)
- [安装](installation.md)
