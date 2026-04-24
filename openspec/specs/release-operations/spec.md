# Capability: release-operations

## Requirement: repository workflows must stay minimal and purposeful

GitHub Actions MUST be limited to workflows that materially support shipping and maintaining the current release line.

### Scenario: repository automation is reviewed

- **WHEN** a workflow does not provide clear value for CI health, Pages publishing, release packaging, or Copilot environment setup
- **THEN** it is removed

## Requirement: release packaging must stay predictable

Tagged releases MUST validate version consistency, run tests, build release artifacts, and publish checksums.

### Scenario: a version tag is pushed

- **WHEN** a `v*` tag triggers release automation
- **THEN** the workflow validates the Cargo version
- **AND** runs the test suite
- **AND** publishes platform archives plus checksum files to the GitHub release

## Requirement: Copilot cloud agent setup must be repository-defined

The repository MUST define how Copilot cloud agent prepares its environment.

### Scenario: Copilot cloud agent starts work

- **WHEN** the repository is opened by Copilot cloud agent
- **THEN** `.github/workflows/copilot-setup-steps.yml` installs the Rust and Node prerequisites needed for this project
