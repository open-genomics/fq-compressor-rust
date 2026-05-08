# CLAUDE.md

Read [`AGENTS.md`](AGENTS.md) first. This file only adds Claude-specific guidance.

## Claude Code usage here

- Start from `openspec/specs/` and the active change folder under `openspec/changes/`.
- Keep one coherent thread of work; avoid fragmenting repository cleanup into multiple speculative branches.
- Favor surgical rewrites over layered patching when a document or workflow is clearly low value.
- Use `/review` before merge for non-trivial changes.
- Avoid `/fleet` unless the task clearly benefits from parallelism.
- Use autopilot only after `proposal.md`, `design.md`, and `tasks.md` are in place and bounded.

## Project-specific context

This is a **FASTQ compression tool** for bioinformatics. Key domain concepts:

- **FASTQ format**: 4-line records (ID, sequence, +, quality)
- **Block-indexed archive**: Enables random access via block index
- **Read length classification**: Short (ABC), Medium/Long (Zstd)
- **Compression modes**: Archive (default), Streaming (`--streaming`), Pipeline (`--pipeline`)

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib --tests
cargo doc --no-deps
npm run docs:build
```

## Conda/glibc workaround

If tests fail with `__tunable_is_initialized@GLIBC_PRIVATE`, ensure `.cargo/config.toml` uses system GCC:

```toml
[target.x86_64-unknown-linux-gnu]
linker = "/usr/bin/gcc"
```

Or run with:
```bash
PATH="/usr/bin:/bin:/usr/local/bin:$HOME/.cargo/bin" cargo test
```
