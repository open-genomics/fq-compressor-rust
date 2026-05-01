# Contributing to fqc

This repository favors **small, high-signal changes** over broad redesigns.

## Before you code

1. Read the relevant living spec in `openspec/specs/`.
2. If behavior or process needs to change, add or update a change in `openspec/changes/`.
3. Finish the full slice: spec, code, tests, and docs.

## Local setup

```bash
cargo build
npm ci
bash scripts/setup-hooks.sh
```

## Validation

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib --tests
cargo doc --no-deps
npm run docs:build
```

Or run:

```bash
bash scripts/validate.sh full
```

## Running benchmarks

```bash
cargo bench
```

**Note:** If you encounter a linker error with `__tunable_is_initialized@GLIBC_PRIVATE`, this is a conda/glibc conflict. Use:

```bash
PATH="/usr/bin:/bin:/usr/local/bin:$HOME/.cargo/bin" cargo bench
```

See `docs/benchmarks/performance-report.md` for details.

## Branching and review

- Prefer short-lived branches.
- Rebase or merge frequently; do not let local and cloud branches drift for long.
- If AI tooling is involved, run `/review` before merge on non-trivial changes.
- Avoid `/fleet` unless the task clearly needs parallel sub-agents.

## What good changes look like here

- They reduce drift between code, docs, workflows, and specs.
- They simplify maintenance instead of adding ceremony.
- They update the relevant OpenSpec files and user-facing docs when behavior changes.
- They do not add broad new dependencies or redundant engineering layers without a concrete repository-specific payoff.
