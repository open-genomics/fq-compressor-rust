# Library API（嵌入 facade）

`fqc` 首先是 CLI；库的稳定嵌入面是 `fqc::api`。其它模块虽然仍是 `pub`（供集成测试和高级用法），但属于 internal API，名称和布局可能在小版本间变化。

## 四个入口

```rust
use fqc::api::{self, CompressRequest, CompressSettings, CompressInputTopology, ReorderMode};

let request = CompressRequest::new(
    fqc::api::CompressionExecutionMode::Archive,
    CompressInputTopology::SingleFile { input_path: "in.fastq".into() },
    "out.fqc",
).with_settings(CompressSettings {
    reorder: ReorderMode::Auto,
    ..Default::default()
});

let outcome = api::compress(request)?;
println!("{} bytes, {} blocks", outcome.stats.output_bytes, outcome.stats.blocks);
```

```rust
let info = fqc::api::info("out.fqc")?;
println!("{} reads in {} blocks", info.total_reads, info.num_blocks);
// info.file_path 是 PathBuf，不会经过 to_string_lossy。
```

```rust
use fqc::api::{DecompressRequest, DecompressSettings};

let stats = fqc::api::decompress(
    DecompressRequest::new("out.fqc", "restored.fastq").with_settings(DecompressSettings::default()),
)?;
println!("{} reads", stats.total_reads);
```

```rust
use fqc::api::VerifyOptions;

let report = fqc::api::verify(&VerifyOptions {
    input_path: "out.fqc".into(),
    ..Default::default()
})?;
assert!(report.passed);
```

## 契约边界

- `CompressRequest` / `DecompressRequest` 字段私有，构造时通过 `new` + `with_settings`；`Validated*Plan` 是唯一结构校验入口。
- `OperationStats` 是唯一统计真源：`input_bytes` / `output_bytes` 是实际字节流，`total_bases` 不再冒充输入字节。
- 路径参数统一使用 `AsRef<Path>` / `PathBuf`：`api::info`、`DecompressRequest`、`VerifyOptions.input_path` 和 `ArchiveInfo.file_path` 保留原始 `OsStr` 字节，不做 `to_string_lossy`。CLI 显示层才使用 `Path::display()`。
- `.fqc` wire format 与 codec 字节不属于本轮 facade 契约；修改格式必须单独评审。
- 内部模块（`algo`、`archive`、`pipeline`、`engine` 等）用 `#[doc(hidden)]` 标记为不稳定。
