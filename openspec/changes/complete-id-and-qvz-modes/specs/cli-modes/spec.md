# CLI ID and Quality Modes

## ADDED Requirements

### Requirement: ID mode is a CLI choice
`compress` SHALL accept `--id-mode exact|tokenize|discard`. Default SHALL be
`tokenize`. The chosen mode SHALL be stored in archive flags.

#### Scenario: Exact does not tokenize
- **GIVEN** patterned Illumina-style IDs
- **WHEN** compress uses `--id-mode exact`
- **THEN** the ID stream uses the exact encoding tag
- **AND** decompress restores the original IDs

#### Scenario: Tokenize may tokenize
- **GIVEN** the same patterned IDs
- **WHEN** compress uses `--id-mode tokenize`
- **THEN** the ID stream uses the tokenize encoding tag
- **AND** decompress restores the original IDs

### Requirement: QVZ is a lossy quality mode
`--lossy-quality qvz` SHALL quantize quality values to a fixed 8-level
codebook and SHALL NOT be an alias of lossless.

#### Scenario: QVZ reconstructs codebook values
- **GIVEN** mixed Phred characters
- **WHEN** compress uses `--lossy-quality qvz` and decompresses
- **THEN** each quality character is one of the QVZ codebook reconstructions
- **AND** the output is not required to match the original string
