# Domain docs layout

This repo uses a **single-context** layout.

## Context file

- Location: `CONTEXT.md` at repo root (not yet created)
- Purpose: Documents the project's domain language, core concepts, and terminology

## ADRs

- Location: `docs/adr/` at repo root (not yet created)
- Purpose: Architecture Decision Records — documents significant architectural decisions

## Consumer rules

Skills that read domain docs should:

1. Read `CONTEXT.md` first to understand the domain language
2. Check `docs/adr/` for relevant past decisions before proposing new architecture
3. If `CONTEXT.md` doesn't exist yet, proceed without domain context (it may be created later)
