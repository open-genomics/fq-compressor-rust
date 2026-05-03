# CLI Development Guidelines

> Rust development standards for the fqc CLI tool.

---

## Guidelines Index

| Guide | Description |
|-------|-------------|
| [Directory Structure](./directory-structure.md) | Module organization in `src/` |
| [Error Handling](./error-handling.md) | Error types, propagation, exit codes |
| [Logging Guidelines](./logging-guidelines.md) | Log levels and patterns |
| [Quality Guidelines](./quality-guidelines.md) | Linting, testing, code standards |

---

## Quick Reference

### Validation

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib --tests
cargo doc --no-deps
```

### Key Rules

- No new `unsafe` code
- No `unwrap()` in library code — use `?` for error propagation
- Errors defined in `src/error.rs` with `thiserror`
- Logging via `log` crate with `env_logger`

### Project Structure

```
src/
├── main.rs          # CLI entry point
├── lib.rs           # Library root
├── error.rs         # Error types
├── types.rs         # Constants and shared types
├── commands/        # CLI command implementations
├── algo/            # Compression algorithms
├── io/              # I/O abstractions
├── pipeline/        # Processing pipelines
├── fastq/           # FASTQ parsing
└── common/          # Shared utilities
```
