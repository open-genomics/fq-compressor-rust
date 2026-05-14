# Algorithms Overview

`fqc` uses different strategies for different FASTQ components and read-length classes.

## Processing Pipeline

```mermaid
flowchart TB
    subgraph Input["Input"]
        A[FASTQ File]
    end
    
    subgraph Parse["Parsing"]
        B[Record Iterator]
        B --> C[Read Length Classification]
    end
    
    subgraph Path["Path Selection"]
        C --> D{Read Length Class}
        D -->|Short| E[ABC Encoding Path]
        D -->|Medium| F[Zstd Encoding Path]
        D -->|Long| F
    end
    
    subgraph Encode["Encoding"]
        E --> G[Component Streams]
        F --> G
        G --> H[ID Stream]
        G --> I[Seq Stream]
        G --> J[Qual Stream]
        G --> K[Aux Stream]
    end
    
    subgraph Output["Output"]
        H --> L[Block Assembly]
        I --> L
        J --> L
        K --> L
        L --> M[.fqc Archive]
    end
    
    A --> B
```

## Sequence Path Decision Tree

```mermaid
flowchart TB
    A[Input Reads] --> B{Analyze Read Lengths}
    B --> C[Compute Length Distribution]
    C --> D{Dominant Class?}
    
    D -->|Short ≤150bp| E[Short-Read Path]
    D -->|Medium 150-1000bp| F[Medium-Read Path]
    D -->|Long >1000bp| G[Long-Read Path]
    
    subgraph ShortPath["Short-Read Path"]
        E --> E1{Block Size?}
        E1 -->|Small| E2[ABC Consensus/Delta]
        E1 -->|Large| E3[Zstd Direct]
        E2 --> E4[Contig Building]
        E4 --> E5[Delta Encoding]
        E5 --> E6[Zstd Compression]
        E3 --> E6
    end
    
    subgraph MediumPath["Medium/Long Path"]
        F --> F1[Zstd Backed]
        G --> G1[Zstd Backed]
        F1 --> F2[Block Storage]
        G1 --> G2[Block Storage]
    end
    
    E6 --> H[Encoded Blocks]
    F2 --> H
    G2 --> H
```

### Sequence Path Details

- **Short-read path**: `fqc` prefers an ABC-style consensus/delta representation for smaller short-read blocks and falls back to Zstd-backed block storage when larger blocks would make that path too expensive.
- **Medium and long reads**: sequence payloads are stored with a Zstd-backed path instead of the short-read consensus model.

The current implementation classifies reads using observed lengths rather than a single CLI preset.

## Quality Path Classification

```mermaid
flowchart TB
    A[Quality Strings] --> B{Quality Mode}
    
    B -->|none| C[Lossless Preservation]
    B -->|illumina8| D[8-Level Binning]
    B -->|qvz| E[QVZ Compression]
    B -->|discard| F[Placeholder Values]
    
    subgraph Lossless["Lossless Mode"]
        C --> C1[Original scores]
        C1 --> C2[Zstd compress]
    end
    
    subgraph Binning["Illumina8 Mode"]
        D --> D1[Bin to 8 levels]
        D1 --> D2[Smaller alphabet]
        D2 --> D3[Better compression]
    end
    
    subgraph QVZ["QVZ Mode"]
        E --> E1[Quality-Value Zstd]
        E1 --> E2[Context modeling]
    end
    
    subgraph Discard["Discard Mode"]
        F --> F1[Store placeholder]
        F1 --> F2[Reconstruct as 'B']
    end
```

Quality strings are compressed separately from sequences:

| Mode | Description | Output |
|------|-------------|--------|
| `none` | Keeps quality scores lossless | Original quality values |
| `illumina8` | Bins qualities to 8 levels | Reduced alphabet, better compression |
| `qvz` | Quality-Value Zstd compression | Context-modeled compression |
| `discard` | Replaces qualities with placeholders | Placeholder 'B' values on decode |

## Reordering Pipeline

```mermaid
flowchart TB
    A[Input Reads] --> B{Archive Mode?}
    B -->|Archive + Short SE| C[Reordering Enabled]
    B -->|Streaming/Pipeline| D[Reordering Disabled]
    B -->|Paired-end| D
    
    subgraph ReorderFlow["Reorder Flow"]
        C --> C1[Compute Similarity]
        C1 --> C2[Group by Locality]
        C2 --> C3[Build Reorder Map]
        C3 --> C4[Store Forward Map]
        C3 --> C5[Store Reverse Map]
    end
    
    subgraph DirectFlow["Direct Flow"]
        D --> D1[Preserve Input Order]
        D1 --> D2[No Reorder Metadata]
    end
    
    C4 --> E[Compressed Archive]
    C5 --> E
    D2 --> E
    
    E --> F{Decompress}
    F --> G[Apply Reverse Map]
    G --> H[Original Order Output]
```

For short single-end archives in non-streaming mode, `fqc` can reorder reads to improve locality and compression efficiency. The archive stores reorder metadata when needed so original-order decompression remains possible.

### Reordering Benefits

```mermaid
flowchart LR
    subgraph Before["Before Reordering"]
        A1[Read 1: ACGT...] 
        A2[Read 2: TGCA...]
        A3[Read 3: ACGT...]
        A4[Read 4: GGCC...]
    end
    
    subgraph After["After Reordering"]
        B1[Read 1: ACGT...]
        B2[Read 3: ACGT...]
        B3[Read 2: TGCA...]
        B4[Read 4: GGCC...]
    end
    
    subgraph Benefit["Compression Benefit"]
        C1[Similar reads adjacent]
        C2[Better delta encoding]
        C3[Higher compression ratio]
    end
    
    Before --> After
    After --> Benefit
```

## Paired-End Layout

Paired-end input can be ingested from separate files or interleaved input and stored with either interleaved or consecutive archive layout metadata.

```mermaid
flowchart TB
    A[Paired-End Input] --> B{Input Format}
    B -->|R1 + R2 Files| C[Split Input]
    B -->|Interleaved| D[Single File]
    
    C --> E{Storage Layout}
    D --> E
    
    E -->|Interleaved| F[R1/R2/R1/R2...]
    E -->|Consecutive| G[R1...R1 | R2...R2]
    
    F --> H[Archive with PE Metadata]
    G --> H
```

## Component Stream Overview

```mermaid
flowchart TB
    A[FASTQ Record] --> B[ID Component]
    A --> C[Sequence Component]
    A --> D[Quality Component]
    A --> E[Aux Component]
    
    B --> F[ID Stream Encoder]
    C --> G[Seq Stream Encoder]
    D --> H[Qual Stream Encoder]
    E --> I[Aux Stream Encoder]
    
    F --> J[Compressed ID Block]
    G --> K[Compressed Seq Block]
    H --> L[Compressed Qual Block]
    I --> M[Compressed Aux Block]
    
    J --> N[Block Container]
    K --> N
    L --> N
    M --> N
```

Encoding components separately allows:

1. **Optimal codec selection** per component type
2. **Independent compression tuning** for ID vs sequence vs quality
3. **Selective decompression** when only certain components are needed
