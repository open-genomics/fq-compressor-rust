# 开发指南

> 另见：[架构设计](architecture.md)、[性能调优](performance.md)

## 环境准备

### 必需

- **Rust 1.75+**（见 `rust-version` in `Cargo.toml`）
- **Git**

### 推荐

- **VS Code** + rust-analyzer 扩展
- **bacon** — 后台持续编译检查

### DevContainer

项目提供 DevContainer 配置，可一键获得完整开发环境：

```bash
# VS Code / Windsurf / Cursor
# F1 → "Dev Containers: Reopen in Container"
```

## 快速开始

```bash
git clone https://github.com/lessup/fq-compressor-rust.git
cd fq-compressor-rust
cargo build
cargo test --lib --tests    # 131 个测试
```

## 开发工作流

```bash
# 1. 编写代码
# 2. 编译检查
cargo build
# 3. 运行测试 (131 tests)
cargo test --lib --tests
# 4. Lint 检查
cargo clippy --all-targets
# 5. 格式化
cargo fmt --all
# 6. 提交
git add -A && git commit -m "feat(algo): description"
```

## 测试体系

| 套件 | 数量 | 关注点 |
|------|------|--------|
| `test_types` | 11 | 类型枚举与常量 |
| `test_format` | 15 | 二进制格式序列化/反序列化 |
| `test_parser` | 19 | FASTQ 解析器功能 |
| `test_reorder_map` | 23 | 重排映射编解码 |
| `test_roundtrip` | 14 | 块压缩器往返测试 (ABC + Zstd) |
| `test_e2e` | 15 | 端到端压缩/解压往返 |
| `test_algo` | 19 | 算法测试 (ID/质量/PE) |
| `test_dna` | 15 | DNA 工具测试 |
| **合计** | **131** | |

### 常用测试命令

```bash
cargo test --test test_e2e                          # 单个套件
cargo test --test test_e2e test_e2e_pipeline        # 单个测试
cargo test --test test_e2e -- --nocapture           # 输出 println
cargo test --lib                                     # 仅库内测试
```

## 代码质量

### Clippy

Clippy pedantic 全局启用。配置见 `Cargo.toml` 的 `[lints.clippy]` 段。

```bash
cargo clippy --all-targets    # 期望 0 warnings
```

### 格式化

```bash
cargo fmt --all -- --check    # 检查
cargo fmt --all               # 修复
taplo check                   # TOML 检查
taplo fmt                     # TOML 修复
```

## 提交规范

遵循 [Conventional Commits](https://www.conventionalcommits.org/)：

```
feat(scope): add new feature
fix(scope): fix a bug
refactor(scope): code restructure
test: add or update tests
docs: documentation changes
chore: build, CI, tooling
```

**Scope**: `algo`, `commands`, `pipeline`, `io`, `parser`, `format`, `error`, `core`

## 发版

```bash
cargo release patch    # 0.1.0 → 0.1.1
cargo release minor    # 0.1.0 → 0.2.0
cargo release major    # 0.1.0 → 1.0.0
```

## 开发工具

| 工具 | 安装 | 用途 |
|------|------|------|
| bacon | `cargo install bacon` | 后台持续检查 |
| cargo-deny | `cargo install cargo-deny` | 依赖审计 |
| cargo-release | `cargo install cargo-release` | 版本管理 |
| git-cliff | `cargo install git-cliff` | Changelog 生成 |
| flamegraph | `cargo install flamegraph` | 性能火焰图 |
| taplo | `cargo install taplo-cli` | TOML 格式化 |
