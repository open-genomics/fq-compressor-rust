# compression-architecture change

## ADDED Requirements

### Requirement: compression orchestration must flow through one deep module

The repository MUST centralize compression request normalization, input topology handling, length profiling, reorder eligibility, and execution dispatch behind one internal compression orchestration module instead of scattering that policy across command methods and shallow forwarding seams.

#### Scenario: compression execution policy changes

- **WHEN** a contributor changes archive, streaming, or pipeline orchestration
- **THEN** they update the shared compression request seam and its internal adapters
- **AND** `CompressCommand` remains a thin CLI seam for validation, logging, and exit handling

### Requirement: compression orchestration must expose decision metadata

The shared compression orchestration module MUST return a compression outcome with both `ProcessingStats` and the key decisions the archive path made, including execution mode, detected `ReadLengthClass`, reorder-map usage, and block counts.

#### Scenario: a test needs to verify compression behavior

- **WHEN** a test exercises compression through the orchestration seam
- **THEN** it can assert on the returned outcome without reopening the archive just to recover the mode or reorder decisions
