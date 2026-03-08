# 开发指南

> 另见：[architecture.md](architecture.md)（项目架构）、[performance.md](performance.md)（性能调优）

## 环境准备

### 必需

- **Rust 1.75+**（见 `rust-version` in `Cargo.toml`）
- **Git**

### 推荐

- **VS Code** + rust-analyzer 扩展（项目已配置 `.vscode/`）
- **bacon** — 后台持续编译检查

### DevContainer（推荐）

项目提供 DevContainer 配置，可一键获得完整开发环境：

```bash
# VS Code / Windsurf / Cursor
# F1 → "Dev Containers: Reopen in Container"
```

详见 [.devcontainer/README.md](../.devcontainer/README.md)。

## 快速开始

```bash
git clone https://github.com/lessup/fq-compressor-rust.git
cd fq-compressor-rust
cargo build
cargo test --lib --tests    # 97 个测试
```

## 开发工作流

```bash
# 1. 编写代码

# 2. 编译检查
cargo build

# 3. 运行测试 (97 tests)
cargo test --lib --tests

# 4. Lint 检查
cargo clippy --all-targets

# 5. 格式化
cargo fmt --all

# 6. 提交
git add -A && git commit -m "feat(algo): description"
```

### 使用 bacon（后台检查器）

```bash
cargo install bacon
bacon              # 默认: clippy-all
bacon test         # 监听测试
bacon clippy-all   # 监听 clippy
```

### VS Code 任务

项目配置了 `.vscode/tasks.json`，可通过 `Ctrl+Shift+B` 快速构建，或 `Ctrl+Shift+P → Tasks: Run Task` 选择：

- `cargo build` / `cargo build (release)`
- `cargo test (all)` / `cargo test (lib only)` / `cargo test (e2e)` / `cargo test (roundtrip)`
- `cargo clippy` / `cargo fmt (check)` / `cargo fmt (fix)`
- `full check (clippy + test + fmt)` — 组合任务

### VS Code 调试

`.vscode/launch.json` 预配置了 14 个调试目标（需安装 CodeLLDB 扩展）：

- **Binary**: `fqc compress` / `decompress` / `info` / `verify` / `custom args`
- **单元测试**: 全量 / 按名过滤
- **集成测试**: 6 个独立配置（test_e2e, test_roundtrip, ...）

---

## 测试体系

### 测试套件

| 套件 | 数量 | 关注点 |
|------|------|--------|
| `test_types` | 11 | 类型枚举与常量 |
| `test_format` | 15 | 二进制格式序列化/反序列化 |
| `test_parser` | 19 | FASTQ 解析器功能 |
| `test_reorder_map` | 23 | 重排映射编解码 |
| `test_roundtrip` | 14 | 块压缩器往返测试 (ABC + Zstd) |
| `test_e2e` | 15 | 端到端压缩/解压往返 |
| **合计** | **97** | |

### 常用测试命令

```bash
# 运行单个套件
cargo test --test test_e2e

# 运行单个测试
cargo test --test test_e2e test_e2e_pipeline_roundtrip

# 输出 println
cargo test --test test_e2e -- --nocapture

# 仅库内测试
cargo test --lib
```

### 测试数据

测试 FASTQ 文件位于 `tests/data/`：

| 文件 | 说明 |
|------|------|
| `test_se.fastq` | 20 条单端短读段 |
| `test_R1.fastq` | 10 对配对端 R1 |
| `test_R2.fastq` | 10 对配对端 R2 |
| `test_interleaved.fastq` | 10 对交错配对端 |

---

## 代码质量

### Clippy

Clippy pedantic 全局启用。配置见 `Cargo.toml` 的 `[lints.clippy]` 段。

```bash
# 检查
cargo clippy --all-targets

# 若 Rust 更新后出现新警告:
# 1. 代码修复合理 → 修复
# 2. 风格偏好不适用 → 在 Cargo.toml [lints.clippy] 中添加 allow
```

### 格式化

```bash
# Rust 代码
cargo fmt --all -- --check     # 检查
cargo fmt --all                # 修复

# TOML 文件
taplo check                    # 检查
taplo fmt                      # 修复
```

### MSRV

MSRV 为 1.75。使用新 API 前请验证：

```bash
cargo +1.75.0 check --all-targets
```

### Unsafe

`unsafe` 代码全局 deny。唯一例外是 `src/common/memory_budget.rs` 中的 Windows FFI 调用（`#[allow(unsafe_code)]`）。

---

## 提交规范

遵循 [Conventional Commits](https://www.conventionalcommits.org/)：

```
feat(scope): add new feature
fix(scope): fix a bug
refactor(scope): code restructure without behavior change
test: add or update tests
docs: documentation changes
chore: build, CI, tooling changes
perf: performance improvements
ci: CI/CD changes
```

**Scope**: `algo`, `commands`, `pipeline`, `io`, `parser`, `format`, `error`, `core`

---

## 发版流程

### 使用 cargo-release（推荐）

```bash
cargo release patch    # 0.1.0 → 0.1.1
cargo release minor    # 0.1.0 → 0.2.0
cargo release major    # 0.1.0 → 1.0.0
```

### 手动发版

```bash
# 1. 更新 Cargo.toml 版本号
# 2. 生成 CHANGELOG
git-cliff -o CHANGELOG.md

# 3. 提交
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore: release v0.2.0"

# 4. 打标签并推送（触发 release workflow）
git tag v0.2.0
git push origin master --tags
```

Release workflow 自动执行：
- 验证 tag 与 Cargo.toml 版本一致
- 在 3 个平台运行测试
- 为 5 个 target 构建二进制
- 创建 GitHub Release（含 checksum）

---

## 开发工具

| 工具 | 安装 | 用途 |
|------|------|------|
| bacon | `cargo install bacon` | 后台持续检查 |
| cargo-deny | `cargo install cargo-deny` | 依赖审计 |
| cargo-release | `cargo install cargo-release` | 版本管理 |
| git-cliff | `cargo install git-cliff` | Changelog 生成 |
| flamegraph | `cargo install flamegraph` | 性能火焰图 |
| taplo | `cargo install taplo-cli` | TOML 格式化 |

> DevContainer 已预装 bacon、cargo-deny、cargo-release、git-cliff、taplo。

---

## CI/CD

项目使用 GitHub Actions，配置位于 `.github/workflows/`：

| Workflow | 触发 | 内容 |
|----------|------|------|
| CI | push / PR | 构建 + 测试 + clippy + fmt |
| Release | tag push | 多平台构建 + GitHub Release |

---

## 故障排除

### target 目录权限问题（DevContainer）

```bash
sudo chown -R vscode:vscode target/
```

### Cargo.lock 冲突

```bash
cargo update
git add Cargo.lock
```

### bzip2/xz 编译失败

确保安装了系统依赖：

```bash
# Debian/Ubuntu
sudo apt install libbz2-dev liblzma-dev pkg-config
```
