# 贡献指南

`fqc` 是业余维护、刻意轻量的 FASTQ 压缩工具。欢迎小步、完整、低维护成本的贡献。

## 开始之前

- 通读 [README.md](README.md) 与 [docs/](docs/README.md)
- 领域语言见 [CONTEXT.md](CONTEXT.md)
- 大改动（格式、兼容性、安全/资源）先走 `openspec/` 变更流程，见 [openspec/AGENTS.md](openspec/AGENTS.md)

## 提交 issue

用 [issue 模板](https://github.com/open-genomics/fq-compressor-rust/issues/new/choose)：
bug 报告请附上复现命令、输入数据（或最小切片）与期望行为。

## 提 PR

1. 从 `master` 开分支，改动小而完整
2. 本地跑一遍门禁（CI 在 GitHub Actions 跑同一套）：
   - `cargo fmt --all -- --check`
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo test --lib --tests`
   - `cargo doc --no-deps`
3. 提交信息用 Conventional Commits（`feat:` / `fix:` / `docs:` / `refactor:` / `test:` / `chore:` 等）
4. 若改了 CLI 行为或默认值，同步更新 [README.md](README.md) 与 [docs/guide/cli.md](docs/guide/cli.md)

## 风格约束

- 不新增 `unsafe`（`[lints.rust] unsafe_code = "deny"`）
- 遵守 `rustfmt.toml` 与 `clippy.toml` 的既有配置

版本策略与发布流程见 [VERSIONING.md](VERSIONING.md)。
