# Verification: complete-id-and-qvz-modes

- Status: `Completed`
- Ready to archive: `yes`

| Case | Evidence | Result |
|---|---|---|
| Exact does not tokenize | `test_id_exact_mode_does_not_tokenize_patterned_ids` | passed |
| Tokenize uses tokenize tag | same test (`ID_MAGIC_TOKENIZE`) | passed |
| Default CLI id mode | `test_e2e_default_id_mode_is_tokenize` | passed |
| Exact / discard flags | same e2e test | passed |
| QVZ codebook unit | `test_quality_qvz_quantizes_to_codebook` | passed |
| QVZ e2e flags + reconstruct | `test_e2e_qvz_quality_quantizes_to_codebook` | passed |

Commands: `cargo fmt --check`, `clippy -D warnings`, `test --lib --tests`, `doc --no-deps`.
