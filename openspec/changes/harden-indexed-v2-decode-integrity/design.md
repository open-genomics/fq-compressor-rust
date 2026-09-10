# Design: harden-indexed-v2-decode-integrity

## Compatibility

`checksum_type = 0` retains its existing XxHash64 meaning. A nonzero
`block_xxhash64` continues to hash the original fully lossless logical record
layout; this is required for the frozen v2 fixture. A block that intentionally
does not preserve IDs or qualities stores zero and relies on the existing
archive-wide compressed-stream checksum during `verify`.

ABC already carries an internal payload revision. Its outer codec revision now
matches that payload: revision 0 reads historical ABC v1/v2 payloads and
revision 1 reads the lossless ABC v3 payload. Decoder/header contradictions
are rejected.

## Validation boundaries

Before allocating or emitting records, the reader/decoders validate:

1. Known global/block identifiers and legal flag values.
2. Canonical, contiguous stream layout matching the writer.
3. Exact counts, varint termination, stream exhaustion, valid UTF-8, and
   sequence/quality length agreement.
4. Reorder maps as mutually inverse permutations of `0..total_reads`.

The CLI rejects inverted inclusive ranges before creating output. Production
FASTQ openers validate Phred+33 bytes because the quality codec is defined only
for that alphabet; the lower-level parser remains configurable for library
callers.
