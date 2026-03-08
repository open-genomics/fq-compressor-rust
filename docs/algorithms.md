# 压缩算法

> 另见：[architecture.md](architecture.md)（模块结构）、[format-spec.md](format-spec.md)（二进制格式）

## 总览

fqc 根据读段长度分类选择不同的压缩策略：

| 读段长度 | 序列编码器 | 质量编码器 | 全局排序 |
|----------|-----------|-----------|---------|
| 短 (< 300bp) | ABC (共识 + Delta) | SCM Order-2 | 是 (minimizer) |
| 中 (300bp – 10kbp) | Zstd | SCM Order-2 | 否 |
| 长 (> 10kbp) | Zstd | SCM Order-1 | 否 |

实现位于 `src/algo/` 目录下：

| 模块 | 职责 |
|------|------|
| `block_compressor.rs` | 块级 ABC/Zstd 压缩与解压 |
| `global_analyzer.rs` | Minimizer 提取与全局排序 |
| `quality_compressor.rs` | SCM 算术编码质量压缩 |
| `id_compressor.rs` | 读段 ID 压缩 |
| `pe_optimizer.rs` | 配对端反向互补优化 |

---

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

**共识**以每位置碱基频率 `[A, C, G, T]` 存储，最终取多数碱基。

### Step 2: Delta 编码

Contig 中每条读段相对最终共识进行 Delta 编码：

| 字段 | 类型 | 说明 |
|------|------|------|
| `position_offset` | i16 | 相对共识的对齐偏移 |
| `is_rc` | bool | 是否反向互补 |
| `mismatch_positions` | Vec\<u16\> | 与共识不同的位置 |
| `mismatch_chars` | Vec\<u8\> | 编码后的差异碱基 |

差异碱基编码规则：
- 重叠区域内的位置：noise 编码（类 XOR）
- 共识范围外的位置：原始碱基

### Step 3: 序列化

每个 contig 序列化为：

1. 共识序列（长度前缀）
2. Delta 数量
3. 每个 delta：original_order, offset, is_rc, read_length, mismatch count, positions, chars

整个块最终 Zstd 压缩。

### 负偏移处理

当 `shift < 0` 时，读段延伸到共识起始位置之前：

- `cons_start = 0`, `read_start = |shift|`
- 重叠前的碱基（位置 `0..read_start`）作为原始碱基存入 mismatch 数据
- 解压时直接恢复（不经过 noise 解码）

---

## 全局读段排序

对短读段，minimizer 排序将相似读段聚集在一起，提升 ABC 压缩比。

实现位于 `src/algo/global_analyzer.rs`。

### 算法

1. 从每条读段提取**规范 k-mer minimizer**（正向与反向互补中较小者）
2. 按 minimizer 值排序所有读段
3. 生成双向 `ReorderMap`（正向 + 反向映射）存入归档

### ReorderMap 编码

重排映射使用 **ZigZag delta + varint** 编码（`src/reorder_map.rs`）：

1. 计算相邻 ID 的差值（delta）
2. ZigZag 编码处理负数：`(n << 1) ^ (n >> 63)`
3. 无符号 varint 编码压缩

---

## SCM 质量压缩

质量分数使用**统计上下文模型 (SCM)** + 算术编码压缩。

实现位于 `src/algo/quality_compressor.rs`。

### 上下文模型

| 读段类型 | 上下文阶数 | 上下文来源 |
|----------|-----------|-----------|
| 短 / 中读段 | Order-2 | 前 2 个质量值 |
| 长读段 | Order-1 | 前 1 个质量值 |

### 算术编码

- 每个上下文维护自适应频率模型
- 当总频率超过阈值时重新缩放
- 32 位精度算术编码器/解码器

### 质量模式

| 模式 | 描述 | 压缩比影响 |
|------|------|-----------|
| Lossless | 精确保留质量值 | 基准 |
| Illumina8Bin | 量化为 8 个代表值 (2,6,15,22,27,33,37,40) | 提升 ~30% |
| Discard | 全部替换为 `!` (Phred 0) | 最高 |

---

## ID 压缩

读段标识符根据 ID 模式压缩。

实现位于 `src/algo/id_compressor.rs`。

| 模式 | 描述 | 典型场景 |
|------|------|---------|
| Exact | 完整保留 ID | 需要精确 ID 匹配 |
| StripComment | 移除第一个空格后的内容 | 常规用途 |
| Discard | 替换为序号 `@read_N` | 最大压缩 |

ID 以换行分隔拼接后 Zstd 压缩为单个数据流。

---

## Zstd 编解码器（中/长读段）

对 > 300bp 的读段，序列使用长度前缀编码后 Zstd 压缩：

```
[u16: read_length][sequence bytes] × N reads
```

Zstd 压缩级别可配置（1-19，默认 3）。

---

## 配对端 (PE) 优化

配对端数据利用 R1/R2 反向互补关系进行优化压缩。

实现位于 `src/algo/pe_optimizer.rs`。

### 算法

1. 对 R2 序列取反向互补
2. 与 R1 比较相似度
3. 若相似度 > 阈值，仅存储差异位置 + 碱基（delta 编码）
4. 质量差异类似处理

### PE 存储布局

| 布局 | 存储方式 | 描述 |
|------|---------|------|
| Interleaved | R1, R2, R1, R2, ... | 交替存储读段对 |
| Consecutive | R1, R1, ..., R2, R2, ... | 先存所有 R1 再存所有 R2 |
