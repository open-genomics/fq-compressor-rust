# ABC Algorithm Deep Dive

ABC (Anchor-Based Compression) is a specialized compression algorithm designed for short-read FASTQ data. This document provides a detailed technical analysis of its design, implementation, and performance characteristics.

## Algorithm Overview

The core insight of ABC is that similar reads can be efficiently represented by encoding their differences relative to a shared consensus sequence, rather than storing each read independently.

```mermaid
flowchart TB
    subgraph Input
        A[Raw Read Collection]
    end
    
    subgraph "Phase 1: Contig Building"
        B[Similarity Grouping]
        C[Hamming Distance Calculation]
        D[Contig Assignment]
    end
    
    subgraph "Phase 2: Consensus Building"
        E[Column Alignment]
        F[Majority Voting]
        G[Consensus Sequence]
    end
    
    subgraph "Phase 3: Delta Encoding"
        H[Shift Calculation]
        I[Orientation Detection]
        J[Mismatch Recording]
    end
    
    subgraph "Phase 4: Noise Encoding"
        K[Mismatch Positions]
        L[Base Substitution Encoding]
    end
    
    subgraph "Phase 5: Final Compression"
        M[Zstd Compression]
    end
    
    A --> B --> C --> D
    D --> E --> F --> G
    G --> H --> I --> J
    J --> K --> L
    L --> M
    
    style A fill:#E3F2FD
    style G fill:#C8E6C9
    style M fill:#FFF9C4
```

## Phase 1: Contig Building

The contig building phase groups similar reads based on Hamming distance, creating clusters that share a common reference sequence.

```mermaid
flowchart LR
    A[Read Collection] --> B{Calculate Hamming Distance}
    B --> C[Similar Read Groups]
    C --> D[Contig 1]
    C --> E[Contig 2]
    C --> F[Contig N...]
    
    D --> G[Consensus 1]
    E --> H[Consensus 2]
    F --> I[Consensus N...]
```

### Hamming Distance

Hamming distance measures the difference between two equal-length strings, defined as the count of positions where characters differ:

```
Read A: ATCGATCGATCG
Read B: ATCGTTCGATCG
               ^
Difference at position 4: A→T
Hamming distance: 1
```

### Grouping Strategy

```mermaid
flowchart TD
    A[Incoming Read] --> B{Compare with Existing Contigs}
    B -->|Distance < Threshold| C[Add to Existing Contig]
    B -->|Distance >= Threshold| D{Can Be New Anchor?}
    D -->|Yes| E[Create New Contig]
    D -->|No| F[Mark as Orphan Read]
    
    C --> G[Update Consensus]
    E --> H[Set as Anchor]
```

The threshold is dynamically adjusted based on read length and observed variability in the data.

## Phase 2: Consensus Sequence Construction

For each contig, a representative consensus sequence is built through column-wise alignment and majority voting.

```mermaid
flowchart TB
    subgraph "Column Alignment Example"
        A1["Read 1: A T C G"]
        A2["Read 2: A T C G"]
        A3["Read 3: A C C G"]
        A4["Read 4: A T C G"]
    end
    
    subgraph "Majority Voting"
        B1["Pos 1: A(4) → A"]
        B2["Pos 2: T(3),C(1) → T"]
        B3["Pos 3: C(4) → C"]
        B4["Pos 4: G(4) → G"]
    end
    
    subgraph Output
        C["Consensus: A T C G"]
    end
    
    A1 --> B1
    A2 --> B1
    A3 --> B1
    A4 --> B1
```

The consensus sequence serves as the reference for delta encoding all reads in the contig.

## Phase 3: Delta Encoding

Each read is encoded as a compact representation relative to its consensus sequence:

```mermaid
flowchart LR
    A[Original Read] --> B[Delta Encoding]
    B --> C["(shift, is_rc, mismatches)"]
    
    C --> D["shift: alignment offset"]
    C --> E["is_rc: reverse complement flag"]
    C --> F["mismatches: difference list"]
```

### Encoding Components

```mermaid
flowchart TB
    subgraph "Read vs Consensus Alignment"
        A["Consensus: A T C G A T C G"]
        B["Read:       - - C G A T T -"]
    end
    
    subgraph "Encoding Result"
        C["shift = 2 (right shift by 2)"]
        D["is_rc = false (forward strand)"]
        E["mismatches = [(6, A→T)]"]
    end
    
    A --> C
    B --> D
    B --> E
```

| Field | Type | Description |
|-------|------|-------------|
| `shift` | u8 | Offset of read relative to consensus start |
| `is_rc` | bool | Whether read is reverse complemented |
| `mismatches` | Vec | List of (position, substitution) pairs |

### Reverse Complement Handling

When a read matches better in reverse complement orientation:

```
Consensus:     5'-ATCGATCG-3'
Read (RC):     5'-CGATCGAT-3'
Aligned:       5'-ATCGATCG-3' (after RC transformation)
Encoding:      is_rc = true, shift = 0, mismatches = []
```

## Phase 4: Noise Encoding

Mismatches are stored using compact '0'-'3' character encoding:

```mermaid
flowchart LR
    A[Mismatch Information] --> B[Encoding Transform]
    B --> C["Position + Base Code"]
    
    subgraph "Base Encoding Table"
        D["A → '0'"]
        E["C → '1'"]
        F["G → '2'"]
        G["T → '3'"]
    end
```

### Encoding Example

```
Mismatch list: [(2, A→C), (5, G→T)]
Position encoding: 2, 5
Substitution encoding: C='1', T='3'
Combined: "2_1_5_3" (compact binary format)
```

The noise encoding achieves approximately 2 bits per mismatch, compared to 8 bits for naive character storage.

## Phase 5: Zstd Final Compression

All delta-encoded data undergoes final Zstd compression:

```mermaid
flowchart LR
    A[Delta Encoded Data] --> B[Zstd Compression]
    B --> C[Compressed Output]
    
    A --> D["Metadata:<br/>- Contig list<br/>- Consensus sequences<br/>- Delta records"]
    D --> B
```

Zstd is chosen for its:
- Fast decompression speed (critical for random access)
- Configurable compression levels
- Good compression ratio on structured data

## Performance Characteristics

### Compression Efficiency

| Scenario | Compression Ratio | Suitability |
|----------|-------------------|-------------|
| High-coverage short reads | 10-50x | Optimal |
| Medium coverage | 5-15x | Good |
| Low coverage | 2-5x | Moderate |
| High variability data | < 2x | Not recommended |

### Computational Complexity

```mermaid
flowchart TD
    subgraph "Time Complexity"
        A["Contig Building: O(n × m × k)<br/>n=reads, m=length, k=contigs"]
        B["Consensus Building: O(k × m × c)<br/>c=reads per contig"]
        C["Delta Encoding: O(n × m)"]
        D["Final Compression: O(data_size)"]
    end
    
    subgraph "Space Complexity"
        E["Peak Memory: O(n + k × m)"]
        F["Output Size: O(k × m + n × d)<br/>d=average delta size"]
    end
```

### Memory Usage

| Component | Memory Consumption | Tunable Parameter |
|-----------|-------------------|-------------------|
| Contig Cache | High | Block size |
| Consensus Storage | Low | - |
| Delta Buffer | Medium | Batch size |
| Zstd Dictionary | Medium | Compression level |

### Performance Benchmarks

Typical performance on Illumina short-read data (150bp paired-end):

| Operation | Throughput | Memory |
|-----------|------------|--------|
| Compression | 50-100 MB/s | 2-4 GB |
| Decompression | 200-500 MB/s | 512 MB |
| Random Access | < 1 ms per block | 64 MB |

## Applicability Decision Tree

```mermaid
flowchart TD
    A{Read Characteristics} --> B{Read Length}
    B -->|< 150bp| C[ABC Recommended]
    B -->|150-500bp| D{Coverage}
    B -->|> 500bp| E[Zstd Recommended]
    
    D -->|High Coverage| C
    D -->|Low Coverage| E
    
    C --> F{Variability}
    F -->|Low| G[ABC Optimal]
    F -->|High| H[ABC Moderate]
    
    style G fill:#90EE90
    style E fill:#87CEEB
```

## Implementation Details

The ABC algorithm is implemented in the `src/algo/` directory:

| Module | Responsibility |
|--------|----------------|
| `contig_builder.rs` | Contig construction and read grouping |
| `consensus.rs` | Consensus sequence generation |
| `delta_encoder.rs` | Delta encoding implementation |
| `noise_coder.rs` | Noise/mismatch compression |

### Key Data Structures

```rust
struct Contig {
    consensus: Vec<u8>,           // Consensus sequence
    reads: Vec<ReadAssignment>,   // Reads in this contig
}

struct ReadAssignment {
    read_id: usize,
    shift: u8,
    is_rc: bool,
    mismatches: Vec<(usize, u8)>, // (position, substitution)
}
```

### Integration with Block Compressor

ABC operates at the block level within the broader compression pipeline:

```mermaid
flowchart LR
    A[Block of Reads] --> B[ABC Compression]
    B --> C[Compressed Block]
    
    subgraph "ABC Pipeline"
        D[Contig Building]
        E[Consensus]
        F[Delta Encoding]
        G[Zstd]
    end
    
    A --> D --> E --> F --> G --> C
```

## Limitations and Trade-offs

### When ABC Excels

- Short reads (≤511 bp) with high similarity
- High-coverage sequencing data
- Low genetic variability within samples
- Illumina-style paired-end reads

### When to Use Alternatives

- Long reads (>500 bp): Use direct Zstd
- Low coverage (<10x): Zstd may be more efficient
- High variability: Consider skipping consensus phase
- Nanopore/PacBio data: Not suitable for ABC

## References

- [Architecture Overview](../architecture/index.md)
- [Binary Format Specification](../reference/format-spec.md)
- [CONTEXT.md](https://github.com/LessUp/fq-compressor-rust/blob/master/CONTEXT.md) - Domain terminology
