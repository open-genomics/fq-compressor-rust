# AGENTS.md

Use this file as the **canonical AI contributor guide** for this repository.

## Project mode

`fqc` is in **stabilization and close-out mode**:

- prefer fixing drift, simplifying structure, and tightening release quality
- avoid speculative features unless an OpenSpec change explicitly requires them
- delete or rewrite stale material instead of preserving low-value legacy content

## Source of truth

- living specs: `openspec/specs/`
- active changes: `openspec/changes/`
- user-facing docs: `docs/`
- implementation: `src/`

Do not treat old chat context or outdated documents as authoritative when they disagree with code or `openspec/`.

## Required workflow

1. Read the relevant spec in `openspec/specs/`.
2. If behavior, structure, or process must change, update or create an OpenSpec change in `openspec/changes/` first.
3. Implement the smallest complete diff that satisfies the spec.
4. Update tests and public docs for any CLI, workflow, or repository-behavior change.
5. Validate with the existing commands before considering the task complete.

## Repository facts

- Binary name: `fqc`
- Format: block-indexed `.fqc`
- Commands: `compress`, `decompress`, `info`, `verify`
- MSRV: **1.75.0**
- Safety rule: no new `unsafe`

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --lib --tests
cargo doc --no-deps
npm run docs:build
```

For local Git hooks:

```bash
bash scripts/setup-hooks.sh
```

## Editing guardrails

- Keep workflows minimal: CI, Pages, release, and Copilot setup should each have a clear reason to exist.
- Prefer high-signal docs over breadth. The docs site should showcase the project and unblock use, not mirror every internal file.
- When changing CLI defaults or behavior, sync:
  - `openspec/specs/cli-surface/spec.md`
  - `README.md`
  - `docs/guide/cli.md`
- Use `log` for status logging. Keep `stdout`/`stderr` output user-facing and deliberate.

## Tooling guidance

- Use `/review` before merge when AI-assisted changes are non-trivial.
- Avoid `/fleet` unless the task genuinely needs parallel sub-agents; it is usually unnecessary here.
- Only use autopilot or allow-all modes **after** OpenSpec tasks are clear and bounded.
- Prefer built-in GitHub integration and repo-local instructions over adding new MCP servers or plugins.
