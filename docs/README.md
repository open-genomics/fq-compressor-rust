# Documentation

This directory contains developer and user documentation for fqc.

## Structure

```
docs/
├── guide/              # User guides and tutorials
│   ├── what-is-fqc.md  # Introduction to fqc
│   ├── installation.md # Installation instructions
│   ├── quick-start.md  # Quick start guide
│   └── cli/            # CLI command documentation
│       └── compress.md # Compression command details
├── architecture/       # High-level architecture documentation
│   └── index.md        # Architecture overview
├── algorithms/         # Algorithm documentation
│   └── index.md        # Algorithm overview
├── changelog/          # Version changelogs
│   └── index.md        # Changelog index
├── public/             # Public assets (favicons, logos)
│   ├── logo.svg
│   └── favicon.svg
└── .vitepress/         # VitePress site configuration
    ├── config.mts      # Main configuration
    └── theme/          # Theme customization
```

## Building Documentation

```bash
# Install dependencies
npm install

# Development server
npm run docs:dev

# Build for production
npm run docs:build

# Preview production build
npm run docs:preview
```

## Documentation Standards

- Use Markdown with VitePress extensions
- English documentation in root directories
- Chinese documentation in `zh/` subdirectories (if needed)
- Include code examples where applicable
- Link to specs in `/specs` directory for technical details

## Relationship with Specs

- **`/specs`** - Formal specifications (what the system should do)
- **`/docs`** - User/developer guides (how to use and understand the system)

When documenting features, link to the corresponding spec:

```markdown
For technical details, see [File Format Specification](../specs/product/file-format.md).
```

## VitePress Configuration

The site is configured for multi-language support (English and Chinese). See `docs/.vitepress/config.mts` for navigation and sidebar configuration.

## Related

- **Specs**: See `/specs` directory for formal specifications
- **README**: See `../README.md` for project overview
- **Contributing**: See `../CONTRIBUTING.md` for contribution guidelines
