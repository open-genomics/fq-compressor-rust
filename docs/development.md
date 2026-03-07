# Development Guide

## Prerequisites

- Rust 1.75+ (see `rust-toolchain.toml`)
- Git

## Setup

```bash
git clone https://github.com/lessup/fq-compressor-rust.git
cd fq-compressor-rust
cargo build
cargo test --lib --tests
```

## Development Workflow

```bash
# 1. Make changes
# 2. Check compilation
cargo build

# 3. Run tests (97 tests)
cargo test --lib --tests

# 4. Lint
cargo clippy --all-targets

# 5. Format
cargo fmt --all

# 6. Commit
git add -A && git commit -m "feat: description"
```

### Using bacon (background checker)

```bash
cargo install bacon
bacon              # default: clippy-all
bacon test         # watch tests
bacon clippy-all   # watch clippy
```

## Test Suites

| Suite | Count | Focus |
|-------|-------|-------|
| `test_e2e` | 15 | End-to-end compress/decompress round-trip |
| `test_roundtrip` | 14 | Block compressor round-trip (ABC + Zstd) |
| `test_parser` | 19 | FASTQ parser features |
| `test_reorder_map` | 23 | Reorder map encoding/decoding |
| `test_format` | 15 | Binary format serialization |
| `test_types` | 11 | Type enums and constants |

```bash
# Run a single suite
cargo test --test test_e2e

# Run a single test
cargo test --test test_e2e test_e2e_pipeline_roundtrip

# Run with output
cargo test --test test_e2e -- --nocapture
```

### Test Data

Test FASTQ files are in `tests/data/`:
- `test_se.fastq` — 20 single-end short reads
- `test_pe_R1.fastq` / `test_pe_R2.fastq` — 10 paired-end read pairs

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

Scopes: `algo`, `commands`, `pipeline`, `io`, `parser`, `format`, `error`, `core`

## Release Process

```bash
# 1. Update version in Cargo.toml
# 2. Update CHANGELOG.md
# 3. Commit
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore: release v0.2.0"

# 4. Tag and push (triggers release workflow)
git tag v0.2.0
git push origin master --tags
```

The `release.yml` workflow automatically:
- Validates tag matches Cargo.toml version
- Runs tests on 3 platforms
- Builds binaries for 5 targets
- Creates GitHub Release with checksums

## Useful Tools

| Tool | Install | Purpose |
|------|---------|---------|
| bacon | `cargo install bacon` | Background checker |
| cargo-deny | `cargo install cargo-deny` | Dependency audit |
| cargo-release | `cargo install cargo-release` | Version management |
| git-cliff | `cargo install git-cliff` | Changelog generation |
| flamegraph | `cargo install flamegraph` | Performance profiling |
| taplo | `cargo install taplo-cli` | TOML formatter |

## Troubleshooting

### Clippy warnings

Clippy pedantic is enabled globally. If a new warning appears after a Rust update:
1. Check if the code fix is reasonable → fix it
2. If it's a style preference that doesn't apply → add to `[lints.clippy]` in `Cargo.toml`

### MSRV issues

MSRV is 1.75. If you use a newer API:
- Check with `cargo +1.75.0 check --all-targets`
- Find an alternative that works on 1.75

### Windows FFI

`unsafe` code is `deny` globally. The only exceptions are Windows FFI calls in `src/common/memory_budget.rs` with `#[allow(unsafe_code)]`.
