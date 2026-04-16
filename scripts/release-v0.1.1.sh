#!/bin/bash
# GitHub Release v0.1.1 发布脚本
# Usage: bash scripts/release-v0.1.1.sh

set -e

VERSION="v0.1.1"

echo "=== 发布 fqc $VERSION ==="
echo ""

# 1. 检查当前分支
echo "1. 检查当前分支..."
git branch --show-current
echo ""

# 2. 推送变更到 GitHub
echo "2. 推送变更到 GitHub..."
git push origin master
echo ""

# 3. 检查 tag 是否存在
echo "3. 检查 tag..."
if git rev-parse "$VERSION" >/dev/null 2>&1; then
    echo "Tag $VERSION 已存在，删除旧 tag..."
    git tag -d "$VERSION" 2>/dev/null || true
fi
echo ""

# 4. 创建并推送 tag
echo "4. 创建 tag $VERSION..."
git tag "$VERSION"
git push origin "$VERSION"
echo ""

# 5. 创建 GitHub Release（双语描述）
echo "5. 创建 GitHub Release..."

gh release create "$VERSION" \
    --title "$VERSION - Documentation & CI Improvements / 文档与 CI 改进" \
    --notes "## 🎉 What's New / 新增内容

This release focuses on documentation improvements, CI/CD enhancements, and security hardening.
本次发布专注于文档改进、CI/CD 增强和安全加固。

### 📚 Documentation / 文档
- SECURITY.md with vulnerability reporting policy / 安全策略与漏洞报告
- GitBook glossary with terminology definitions / GitBook 词汇表
- Enhanced navigation with anchors and search-pro plugins / 增强导航
- PDF support in GitBook configuration / PDF 支持

### 🔧 CI/CD
- Trivy container scanning for security / Trivy 容器扫描
- PR preview for documentation changes / PR 预览
- SHA512 checksums alongside SHA256 / SHA512 校验和
- Consolidated CI status reporting / CI 状态汇总

### 🐛 Fixes / 修复
- Corrected default compression level (3 → 6) / 默认压缩级别更正
- Fixed workflow permissions / 工作流权限修复

---

📚 Documentation: https://github.com/LessUp/fq-compressor-rust/tree/master/docs
📝 Changelog: https://github.com/LessUp/fq-compressor-rust/blob/master/CHANGELOG.md
🔗 Full Release Notes: https://github.com/LessUp/fq-compressor-rust/blob/master/changelog/releases/v0.1.1.md" \
    --target master

echo ""
echo "=== Release completed / 发布完成！==="
echo "Release URL: https://github.com/LessUp/fq-compressor-rust/releases/tag/$VERSION"
