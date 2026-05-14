# Quick Start

## 1. Build the binary

```bash
cargo build --release
```

## 2. Compress a FASTQ file

```bash
./target/release/fqc compress -i reads.fastq -o reads.fqc
```

Useful variations:

```bash
./target/release/fqc compress -i reads.fastq -o reads.fqc --pipeline
./target/release/fqc compress -i reads.fastq -o reads.fqc --streaming
./target/release/fqc compress -i reads_R1.fastq -2 reads_R2.fastq -o paired.fqc
```

## 3. Inspect and verify the result

```bash
./target/release/fqc info -i reads.fqc --detailed
./target/release/fqc verify -i reads.fqc
```

## 4. Decompress

```bash
./target/release/fqc decompress -i reads.fqc -o restored.fastq
```

Useful variations:

```bash
./target/release/fqc decompress -i reads.fqc -o subset.fastq --range 1:1000
./target/release/fqc decompress -i reads.fqc -o restored.fastq --original-order
./target/release/fqc decompress -i paired.fqc -o paired.fastq --split-pe
```
