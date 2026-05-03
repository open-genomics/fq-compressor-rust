# developer-workflow change

## ADDED Requirements

### Requirement: performance work must stay bounded to reviewable worktree slices

Contributors MUST break performance-oriented work into isolated worktree slices with an explicit OpenSpec change instead of broad autonomous sprawl.

#### Scenario: a contributor prepares a performance slice

- **WHEN** the work affects performance planning, docs, or implementation direction
- **THEN** they use a dedicated worktree and a bounded OpenSpec change for that slice
- **AND** keep the work scoped for local validation and review before moving to the next slice
