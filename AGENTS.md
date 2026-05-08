# AGENTS.md

Use this file as the **canonical AI contributor guide** for this repository.

## Project mode

`fqc` is in **stabilization and close-out mode**:

- Prefer fixing drift, simplifying structure, and tightening release quality
- Avoid speculative features unless an OpenSpec change explicitly requires them
- Delete or rewrite stale material instead of preserving low-value legacy content

## Source of truth

| Source | Purpose |
|--------|---------|
| `openspec/specs/` | Living specifications |
| `openspec/changes/` | Active change proposals |
| `CONTEXT.md` | Domain language and concepts |
| `docs/` | User-facing documentation |
| `src/` | Implementation |

Do not treat old chat context or outdated documents as authoritative when they disagree with code or `openspec/`.

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

## Required workflow

1. Read the relevant spec in `openspec/specs/`.
2. If behavior, structure, or process must change, create an OpenSpec change in `openspec/changes/` first.
3. Implement the smallest complete diff that satisfies the spec.
4. Update tests and public docs for any CLI, workflow, or repository-behavior change.
5. Validate with the existing commands before considering the task complete.

## Repository facts

- **Binary name**: `fqc`
- **Archive format**: block-indexed `.fqc` with global header, blocks, reorder map, footer
- **Commands**: `compress`, `decompress`, `info`, `verify`
- **MSRV**: 1.75.0
- **Safety rule**: no new `unsafe`

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib --tests
cargo doc --no-deps
npm run docs:build
```

## Editing guardrails

- Keep workflows minimal: CI, Pages, release, and Copilot setup should each have a clear reason to exist
- Prefer high-signal docs over breadth
- When changing CLI defaults or behavior, sync:
  - `openspec/specs/cli-surface/spec.md`
  - `README.md`
  - `docs/guide/cli.md`
- Use `log` crate for status logging; keep `stdout`/`stderr` user-facing

## Tooling guidance

- Use `/review` before merge for non-trivial AI-assisted changes
- Avoid `/fleet` unless parallel sub-agents are genuinely needed
- Only use autopilot after OpenSpec tasks are clear and bounded
- Prefer built-in GitHub integration over new MCP servers

## Agent skills

- **Issue tracker**: GitHub Issues with `gh` CLI
- **Triage labels**: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`
- **Domain docs**: Single-context layout in `CONTEXT.md`
