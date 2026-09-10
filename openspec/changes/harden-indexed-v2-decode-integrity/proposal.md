# Change Proposal: harden-indexed-v2-decode-integrity

## Metadata

- Status: `Applying`
- Capability: `archive-format`, `decode-budget`, `cli-modes`

## Why

The current worktree uncovered several correctness and safety gaps in the
indexed v2 decoder. In particular, an unversioned block-checksum rewrite makes
the committed v2 fixture fail `verify`; malformed layouts and map payloads can
be accepted too far into decoding; and the ABC stream needs an explicit codec
revision for its lossless v3 payload.

## Changes

- Preserve the existing v2 logical block checksum for fully lossless blocks.
  Blocks with discarded or lossy fields omit that optional checksum and remain
  protected by the archive-wide compressed-stream checksum.
- Encode ABC payload v3 as codec `AbcV1` revision 1, while continuing to read
  historical revision-0 ABC payloads.
- Reject malformed stream layout, unknown identifiers/flag values, malformed
  varints/reorder maps, inconsistent decoded stream counts, and invalid ranges.
- Keep ordinary FASTQ compression lossless by rejecting invalid Phred+33 input
  at production parser entry points.

## Out of scope

- Cryptographic authentication or a new archive major version.
- Replacing the existing archive-wide XxHash64 design.
- Archiving the separate documentation-only format-family change.
