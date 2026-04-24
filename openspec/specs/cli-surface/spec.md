# Capability: cli-surface

## Requirement: the CLI surface must be documented from implementation reality

The documented CLI MUST match the command names, options, defaults, and semantics implemented in `src/main.rs` and the command modules.

### Scenario: CLI defaults or flags change

- **WHEN** a command default, option set, or output mode changes
- **THEN** the README and docs CLI reference are updated in the same change

## Requirement: global memory semantics must stay explicit

The memory limit option MUST document and preserve the meaning of `0` as automatic memory selection.

### Scenario: a user omits an explicit memory budget

- **WHEN** `--memory-limit 0` is used or left at its default
- **THEN** `fqc` treats that value as automatic memory selection rather than a fixed numeric cap
