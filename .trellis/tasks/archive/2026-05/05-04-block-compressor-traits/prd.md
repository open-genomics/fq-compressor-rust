# 重构 BlockCompressor：引入压缩器 Traits

## 目标

将 `BlockCompressor` 从 god module 重构为协调器，通过 traits 解耦各压缩器实现，提高代码的 locality 和 testability。

## 背景

当前问题：
- `block_compressor.rs` (569 行) 知道所有压缩器的实现细节
- 添加新 codec 需要修改多处：`compress()`、`decompress_raw()`、`get_xxx_codec()`
- Zstd 压缩逻辑内联在 block_compressor 中，不在独立模块
- 配置耦合：`BlockCompressorConfig` 包含 ABC 特定参数

## 已完成

- [x] 创建 `compressor_traits.rs` 定义四个 trait
- [x] 创建 `zstd_sequence.rs` 实现 Zstd 序列压缩
- [x] `AbcCompressor` 实现 `SequenceCompressor` trait
- [x] 更新 `CONTEXT.md` 记录架构

## 待完成

### Phase 1: 实现剩余 Traits

- [x] `QualityCompressor` 实现 `quality_compressor::QualityCompressor` trait
- [x] 创建 `DeltaZstdIdCompressor` 实现 `IdCompressor` trait
- [x] 创建 `DeltaVarintAuxCompressor` 实现 `AuxCompressor` trait

### Phase 2: 重构 BlockCompressor

- [x] 修改 `BlockCompressor` 使用 trait objects
- [x] 移除内联的 Zstd 压缩代码
- [x] 移除 `BlockCompressorConfig` 中的 codec 选择逻辑
- [x] 更新所有调用方
- [x] 动态选择 ABC vs Zstd（基于 read count）

### Phase 3: 测试与验证

- [x] 运行现有测试确保功能不变
- [x] 验证压缩/解压 roundtrip

## 验收标准

- [x] `BlockCompressor.compress()` 简化为 ~20 行（实际 40 行，包含动态选择逻辑）
- [x] 添加新 codec 只需新建文件 + 注册，不修改 `block_compressor.rs`（trait 已就绪）
- [x] 所有测试通过（110 tests）
- [x] `cargo clippy` 无警告

## 技术方案

### Trait 接口

```rust
pub trait SequenceCompressor: Send + Sync {
    fn compress(&self, reads: &[ReadRecord]) -> Result<Vec<u8>>;
    fn decompress(&self, data: &[u8], read_count: u32, uniform_length: u32, lengths: &[u32]) -> Result<Vec<String>>;
    fn codec_id(&self) -> u8;
}
```

### BlockCompressor 新结构

```rust
pub struct BlockCompressor {
    sequence: Box<dyn SequenceCompressor>,
    quality: Box<dyn QualityCompressor>,
    id: Box<dyn IdCompressor>,
    aux: Box<dyn AuxCompressor>,
}
```

### 工厂方法

```rust
impl BlockCompressor {
    pub fn for_short_reads(config: &BlockCompressorConfig) -> Self {
        Self {
            sequence: Box::new(AbcCompressor::new(config.to_abc_config())),
            quality: Box::new(ScmQualityCompressor::new(...)),
            ...
        }
    }

    pub fn for_long_reads(config: &BlockCompressorConfig) -> Self {
        Self {
            sequence: Box::new(ZstdSequenceCompressor::new(config.zstd_level)),
            ...
        }
    }
}
```

## 排除范围

- 不改变压缩格式或算法
- 不修改 CLI 接口
- 不优化性能（仅重构）

## 技术笔记

- 相关文件：`src/algo/block_compressor.rs`, `src/algo/quality_compressor.rs`, `src/algo/id_compressor.rs`
- 现有测试：`src/algo/abc.rs` 中的 10 个测试
- 注意 conda/glibc 冲突，测试时用 `PATH="/usr/bin:/bin:$HOME/.cargo/bin" cargo test`
