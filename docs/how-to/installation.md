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

> **同名二进制 `PATH` 覆盖风险**：本实现与 C++ `fq-compressor` 都安装名为 `fqc` 的二进制。若两者同时在 `PATH`，后者（或更靠前的目录）覆盖前者——安装后用 `which fqc` 确认实际调用的实现。
