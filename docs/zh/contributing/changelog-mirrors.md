---
title: 更新日志镜像
description: pyrs-yaml 更新日志镜像的维护指南，确保多语言版本间的结构一致性。
tags:
  - docs
status: new
---

## 更新日志镜像

更新日志有一个特殊结构：`docs/{en,ja,ko,zh}/changelog.md` 镜像根目录的 `CHANGELOG.md`，但 `[Unreleased]` 部分**按语言翻译**，而历史记录保持英文。

### Structural Parity

检查脚本 `scripts/check_changelog_mirrors.py` 检查**结构一致性**（相同的版本标题、存在 `[Unreleased]` 部分）而非逐字文本相等。这允许翻译差异，同时捕获遗漏的镜像。

### Workflow

1. 先在根目录 `CHANGELOG.md`（英文，规范版本）中编写条目
2. 将相同的 `[Unreleased]` 条目翻译到 `docs/{zh,ja,ko}/changelog.md`（保持版本标题如 `## [Unreleased]` 和 `### Added` 的翻译）
3. 验证：

```bash title="检查镜像一致性"
uv run python scripts/check_changelog_mirrors.py
```

### Rules

| Rule | 描述 |
|------|-------------|
| **根目录为规范版本** | `CHANGELOG.md` 是主要的英文源 |
| **Unreleased 被翻译** | 仅 `[Unreleased]` 部分按语言不同 |
| **历史记录保持英文** | 所有过去版本条目（`[v0.x.y]`）在所有镜像中保持英文 |
| **从不部分更新** | 提交前必须同时更新所有 4 种语言 |
| **标题保留** | 版本标题（`## [Unreleased]`、`### Added` 等）必须在每种语言中存在 |
