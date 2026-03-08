# Development Guide

> See also: [Architecture](architecture.md), [Performance](performance.md)

## Prerequisites

- **Rust 1.75+** (see `rust-version` in `Cargo.toml`)
- **Git**

### Recommended

- **VS Code** + rust-analyzer extension
- **bacon** — background continuous checking

### DevContainer

The project provides a DevContainer configuration for a complete development environment:

```bash
# VS Code / Windsurf / Cursor
# F1 → "Dev Containers: Reopen in Container"
```

## Quick Start

```bash
git clone https://github.com/lessup/fq-compressor-rust.git
cd fq-compressor-rust
cargo build
cargo test --lib --tests    # 131 tests
```

## Development Workflow

```bash
# 1. Write code
# 2. Build check
cargo build
# 3. Run tests (131 tests)
cargo test --lib --tests
# 4. Lint
cargo clippy --all-targets
# 5. Format
cargo fmt --all
# 6. Commit
git add -A && git commit -m "feat(algo): description"
```

## Test Suites

| Suite | Count | Focus |
|-------|-------|-------|
| `test_types` | 11 | Type enums and constants |
| `test_format` | 15 | Binary format serialization |
| `test_parser` | 19 | FASTQ parser functionality |
| `test_reorder_map` | 23 | Reorder map encoding/decoding |
| `test_roundtrip` | 14 | Block compressor round-trip (ABC + Zstd) |
| `test_e2e` | 15 | End-to-end compress/decompress |
| `test_algo` | 19 | Algorithm tests (ID/quality/PE) |
| `test_dna` | 15 | DNA utility tests |
| **Total** | **131** | |

### Common Test Commands

```bash
cargo test --test test_e2e                          # Single suite
cargo test --test test_e2e test_e2e_pipeline        # Single test
cargo test --test test_e2e -- --nocapture           # With output
cargo test --lib                                     # Library tests only
```

## Code Quality

### Clippy

Clippy pedantic is globally enabled. Configuration in `Cargo.toml` `[lints.clippy]` section.

```bash
cargo clippy --all-targets    # 0 warnings expected
```

### Formatting

```bash
cargo fmt --all -- --check    # Check
cargo fmt --all               # Fix
taplo check                   # TOML check
taplo fmt                     # TOML fix
```

## Commit Convention

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(scope): add new feature
fix(scope): fix a bug
refactor(scope): code restructure
test: add or update tests
docs: documentation changes
chore: build, CI, tooling
```

**Scopes**: `algo`, `commands`, `pipeline`, `io`, `parser`, `format`, `error`, `core`

## Release

```bash
cargo release patch    # 0.1.0 → 0.1.1
cargo release minor    # 0.1.0 → 0.2.0
cargo release major    # 0.1.0 → 1.0.0
```

## Development Tools

| Tool | Install | Purpose |
|------|---------|---------|
| bacon | `cargo install bacon` | Background checking |
| cargo-deny | `cargo install cargo-deny` | Dependency audit |
| cargo-release | `cargo install cargo-release` | Version management |
| git-cliff | `cargo install git-cliff` | Changelog generation |
| flamegraph | `cargo install flamegraph` | Performance flame graphs |
| taplo | `cargo install taplo-cli` | TOML formatting |
