# Capability: project-governance

## Requirement: repository structure must stay intentional

The repository MUST keep only project-specific documentation, configuration, and automation that directly supports `fqc`.

### Scenario: stale or generic process content is discovered

- **WHEN** a document, workflow, or config no longer reflects the current repository
- **THEN** it is deleted or rewritten instead of being kept as low-value historical clutter

## Requirement: OpenSpec is the planning source of truth

Living requirements MUST live under `openspec/specs/`, and pending changes MUST be tracked under `openspec/changes/`.

### Scenario: a contributor wants to change behavior

- **WHEN** behavior, workflow, or repository structure changes
- **THEN** the relevant OpenSpec living spec is consulted first
- **AND** a change folder is updated or created before implementation proceeds
