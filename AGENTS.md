# AGENTS.md

Use this file as the **canonical AI contributor guide** for this repository.

## Project positioning

`fqc` is an amateur-maintained FASTQ compressor. It optimizes for being
lightweight, nimble, and low-maintenance:

- Prefer fixing drift and simplifying structure over speculative features
- No heavy process: single lightweight CI workflow (four-gate); no docs-site build
- Non-trivial changes (behavior, architecture, cross-file contracts, process,
  testing strategy, on-disk/wire/config formats) must carry an Agent Note under
  `.agents/notes/` recording the why and rejected alternatives; purely mechanical
  edits are exempt. Convention adopted in
  `.agents/notes/implemented/process/2026-09-27-agent-notes-replace-openspec.md`
- The former OpenSpec change workflow under `openspec/changes/` is retired;
  `openspec/specs/` remains as capability-spec reference
- Breaking changes are allowed; backward compatibility is not guaranteed

## Source of truth

| Source | Purpose |
|--------|---------|
| `src/` | Implementation (wins over any document when they disagree) |
| `CONTEXT.md` | Domain vocabulary (contig, minimizer, SCM, streams); read before `algo/` or `archive/` work when terms are unfamiliar |
| `docs/` | Plain-Markdown technical docs (whitepaper, architecture, algorithms, format spec) |
| `CHANGELOG.md` | Single-file change history |
| `.agents/notes/` | Decision records: why changes were made and what was rejected |
| `openspec/specs/` | Capability specs (read-only reference; change workflow retired) |

Do not treat old chat context or outdated documents as authoritative when they
disagree with the code.

## Architecture overview

```
src/
├── main.rs              # CLI entry point
├── commands/            # CLI command implementations
│   ├── compress.rs      # Compression CLI orchestration
│   ├── decompress.rs    # Decompression CLI
│   ├── info.rs          # Archive metadata display
│   └── verify.rs        # Integrity verification
├── engine/              # Core compression engine
│   ├── compression_engine.rs   # ExecutionMode routing (archive/streaming/pipeline)
│   └── compression_request.rs  # Normalized request types
├── algo/                # Compression algorithms
│   ├── abc.rs           # Anchor-Based Compression (short reads)
│   ├── block_compressor.rs  # Block-level compression coordinator
│   ├── quality_compressor.rs  # SCM quality compression
│   └── global_analyzer.rs  # Minimizer extraction, reordering
├── archive/             # .fqc archive format
│   ├── format.rs        # Binary layout (headers, blocks, footer)
│   ├── reader.rs        # Archive reader
│   ├── writer.rs        # Archive writer
│   └── traits.rs        # Shared reader/writer traits
├── pipeline/            # Parallel processing stages
├── fastq/               # FASTQ parser
├── io/                  # Compressed input streams
├── memory_budget.rs     # Memory budget estimation
└── types.rs             # Public types and defaults
```

### Execution modes

Compression routes through `CompressionEngine` with three distinct modes:

| Mode | Flag | Memory | Reorder | Use case |
|------|------|--------|---------|----------|
| Archive | (default) | Full ingest | Yes | Best ratio |
| Streaming | `--streaming` | Bounded | No | Large files, low memory |
| Pipeline | `--pipeline` | Staged | Limited (short single-end) | Balanced throughput |

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
- **CI**: `.github/workflows/ci.yml` (GitHub Actions, stable, four-gate)
- **Safety rule**: no new `unsafe` (enforced by `[lints.rust] unsafe_code = "deny"`)

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib --tests
cargo doc --no-deps
npm run verify-notes   # only when .agents/notes/ changed (needs Node/tsx via npx)
```

## Editing guardrails

- Keep changes small and complete; delete or rewrite stale material instead of
  preserving low-value legacy content
- When changing CLI defaults or behavior, sync `README.md` and `docs/reference/cli.md`
- Use `log` crate for status logging; keep `stdout`/`stderr` user-facing
- Facts recorded in an existing Agent Note (paths, names, defaults) are updated
  in place; a reversed decision gets a new note with interlinks, never a rewrite

## Troubleshooting

- If tests or benches fail with `__tunable_is_initialized@GLIBC_PRIVATE`, it is a
  conda/glibc conflict. Prefix the command with
  `PATH="/usr/bin:/bin:/usr/local/bin:$HOME/.cargo/bin"`.
