# Design

## Decision 1: replace `specs/` with canonical `openspec/`

This repository now keeps living requirements in `openspec/specs/` and change-scoped work in `openspec/changes/`. The old ad-hoc `specs/` tree is removed to prevent contributors from reading the wrong source of truth.

## Decision 2: optimize for repository finishability

The repository should be easy to understand, validate, and ship. That means fewer workflows, fewer duplicate docs, and guidance that encodes the actual maintenance pattern instead of an aspirational roadmap.

## Decision 3: treat docs as product surface, not a storage bin

GitHub Pages should help new users quickly understand and use `fqc`. Internal process details belong in OpenSpec and contributor-facing files, not in a sprawling public doc tree.

## Decision 4: keep Copilot and editor setup repo-local

Project-specific behavior is captured with:

- `.github/copilot-instructions.md`
- `.github/instructions/openspec.instructions.md`
- `.github/lsp.json`
- `.github/workflows/copilot-setup-steps.yml`
- `.vscode/` and `.devcontainer/`

This gives Copilot, Claude-oriented tooling, and editor integrations a shared project policy without depending on heavy MCP additions.
