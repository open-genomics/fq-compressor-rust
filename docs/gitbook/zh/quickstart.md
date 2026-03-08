# 快速开始

## 压缩 FASTQ 文件

```bash
fqc compress -i reads.fastq -o reads.fqc
```

自动检测读段长度并选择最优压缩策略。

## 解压

```bash
fqc decompress -i reads.fqc -o reads.fastq
```

## 查看归档信息

```bash
fqc info -i reads.fqc
```

## 验证完整性

```bash
fqc verify -i reads.fqc
```

## 常见场景

### 压缩输入

fqc 透明处理压缩格式的 FASTQ 文件：

```bash
fqc compress -i reads.fastq.gz -o reads.fqc
fqc compress -i reads.fastq.bz2 -o reads.fqc
```

### Pipeline 模式

对大文件使用 3 阶段流水线以获得更高吞吐：

```bash
fqc compress -i reads.fastq -o reads.fqc --pipeline
fqc decompress -i reads.fqc -o reads.fastq --pipeline
```

### 配对端数据

```bash
# 独立文件
fqc compress -i reads_R1.fastq -2 reads_R2.fastq -o reads.fqc

# 交错格式
fqc compress -i interleaved.fastq -o reads.fqc --interleaved
```

### 有损质量压缩

```bash
# Illumina 8-bin 量化（压缩比提升约 30%）
fqc compress -i reads.fastq -o reads.fqc --lossy-quality illumina8

# 丢弃质量（最大压缩）
fqc compress -i reads.fastq -o reads.fqc --lossy-quality discard
```

### 随机访问

```bash
# 提取第 1-1000 条读段
fqc decompress -i reads.fqc -o subset.fastq --range 1:1000
```
