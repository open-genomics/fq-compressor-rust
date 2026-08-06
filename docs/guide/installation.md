# 安装

## 从源码构建

依赖要求：

- Rust **1.75.0+**（MSRV）
- Git

```bash
git clone https://github.com/open-genomics/fq-compressor-rust.git
cd fq-compressor-rust
cargo build --release
./target/release/fqc --help
```

## 本地安装

```bash
cargo install --path .
```
