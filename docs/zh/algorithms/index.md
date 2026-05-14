# 算法概述

`fqc` 对不同的 FASTQ 组件和读长类别使用不同的压缩策略。

## 序列路径决策树

```mermaid
flowchart TD
    A[FASTQ 输入] --> B{读长分类}
    B -->|短读段| C[ABC 路径]
    B -->|中等读段| D[Zstd 路径]
    B -->|长读段| D
    
    C --> E{块大小评估}
    E -->|小块| F[共识/增量编码]
    E -->|大块| G[Zstd 后备]
    
    F --> H[Contig 构建]
    H --> I[增量编码]
    I --> J[Zstd 最终压缩]
    
    G --> J
    D --> K[Zstd 直接压缩]
    
    J --> L[压缩输出]
    K --> L
    
    style C fill:#E8F5E9
    style F fill:#C8E6C9
```

- **短读段路径**：`fqc` 对较小的短读段块优先使用 ABC 风格的共识/增量表示，当较大块会使该路径成本过高时回退到 Zstd 支持的块存储。
- **中等和长读段**：序列负载使用 Zstd 支持的路径存储，而非短读段共识模型。

当前实现使用观测到的长度而非单一 CLI 预设来分类读段。

## 质量路径分类

质量字符串与序列分开压缩：

```mermaid
flowchart LR
    A[质量字符串] --> B{质量模式}
    
    B -->|none| C[无损保留]
    B -->|illumina8| D[分箱压缩]
    B -->|qvz| E[QVZ 压缩]
    B -->|discard| F[丢弃替换]
    
    C --> G[完整质量分数]
    D --> H[8 级分箱]
    E --> I[有损压缩]
    F --> J[占位符]
    
    style C fill:#90EE90
    style D fill:#87CEEB
    style E fill:#DDA0DD
    style F fill:#FFB6C1
```

| 模式 | 描述 | 数据保留 |
|------|------|----------|
| `none` | 保持质量分数无损 | 完整保留 |
| `illumina8` | 将质量分箱到 8 个级别 | 有损分箱 |
| `qvz` | QVZ 压缩算法 | 类型表面暴露 |
| `discard` | 解码时用占位符替换 | 丢弃 |

## 重排管道

对于非流式模式下的短单端归档，`fqc` 可以重排读段以提高局部性和压缩效率：

```mermaid
flowchart TD
    A[原始读段序列] --> B[相似性分析]
    B --> C[局部性优化]
    C --> D[重排后序列]
    D --> E[压缩]
    
    E --> F[存储重排元数据]
    F --> G[归档输出]
    
    G --> H[解压]
    H --> I[逆重排映射]
    I --> J[恢复原始顺序]
    
    style C fill:#FFF9C4
    style F fill:#E1BEE7
```

归档在需要时存储重排元数据，因此仍可按原始顺序解压。

## 双端布局

```mermaid
flowchart LR
    subgraph 输入格式
        A1[分离文件<br/>R1/R2]
        A2[交错输入]
    end
    
    subgraph 归档布局
        B1[交错布局]
        B2[连续布局]
    end
    
    A1 --> B1
    A1 --> B2
    A2 --> B1
    A2 --> B2
    
    B1 --> C[配对元数据]
    B2 --> C
```

双端输入可从分离文件或交错输入中获取，并以交错或连续的归档布局元数据存储。
