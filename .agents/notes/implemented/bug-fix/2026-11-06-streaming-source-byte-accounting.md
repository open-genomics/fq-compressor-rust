# Agent Note: streaming 输入字节统计接入块源契约

Status: implemented

## 问题

`OperationStats.input_bytes` 的契约是“实际进入操作的字节数”，但 streaming
从 stdin 读取时，`input_bytes_for` 的 fallback 传入的是 `report.total_bases`。
这会把 FASTQ 的 ID、加号行、质量和换行全部丢掉，只剩序列碱基数。例如
16 字节的 `@r1\nACGT\n+\nIIII\n` 会被记成 4 字节。

进一步检查发现，`FastqParser` 的 `ParserStats.total_bytes_read` 只在
`collect_stats = true` 时更新，而生产路径的 `open_fastq` / `open_fastq_stdin`
没有打开该选项，所以即使想用 parser 字节数也始终是 0。打开 `collect_stats`
又会为每条读段额外扫描 N 碱基并维护 min/max，代价与目标无关。

## 决定

- 给共享块源增加显式 `ReadBlock { reads, input_bytes }`：
  - archive/pipeline 的预切块源传 `0`，调用方仍用文件 metadata 统计；
  - streaming 源传本次块实际消费的字节数。
- `BlockPipelineReport` 增加 `input_bytes`，由 reader 阶段累加块源上报值。
- `FastqParser` 无条件维护轻量 `bytes_read` 计数器（只做一次加法），
  不依赖 `collect_stats`；
- `PairedFastqReader` / `InterleavedPeParser` 暴露两个底层 parser 的合计
  `bytes_read`；
- streaming 的 single/paired/interleaved 闭包用前后差值填充
  `ReadBlock.input_bytes`；
- `run_core` 只把这个值作为 `input_bytes_for` 的 fallback：普通文件仍然用
  metadata 的实际大小，stdin 才使用 parser 字节数。

## 备选方案

- **打开 `collect_stats`**：能复用现成字段，但会为每条读段增加 N 扫描和
  min/max 维护，为了一个字节计数引入无关热路径成本，否决。
- **继续用 `total_bases` 估算**：字节数会系统性偏小，压缩比和吞吐都会被
  高估；契约要求实际输入字节，否决。
- **在 stdin 外层包 counting reader**：能精确统计压缩前字节，但要改
  `open_fastq_stdin` 的返回类型并处理压缩流边界；当前 stdin 仅支持明文，
  parser 计数已经精确，等引入压缩 stdin 时再评估。
- **让块源只返回 reads，另开 side-channel 原子计数**：会引入隐式共享状态，
  不如把字节数放进块源返回值显式。

## 后果

- streaming stdin 的 `input_bytes` 现在是实际消费的 FASTQ 字节数；
- 普通文件 streaming 仍使用文件 metadata，不受 parser 计数影响；
- `ParserStats.total_bytes_read` 保持原有“仅 collect_stats 时更新”的含义，
  新 `bytes_read` 才是始终可用的原始字节计数；
- 块源契约多一个字段，archive/pipeline 显式传 0，避免把“文件字节”和
  “源码消费字节”混为一谈。

## 边界

`bytes_read` 统计的是 parser 从底层 reader 消费的字节。当前 stdin 只接受
明文 FASTQ，因此等于原始输入字节；如果未来支持压缩 stdin，需要在原始
压缩流外层计数，不能直接复用解压后的 parser 计数。

## 验证

- 新增 `report_aggregates_source_input_bytes`：两个块上报 11 + 22 字节，
  report 必须为 33，同时 reads/bases 仍正确；
- `test_parser_stats` 改为精确断言 `total_bytes_read == data.len()`；
- 新增 paired / interleaved 的 `bytes_read` 精确计数测试；
- 16 字节 stdin 经伪终端端到端压缩：报告压缩比 `0.06x`（16/268），
  不再是按 4 个碱基计算的 `0.01x`；
- `cargo fmt`、`clippy -D warnings`、`cargo test --lib --tests`、
  `cargo doc --no-deps`、MSRV、`npm run verify-notes` 全绿。
