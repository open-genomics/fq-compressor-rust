# ADR-003: Component Encoding

## Status

Accepted

## Context

FASTQ records contain four distinct components:

1. **Identifier (ID)**: Read name, often with instrument/run/flowcell information
2. **Sequence (Seq)**: DNA/RNA bases (A, C, G, T, N)
3. **Quality (Qual)**: Phred scores for each base
4. **Auxiliary (Aux)**: Optional metadata tags

Each component has different characteristics:

| Component | Alphabet | Pattern | Typical Size |
|-----------|----------|---------|--------------|
| ID | ASCII | Instrument-specific format | 20-100 bytes |
| Sequence | A, C, G, T, N | Biological patterns | 50-30,000 bytes |
| Quality | ASCII 33-126 | Instrument error model | Same as sequence |
| Aux | ASCII | Key:value pairs | 0-200 bytes |

A unified compression strategy cannot optimally handle all these different data types.

## Decision

We encode each component in **separate, independent streams** with component-specific codecs:

```mermaid
flowchart TB
    A[FASTQ Record] --> B[ID Stream]
    A --> C[Sequence Stream]
    A --> D[Quality Stream]
    A --> E[Aux Stream]
    
    B --> F[Codec Selection]
    C --> G[Codec Selection]
    D --> H[Codec Selection]
    E --> I[Codec Selection]
    
    F --> J[ID Encoding]
    G --> K[Seq Encoding]
    H --> L[Qual Encoding]
    I --> M[Aux Encoding]
    
    J --> N[Compressed Block]
    K --> N
    L --> N
    M --> N
```

### Stream-Specific Codecs

```mermaid
flowchart TB
    subgraph IDStream["ID Stream"]
        ID1[Parse ID format]
        ID2[Extract structure]
        ID3[Zstd compress]
    end
    
    subgraph SeqStream["Sequence Stream"]
        SEQ1{Read Length?}
        SEQ1 -->|Short| SEQ2[ABC Encoding]
        SEQ1 -->|Medium/Long| SEQ3[Zstd Direct]
        SEQ2 --> SEQ4[Zstd Final]
        SEQ3 --> SEQ4
    end
    
    subgraph QualStream["Quality Stream"]
        Q1{Quality Mode?}
        Q1 -->|Lossless| Q2[Zstd compress]
        Q1 -->|Illumina8| Q3[Bin + Zstd]
        Q1 -->|Discard| Q4[Skip storage]
    end
    
    subgraph AuxStream["Aux Stream"]
        AUX1[Serialize tags]
        AUX2[Zstd compress]
    end
```

## Component-Specific Strategies

### ID Stream Encoding

```mermaid
flowchart LR
    A["@SEQ_ID:1:1106:2234:1234/1"] --> B[Extract structure]
    B --> C[Instrument: SEQ_ID]
    B --> D[Run: 1]
    B --> E[Flowcell: 1106]
    B --> F[Coordinates: 2234:1234]
    B --> G[Pair: /1]
    
    C --> H[Structured encoding]
    D --> H
    E --> H
    F --> H
    G --> H
    
    H --> I[Zstd compress]
```

**Strategy**: Extract common prefixes and patterns, encode structure, compress with Zstd.

### Sequence Stream Encoding

```mermaid
flowchart TB
    A[Sequence Data] --> B{Read Length Class}
    
    B -->|Short ≤150bp| C[ABC Path]
    B -->|Medium 150-1000bp| D[Zstd Path]
    B -->|Long >1000bp| D
    
    subgraph ABCPath["ABC Encoding"]
        C --> C1[Build contigs]
        C1 --> C2[Generate consensus]
        C2 --> C3[Delta encode]
        C3 --> C4[Noise encode]
        C4 --> C5[Zstd compress]
    end
    
    subgraph ZstdPath["Direct Zstd"]
        D --> D1[Pack bases]
        D1 --> D2[Zstd compress]
    end
```

**Strategy**: Use ABC for short reads with high similarity; direct Zstd for others.

### Quality Stream Encoding

```mermaid
flowchart TB
    A[Quality Scores] --> B{Quality Mode}
    
    B -->|none| C[Preserve raw scores]
    B -->|illumina8| D[Bin to 8 levels]
    B -->|qvz| E[QVZ context model]
    B -->|discard| F[Store placeholder]
    
    C --> G[Zstd compress]
    D --> H[Encode bins]
    H --> I[Zstd compress]
    E --> J[Context compress]
    F --> K[No storage needed]
```

**Strategy**: Default to lossless; offer binning for size reduction; discard when quality unused.

### Aux Stream Encoding

```mermaid
flowchart TB
    A[Aux Tags] --> B{Tags present?}
    B -->|No| C[Skip stream]
    B -->|Yes| D[Serialize to compact form]
    D --> E[Zstd compress]
```

**Strategy**: Optional stream; only present when auxiliary data exists.

## Block Structure

Each block contains the four streams independently:

```mermaid
flowchart TB
    subgraph Block["Block Structure"]
        A[Block Header]
        B[ID Stream<br/>codec_ids, offset_ids, size_ids]
        C[Seq Stream<br/>codec_seq, offset_seq, size_seq]
        D[Qual Stream<br/>codec_qual, offset_qual, size_qual]
        E[Aux Stream<br/>codec_aux, offset_aux, size_aux]
    end
    
    A --> B
    A --> C
    A --> D
    A --> E
```

## Alternatives Considered

### Alternative 1: Unified Record Encoding

Compress complete FASTQ records as single units.

```mermaid
flowchart LR
    A[FASTQ Record] --> B[Serialize to line format]
    B --> C[Zstd compress]
```

**Pros:**
- Simpler implementation
- Single compression operation
- Natural text-like structure

**Cons:**
- Suboptimal compression - each component's patterns are mixed
- No component-level optimization
- No selective decompression
- Quality and sequence treated identically despite different alphabets

**Rejected because**: Component patterns are fundamentally different and benefit from specialized handling.

### Alternative 2: Column-Based Storage

Store all IDs, then all sequences, then all qualities.

```
[ID1][ID2]...[IDN][SEQ1][SEQ2]...[SEQN][QUAL1][QUAL2]...[QUALN]
```

**Pros:**
- Excellent compression - similar data adjacent
- Clear separation of concerns
- Simple structure

**Cons:**
- Poor random access - must seek across multiple regions
- Complex decompression - need to gather from multiple locations
- Memory overhead - must read all columns to reconstruct records

**Rejected because**: Random access is a key requirement for the block-indexed format.

### Alternative 3: Interleaved with Shared Dictionary

Store records interleaved but with a shared compression dictionary.

```mermaid
flowchart LR
    A[Train dictionary on sample] --> B[Compress all records with dict]
```

**Pros:**
- Better compression than naive interleaving
- Preserves record structure
- Standard Zstd feature

**Cons:**
- Dictionary training is expensive
- Dictionary must be stored with archive
- Still treats all components uniformly
- Less effective for heterogeneous data

**Rejected because**: Component-specific encoding provides better compression than shared dictionaries.

## Consequences

### Positive

1. **Optimal compression per component**: Each stream uses the best codec for its data type
2. **Independent tuning**: Compression parameters can be adjusted per stream
3. **Selective decompression**: Can decompress only needed components
4. **Clear codec reporting**: Users can see which codec was used for each stream
5. **Extensible**: New codecs can be added per component without affecting others

### Negative

1. **More complex block structure**: Four streams to manage per block
2. **Additional header fields**: Block header must specify four codecs and offsets
3. **Potential fragmentation**: Four separate streams may have access pattern implications

### Mitigations

```mermaid
flowchart TB
    A[Block Write] --> B[Compute all stream sizes]
    B --> C[Write header with offsets]
    C --> D[Write streams contiguously]
    D --> E[Single seek to block start<br/>reads entire block]
```

**Access pattern**: Blocks are written contiguously; single seek + read retrieves all streams.

## Codec Selection Logic

```mermaid
flowchart TB
    A[Component Type] --> B{Which?}
    
    B -->|ID| C[Always Zstd]
    B -->|Sequence| D{Read Length + Similarity}
    B -->|Quality| E{Quality Mode Setting}
    B -->|Aux| F{Present?}
    
    D -->|Short + High similarity| G[ABC]
    D -->|Otherwise| H[Zstd]
    
    E -->|Lossless| I[Zstd]
    E -->|Binned| J[Bin + Zstd]
    E -->|Discarded| K[Raw/Empty]
    
    F -->|Yes| L[Zstd]
    F -->|No| M[Empty]
```

## Stream Metadata in Block Header

| Field | Description | Usage |
|-------|-------------|-------|
| `codec_ids` | ID stream codec | Decoder selection |
| `codec_seq` | Sequence stream codec | Decoder selection |
| `codec_qual` | Quality stream codec | Decoder selection |
| `codec_aux` | Aux stream codec | Decoder selection |
| `offset_ids` | ID stream offset in block | Seek position |
| `offset_seq` | Seq stream offset in block | Seek position |
| `offset_qual` | Qual stream offset in block | Seek position |
| `offset_aux` | Aux stream offset in block | Seek position |
| `size_ids` | ID stream compressed size | Read length |
| `size_seq` | Seq stream compressed size | Read length |
| `size_qual` | Qual stream compressed size | Read length |
| `size_aux` | Aux stream compressed size | Read length |

## References

- [Format Specification](../../reference/format-spec.md) - Block header details
- [ABC Algorithm](../../algorithms/abc-deep-dive.md) - Sequence encoding details
- [Algorithms Overview](../../algorithms/index.md) - Quality modes
