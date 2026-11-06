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
├── main.rs              # Thin CLI entry point
├── api.rs               # Stable operation facade (compress/decompress/info/verify)
├── commands/            # CLI command implementations and printing
│   ├── compress.rs
│   ├── decompress.rs
│   ├── info.rs
│   └── verify.rs
├── plan/                # Validated requests, execution modes, block/worker planning
│   ├── compress.rs      # CompressionRequest + ValidatedCompressPlan
│   ├── decompress.rs    # DecompressRequest + ValidatedDecompressPlan
│   └── block_plan.rs    # block_size/workers fixed point and memory profile
├── engine/              # Compression mode routing and runners
│   ├── compression_engine.rs
│   └── compression_engine/{archive,streaming,pipeline}_runner.rs
├── block/               # Neutral block domain shared by algo and archive
│   ├── mod.rs           # BlockMeta / EncodedBlock / DecodedBlock
│   └── id_codec.rs      # delta + zig-zag + varint IDs
├── algo/                # Compression algorithms (never imports archive)
│   ├── abc.rs
│   ├── block_compressor.rs
│   ├── quality_compressor.rs
│   └── global_analyzer.rs
├── archive/             # .fqc layout and file I/O (never imports algo)
│   ├── format.rs
│   ├── reader.rs
│   └── writer.rs
├── pipeline/            # Parallel block compression / decompression
├── fastq/               # FASTQ parser (no archive/algo dependency)
├── io/                  # Generic compressed streams and output transactions
├── memory/              # ingest / encode / decode budgets
├── dna.rs               # Neutral DNA tables and validation
├── stats.rs             # Single OperationStats source of truth
└── types.rs             # Core enums, ReadRecord, sizing defaults
```

Dependency direction is enforced by `tests/test_architecture.rs`:

```
commands/api -> engine -> pipeline -> algo / archive -> block
                         -> fastq / io / memory / plan -> types
```

`archive` and `algo` must only meet through `block`; `io`, `fastq`, `memory`
and `plan` must not import higher layers.

### Execution modes

Compression routes through `CompressionEngine` with three distinct modes:

| Mode | Flag | Memory | Reorder | Use case |
|------|------|--------|---------|----------|
| Archive | (default) | Full ingest | Yes | Best ratio |
| Streaming | `--streaming` | Bounded | No | Large files, low memory |
| Pipeline | `--pipeline` | Full ingest | Yes | Same implementation as archive; keeps the ≥100 reads/block floor |

Archive and pipeline share one full-ingest implementation
(`full_ingest_runner`); streaming shares the block stage but parses blocks on
demand. A change to ingest, classification or reordering must keep all three
modes consistent.

### Compression path selection

```
Read length classification:
├── Short (≤511 bp) and block reads ≤512 → ABC consensus/delta + Zstd
├── Short (≤511 bp), larger block → Zstd direct
├── Medium (512 bp-10 KB) → Zstd direct
└── Long (≥10 KB) → Zstd with large-block settings
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
