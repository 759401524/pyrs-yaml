---
title: Site-wide i18n
description: How the documentation site supports internationalization, including directory structure and link rules.
tags:
  - docs
status: new
---

## Site-wide i18n

The pyrs-yaml documentation site supports **site-wide internationalization** using the Material theme's built-in i18n. Users can view docs in English (`en`), Chinese (`zh-CN`), Japanese (`ja-JP`), and Korean (`ko-KR`).

See the runtime error-message i18n guide at [guides/i18n.md](../guides/i18n.md) for `set_language()` / `get_language()`.

### How It Works

Each language gets its own URL path (`/zh-CN/`, `/ja-JP/`, `/ko-KR/`) and shares one navigation with a language switcher in the top-right corner, configured in `zensical.toml`:

```toml title="zensical.toml i18n config"
[project.extra]
alternate = [
    {name = "English", link = "/pyrs-yaml/en/", lang = "en"},
    {name = "中文", link = "/pyrs-yaml/zh/", lang = "zh"},
    {name = "日本語", link = "/pyrs-yaml/ja/", lang = "ja"},
    {name = "한국어", link = "/pyrs-yaml/ko/", lang = "ko"},
]
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
# Build all 4 locales
uv run --group docs python scripts/build-docs.py

# Serve a single locale for development
uv run --group docs python -m zensical serve --config-file zensical.toml --dirty
```

### Troubleshooting

| Issue | Solution |
|-------|----------|
| Language switcher not showing | Ensure `i18n` block is configured and every `alternate_languages.lang` has a matching directory |
| Broken links | Verify internal links use relative paths (no lang prefix) |
| Frontmatter not parsed | Every file starts with `---` before any markdown content |
| Search not per-language | Rebuild with `uv run --group docs python scripts/build-docs.py` |
