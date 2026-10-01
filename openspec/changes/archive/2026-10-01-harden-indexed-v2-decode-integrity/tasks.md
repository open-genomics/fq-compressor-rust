# Tasks: harden-indexed-v2-decode-integrity

## 1. Regressions and compatibility

- [x] 1.1 Freeze checksum behavior with the committed indexed-v2 fixture.
- [x] 1.2 Cover lossy/discard checksum omission and automatic block validation.
- [x] 1.3 Cover ABC codec revision, case/IUPAC preservation, and malformed payloads.

## 2. Decoder hardening

- [x] 2.1 Validate identifiers, global flags, stream layout, and reorder maps.
- [x] 2.2 Make ABC, Zstd sequence, aux, and map varint decoders fail closed.
- [x] 2.3 Reject inconsistent stream counts/lengths and invalid CLI ranges.

## 3. Documentation and verification

- [x] 3.1 Synchronize format documentation and CHANGELOG.
- [x] 3.2 Run fmt, clippy, lib/integration tests, docs, frozen-fixture verify, and diff checks.
