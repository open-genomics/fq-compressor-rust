# fqc (Rust) — Project Context

## Identity

- **Canonical repository**: `open-genomics/fq-compressor-rust`
- **Product name**: `fqc`
- **Archive extension**: `.fqc`
- **Format family**: `fqc-indexed/v2`
- **Binary**: `fqc` (compress, decompress, info, verify)
- **Lifecycle**: experimental; breaking changes allowed
- **MSRV**: 1.75.0
- **Safety**: `unsafe_code = "deny"`

## Core contracts

| Capability | Path | Description |
|---|---|---|
| `archive-format` | `openspec/specs/archive-format/` | Binary `.fqc` indexed v2 layout |
| `file-output` | `openspec/specs/file-output/` | Transactional ordinary-file outputs |

## External boundaries

- **`fq-compressor` (C++)**: shares product name `fqc` and extension `.fqc` but
  uses a different format family (`fqc-sequential/v2`). Each reader must reject
  the other family's magic.
- **Decision `FQC-DEC-001`**: both implementations keep `fqc` / `.fqc`; no
  separate suffix. Format family is distinguished by archive magic.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib --tests
cargo doc --no-deps
```

## Authority rules

- `src/` is the implementation source of truth.
- Models must not commit, push, create PRs, or publish without explicit
  authorization.
- High-risk changes (format, compatibility, security/resource) use the
  lightweight OpenSpec change workflow described in `openspec/AGENTS.md`.
- Low-risk fixes may follow the repository's existing process directly.

## Decision index

| ID | Decision |
|---|---|
| `FQC-DEC-001` | C++ and Rust both use `fqc` / `.fqc`; format family distinguished by magic, not suffix |
