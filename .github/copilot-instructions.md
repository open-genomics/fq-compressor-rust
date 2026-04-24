# Copilot instructions for fqc

This repository uses **OpenSpec**. Before changing code, read the relevant living specs in `openspec/specs/` and any active change in `openspec/changes/`.

## Project mode

- Optimize for **stabilization and high-signal cleanup**, not feature sprawl.
- Prefer removing stale or redundant content over preserving it.
- Keep docs, workflows, and automation proportional to a small Rust CLI project.

## Required workflow

1. Review the relevant spec in `openspec/specs/`.
2. If behavior or structure must change, update or create an OpenSpec change under `openspec/changes/` first.
3. Implement only what the spec or change requires.
4. Update tests and user-facing docs when CLI behavior changes.

## Validation commands

Run the existing commands instead of inventing new tooling:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib --tests
cargo doc --no-deps
npm run docs:build
```

## Repository-specific guardrails

- Rust toolchain is pinned to **1.75.0**.
- Do not introduce `unsafe` code.
- Use `log` for status logging; keep CLI output intentional and user-facing.
- Keep GitHub Actions minimal. If a workflow has no clear maintenance value, remove it.
- Prefer `/review` before merge and avoid long-lived parallel branches or broad speculative refactors.
