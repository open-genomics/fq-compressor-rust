# 安装指南

## 前置依赖

- **Rust 1.75+**（见 [rustup.rs](https://rustup.rs/)）
- **Git**
- 压缩输入支持的系统依赖：
  - Debian/Ubuntu: `sudo apt install libbz2-dev liblzma-dev pkg-config`
  - macOS: `brew install xz`

## 从源码构建

```bash
git clone https://github.com/lessup/fq-compressor-rust.git
cd fq-compressor-rust
cargo build --release
```

二进制文件位于 `target/release/fqc`（Windows 上为 `fqc.exe`）。

## Native CPU 构建

针对当前 CPU 架构优化，获得最大性能：

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

## 精简构建

禁用可选的压缩输入格式以减小二进制体积：

```bash
# 仅 Zstd（不支持 gz/bz2/xz 输入）
cargo build --release --no-default-features

# 仅 gzip 输入支持
cargo build --release --no-default-features --features gz
```

## Docker

```bash
# 从 GitHub Container Registry 拉取
docker pull ghcr.io/lessup/fq-compressor-rust:latest

# 或本地构建
docker build -t fqc .

# 运行
docker run --rm -v $(pwd):/data fqc compress -i /data/reads.fastq -o /data/reads.fqc
```

## 验证安装

```bash
fqc --version
fqc --help
```
