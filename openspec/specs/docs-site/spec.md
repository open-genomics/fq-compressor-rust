# Capability: docs-site

## Requirement: the documentation site must be a concise project showcase

The published documentation MUST help a new user understand what `fqc` is, how to install it, how to run the CLI, and where to find release information.

### Scenario: a reader lands on GitHub Pages

- **WHEN** they open the site homepage
- **THEN** they see a clear project description
- **AND** direct links to quick start, CLI reference, architecture, algorithms, and GitHub

## Requirement: the docs site must avoid drift

The docs site MUST not contain large sections of low-signal duplicated material that diverge from the codebase.

### Scenario: a page becomes stale or redundant

- **WHEN** a page duplicates README, OpenSpec, or obsolete tooling documentation
- **THEN** it is collapsed, rewritten, or removed
- **AND** the resulting docs build without dead links
