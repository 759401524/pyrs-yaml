---
title: Site-wide i18n (MkDocs)
description: How the documentation site supports internationalization via MkDocs Material theme, including directory structure and link rules.
tags:
  - docs
status: new
---

## Site-wide i18n (MkDocs)

The pyrs-yaml documentation site supports **site-wide internationalization** using MkDocs Material theme's built-in i18n. Users can view docs in English (`en`), Chinese (`zh-CN`), Japanese (`ja-JP`), and Korean (`ko-KR`).

See the runtime error-message i18n guide at [guides/i18n.md](../guides/i18n.md) for `set_language()` / `get_language()`.

### How It Works

Each language gets its own URL path (`/zh-CN/`, `/ja-JP/`, `/ko-KR/`) and shares one navigation with a language switcher in the top-right corner, configured in `mkdocs.yml`:

```yaml title="mkdocs.yml i18n config"
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

Each locale lives under `docs/<lang>/` mirroring the English `docs/en/` tree:

```text title="Locale directory structure"
docs/en/  (canonical English)
docs/zh-CN/  (or docs/zh/)
docs/ja/  (or docs/ja-JP)
docs/ko/  (or docs/ko-KR)
```

### Frontmatter

Every translated file **must** carry YAML frontmatter with the `lang` field:

```yaml title="Translated frontmatter"
---
title: 文档标题
lang: zh-CN
---
```

### Link Rules

- **Do NOT** include language prefixes in internal links — use relative paths (`quick-start.md`).
- Code examples stay unchanged across languages.
- License legal text stays English; only headings/explanations are translated.

### Verification

```bash title="Build and serve the docs"
uv sync
mkdocs build --clean-site
mkdocs serve   # http://localhost:8000/
```

### Troubleshooting

| Issue | Solution |
|-------|----------|
| Language switcher not showing | Ensure `i18n` block is configured and every `alternate_languages.lang` has a matching directory |
| Broken links | Verify internal links use relative paths (no lang prefix) |
| Frontmatter not parsed | Every file starts with `---` before any markdown content |
| Search not per-language | Rebuild with `mkdocs build --clean-site` |
