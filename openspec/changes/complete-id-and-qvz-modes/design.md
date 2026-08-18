# Design: complete-id-and-qvz-modes

`--id-mode` is stored on archive flags as today (`Exact=0`, `Tokenize=1`,
`Discard=2`). `compress_ids` takes `IdMode`: Discard → magic 0x03; Exact →
always exact (0x01); Tokenize → pattern detect then exact fallback.

`qvz` quantizes each Phred value to the nearest of
`[7, 15, 20, 25, 30, 35, 40, 41]`, encodes the index with existing SCM, and
reconstructs the codebook value. Illumina8 bins stay unchanged.

Default CLI id mode is `tokenize` so existing ratio behavior stays; archives
now record Tokenize instead of lying Exact when that path is used.
