# GitHub Release v0.1.1 Draft

## 发布命令

### 1. 推送变更到 GitHub

```bash
git push origin master
```

### 2. 创建 GitHub Release（双语描述）

```bash
gh release create v0.1.1 \
  --title "v0.1.1 - Documentation & CI Improvements" \
  --notes-file RELEASE_NOTES_CONTENT.md \
  --target master
```

或者手动在 GitHub 上创建 Release，复制以下内容：

---

## 英文版本 (English)

### 🎉 What's New in v0.1.1

This is a maintenance release focusing on:
- 📚 Documentation improvements
- 🔧 CI/CD workflow enhancements  
- 🔒 Security hardening

### 📚 Documentation

- **SECURITY.md** — Security policy with vulnerability reporting
- **GitBook Glossary** — Terminology definitions for domain concepts
- **Enhanced navigation** — Anchors, search-pro plugins, sidebar links
- **PDF support** — GitBook PDF output configuration

### 🔧 CI/CD

- **Trivy scanning** — Docker image vulnerability scanning
- **PR preview** — Documentation previews for pull requests
- **SHA512 checksums** — Additional checksum for release artifacts
- **CI summary** — Consolidated status reporting

### 🐛 Fixes

- Corrected default compression level from 3 to 6 in performance docs
- Fixed Docker workflow permissions for security scanning
- Fixed Pages workflow configuration steps

### 📦 Assets

| Platform | Architecture | Type |
|----------|-------------|------|
| Linux | x64, ARM64 | glibc, musl (static) |
| macOS | x64, ARM64 | Intel, Apple Silicon |
| Windows | x64 | MSVC |

---

## 中文版本 (Chinese)

### 🎉 v0.1.1 新增内容

这是一个维护性发布，专注于：
- 📚 文档改进
- 🔧 CI/CD 工作流增强
- 🔒 安全加固

### 📚 文档

- **SECURITY.md** — 安全策略与漏洞报告指南
- **GitBook 词汇表** — 领域术语定义
- **增强导航** — Anchors、search-pro 插件、侧边栏链接
- **PDF 支持** — GitBook PDF 输出配置

### 🔧 CI/CD

- **Trivy 扫描** — Docker 镜像漏洞扫描
- **PR 预览** — 拉取请求的文档预览
- **SHA512 校验和** — 发布产物附加校验和
- **CI 汇总** — 统一状态报告

### 🐛 修复

- 性能文档中默认压缩级别从 3 更正为 6
- Docker 工作流安全扫描权限
- Pages 工作流配置步骤

### 📦 资产

| 平台 | 架构 | 类型 |
|----------|-------------|------|
| Linux | x64, ARM64 | glibc, musl (静态) |
| macOS | x64, ARM64 | Intel, Apple Silicon |
| Windows | x64 | MSVC |

---

## Full Documentation

- **English**: [docs/en/](docs/en/)
- **中文**: [docs/zh/](docs/zh/)

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for complete history.
