# 命令行参考

## 全局选项

| 选项 | 说明 |
|------|------|
| `--version` | 打印版本 |
| `--help` | 打印帮助 |
| `-v, --verbose` | 增加详细程度（-v 信息，-vv 调试） |
| `-q, --quiet` | 静默非错误输出 |
| `-t, --threads <N>` | 线程数（0 = 自动检测） |
| `--memory-limit <MB>` | 内存限制 MB（0 = 自动） |
| `--no-progress` | 禁用进度显示 |

## compress

将 FASTQ 文件压缩为 FQC 格式。

```
fqc compress [OPTIONS] -i <INPUT> -o <OUTPUT>
```

### 必需选项

| 选项 | 缩写 | 说明 |
|------|------|------|
| `--input <FILE>` | `-i` | 输入 FASTQ 文件（`-` 表示 stdin） |
| `--output <FILE>` | `-o` | 输出 FQC 文件 |

### 压缩选项

| 选项 | 缩写 | 默认值 | 说明 |
|------|------|--------|------|
| `--level <N>` | `-l` | `6` | 压缩级别 (1-9) |
| `--lossy-quality <MODE>` | | `none` | 质量模式：`none`（无损）、`illumina8`、`qvz`、`discard` |
| `--long-read-mode <MODE>` | | `auto` | 强制读段类型：`auto`、`short`、`medium`、`long` |

### 处理选项

| 选项 | 默认值 | 说明 |
|------|--------|------|
| `--pipeline` | false | 启用 3 阶段流水线模式 |
| `--streaming` | false | 流式模式（不进行全局重排，用于 stdin） |
| `--reorder` | true | 启用全局读段重排（仅短读段） |

### 配对端选项

| 选项 | 说明 |
|------|------|
| `--input2 <FILE>` | `-2` | 第二个输入文件（PE 分离文件） |
| `--interleaved` | 输入为交错配对端 |
| `--pe-layout <LAYOUT>` | PE 存储方式：`interleaved`（默认）、`consecutive` |

### 高级选项

| 选项 | 默认值 | 说明 |
|------|--------|------|
| `--max-block-bases <N>` | `0` | 每块最大碱基数（0 = 自动） |
| `--scan-all-lengths` | false | 扫描所有读段检测长度（较慢但准确） |
| `--force` | `-f` | 覆盖已存在的输出文件 |

### 示例

```bash
# 基本压缩
fqc compress -i reads.fastq -o reads.fqc

# 最大压缩
fqc compress -i reads.fastq -o reads.fqc -l 9

# 从 stdin 流式压缩
cat reads.fastq | fqc compress --streaming -i - -o reads.fqc

# 流水线模式（最佳吞吐）
fqc compress -i reads.fastq -o reads.fqc --pipeline

# 配对端（分离文件）
fqc compress -i R1.fastq -2 R2.fastq -o paired.fqc

# 丢弃质量值以获得最小输出
fqc compress -i reads.fastq -o reads.fqc --lossy-quality discard

# 压缩输入（自动检测）
fqc compress -i reads.fastq.gz -o reads.fqc
```

## decompress

将 FQC 文件解压为 FASTQ 格式。

```
fqc decompress [OPTIONS] -i <INPUT> -o <OUTPUT>
```

### 必需选项

| 选项 | 缩写 | 说明 |
|------|------|------|
| `--input <FILE>` | `-i` | 输入 FQC 文件 |
| `--output <FILE>` | `-o` | 输出 FASTQ 文件（`-` 表示 stdout） |

### 提取选项

| 选项 | 说明 |
|------|------|
| `--range <START:END>` | 提取读段范围（1-based，如 `1:1000`、`100:`） |
| `--header-only` | 仅输出读段头部（ID） |
| `--original-order` | 按原始顺序输出读段（需要 reorder map） |

### 处理选项

| 选项 | 默认值 | 说明 |
|------|--------|------|
| `--pipeline` | false | 启用 3 阶段流水线模式 |
| `--skip-corrupted` | false | 跳过损坏块而非失败 |
| `--split-pe` | false | 分离配对端输出到 R1/R2 文件 |

### 其他选项

| 选项 | 说明 |
|------|------|
| `--corrupted-placeholder <SEQ>` | 损坏读段的占位序列 |
| `--force` | `-f` | 覆盖已存在的输出文件 |

### 示例

```bash
# 完整解压
fqc decompress -i reads.fqc -o reads.fastq

# 提取前 1000 条读段
fqc decompress -i reads.fqc -o subset.fastq --range 1:1000

# 输出到 stdout
fqc decompress -i reads.fqc -o -

# 恢复原始顺序
fqc decompress -i reads.fqc -o reads.fastq --original-order

# 分离配对端
fqc decompress -i paired.fqc -o output.fastq --split-pe
# 生成 output_R1.fastq 和 output_R2.fastq

# 流水线模式
fqc decompress -i reads.fqc -o reads.fastq --pipeline
```

## info

显示归档信息。

```
fqc info [OPTIONS] -i <INPUT>
```

| 选项 | 缩写 | 说明 |
|------|------|------|
| `--input <FILE>` | `-i` | 输入 FQC 文件 |
| `--json` | | 以 JSON 格式输出 |
| `--detailed` | | 显示块索引详情 |
| `--show-codecs` | | 显示每块的编码器信息 |

### 示例输出

```
File:              reads.fqc
File size:         12345678 bytes
Total reads:       1000000
Num blocks:        10
Original filename: reads.fastq
Is paired-end:     false
Has reorder map:   true
Preserve order:    false
Streaming mode:    false
Quality mode:      lossless
ID mode:           exact
PE layout:         interleaved
Read length class: short
```

## verify

验证归档完整性。

```
fqc verify [OPTIONS] -i <INPUT>
```

| 选项 | 缩写 | 说明 |
|------|------|------|
| `--input <FILE>` | `-i` | 输入 FQC 文件 |
| `--verbose` | `-v` | 详细输出（逐块进度） |
| `--fail-fast` | | 首次错误即停止 |
| `--quick` | | 快速模式：仅检查头部和尾部 |

### 示例

```bash
# 验证完整性
fqc verify -i reads.fqc

# 详细输出
fqc verify -i reads.fqc --verbose

# 快速检查
fqc verify -i reads.fqc --quick
```

## 退出码

所有命令返回标准化退出码：

| 码 | 名称 | 说明 |
|----|------|------|
| 0 | Success | 操作成功完成 |
| 1 | Usage | 无效参数或缺失文件 |
| 2 | IoError | I/O 错误（文件未找到、权限拒绝、磁盘满） |
| 3 | FormatError | 无效魔数、头部错误、数据损坏 |
| 4 | ChecksumError | 校验和不匹配或完整性违规 |
| 5 | Unsupported | 不支持的编码器或版本 |
