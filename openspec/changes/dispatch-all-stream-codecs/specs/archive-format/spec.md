# Archive Format Delta — Stream Codec Dispatch

## ADDED Requirements

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
