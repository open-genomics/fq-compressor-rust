# Capability: developer-workflow

## Requirement: work must follow a short, reviewable loop

Contributors MUST work in small, mergeable slices instead of long-running drifting branches.

### Scenario: a non-trivial change is prepared

- **WHEN** code, docs, workflows, or repository policy changes in a meaningful way
- **THEN** the contributor validates the change locally
- **AND** uses review before merge if AI assistance produced or reshaped the change substantially

## Requirement: automation usage must stay proportional

The project MUST prefer a small number of high-value tools and workflows over maximal automation.

### Scenario: an AI-assisted development session is planned

- **WHEN** a contributor chooses between long-running autonomous modes or parallel agent modes
- **THEN** they prefer a bounded OpenSpec task flow
- **AND** avoid `/fleet` unless the task clearly benefits from parallel sub-agents
- **AND** only use autopilot after `proposal.md`, `design.md`, and `tasks.md` provide stable scope

## Requirement: local validation must be standardized

The repository MUST provide one consistent validation surface for local contributors.

### Scenario: a contributor prepares a change for push

- **WHEN** they run the local workflow
- **THEN** `scripts/validate.sh` and `.githooks/` provide the standard checks
