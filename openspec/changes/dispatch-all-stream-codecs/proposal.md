# Change Proposal: dispatch-all-stream-codecs

## Metadata

- Status: `Applying`
- Repository: `open-genomics/fq-compressor-rust`
- Capability: `archive-format`
- Task IDs: `FQCR-CODEC-001`
- Prerequisites: `correct-indexed-v2-spec` (FQCR-SPEC-001)

## Why

Block headers store four codec IDs, but decompression only consults `codec_seq`
and derives ID/quality/aux decoders from global flags. Tampered or future
per-stream codecs can be silently mis-decoded.

## Changes

**Header-driven stream codec dispatch**

- From: `decompress_raw` takes only `codec_seq`; other streams use config-built compressors.
- To: every stream selects its decoder from its header codec byte against an
  allow-list; unknown family/version and flag contradictions fail closed.
- Reason: make the on-disk header the decode truth.
- Impact: no writer/format byte changes for valid archives; invalid headers that
  previously might decode wrongly now error.

## Out of scope

- Plugin codecs, memory budget, format-family recognition, product rename.
