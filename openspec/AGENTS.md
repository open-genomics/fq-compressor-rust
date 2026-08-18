# OpenSpec AI Agent Guide

## When to use this workflow

Use the lightweight change workflow for high-risk changes:

- Binary format, CLI/schema, or compatibility changes
- Data integrity, resource limits, or output atomicity
- CI/release, supply chain, or cross-repository product boundaries

Small, low-risk fixes (typos, dead links, mechanical formatting) may follow the
repository's existing process directly.

## What this workflow is

Pure Markdown artifacts under `openspec/`. No Node.js, no global CLI, no
dashboard, no docs-site build, no tool-specific configuration directories.

## Change lifecycle

```
Draft -> Proposed -> Approved -> Applying -> Verifying -> Ready to archive -> Archived
```

See `openspec/project.md` for validation commands and authority rules.

## Artifact contract

Each change directory contains:

- `proposal.md` — why, what, impact, scope, rollback
- `design.md` — how, evidence, allowed surface, risk (optional for trivial docs)
- `tasks.md` — ordered checklist with verification commands
- `verification.md` — evidence matrix (required before archive)
- `specs/<capability>/spec.md` — ADDED/MODIFIED/REMOVED/RENAMED requirements

## Key rules

- One change per repository at a time for overlapping modules.
- Tests/fixtures before implementation.
- Never mark a task complete without running its verification command.
- Never commit, push, publish, or archive without explicit authorization.
- The root `AGENTS.md` remains the highest-priority project guide.

## Archiving

When `verification.md` is complete and the user authorizes archive:

1. Merge `ADDED` / `MODIFIED` requirements from `changes/<id>/specs/` into
   `openspec/specs/<capability>/spec.md` (drop the delta wrapper; keep scenarios).
2. Update the capability table and archived-change list in `openspec/project.md`.
3. Set the proposal `Status` to `Archived`.
4. Move `openspec/changes/<id>/` to `openspec/changes/archive/YYYY-MM-DD-<id>/`.
5. Keep proposal, design, tasks, and verification evidence; do not delete them.
