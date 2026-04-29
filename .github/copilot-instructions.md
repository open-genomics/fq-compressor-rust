# Copilot Instructions For fqc

Read [`AGENTS.md`](../AGENTS.md) first. It is the canonical AI contributor guide for this repository.

Copilot-specific rules:

- Start every non-trivial change from `openspec/specs/` and the active folder in `openspec/changes/`.
- Keep generated edits small enough to review; avoid broad speculative rewrites.
- Run the validation commands listed in `AGENTS.md` before proposing merge-ready work.
- Use repository-local tooling and GitHub integration; do not add MCP servers or plugins unless an OpenSpec change justifies them.
- Keep Copilot cloud setup aligned with `.github/workflows/copilot-setup-steps.yml`.
