# Decode Resource Budget Specification

## ADDED Requirements

### Requirement: Decode operations use an operation-scoped budget
`decompress` and full `verify` SHALL resolve `--memory-limit` into a
per-operation `DecodeBudget`. Automatic mode (`0`) SHALL select a finite
budget and SHALL NOT mean unlimited memory.

#### Scenario: Automatic is finite
- **GIVEN** `--memory-limit 0`
- **WHEN** decompress or verify opens an archive
- **THEN** allocations are still capped by a resolved finite budget and hard
  structural ceilings

#### Scenario: Explicit limit is honored
- **GIVEN** `--memory-limit N` with `N >= 16`
- **WHEN** an archive declares sizes whose estimated peak exceeds the budget
- **THEN** the operation fails with `ResourceLimit` naming location, declared,
  and allowed values

### Requirement: Declared sizes are checked before allocation
Readers SHALL reject oversized `num_blocks`, stream sizes, and reorder map
sizes before allocating based on attacker-controlled fields.

#### Scenario: Huge block count
- **GIVEN** a forged block index `num_blocks` exceeding budget or file region
- **WHEN** the archive is opened
- **THEN** open fails with a controlled `ResourceLimit` or format error
- **AND** the process does not allocate the forged entry count

### Requirement: Zstd decompress is bounded
All zstd decompress paths used while reading archives SHALL pass an explicit
output ceiling; unbounded `decode_all` SHALL NOT be used on archive data.

#### Scenario: Expansion exceeds ceiling
- **GIVEN** compressed bytes that expand beyond the allowed output size
- **WHEN** bounded decompress runs
- **THEN** it fails with `ResourceLimit` or a decompress error that includes
  the ceiling

### Requirement: Original-order fails before creating outputs
When `--original-order` is requested, the implementation SHALL estimate peak
memory and SHALL fail before creating FASTQ output files if the estimate
exceeds the budget.

#### Scenario: Peak over budget
- **GIVEN** an archive whose original-order peak estimate exceeds the limit
- **WHEN** decompress runs with `--original-order`
- **THEN** the command fails with `ResourceLimit`
- **AND** the final FASTQ path is not created (or remains unchanged under force)

### Requirement: Tiny fixtures succeed under a low budget
A valid minimal archive SHALL decompress and verify under a low but legal
budget (e.g. 16–64 MB).

#### Scenario: Low-budget happy path
- **GIVEN** a tiny valid `.fqc` and `--memory-limit 16`
- **WHEN** decompress or verify runs
- **THEN** the operation succeeds
