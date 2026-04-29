# cli-surface change

## MODIFIED Requirements

### Requirement: global memory semantics must stay explicit

The memory limit option MUST document and preserve the meaning of `0` as automatic memory selection based on available memory rather than as an unlimited fixed cap.

#### Scenario: a user omits an explicit memory budget

- **WHEN** `--memory-limit 0` is used or left at its default
- **THEN** `fqc` treats that value as automatic memory selection
- **AND** the maintained docs and future implementation changes describe it with that meaning consistently
