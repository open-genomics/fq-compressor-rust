# closure-hardening-foundation

## Why

The repository had drifted away from both the actual codebase and canonical OpenSpec structure:

- legacy `specs/` content no longer matched implementation reality
- the documentation site contained large amounts of low-value and broken content
- GitHub Actions had grown noisy and overdesigned
- AI/editor/Copilot guidance was fragmented and repetitive
- repository metadata and docs presentation were weaker than the software itself

## What changes

- move the repository to canonical `openspec/` structure
- rewrite root docs and AI guidance around the real project
- simplify workflows to CI, Pages, release, and Copilot setup
- rebuild the docs site as a focused showcase and onboarding surface
- standardize hooks, LSP, and repo-local Copilot instructions
- fix verified code/config drift such as CLI version and memory-limit semantics

## Non-goals

- starting a new feature roadmap
- expanding the public surface area beyond the current release line
- preserving low-quality documents for history's sake
