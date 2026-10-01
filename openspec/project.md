# fqc (Rust) — Project Context

> **Superseded for new work**: the OpenSpec change workflow is retired; record
> non-trivial decisions as Agent Notes under `.agents/notes/`. `openspec/specs/`
> below remains the capability-spec reference; `openspec/changes/archive/` is
> historical evidence.


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
| `archive-format` | `openspec/specs/archive-format/` | Binary `.fqc` indexed v2 layout, codec dispatch, family rejection |
| `file-output` | `openspec/specs/file-output/` | Transactional ordinary-file outputs |
| `decode-budget` | `openspec/specs/decode-budget/` | Operation-scoped decompress/verify memory budget |
| `compress-budget` | `openspec/specs/compress-budget/` | Archive/pipeline ingest peak budget |
| `cli-modes` | `openspec/specs/cli-modes/` | `--id-mode` choices and lossy QVZ quality mode |
| `format-governance` | `openspec/specs/format-governance/` | 同名 `fqc`/`.fqc` 格式族共存的文档约束（对照表、PATH 风险、无迁移措辞） |

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
  lightweight OpenSpec change workflow described in `openspec/AGENTS.md`
  (retired for new changes — record decisions as Agent Notes under
  `.agents/notes/` instead).
- Low-risk fixes may follow the repository's existing process directly.

## Decision index

| ID | Decision |
|---|---|
| `FQC-DEC-001` | C++ and Rust both use `fqc` / `.fqc`; format family distinguished by magic, not suffix |

## Archived changes

Completed, merged, and moved under `openspec/changes/archive/`:

| Date | Change | Capability |
|---|---|---|
| 2026-10-01 | `harden-indexed-v2-decode-integrity` | `archive-format` |
| 2026-10-01 | `document-fqc-format-family` | `format-governance` |
| 2026-08-19 | `complete-id-and-qvz-modes` | `cli-modes` |
| 2026-08-18 | `correct-indexed-v2-spec` | `archive-format` |
| 2026-08-18 | `make-file-output-atomic` | `file-output` |
| 2026-08-18 | `dispatch-all-stream-codecs` | `archive-format` |
| 2026-08-18 | `enforce-decode-resource-budget` | `decode-budget` |
| 2026-08-18 | `recognize-sequential-fqc-family` | `archive-format` |
| 2026-08-18 | `enforce-compress-archive-budget` | `compress-budget` |
