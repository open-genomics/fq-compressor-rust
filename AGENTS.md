# AGENTS.md

Use this file as the **canonical AI contributor guide** for this repository.

## Project positioning

`fqc` is an amateur-maintained FASTQ compressor. It optimizes for being
lightweight, nimble, and low-maintenance:

- Prefer fixing drift and simplifying structure over speculative features
- No heavy process: no spec-driven change management, no CI, no docs-site build
- Breaking changes are allowed; backward compatibility is not guaranteed

## Source of truth

| Source | Purpose |
|--------|---------|
| `src/` | Implementation (wins over any document when they disagree) |
| `CONTEXT.md` | Domain language and concepts |
| `docs/` | Plain-Markdown technical docs (whitepaper, architecture, algorithms, format spec) |
| `CHANGELOG.md` | Single-file change history |

Do not treat old chat context or outdated documents as authoritative when they
disagree with the code.

## Architecture overview

```
src/
├── main.rs              # CLI entry point
├── commands/            # CLI command implementations
│   ├── compress.rs      # Compression CLI orchestration
│   ├── compression_engine.rs   # Core compression engine (ExecutionMode routing)
│   ├── compression_request.rs  # Normalized request types
│   └── decompress.rs    # Decompression CLI
├── algo/                # Compression algorithms
│   ├── abc.rs           # Anchor-Based Compression (short reads)
│   ├── block_compressor.rs  # Block-level compression coordinator
│   ├── quality_compressor.rs  # SCM quality compression
│   └── global_analyzer.rs  # Minimizer extraction, reordering
├── pipeline/            # Parallel processing stages
├── format.rs            # Binary archive format
├── fqc_writer.rs        # Archive writer
├── fqc_reader.rs        # Archive reader
└── types.rs             # Public types and defaults
```

### Execution modes

Compression routes through `CompressionEngine` with three distinct modes:

| Mode | Flag | Memory | Reorder | Use case |
|------|------|--------|---------|----------|
| Archive | (default) | Full ingest | Yes | Best ratio |
| Streaming | `--streaming` | Bounded | No | Large files, low memory |
| Pipeline | `--pipeline` | Staged | No | Balanced throughput |

### Compression path selection

```
Read length classification:
├── Short (≤511 bp) → ABC consensus/delta + Zstd
├── Medium (512 bp-10 KB) → Zstd direct
└── Long (>10 KB) → Zstd with large-block settings
```

## Repository facts

- **Binary name**: `fqc`
- **Archive format**: block-indexed `.fqc` with global header, blocks, reorder map, footer
- **Commands**: `compress`, `decompress`, `info`, `verify`
- **MSRV**: 1.75.0 (declared via `rust-version` in `Cargo.toml`)
- **Safety rule**: no new `unsafe` (enforced by `[lints.rust] unsafe_code = "deny"`)

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib --tests
cargo doc --no-deps
```

## Editing guardrails

- Keep changes small and complete; delete or rewrite stale material instead of
  preserving low-value legacy content
- When changing CLI defaults or behavior, sync `README.md` and `docs/guide/cli.md`
- Use `log` crate for status logging; keep `stdout`/`stderr` user-facing

## Troubleshooting

- If tests or benches fail with `__tunable_is_initialized@GLIBC_PRIVATE`, it is a
  conda/glibc conflict. Prefix the command with
  `PATH="/usr/bin:/bin:/usr/local/bin:$HOME/.cargo/bin"`.
