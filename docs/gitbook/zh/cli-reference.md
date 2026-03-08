# 命令行参考

## 全局选项

| 选项 | 说明 |
|------|------|
| `--version` | 打印版本 |
| `--help` | 打印帮助 |

## compress

将 FASTQ 文件压缩为 FQC 格式。

```
fqc compress [OPTIONS] -i <INPUT> -o <OUTPUT>
```

| 选项 | 缩写 | 默认值 | 说明 |
|------|------|--------|------|
| `--input` | `-i` | 必需 | 输入 FASTQ 文件（或 `-` 表示 stdin） |
| `--output` | `-o` | 必需 | 输出 FQC 文件 |
| `--input2` | `-2` | | 第二个输入文件（PE 独立文件） |
| `--level` | `-l` | `3` | Zstd 压缩级别 (1-19) |
| `--threads` | `-t` | 全部核心 | 线程数 |
| `--block-size` | | 自动 | 每块读段数 |
| `--pipeline` | | false | 启用 3 阶段流水线模式 |
| `--streaming` | | false | 流式模式（不进行全局排序） |
| `--interleaved` | | false | 输入为交错配对端 |
| `--lossy-quality` | | lossless | 质量模式：`lossless`、`illumina8`、`discard` |
| `--id-mode` | | `strip` | ID 模式：`exact`、`strip`、`discard` |
| `--long-read-mode` | | 自动 | 强制指定：`short`、`medium`、`long` |
| `--memory-limit` | | 自动 | 内存限制（MB） |

## decompress

将 FQC 文件解压为 FASTQ 格式。

```
fqc decompress [OPTIONS] -i <INPUT> -o <OUTPUT>
```

| 选项 | 缩写 | 默认值 | 说明 |
|------|------|--------|------|
| `--input` | `-i` | 必需 | 输入 FQC 文件 |
| `--output` | `-o` | 必需 | 输出 FASTQ 文件（或 `-` 表示 stdout） |
| `--threads` | `-t` | 全部核心 | 线程数 |
| `--pipeline` | | false | 启用 3 阶段流水线模式 |
| `--range` | | | 提取读段范围（1-based，如 `1:1000`） |
| `--header-only` | | false | 仅输出头部 |

## info

显示归档信息。

```
fqc info [OPTIONS] -i <INPUT>
```

| 选项 | 缩写 | 说明 |
|------|------|------|
| `--input` | `-i` | 输入 FQC 文件 |
| `--json` | | 以 JSON 格式输出 |
| `--detailed` | | 显示块索引详情 |

## verify

验证归档完整性。

```
fqc verify [OPTIONS] -i <INPUT>
```

| 选项 | 缩写 | 说明 |
|------|------|------|
| `--input` | `-i` | 输入 FQC 文件 |
| `--verbose` | `-v` | 详细输出 |

## 退出码

| 码 | 含义 |
|----|------|
| 0 | 成功 |
| 1 | 通用错误 |
| 2 | I/O 错误 |
| 3 | 格式错误 |
| 4 | 校验和不匹配 |
| 5 | 参数错误 |
