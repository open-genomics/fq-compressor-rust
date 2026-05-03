# Documentation Guidelines

> How to maintain the VitePress documentation site.

---

## Overview

This project uses **VitePress** for documentation. The docs site is a static site generated from Markdown files in `docs/`.

---

## Directory Layout

```
docs/
├── .vitepress/
│   └── config.mts          # VitePress configuration
├── public/                  # Static assets (favicon, logo)
├── guide/                   # User documentation
│   ├── installation.md
│   ├── quick-start.md
│   └── cli.md
├── architecture/            # Technical architecture docs
├── algorithms/              # Algorithm documentation
├── benchmarks/              # Performance benchmarks
├── agents/                  # AI agent instructions
├── superpowers/             # Project management (specs/plans)
├── adr/                     # Architecture Decision Records
├── index.md                 # Landing page
└── release-notes.md
```

---

## Commands

```bash
npm run docs:dev      # Development server (port 5173)
npm run docs:build    # Production build
npm run docs:preview  # Preview build (port 4173)
```

---

## Markdown Standards

### Headings

- Only one `#` (H1) per page
- Follow logical hierarchy: `#` → `##` → `###`

### Code Blocks

Always specify language:

````markdown
```rust
fn main() {}
```
````

### Links

- Internal: `[CLI](/guide/cli.md)`
- External: `[GitHub](https://github.com/...)`

### Admonitions

```markdown
::: tip
Helpful information
:::

::: warning
Caution needed
:::

::: danger
Critical information
:::
```

---

## Configuration

Edit `docs/.vitepress/config.mts` for:
- Navigation bar
- Sidebar structure
- Site metadata

---

## Quality Checks

```bash
npm run docs:build    # Must succeed (catches broken links)
```

VitePress fails the build on dead links when `ignoreDeadLinks: false`.

---

## Naming Conventions

| Category | Convention | Example |
|----------|------------|---------|
| Markdown files | kebab-case | `performance-report.md` |
| Directories | kebab-case | `benchmarks/` |
| Index files | `index.md` | `docs/architecture/index.md` |
| Dated specs | `YYYY-MM-DD-slug.md` | `2026-05-01-design.md` |

---

## Deployment

Docs deploy to GitHub Pages on push to `master`:
- URL: `https://lessup.github.io/fq-compressor-rust/`
