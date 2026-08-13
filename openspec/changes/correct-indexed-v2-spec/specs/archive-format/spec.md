# Indexed Archive Format Change Specification

## ADDED Requirements

### Requirement: Indexed archive identity
The implementation SHALL identify its archive contract as `fqc-indexed/v2`, while retaining command `fqc`, extension `.fqc`, and the existing 8-byte indexed magic.

#### Scenario: Reader accepts indexed family
- **GIVEN** an archive beginning with `89 46 51 43 0D 0A 1A 0A`
- **WHEN** the indexed reader opens it
- **THEN** the reader SHALL continue with indexed format version validation

### Requirement: Encoded identifiers match implementation
The normative specification SHALL define codec identifiers using the exact encoding emitted and consumed by the audited writer and reader, including the checksum identifier value and meaning.

#### Scenario: Codec identifier is documented and tested
- **WHEN** a block header is serialized with a supported codec family and version
- **THEN** its identifier SHALL equal `(family << 4) | version`
- **AND** a structure test SHALL verify the exact byte

#### Scenario: Checksum identifier zero
- **WHEN** checksum ID `0` appears in an indexed v2 archive
- **THEN** the specification and reader SHALL interpret it as the implemented XxHash64 variant

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
