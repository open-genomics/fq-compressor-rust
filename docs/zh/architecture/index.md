# 架构概览

fqc 的高性能 FASTQ 压缩器架构设计。

## 核心组件

### 1. 压缩管道
```
FASTQ 输入 → 解析 → 优化 → 压缩 → FQC 输出
```

### 2. 解压管道
```
FQC 输入 → 读取 → 解压 → 重组 → FASTQ 输出
```

## 模块结构

```
src/
├── main.rs           # CLI 入口
├── lib.rs            # 库导出
├── types.rs          # 核心类型
├── error.rs          # 错误处理
├── format.rs         # 二进制格式
├── algo/             # 压缩算法
├── commands/         # CLI 命令
├── pipeline/         # 管道处理
├── fastq/            # FASTQ 解析
└── io/               # I/O 操作
```

## 关键设计决策

- **无 unsafe 代码** - 内存安全保证
- **MSRV 1.75** - 最小 Rust 版本支持
- **并行处理** - 使用 rayon 进行多线程压缩
- **管道架构** - 三阶段并行处理

## 相关文档

- [算法文档](../algorithms/)
- [指南](../guide/what-is-fqc.md)
