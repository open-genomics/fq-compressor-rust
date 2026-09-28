# 快速开始

## 1. 构建二进制

```bash
cargo build --release
```

## 2. 压缩 FASTQ 文件

```bash
./target/release/fqc compress -i reads.fastq -o reads.fqc
```

常用变体：

```bash
./target/release/fqc compress -i reads.fastq -o reads.fqc --pipeline
./target/release/fqc compress -i reads.fastq -o reads.fqc --streaming
./target/release/fqc compress -i reads_R1.fastq -2 reads_R2.fastq -o paired.fqc
```

## 3. 检查和验证结果

```bash
./target/release/fqc info -i reads.fqc --detailed
./target/release/fqc verify -i reads.fqc
```

## 4. 解压

```bash
./target/release/fqc decompress -i reads.fqc -o restored.fastq
```

常用变体：

```bash
./target/release/fqc decompress -i reads.fqc -o subset.fastq --range 1:1000
./target/release/fqc decompress -i reads.fqc -o restored.fastq --original-order
./target/release/fqc decompress -i paired.fqc -o paired.fastq --split-pe
```
