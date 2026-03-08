# 核心算法

> 另见：[架构设计](architecture.md)、[FQC 文件格式](format-spec.md)

## 总览

fqc 根据读段长度分类选择不同的压缩策略：

| 读段长度 | 序列编码器 | 质量编码器 | 全局排序 |
|----------|-----------|-----------|---------|
| 短 (< 300bp) | ABC (共识 + Delta) | SCM Order-2 | 是 (minimizer) |
| 中 (300bp – 10kbp) | Zstd | SCM Order-2 | 否 |
| 长 (> 10kbp) | Zstd | SCM Order-1 | 否 |

## ABC 算法 (Alignment-Based Compression)

ABC 用于短读段（如 Illumina），利用高序列相似性进行共识 + Delta 编码。

### 处理流程

```
读段 → 全局排序 → 块分割 → 每块:
  1. 构建 Contig (共识 + 对齐)
  2. Delta 编码 (仅存差异)
  3. 序列化 + Zstd 压缩
```

### Step 1: 共识构建

每个块内，读段被聚类为 **contig**（对齐读段簇）：

1. 选取未分配的读段作为种子，用其序列初始化共识
2. 对每个剩余读段，尝试在 `[-max_shift, +max_shift]` 范围内对齐（正向 + 反向互补）
3. 若 Hamming 距离 ≤ 阈值，将读段加入 contig，更新碱基频率计数
4. 所有读段处理完后，以每个位置的多数碱基重新计算最终共识

### Step 2: Delta 编码

Contig 中每条读段相对最终共识进行 Delta 编码：

| 字段 | 类型 | 说明 |
|------|------|------|
| `position_offset` | i16 | 相对共识的对齐偏移 |
| `is_rc` | bool | 是否反向互补 |
| `mismatch_positions` | Vec\<u16\> | 与共识不同的位置 |
| `mismatch_chars` | Vec\<u8\> | 编码后的差异碱基 |

### Step 3: 序列化

每个 contig 序列化为：共识序列（长度前缀）、Delta 数量、每个 delta 的元数据。整个块最终 Zstd 压缩。

## 全局读段排序

对短读段，minimizer 排序将相似读段聚集在一起，提升 ABC 压缩比。

### 算法

1. 从每条读段提取**规范 k-mer minimizer**（正向与反向互补中较小者）
2. 按 minimizer 值排序所有读段
3. 生成双向 `ReorderMap`（正向 + 反向映射）存入归档

### ReorderMap 编码

使用 **ZigZag delta + varint** 编码（`src/reorder_map.rs`）：

```
delta = current_id - previous_id
zigzag = (delta << 1) ^ (delta >> 63)    // 将负数映射到正数
varint: 7 bits/byte, MSB=1 表示还有后续字节
```

## SCM 质量压缩

质量分数使用**统计上下文模型 (SCM)** + 算术编码压缩。

### 上下文模型

| 读段类型 | 上下文阶数 | 上下文来源 |
|----------|-----------|-----------|
| 短 / 中读段 | Order-2 | 前 2 个质量值 |
| 长读段 | Order-1 | 前 1 个质量值 |

### 质量模式

| 模式 | 描述 | 压缩比影响 |
|------|------|-----------|
| Lossless | 精确保留质量值 | 基准 |
| Illumina8Bin | 量化为 8 个代表值 | 提升 ~30% |
| Discard | 全部替换为 `!` (Phred 0) | 最高 |

## ID 压缩

| 模式 | 描述 | 典型场景 |
|------|------|---------|
| Exact | 完整保留 ID | 需要精确 ID 匹配 |
| StripComment | 移除第一个空格后的内容 | 常规用途 |
| Discard | 替换为序号 `@read_N` | 最大压缩 |

## 配对端 (PE) 优化

配对端数据利用 R1/R2 反向互补关系进行优化压缩：

1. 对 R2 序列取反向互补
2. 与 R1 比较相似度
3. 若相似度 > 阈值，仅存储差异位置 + 碱基（delta 编码）
