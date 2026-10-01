# Indexed Archive Format

Baseline contract for `fqc-indexed/v2`.

## Requirements

### Requirement: Indexed archive identity
The implementation SHALL identify its archive contract as `fqc-indexed/v2`, while retaining command `fqc`, extension `.fqc`, and the existing 8-byte indexed magic.

#### Scenario: Reader accepts indexed family
- **GIVEN** an archive beginning with `89 46 51 43 0D 0A 1A 0A`
- **WHEN** the indexed reader opens it
- **THEN** the reader SHALL continue with indexed format version validation

### Requirement: Encoded identifiers match implementation
The indexed reader SHALL reject unknown checksum identifiers and SHALL use the
codec revision nibble to distinguish ABC payload revisions. ABC codec revision
0 represents historical ABC v1/v2 payloads; revision 1 represents the
lossless ABC v3 payload.

#### Scenario: Codec identifier is documented and tested
- **WHEN** a block header is serialized with a supported codec family and version
- **THEN** its identifier SHALL equal `(family << 4) | version`
- **AND** a structure test SHALL verify the exact byte

#### Scenario: Checksum identifier zero
- **WHEN** checksum ID `0` appears in an indexed v2 archive
- **THEN** the specification and reader SHALL interpret it as the implemented XxHash64 variant

#### Scenario: ABC v3 is explicitly identified
- **WHEN** the writer emits an ABC v3 sequence stream
- **THEN** its stream codec byte SHALL be `0x11`
- **AND** a reader SHALL reject a header/payload revision contradiction.

### Requirement: Canonical block layout and optional logical checksum

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

### Requirement: Version compatibility is explicit
The indexed reader SHALL accept only the format major versions implemented by the audited reader, and the documentation SHALL NOT claim an unimplemented v1 fallback.

#### Scenario: Unsupported major
- **GIVEN** the indexed magic followed by an unsupported major version
- **WHEN** the reader opens the archive
- **THEN** it SHALL return an unsupported-version error before block decoding

### Requirement: Frozen decoder fixture
The repository SHALL contain a small indexed v2 archive and manifest sufficient to prove future decoder compatibility with the audited format.

#### Scenario: Future reader decodes frozen archive
- **GIVEN** the committed archive and its original input
- **WHEN** the current reader decompresses the archive
- **THEN** the output SHALL match the original input exactly
- **AND** the manifest hashes SHALL match

### Requirement: Unknown identifiers fail closed
The reader SHALL reject unknown codec, checksum, or incompatible version identifiers without falling back to another algorithm.

#### Scenario: Unknown codec identifier
- **GIVEN** an otherwise structurally valid fixture with an unsupported codec identifier
- **WHEN** the reader parses the relevant header
- **THEN** it SHALL return a format/unsupported-codec error
- **AND** it SHALL NOT silently use command-line defaults

### Requirement: Block stream codecs drive decode
The decoder SHALL select ID, sequence, quality and aux decoders from the four
codec bytes in the block header, and SHALL reject unknown families, families
not allowed for that stream, and unimplemented versions before interpreting
payload bytes.

#### Scenario: Inapplicable family on a stream
- **GIVEN** a block whose one stream codec byte is a known family illegal for that stream
- **WHEN** the block is decompressed
- **THEN** decoding SHALL fail with an unsupported-format error naming the block ID, stream, and codec byte

### Requirement: Global flags cannot override stream codecs
When global quality or ID mode implies discard/exact semantics that contradict
the block's declared stream family, the decoder SHALL return a format error.

### Requirement: Cross-family magic is rejected
The reader SHALL classify the first 8 bytes before version or header parsing.
The C++ `fqc-sequential/v2` magic (`46 51 43 56 32 0D 0A 1A`) SHALL fail as a
known unsupported family. Unknown and truncated magics SHALL remain distinct
format errors. Sequential archives SHALL NOT be decoded.

#### Scenario: Sequential family
- **GIVEN** an archive beginning with `46 51 43 56 32 0D 0A 1A`
- **WHEN** info, verify, or decompress opens it
- **THEN** the command SHALL fail with an unsupported-format-family error
  naming `fqc-sequential/v2` and `open-genomics/fq-compressor`
- **AND** decompress SHALL NOT create the output path
