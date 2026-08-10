---
title: 站点级 i18n
description: pyrs-yaml 文档站点级国际化的配置说明，包括目录结构、Frontmatter 要求和链接规则。
tags:
  - docs
status: new
---

## 站点级 i18n (MkDocs)

pyrs-yaml 文档站点使用 MkDocs Material 主题内置的 i18n 支持**站点级国际化**。用户可以使用英语（`en`）、中文（`zh-CN`）、日语（`ja-JP`）和韩语（`ko-KR`）查看文档。

运行时错误消息的 i18n 指南请参见 [guides/i18n.md](../guides/i18n.md) 中关于 `set_language()` / `get_language()` 的内容。

### How It Works

每种语言都有独立的 URL 路径（`/zh-CN/`、`/ja-JP/`、`/ko-KR/`），共享一个导航栏，右上角提供语言切换器，在 `mkdocs.yml` 中配置：

```yaml
i18n:
  default_lang: en
  alternate_languages:
    - lang: zh-CN
      url: /zh-CN/
    - lang: ja-JP
      url: /ja-JP/
    - lang: ko-KR
      url: /ko-KR/
```

### Directory Structure

每个语言版本位于 `docs/<lang>/` 目录下，镜像 `docs/en/` 的树结构：

```text
docs/en/  (规范英文版)
docs/zh-CN/  (或 docs/zh/)
docs/ja/  (或 docs/ja-JP)
docs/ko/  (或 docs/ko-KR)
```

### Frontmatter

每个翻译文件**必须**携带 YAML frontmatter，包含 `lang` 字段：

```yaml
---
title: 文档标题
lang: zh-CN
---
```

### Link Rules

- **不要**在内部链接中包含语言前缀 — 使用相对路径（`quick-start.md`）。
- 代码示例在各语言间保持不变。
- 许可证法律文本保持英文；仅翻译标题/说明。

### Verification

```bash
uv sync
mkdocs build --clean-site
mkdocs serve   # http://localhost:8000/
```

### Troubleshooting

| Issue | 解决方案 |
|-------|----------|
| 语言切换器未显示 | 确保 `i18n` 块已配置，且每个 `alternate_languages.lang` 有对应的目录 |
| 链接损坏 | 确认内部链接使用相对路径（无语言前缀） |
| Frontmatter 未解析 | 确保每个文件以 `---` 开头，后跟 Markdown 内容 |
| 搜索无法按语言区分 | 使用 `mkdocs build --clean-site` 重新构建 |
