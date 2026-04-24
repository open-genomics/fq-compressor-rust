# CLAUDE.md

Read [`AGENTS.md`](AGENTS.md) first. This file only adds Claude-specific guidance.

## Claude Code usage here

- Start from `openspec/specs/` and the active change folder under `openspec/changes/`.
- Keep one coherent thread of work; avoid fragmenting repository cleanup into multiple speculative branches.
- Favor surgical rewrites over layered patching when a document or workflow is clearly low value.
- Use `/review` before merge for non-trivial changes.
- Avoid `/fleet` unless the task clearly benefits from parallelism.
- Use autopilot only after `proposal.md`, `design.md`, and `tasks.md` are in place and bounded.

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib --tests
cargo doc --no-deps
npm run docs:build
```
