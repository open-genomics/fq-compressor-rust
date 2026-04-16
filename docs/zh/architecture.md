# 项目架构

## 概述

fqc 是一个高性能 FASTQ 压缩器，采用分层模块化架构设计。核心设计围绕**块级压缩**展开：输入数据被分割为固定大小的块，每块独立压缩，支持随机访问。

## 数据流

### 压缩流程

```
FASTQ 输入
    │
    ▼
┌─────────────┐     ┌──────────────────┐
│ FASTQ 解析器 │────▶│   全局分析器      │  (可选) Minimizer 排序
│  fastq/      │     │ global_analyzer  │  生成 ReorderMap
└─────────────┘     └────────┬─────────┘
                             │
                    ┌────────▼─────────┐
                    │     块分割        │  按 block_size 切分
                    └────────┬─────────┘
                             │ (并行处理)
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
        ┌──────────┐  ┌──────────┐  ┌──────────┐
        │  Block 0 │  │  Block 1 │  │  Block N │
        │  压缩     │  │  压缩    │  │  压缩    │
        └────┬─────┘  └────┬─────┘  └────┬─────┘
             │              │              │
             ▼              ▼              ▼
        ┌─────────────────────────────────────┐
        │           FQC 写入器                 │  写入 header + blocks
        │  → 块索引 + 文件尾部 + 校验和          │  + 索引 + 尾部
        └─────────────────────────────────────┘
```

### 解压流程

```
FQC 文件
    │
    ▼
┌──────────────┐
│  FQC 读取器   │  读取 header + 块索引
└──────┬───────┘
       │ (随机访问或顺序)
       ▼
┌──────────────┐     ┌─────────────────┐
│   块解压      │────▶│  重排序恢复      │  (可选) 恢复原始顺序
└──────────────┘     └────────┬────────┘
                              ▼
                         FASTQ 输出
```

### Pipeline 模式

Pipeline 模式使用 3 阶段流水线，通过 crossbeam 有界通道实现背压：

```
┌────────┐  通道   ┌────────────┐  通道   ┌────────┐
│ 读取器 │────────▶│  压缩器    │────────▶│ 写入器 │
│(串行)  │  有界   │ (并行)     │  有界   │(串行)  │
└────────┘         └────────────┘         └────────┘
```

## 模块结构

```
src/
├── main.rs                    # CLI 入口 (clap)
├── lib.rs                     # 库入口 (pub mod 导出)
│
├── algo/                      # 压缩算法
│   ├── block_compressor.rs    # 块压缩/解压 (ABC + Zstd 双路径)
│   ├── global_analyzer.rs     # 全局读段分析 + minimizer 排序
│   ├── id_compressor.rs       # 读段 ID 压缩 (精确/去注释/丢弃)
│   ├── pe_optimizer.rs        # 配对端 (PE) 反向互补优化
│   └── quality_compressor.rs  # 质量值 SCM 算术编码
│
├── commands/                  # CLI 子命令实现
│   ├── compress.rs            # 压缩命令 (默认/流式/pipeline)
│   ├── decompress.rs          # 解压命令 (顺序/并行/重排序)
│   ├── info.rs                # 信息命令 (归档信息显示)
│   └── verify.rs              # 验证命令 (完整性校验)
│
├── common/
│   └── memory_budget.rs       # 系统内存检测 + 动态分块
│
├── fastq/
│   └── parser.rs              # FASTQ 解析器 (单端/配对端/交错/采样/验证)
│
├── io/
│   ├── async_io.rs            # 异步 I/O (预读/写后缓冲)
│   └── compressed_stream.rs   # 透明解压流 (.gz/.bz2/.xz/.zst)
│
├── pipeline/
│   ├── compression.rs         # 3 阶段压缩流水线
│   └── decompression.rs       # 3 阶段解压流水线
│
├── error.rs                   # FqcError 枚举 + 退出码映射 (0-5)
├── format.rs                  # FQC 二进制格式结构体 (头部/块/尾部)
├── fqc_reader.rs              # FQC 归档读取器 (随机访问)
├── fqc_writer.rs              # FQC 归档写入器 (块索引)
├── reorder_map.rs             # 双向重排映射 (ZigZag + varint)
└── types.rs                   # 核心类型与常量
```

## 核心模块职责

### `algo/block_compressor.rs`

块级压缩/解压的核心逻辑。根据读段长度选择不同编码器：

- **短读段 (< 300bp)** → ABC 算法：共识构建 + 增量编码 + Zstd
- **中等读段 (300bp – 10kbp)** → Zstd 直接压缩（长度前缀编码）
- **长读段 (> 10kbp)** → Zstd 直接压缩

每块包含 4 个独立压缩流：ID、序列、质量值、辅助数据。

### `algo/global_analyzer.rs`

全局读段分析器，执行 minimizer 排序：

1. 从每条读段提取规范 k-mer minimizer
2. 按 minimizer 值排序，使相似读段相邻
3. 生成 `ReorderMap`（双向映射）存入归档

### `algo/quality_compressor.rs`

质量值压缩器，使用统计上下文模型 (SCM) + 算术编码：

- 二阶上下文（短/中等读段）：以 2 个前序质量值为上下文
- 一阶上下文（长读段）：以 1 个前序质量值为上下文
- 自适应频率模型 + 32 位精度算术编码

### `pipeline/`

基于 crossbeam-channel 的 3 阶段流水线：

- **读取器** — 串行读取 FASTQ，按 chunk 发送
- **压缩器** — 使用 Rayon 并行压缩各块
- **写入器** — 串行写入，AsyncWriter 提供写后缓冲

通过有界通道实现背压，`PipelineControl` 提供取消和进度追踪。

### `error.rs`

统一错误体系：

| 退出码 | 含义 |
|--------|------|
| 0 | 成功 |
| 1 | 通用错误 |
| 2 | I/O 错误 |
| 3 | 格式错误 |
| 4 | 校验和不匹配 |
| 5 | 参数错误 |

### `reorder_map.rs`

双向读段重排映射：

- `forward_map[original_id] → archive_id`
- `reverse_map[archive_id] → original_id`
- 使用 ZigZag 增量 + varint 编码实现紧凑存储

## 依赖关系

```
main.rs
  └── commands/*
        ├── algo/*           # 压缩算法
        ├── pipeline/*       # 流水线 (可选)
        ├── fastq/parser     # 输入解析
        ├── io/*             # I/O 层
        ├── fqc_reader       # 归档读取
        ├── fqc_writer       # 归档写入
        └── reorder_map      # 重排映射
```

## 关键设计决策

1. **块独立性** — 每块可独立压缩/解压，支持随机访问和并行处理
2. **编解码器分离** — 序列/质量值/ID 使用独立的编解码器和压缩流
3. **双路径策略** — 短读段用 ABC（高压缩比），中长读段用 Zstd（通用）
4. **背压流水线** — 有界通道防止内存溢出，适应不同 I/O 速度
5. **unsafe 禁用** — 全局禁止 unsafe 代码（仅 Windows FFI 例外）
