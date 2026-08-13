# File Output

## Requirements

### Requirement: Ordinary file outputs are transactional
Compress and decompress writers that target an ordinary filesystem path SHALL
write to a temporary file in the destination's directory and SHALL replace the
final path only after the writer has been flushed and closed successfully.

### Requirement: Existing targets require force
When the final path exists and force overwrite is false, the operation SHALL
fail before creating or truncating any output file.

### Requirement: Split-PE is a logical two-file transaction
`--split-pe` SHALL write both R1 and R2 through temporary files and SHALL
commit them only after both streams have been fully written and flushed.
POSIX cannot atomically rename two paths; commit order is R1 then R2.

### Requirement: Stdout is non-transactional
Writing to stdout (`-`) SHALL NOT use filesystem rename transactions.
