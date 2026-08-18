# Compress Archive Budget Change Specification

## ADDED Requirements

### Requirement: Compress automatic limit is finite
`--memory-limit 0` on archive compress SHALL resolve to a finite budget
(approximately 75% of available memory, with hard structural ceilings) and
SHALL NOT skip the ingest check.

#### Scenario: Automatic is finite
- **GIVEN** `--memory-limit 0`
- **WHEN** archive compress estimates ingest peak
- **THEN** the estimate is compared against a resolved finite limit

### Requirement: Over-budget archive ingest fails before output
When the running ingest peak estimate exceeds the resolved budget, archive
compress SHALL fail with `ResourceLimit` and SHALL NOT create the final `.fqc`.

#### Scenario: Explicit low limit
- **GIVEN** `--memory-limit 16` and input whose archive peak estimate exceeds 16 MB
- **WHEN** archive compress runs
- **THEN** the command fails with `ResourceLimit`
- **AND** the final output path is absent (or unchanged under `--force`)

### Requirement: Tiny inputs succeed
A valid minimal FASTQ SHALL compress under `--memory-limit 0` and `--memory-limit 16`.

#### Scenario: Low-budget happy path
- **GIVEN** the repository's tiny SE fixture
- **WHEN** archive compress runs with `--memory-limit 0` or `16`
- **THEN** the command succeeds

### Requirement: Streaming is not this check
`--streaming` SHALL NOT apply the archive full-ingest peak check.

#### Scenario: Streaming accepts large-for-archive input
- **GIVEN** input that archive mode would reject at `--memory-limit 16`
- **WHEN** compress runs with `--streaming --memory-limit 16`
- **THEN** the command succeeds

### Requirement: Pipeline full ingest uses the same budget
`--pipeline` SHALL apply the same ingest peak check as archive mode. It SHALL
NOT become a bounded-memory streaming path.

#### Scenario: Pipeline over-budget
- **GIVEN** input that archive mode rejects at `--memory-limit 16`
- **WHEN** compress runs with `--pipeline --memory-limit 16`
- **THEN** the command fails with `ResourceLimit`
- **AND** the final output path is absent
