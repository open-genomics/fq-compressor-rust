# CLI 参考

`fqc` 提供四个顶层命令：

- `compress`
- `decompress`
- `info`
- `verify`

全局选项：

| 选项 | 含义 |
| --- | --- |
| `-t, --threads` | 线程数（`0` 表示自动） |
| `-v, --verbose` | 增加日志详细度 |
| `-q, --quiet` | 抑制非错误输出 |
| `--memory-limit` | 内存预算 MB（`0` = 自动有限预算，非无限；对 compress/decompress/verify 均生效） |
| `--no-progress` | 禁用进度摘要 |

## `compress`

```bash
fqc compress -i INPUT -o OUTPUT [OPTIONS]
```

| 选项 | 含义 |
| --- | --- |
| `-2, --input2` | 双端输入的第二个 FASTQ 文件 |
| `-l, --level` | 压缩级别 `1..9`（默认：`5`） |
| `--reorder <true|false>` | 启用或禁用全局读段重排 |
| `--streaming` | 禁用重排，增量处理输入 |
| `--lossy-quality` | `none`、`illumina8`、`qvz`（无损别名，真 QVZ 未实现）或 `discard` |
| `--long-read-mode` | `auto`、`short`、`medium` 或 `long` |
| `--interleaved` | 将输入视为交错双端 FASTQ |
| `--max-block-bases` | 限制中长读每块碱基数 |
| `--scan-all-lengths` | 检查完整输入而非采样检测长度 |
| `--pipeline` | 使用分段压缩流水线 |
| `--pe-layout` | 双端归档元数据 `interleaved` 或 `consecutive` |
| `-f, --force` | 覆盖已存在的输出（仅在成功完成后原子替换；失败时保留旧文件） |

说明：

- archive 模式将完整读段集保留在内存中进行全局分析和可选重排
- `--memory-limit 0` 根据可用系统内存自动选择有限预算（约 75%，带硬性结构上限），不是无限内存
- 明确的低内存运行建议使用 `--streaming`
- pipeline 模式是分段执行路径，非严格低内存摄入模式
- 普通文件输出先写同目录临时文件，成功后再 rename；stdout（`-`）不走事务

## `decompress`

```bash
fqc decompress -i INPUT -o OUTPUT [OPTIONS]
```

| 选项 | 含义 |
| --- | --- |
| `--range` | 提取读段范围如 `1:1000` 或 `100:` |
| `--header-only` | 仅写入读段头 |
| `--original-order` | 如存在重排元数据则还原原始顺序 |
| `--skip-corrupted` | 块完整性检查失败时继续 |
| `--corrupted-placeholder` | 跳过块的占位序列 |
| `--split-pe` | 双端输出写入独立文件（两路均经临时文件；按 R1→R2 提交，POSIX 无法双路径原子 rename） |
| `--pipeline` | 使用分段解压流水线 |
| `-f, --force` | 覆盖已存在的输出（仅在成功完成后原子替换；失败时保留旧文件） |

## `info`

```bash
fqc info -i INPUT [--json] [--detailed] [--show-codecs]
```

- `--json` 输出机器可读的归档元数据
- `--detailed` 显示块索引条目
- `--show-codecs` 报告每块编解码器字节

完整解压与 `--original-order` 受同一 `--memory-limit` 约束；超预算会在创建输出前失败。

## `verify`

```bash
fqc verify -i INPUT [--quick] [--fail-fast] [--verbose]
```

完整 verify 与解压共享 decode 预算；`--quick` 跳过块解压但仍受 archive 结构预算约束。

- `--quick` 仅检查归档框架和全局校验和，不解压块
- `--fail-fast` 在首个失败块停止
- `--verbose` 打印每块验证进度
