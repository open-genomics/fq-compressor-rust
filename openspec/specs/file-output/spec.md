# File Output

## Requirements

### Requirement: Ordinary file outputs are transactional
Compress and decompress writers that target an ordinary filesystem path SHALL
write to a temporary file in the destination's directory and SHALL replace the
final path only after the writer has been flushed and closed successfully.

#### Scenario: Target missing and mid-run failure
- **GIVEN** the final output path does not exist
- **WHEN** compression or decompression fails after creating a temporary file
- **THEN** the final path SHALL still be absent
- **AND** the temporary file SHALL be removed

#### Scenario: Force overwrite keeps old content on failure
- **GIVEN** the final output path already exists and `--force` is set
- **WHEN** the operation fails before commit
- **THEN** the original final path content SHALL remain unchanged

#### Scenario: Successful replace
- **GIVEN** a successful compress or decompress to an ordinary path
- **WHEN** the command exits successfully
- **THEN** the final path SHALL contain the complete output
- **AND** no leftover transaction temporary shall remain beside it

### Requirement: Existing targets require force
When the final path exists and force overwrite is false, the operation SHALL
fail before creating or truncating any output file.

#### Scenario: Refuse without force
- **GIVEN** an existing output path
- **WHEN** compress or decompress runs without force
- **THEN** the command SHALL fail with a usage/invalid-argument error
- **AND** the existing file content SHALL be unchanged

### Requirement: Split-PE is a logical two-file transaction
`--split-pe` SHALL write both R1 and R2 through temporary files and SHALL
commit them only after both streams have been fully written and flushed.
POSIX cannot atomically rename two paths; commit order is R1 then R2.
If the second commit fails after the first, the implementation SHALL report
the error and document the limitation.

#### Scenario: Split-PE success
- **GIVEN** a paired archive and `--split-pe`
- **WHEN** decompression succeeds
- **THEN** both derived R1 and R2 paths SHALL exist with complete content

### Requirement: Stdout is non-transactional
Writing to stdout (`-`) SHALL NOT use filesystem rename transactions and SHALL
NOT attempt to roll back already emitted pipe data.

#### Scenario: Stdout path
- **GIVEN** output path `-`
- **WHEN** decompression or compression writes to stdout
- **THEN** no temporary file beside a destination path is created
