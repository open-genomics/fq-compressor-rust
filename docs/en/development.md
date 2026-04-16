# Development Guide

> See also: [architecture.md](architecture.md) (project architecture), [performance.md](performance.md) (performance tuning)

## Prerequisites

### Required

- **Rust 1.75+** (see `rust-version` in `Cargo.toml`)
- **Git**

### Recommended

- **VS Code** + rust-analyzer extension (project configured with `.vscode/`)
- **bacon** — Background continuous compilation checking

### DevContainer (Recommended)

Project provides DevContainer configuration for one-click full development environment:

```bash
# VS Code / Windsurf / Cursor
# F1 → "Dev Containers: Reopen in Container"
```

See [.devcontainer/README.md](../../.devcontainer/README.md) for details.

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

# 2. Compile check
cargo build

# 3. Run tests (131 tests)
cargo test --lib --tests

# 4. Lint check
cargo clippy --all-targets

# 5. Format
cargo fmt --all

# 6. Commit
git add -A && git commit -m "feat(algo): description"
```

### Using bacon (background checker)

```bash
cargo install bacon
bacon              # Default: clippy-all
bacon test         # Run tests
bacon clippy-all   # Run clippy
```

### VS Code Tasks

Project configures `.vscode/tasks.json`. Use `Ctrl+Shift+B` for quick build, or `Ctrl+Shift+P → Tasks: Run Task` to select:

- `cargo build` / `cargo build (release)`
- `cargo test (all)` / `cargo test (lib only)` / `cargo test (e2e)` / `cargo test (roundtrip)`
- `cargo clippy` / `cargo fmt (check)` / `cargo fmt (fix)`
- `full check (clippy + test + fmt)` — Combined task

### VS Code Debugging

`.vscode/launch.json` pre-configures 14 debugging targets (requires CodeLLDB extension):

- **Binary**: `fqc compress` / `decompress` / `info` / `verify` / `custom args`
- **Unit tests**: Full suite / filter by name
- **Integration tests**: 6 independent configurations (test_e2e, test_roundtrip, ...)

---

## Testing

### Test Suites

| Suite | Count | Focus |
|-------|-------|-------|
| `test_types` | 11 | Type enums and constants |
| `test_format` | 15 | Binary format serialization/deserialization |
| `test_parser` | 19 | FASTQ parser functionality |
| `test_reorder_map` | 23 | Reorder map encoding/decoding |
| `test_roundtrip` | 14 | Block compressor round-trip (ABC + Zstd) |
| `test_algo` | 19 | Algorithm tests (ID/quality compressor, PE optimizer) |
| `test_e2e` | 15 | End-to-end compression/decompression round-trip |
| `test_dna` | 15 | DNA utility tests |
| **Total** | **131** | |

### Common Test Commands

```bash
# Run single suite
cargo test --test test_e2e

# Run single test
cargo test --test test_e2e test_e2e_pipeline_roundtrip

# Output with println
cargo test --test test_e2e -- --nocapture

# Library tests only
cargo test --lib
```

### Test Data

Test FASTQ files are in `tests/data/`:

| File | Description |
|------|-------------|
| `test_se.fastq` | 20 single-end short reads |
| `test_R1.fastq` | 10 paired-end R1 reads |
| `test_R2.fastq` | 10 paired-end R2 reads |
| `test_interleaved.fastq` | 10 interleaved paired-end |

---

## Code Quality

### Clippy

Clippy pedantic enabled globally. Configuration in `Cargo.toml` `[lints.clippy]` section.

```bash
# Check
cargo clippy --all-targets

# If new warnings appear after Rust update:
# 1. Fix if reasonable
# 2. If style preference not applicable → add allow in Cargo.toml [lints.clippy]
```

### Formatting

```bash
# Rust code
cargo fmt --all -- --check     # Check
cargo fmt --all                # Fix

# TOML files
taplo check                    # Check
taplo fmt                      # Fix
```

### MSRV

MSRV is 1.75. Verify before using new APIs:

```bash
cargo +1.75.0 check --all-targets
```

### Unsafe

`unsafe` code globally denied. Only exception is `src/common/memory_budget.rs` Windows FFI calls (`#[allow(unsafe_code)]`).

---

## Commit Convention

Follow [Conventional Commits](https://www.conventionalcommits.org/):

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

**Scopes**: `algo`, `commands`, `pipeline`, `io`, `parser`, `format`, `error`, `core`

---

## Release Process

### Using cargo-release (Recommended)

```bash
cargo release patch    # 0.1.0 → 0.1.1
cargo release minor    # 0.1.0 → 0.2.0
cargo release major    # 0.1.0 → 1.0.0
```

### Manual Release

```bash
# 1. Update Cargo.toml version
# 2. Generate CHANGELOG
git-cliff -o CHANGELOG.md

# 3. Commit
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore: release v0.2.0"

# 4. Tag and push (triggers release workflow)
git tag v0.2.0
git push origin master --tags
```

Release workflow automatically:
- Validates tag matches Cargo.toml version
- Runs tests on 3 platforms
- Builds binaries for 5 targets
- Creates GitHub Release (with checksum)

---

## Developer Tools

| Tool | Install | Purpose |
|------|---------|---------|
| bacon | `cargo install bacon` | Background continuous check |
| cargo-deny | `cargo install cargo-deny` | Dependency audit |
| cargo-release | `cargo install cargo-release` | Version management |
| git-cliff | `cargo install git-cliff` | Changelog generation |
| flamegraph | `cargo install flamegraph` | Performance flamegraph |
| taplo | `cargo install taplo-cli` | TOML formatting |

> DevContainer pre-installs: bacon, cargo-deny, cargo-release, git-cliff, taplo.

---

## CI/CD

Project uses GitHub Actions, configurations in `.github/workflows/`:

| Workflow | Trigger | Content |
|----------|---------|---------|
| CI | push / PR | Build + Test + Clippy + Fmt |
| Release | tag push | Multi-platform build + GitHub Release |

---

## Troubleshooting

### Permission Issues with target Directory (DevContainer)

```bash
sudo chown -R vscode:vscode target/
```

### Cargo.lock Conflicts

```bash
cargo update
git add Cargo.lock
```

### bzip2/xz Compilation Failures

Ensure system dependencies are installed:

```bash
# Debian/Ubuntu
sudo apt install libbz2-dev liblzma-dev pkg-config
```
