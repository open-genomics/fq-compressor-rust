# Design: recognize-sequential-fqc-family

Classify the first 8 bytes with `classify_magic` / `magic_dispatch_error`
before reading the version byte. Sequential magic maps to
`UnsupportedFormat` with a locked message naming `fqc-sequential/v2` and
`open-genomics/fq-compressor`. Unknown and truncated magics stay `Format`.
No sequential decoding is implemented.

Fixture: copy of C++ `frozen_se.fqc` under `tests/fixtures/foreign-sequential-v2/`.
