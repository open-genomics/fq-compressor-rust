# Capability: repository-metadata

## Requirement: repository presentation must accurately describe the project

The GitHub About section, homepage, and topic tags MUST describe `fqc` as a Rust FASTQ compression CLI and point to the live documentation site.

### Scenario: repository metadata is updated

- **WHEN** the maintainer reviews GitHub repository settings
- **THEN** the description, homepage, and topics reflect the actual project scope and audience

## Requirement: root repository docs must stay aligned

The root README, changelog, security policy, and contributor guidance MUST stay consistent with the current release line and repository workflow.

### Scenario: a public-facing repository file becomes misleading

- **WHEN** drift is found between root docs and the actual repository state
- **THEN** the file is rewritten to match current reality
