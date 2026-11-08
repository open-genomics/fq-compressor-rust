# Agent Note: gzip/bzip2/xz 输入解码所有串联 member/stream

Status: implemented

## 问题

`open_compressed_reader`（`src/io/compressed_stream.rs`）原先对 gzip、
bzip2、xz 分别使用 `flate2::read::GzDecoder`、`bzip2::read::BzDecoder`、
`xz2::read::XzDecoder::new`。三者都只处理第一个 member/stream：

- 多 member gzip 是 RFC 1952 合法格式，BGZF/bgzip 以及
  `cat a.fastq.gz b.fastq.gz` 都是这种结构；
- bzip2 允许串联多个 stream；
- xz 同样允许串联多个 stream。

复现：用 Python `gzip.compress` 把两段完整 FASTQ（3 条 + 4 条记录）分别压缩
后顺序写入 `multi.fastq.gz`。`fqc compress` exit 0、`fqc decompress` exit 0，
但输出只有 3 条记录，`fqc info` 也显示 `Total reads: 3`，后 4 条被静默丢弃
（Python 读取同一文件得到 7 条）。bzip2 串联同样只得到 3/7 条；xz 串联则
直接报 `Compression failed: I/O error: corrupt xz stream`。

现有 `tests/test_compressed_inputs.rs` 只覆盖单 member 编码器生成的文件，
因此该行为没有被回归测试捕获。

## 决定

改用各库提供的多流解码器，feature gate 与错误映射保持不变：

- gzip：`flate2::read::MultiGzDecoder::new(file)`
- bzip2：`bzip2::read::MultiBzDecoder::new(file)`
- xz：`xz2::read::XzDecoder::new_multi_decoder(file)`

zstd 分支不变：`zstd::Decoder` 的流式 API 本身会继续解码后续 frame。

## 备选方案

- **只在扩展名/魔数检测时重写整个文件为单 member。** 需要额外临时文件或
  全量读入内存，违背 `open_compressed_reader` 流式读取的定位。
- **手工循环构造并拼接解码器。** `MultiGzDecoder` / `MultiBzDecoder` /
  `new_multi_decoder` 已由依赖库维护 member 边界与校验和，自研循环只会增加
  错误路径。
- **继续只支持单 member，文档注明限制。** 与 `cat a.fastq.gz b.fastq.gz`、
  BGZF 等常见输入习惯冲突，且当前行为是静默截断，比报错更危险。

## 后果

- 串联的 gzip/bzip2/xz 输入会完整解压，不再静默丢读段；
- 单 member 输入行为不变；
- 第一个 member 之后若跟随非该格式的尾随垃圾，多流解码器会报错而不是静默
  忽略，属于期望的损坏检测；
- 依赖版本不变，无新增依赖。

## 验证

- 新增 `multi_member_gzip_input_roundtrip`、
  `multi_member_bzip2_input_roundtrip`、`multi_stream_xz_input_roundtrip`
  （`tests/test_compressed_inputs.rs`）：分别用两个独立编码器写入完整
  FASTQ 段并拼接，复用 `compressed_input_roundtrip` 断言 7 条记录逐条一致。
- 修复前 CLI 复现：多 member gzip 只输出 3/7 条；修复后输出 7/7 条。
- `cargo test --lib --tests`、`cargo fmt --all -- --check`、
  `cargo clippy --all-targets -- -D warnings` 全部通过。

## 边界

本修复只影响输入侧的解压读取，不改变 `fqc` 自身归档格式，也不改变压缩输出
所用的单流编码方式。
