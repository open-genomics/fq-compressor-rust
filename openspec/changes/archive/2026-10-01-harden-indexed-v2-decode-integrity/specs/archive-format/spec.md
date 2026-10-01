# MODIFIED Requirements

## Requirement: Encoded identifiers match implementation

The indexed reader SHALL reject unknown checksum identifiers and SHALL use the
codec revision nibble to distinguish ABC payload revisions. ABC codec revision
0 represents historical ABC v1/v2 payloads; revision 1 represents the
lossless ABC v3 payload.

#### Scenario: ABC v3 is explicitly identified

- **WHEN** the writer emits an ABC v3 sequence stream
- **THEN** its stream codec byte SHALL be `0x11`
- **AND** a reader SHALL reject a header/payload revision contradiction.

## Requirement: Canonical block layout and optional logical checksum

Each indexed block SHALL describe the writer's contiguous IDs, sequence,
quality, and auxiliary streams exactly. A nonzero `block_xxhash64` SHALL use
the historical fully lossless logical-record XxHash64 layout. Blocks that
discard IDs or apply a lossy quality mode SHALL store zero in that optional
field; archive-wide compressed-stream verification remains available.

#### Scenario: Frozen lossless archive verifies

- **WHEN** `verify` runs on the committed indexed-v2 fixture
- **THEN** it SHALL succeed with its stored block checksum.

#### Scenario: Lossy block is structurally verified

- **WHEN** a block discards IDs or quality data is lossy
- **THEN** its optional logical block checksum SHALL be zero
- **AND** decoding SHALL still validate structure and record lengths.

#### Scenario: Malformed block layout is rejected

- **WHEN** stream offsets overlap, leave a gap, or disagree with the declared
  payload size
- **THEN** opening the archive SHALL fail before stream allocation.
