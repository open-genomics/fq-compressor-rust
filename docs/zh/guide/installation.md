# 安装

## 发布二进制

预编译二进制文件发布在
[GitHub Releases](https://github.com/LessUp/fq-compressor-rust/releases) 页面。

当前发布自动化目标：

- Linux x86_64（`gnu` 和 `musl`）
- macOS Intel
- macOS Apple Silicon
- Windows x86_64

## 从源码构建

依赖要求：

- Rust **1.75.0**
- Git

```bash
git clone https://github.com/LessUp/fq-compressor-rust.git
cd fq-compressor-rust
cargo build --release
./target/release/fqc --help
```

## 本地安装

```bash
cargo install --path .
```

## 容器镜像

仓库包含 `Dockerfile` 用于本地或 CI 构建：

```bash
docker build -t fqc .
docker run --rm -v "$(pwd):/data" fqc --help
```
