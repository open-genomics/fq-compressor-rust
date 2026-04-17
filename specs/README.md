# fqc Specifications

This directory is the **Single Source of Truth** for all formal specifications in the fqc project.

## Philosophy: Spec-Driven Development (SDD)

All code implementations must follow the specs defined here. Before writing any code:

1. **Read the relevant spec** - Understand requirements first
2. **Update spec if needed** - Propose changes before implementing
3. **Implement to spec** - Follow definitions exactly
4. **Test against spec** - Verify acceptance criteria

## Directory Structure

```
specs/
├── product/            # Product features & acceptance criteria
│   ├── core-compression.md    # Core compression functionality
│   ├── cli-commands.md        # CLI command specifications
│   └── file-format.md         # FQC binary format
├── rfc/                # Technical design documents
│   ├── 0001-core-architecture.md
│   ├── 0002-compression-algorithms.md
│   └── 0003-pipeline-architecture.md
├── api/                # API interface definitions
│   └── README.md              # CLI and library API
├── db/                 # Database schemas (not used)
│   └── README.md              # Explains no DB requirement
└── testing/            # BDD test specifications
    └── README.md              # Test suites and acceptance criteria
```

## Quick Reference

| What you need | Where to look |
|---------------|---------------|
| Feature requirements | [product/](./product/) |
| Architecture decisions | [rfc/](./rfc/) |
| CLI usage | [product/cli-commands.md](./product/cli-commands.md) |
| File format | [product/file-format.md](./product/file-format.md) |
| API types | [api/](./api/) |
| Test criteria | [testing/](./testing/) |

## Specification Lifecycle

```
📋 Draft → 🔄 Review → ✅ Accepted → 🔨 Implemented → 📦 Final
```

## Contributing to Specs

When adding or modifying specs:

1. Follow the templates in each directory's README
2. Update related specs if changes affect them
3. Ensure backward compatibility or document breaking changes
4. Link to implementation files in the spec

## Related Documents

- [CLAUDE.md](../CLAUDE.md) - AI assistant guidelines with SDD workflow
- [AGENTS.md](../AGENTS.md) - Agent development guidelines
- [CONTRIBUTING.md](../CONTRIBUTING.md) - Contribution guidelines
