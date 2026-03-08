# FAQ

## General

### What is fqc?

fqc is a high-performance FASTQ compressor written in Rust. It uses the ABC (Alignment-Based Compression) algorithm for short reads and Zstd for medium/long reads.

### How does fqc differ from the C++ version (fq-compressor)?

Both versions share the same `.fqc` archive format and core algorithms (ABC, SCM). The Rust version uses Rayon + crossbeam instead of Intel TBB, and adds async I/O. Archives are cross-compatible between versions.

### What read lengths does fqc support?

fqc supports all read lengths. Short reads (< 300bp) get the best compression ratio via the ABC algorithm. Medium (300bp – 10kbp) and long reads (> 10kbp) use Zstd compression.

## Compression

### How do I get the best compression ratio?

```bash
fqc compress -i reads.fastq -o reads.fqc -l 9 --lossy-quality illumina8 --block-size 50000
```

Use high compression level, lossy quality quantization, and large block sizes.

### Is the compression lossless?

By default, yes. Sequences and quality scores are preserved exactly. Use `--lossy-quality` to enable lossy quality compression for better ratios.

### Can fqc compress gzipped FASTQ files?

Yes. fqc transparently decompresses `.gz`, `.bz2`, `.xz`, and `.zst` input files.

### What is streaming mode?

Streaming mode (`--streaming`) disables global read reordering and processes reads as they arrive. Useful for stdin/pipe input or when memory is limited. Compression ratio will be slightly lower.

## Performance

### How do I speed up compression?

1. Use `--pipeline` mode for large files
2. Build with `RUSTFLAGS="-C target-cpu=native"`
3. Ensure sufficient threads (`-t`)
4. Use lossy quality if acceptable

### What is pipeline mode?

Pipeline mode (`--pipeline`) enables a 3-stage Reader→Compressor→Writer pipeline that overlaps I/O with computation. Recommended for files > 1GB.

### How much memory does fqc use?

fqc automatically detects system memory and uses ~75% by default. Use `--memory-limit` to set a manual cap in MB.

## Compatibility

### Can I decompress a file created by the C++ version?

Yes. Both versions use the same FQC format specification. Archives are cross-compatible.

### What platforms are supported?

- Linux (x86_64, aarch64)
- macOS (x86_64, aarch64)
- Windows (x86_64)

## Troubleshooting

### bzip2/xz compilation fails

Install system dependencies:

```bash
# Debian/Ubuntu
sudo apt install libbz2-dev liblzma-dev pkg-config

# macOS
brew install xz
```

### Out of memory

Reduce block size or set a memory limit:

```bash
fqc compress -i reads.fastq -o reads.fqc --block-size 1000 --memory-limit 2048
```
