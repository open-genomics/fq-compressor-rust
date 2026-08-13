# Design: dispatch-all-stream-codecs

`decompress_raw` takes four codec bytes. Each is parsed with
`CodecFamily::try_from_nibble` + version nibble; allowed families:

| Stream | Families |
|---|---|
| ids | Raw, DeltaZstd |
| seq | AbcV1, ZstdPlain |
| qual | Raw, ScmV1, ScmOrder1 |
| aux | DeltaVarint |

Version must be 0. Global `id_mode`/`quality_mode` must agree with whether the
stream codec is Raw (discard) or not. Decoders are constructed from the header
family, not from the config-built trait objects alone.
