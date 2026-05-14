# ABC 算法深度解析

ABC（锚基压缩，Anchor-Based Compression）是一种专为短读段 FASTQ 数据设计的压缩算法。本文档详细介绍其工作原理和实现细节。

## 算法概述

ABC 算法的核心思想是将相似的读段分组，构建共识序列，然后以紧凑的方式编码每个读段相对于共识序列的差异。

```mermaid
flowchart TB
    subgraph 输入
        A[原始读段集合]
    end
    
    subgraph 第一阶段: Contig构建
        B[相似性分组]
        C[汉明距离计算]
        D[Contig 分组]
    end
    
    subgraph 第二阶段: 共识构建
        E[列比对]
        F[多数投票]
        G[共识序列]
    end
    
    subgraph 第三阶段: Delta编码
        H[位移计算]
        I[方向检测]
        J[错配记录]
    end
    
    subgraph 第四阶段: 噪声编码
        K[错配位置]
        L[碱基替换编码]
    end
    
    subgraph 第五阶段: 最终压缩
        M[Zstd 压缩]
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

## 阶段一：Contig 构建

Contig 构建阶段将相似的读段按汉明距离分组。

```mermaid
flowchart LR
    A[读段集合] --> B{计算汉明距离}
    B --> C[相似读段组]
    C --> D[Contig 1]
    C --> E[Contig 2]
    C --> F[Contig N...]
    
    D --> G[共识序列 1]
    E --> H[共识序列 2]
    F --> I[共识序列 N...]
```

### 汉明距离计算

汉明距离衡量两个等长字符串之间的差异，定义为对应位置上不同字符的数量：

```
读段 A: ATCGATCGATCG
读段 B: ATCGTTCGATCG
差异位置:    ^
汉明距离: 1
```

### 分组策略

```mermaid
flowchart TD
    A[读段] --> B{与现有 Contig 比较}
    B -->|距离 < 阈值| C[加入现有 Contig]
    B -->|距离 >= 阈值| D{是否可作为新锚点?}
    D -->|是| E[创建新 Contig]
    D -->|否| F[标记为孤立读段]
    
    C --> G[更新共识]
    E --> H[设为锚点]
```

## 阶段二：共识序列构建

为每个 Contig 构建代表序列（共识序列）。

```mermaid
flowchart TB
    subgraph 列比对示例
        A1["读段1: A T C G"]
        A2["读段2: A T C G"]
        A3["读段3: A C C G"]
        A4["读段4: A T C G"]
    end
    
    subgraph 多数投票
        B1["位置1: A(4) → A"]
        B2["位置2: T(3),C(1) → T"]
        B3["位置3: C(4) → C"]
        B4["位置4: G(4) → G"]
    end
    
    subgraph 输出
        C["共识: A T C G"]
    end
    
    A1 --> B1
    A2 --> B1
    A3 --> B1
    A4 --> B1
```

## 阶段三：Delta 编码

将每个读段编码为相对于共识序列的紧凑表示：

```mermaid
flowchart LR
    A[原始读段] --> B[Delta 编码]
    B --> C["(shift, is_rc, mismatches)"]
    
    C --> D[shift: 对齐位移]
    C --> E[is_rc: 是否反向互补]
    C --> F[mismatches: 错配列表]
```

### 编码组件详解

```mermaid
flowchart TB
    subgraph 原始读段与共识
        A["共识序列: A T C G A T C G"]
        B["读段:     - - C G A T T -"]
    end
    
    subgraph 编码结果
        C["shift = 2 (右移2位)"]
        D["is_rc = false (正向)"]
        E["mismatches = [(6, A→T)]"]
    end
    
    A --> C
    B --> D
    B --> E
```

| 字段 | 类型 | 描述 |
|------|------|------|
| `shift` | u8 | 读段相对于共识序列的位移量 |
| `is_rc` | bool | 读段是否为反向互补链 |
| `mismatches` | Vec | 错配位置和替换碱基列表 |

## 阶段四：噪声编码

错配以紧凑的 '0'-'3' 字符形式存储：

```mermaid
flowchart LR
    A[错配信息] --> B[编码转换]
    B --> C["位置 + 碱基编码"]
    
    subgraph 碱基编码
        D["A → '0'"]
        E["C → '1'"]
        F["G → '2'"]
        G["T → '3'"]
    end
```

### 编码示例

```
错配列表: [(2, A→C), (5, G→T)]
编码结果: "213" (位置编码 + 碱基编码组合)
```

## 阶段五：Zstd 最终压缩

所有 Delta 编码数据使用 Zstd 进行最终压缩：

```mermaid
flowchart LR
    A[Delta 编码数据] --> B[Zstd 压缩]
    B --> C[压缩输出]
    
    A --> D["元数据:<br/>- Contig 列表<br/>- 共识序列<br/>- Delta 记录"]
    D --> B
```

## 性能特征

### 压缩效率

| 场景 | 压缩率 | 适用性 |
|------|--------|--------|
| 高覆盖率短读段 | 10-50x | 最佳 |
| 中等覆盖度 | 5-15x | 良好 |
| 低覆盖度 | 2-5x | 一般 |
| 高变异性数据 | < 2x | 不推荐 |

### 计算复杂度

```mermaid
flowchart TD
    subgraph 时间复杂度
        A["Contig 构建: O(n × m × k)<br/>n=读段数, m=读长, k=Contig数"]
        B["共识构建: O(k × m × c)<br/>c=每Contig读段数"]
        C["Delta 编码: O(n × m)"]
        D["最终压缩: O(data_size)"]
    end
    
    subgraph 空间复杂度
        E["内存峰值: O(n + k × m)"]
        F["输出大小: O(k × m + n × d)<br/>d=平均Delta大小"]
    end
```

### 内存使用

| 组件 | 内存消耗 | 可调参数 |
|------|----------|----------|
| Contig 缓存 | 高 | 块大小 |
| 共识存储 | 低 | - |
| Delta 缓冲 | 中 | 批处理大小 |
| Zstd 字典 | 中 | 压缩级别 |

## 适用场景

```mermaid
flowchart TD
    A{读段特征} --> B{读长}
    B -->|< 150bp| C[ABC 推荐]
    B -->|150-500bp| D{覆盖率}
    B -->|> 500bp| E[Zstd 推荐]
    
    D -->|高覆盖| C
    D -->|低覆盖| E
    
    C --> F{变异性}
    F -->|低| G[ABC 最佳]
    F -->|高| H[ABC 一般]
    
    style G fill:#90EE90
    style E fill:#87CEEB
```

## 实现细节

ABC 算法在 `src/algo/` 目录下实现，主要组件包括：

- `contig_builder.rs`：Contig 构建和分组逻辑
- `consensus.rs`：共识序列生成
- `delta_encoder.rs`：Delta 编码实现
- `noise_coder.rs`：噪声/错配压缩

详细的 API 文档请参考各模块的 Rust 文档注释。
