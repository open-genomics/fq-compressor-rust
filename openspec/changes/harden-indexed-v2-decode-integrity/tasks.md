# Tasks: harden-indexed-v2-decode-integrity

## 1. Regressions and compatibility

- [ ] 1.1 Freeze checksum behavior with the committed indexed-v2 fixture.
- [ ] 1.2 Cover lossy/discard checksum omission and automatic block validation.
- [ ] 1.3 Cover ABC codec revision, case/IUPAC preservation, and malformed payloads.

## 2. Decoder hardening

- [ ] 2.1 Validate identifiers, global flags, stream layout, and reorder maps.
- [ ] 2.2 Make ABC, Zstd sequence, aux, and map varint decoders fail closed.
- [ ] 2.3 Reject inconsistent stream counts/lengths and invalid CLI ranges.

## 3. Documentation and verification

- [ ] 3.1 Synchronize format documentation and CHANGELOG.
- [ ] 3.2 Run fmt, clippy, lib/integration tests, docs, frozen-fixture verify, and diff checks.
