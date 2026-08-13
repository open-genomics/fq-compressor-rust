# Verification: dispatch-all-stream-codecs

- Status: `Completed`
- Ready to archive: `yes`
- Date: 2026-08-13

| Requirement | Evidence | Result |
|---|---|---|
| Stream codecs drive decode | `tests/test_codec_dispatch.rs` (ids/seq/qual/aux wrong family + version) | passed |
| Flags cannot override | `rejects_quality_codec_contradicting_global_flags` | passed |
| Existing paths | full `cargo test --lib --tests` | passed |

Commands: fmt check, clippy `-D warnings`, test, doc — all exit 0.
