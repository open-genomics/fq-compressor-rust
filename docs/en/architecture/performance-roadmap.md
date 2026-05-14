# Performance Roadmap

This page is the maintained summary for `fqc` performance work. It captures the current shape of the problem and the next bounded slice without turning the docs site into a research archive.

## Architecture Overview

```mermaid
flowchart TB
    subgraph Input["Input Processing"]
        A[FASTQ Input] --> B{Read Length Analysis}
        B -->|Short| C[ABC Path]
        B -->|Medium/Long| D[Zstd Path]
    end
    
    subgraph Memory["Memory Strategy"]
        E{Mode Selection}
        E -->|Default| F[Archive Mode<br/>Global Analysis]
        E -->|--streaming| G[Streaming Mode<br/>Incremental]
        E -->|--pipeline| H[Pipeline Mode<br/>Staged]
    end
    
    subgraph Output["Archive Writing"]
        I[Block Assembly]
        J[Block Index]
        K[Reorder Map]
    end
    
    C --> I
    D --> I
    F --> I
    G --> I
    H --> I
    I --> J
    I --> K
```

## Current Bottlenecks

### Memory Pressure Analysis

```mermaid
flowchart LR
    subgraph Phase1["Phase 1: Analysis"]
        A1[Full Dataset Scan] --> A2[Read Classification]
        A2 --> A3[Reorder Planning]
        A3 --> A4[Memory Peak]
    end
    
    subgraph Phase2["Phase 2: Encoding"]
        B1[Block Encoding] --> B2[Stream Compression]
        B2 --> B3[Archive Write]
    end
    
    A4 -.->|"High Memory"| B1
    
```

- **Short-read compression still pays for a global analysis pass.** The main compression path can analyze the full dataset before writing blocks so reorder-aware compression improves ratio, but that also concentrates memory and wall-clock pressure in phase 1.
- **Lower-memory modes trade flexibility for predictability.** `--streaming` avoids global reordering and pipeline mode spreads work across stages, but the project still needs clearer guidance on when those modes are the preferred foundation.
- **Memory semantics need one explicit contract.** Future implementation work should align on `--memory-limit 0` meaning automatic memory selection so tuning and docs describe the same behavior.

## Execution Mode Comparison

| Feature | Archive Mode | Streaming Mode | Pipeline Mode |
|---------|--------------|----------------|---------------|
| Memory Usage | High (full dataset) | Low (incremental) | Medium (staged) |
| Reordering | Full optimization | Disabled | Partial |
| Compression Ratio | Best | Good | Good |
| Use Case | Small/medium files | Large files | Pipeline integration |

```mermaid
flowchart TB
    subgraph ArchiveMode["Archive Mode"]
        A1[Load All Reads] --> A2[Analyze & Reorder]
        A2 --> A3[Optimal Block Write]
    end
    
    subgraph StreamingMode["Streaming Mode"]
        S1[Read Chunk] --> S2[Compress Block]
        S2 --> S3[Write Block]
        S3 --> S1
    end
    
    subgraph PipelineMode["Pipeline Mode"]
        P1[Stage 1: Parse] --> P2[Stage 2: Encode]
        P2 --> P3[Stage 3: Compress]
        P3 --> P4[Stage 4: Write]
    end
```

## Recommended Direction

1. Keep the next slices focused on memory predictability and measured hot paths rather than new codecs or broad rewrites.
2. Treat streaming and pipeline flows as the practical foundation for follow-up optimization, with reorder-heavy paths improved only after memory behavior is explicit.

## Phase 1 Scope

This slice only establishes shared direction:

- capture the maintained roadmap in public docs
- record the intended `--memory-limit 0` semantics for later implementation alignment

No runtime performance behavior changes ship in phase 1.

## Memory Limit Semantics

```mermaid
flowchart TB
    A["--memory-limit 0"] --> B[Automatic Selection]
    B --> C{Dataset Size?}
    C -->|Small| D[In-Memory Processing]
    C -->|Medium| E[Block-Based Processing]
    C -->|Large| F[Streaming Processing]
    
    D --> G[Optimal Compression]
    E --> G
    F --> H[Good Compression<br/>Lower Memory]
```

### Future Contract

| Flag | Current Behavior | Target Behavior |
|------|------------------|-----------------|
| `--memory-limit 0` | Unspecified | Automatic selection based on dataset |
| `--memory-limit N` | Hint only | Hard limit with fallback strategies |

## Deferred Follow-up

Later slices can build on this summary by:

- implementing automatic memory selection consistently behind `--memory-limit 0`
- adding targeted measurement for parser, reorder, pipeline, and archive-writing hotspots
- narrowing any larger algorithm or data-structure changes to separately reviewable OpenSpec slices

## Performance Measurement Points

```mermaid
flowchart LR
    subgraph HotPaths["Identified Hot Paths"]
        H1[FASTQ Parser]
        H2[Reorder Algorithm]
        H3[ABC Encoding]
        H4[Zstd Compression]
        H5[Archive Writing]
    end
    
    subgraph Metrics["Target Metrics"]
        M1[Throughput MB/s]
        M2[Memory Peak]
        M3[Block Latency]
        M4[Total Time]
    end
    
    H1 --> M1
    H2 --> M2
    H3 --> M3
    H4 --> M3
    H5 --> M4
```
