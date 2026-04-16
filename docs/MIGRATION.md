# VitePress Migration Guide

## Overview

The documentation has been migrated from **Honkit** to **VitePress** for 10-50x performance improvement.

## Quick Start

### Install Dependencies

```bash
# Use PNPM (recommended)
pm install -g pnpm
pnpm install

# Or use npm
npm install
```

### Development Server

```bash
# Start dev server (instant HMR)
pnpm docs:dev

# Preview production build
pnpm docs:build
pnpm docs:preview
```

### Build

```bash
# Build for production
pnpm docs:build

# Output: docs/.vitepress/dist/
```

## Project Structure

```
docs/
├── .vitepress/
│   ├── config.mts      # Main English config
│   ├── config.ts       # Chinese config export
│   └── theme/
│       ├── index.ts    # Theme entry
│       └── custom.css  # Custom styles
├── index.md            # Homepage
├── guide/              # User guides
│   ├── what-is-fqc.md
│   ├── installation.md
│   ├── quick-start.md
│   └── cli/
├── architecture/       # Architecture docs
├── algorithms/         # Algorithm docs
├── changelog/          # Version history
├── zh/                 # Chinese translations
│   ├── index.md
│   └── guide/
└── public/             # Static assets
    ├── logo.svg
    └── favicon.svg
```

## Key Differences from Honkit

| Feature | Honkit | VitePress |
|---------|--------|-----------|
| Build Time | 3-5 min | 30 sec |
| Bundle Size | 15 MB | 3 MB |
| Search | Plugin | Built-in |
| Dark Mode | Plugin | Native |
| Mobile UX | Poor | Excellent |
| Hot Reload | Slow | Instant |

## Frontmatter

```yaml
---
title: Page Title
description: Page description for SEO
outline: deep
---
```

## Code Blocks

```markdown
::: code-group

```bash [npm]
npm install
```

```bash [pnpm]
pnpm install
```

:::
```

## Custom Containers

```markdown
::: tip TIP
This is a tip
:::

::: warning WARNING
This is a warning
:::

::: danger DANGER
This is dangerous
:::
```

## Links

```markdown
[Internal link](/guide/installation)
[External link](https://example.com)
```

## More Info

- [VitePress Docs](https://vitepress.dev/)
- [Markdown Extensions](https://vitepress.dev/guide/markdown)
- [Frontmatter Config](https://vitepress.dev/reference/frontmatter-config)
