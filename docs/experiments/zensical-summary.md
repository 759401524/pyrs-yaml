# Zensical Experiment Summary

**Date:** 2026-07-28
**Branch:** `experiment/zensical`
**Zensical Version:** 0.0.51 (Alpha)

## Result: ✅ Works with zero config changes

Zensical built the entire pyyaml-rs documentation (4 locales, 72 pages) with zero errors or warnings. No `mkdocs.yml` changes were required for a basic build.

## What Worked

| Feature | Status |
|---------|--------|
| Build all 4 locales (en/zh/ja/ko) | ✅ |
| Theme (Material design, palette, fonts) | ✅ |
| `mkdocs.yml` compatibility | ✅ — reads it directly |
| All Markdown extensions (pymdownx, admonition, etc.) | ✅ |
| `extra.alternate` language selector | ✅ — 4 `<link rel="alternate">` + header dropdown |
| Navigation (sections, tabs, path, tracking) | ✅ |
| Search | ✅ |
| Code blocks with highlighting | ✅ |
| 404 page | ✅ |
| Strict mode | ✅ — `--strict` passes |
| Build speed | **~1s** (vs ~5-10s for mkdocs) |

## What Didn't Work / Limitations

| Limitation | Impact | Notes |
|------------|--------|-------|
| **`lang` attribute mismatch** | zh/ja/ko pages show `lang="en"` | Zensical uses global `theme.language`; locale-specific `lang` not supported |
| **`mkdocs-static-i18n` plugin ignored** | None for build, but locale-specific features lost | Zensical uses `alternate` links, not folder-based i18n |
| **`mkdocstrings` plugin ignored** | API reference pages are just raw Markdown | Plugin not supported yet (Phase 3 roadmap) |
| **Alpha status** | Risk of breaking changes | Version 0.0.51 |
| **Python >= 3.10 required** | CI must handle separately | Already supported in CI matrix |

## Key Observations

### i18n Approach Comparison

| Aspect | mkdocs-static-i18n | Zensical (alternate) |
|--------|-------------------|---------------------|
| Config | Plugin in `plugins:` | `extra.alternate` list |
| Theme lang | Auto-set per locale | Global `theme.language` |
| Nav structure | `{locale}/path.md` | `locale/path/index.html` |
| SEO hreflang | Plugin-generated | Built-in |
| Language selector | Plugin-generated | Built-in |

### mkdocstrings Status

The `mkdocstrings` plugin is silently ignored by zensical. API reference pages render as plain Markdown. Per the roadmap, mkdocstrings support is planned for Phase 3 (feature parity). For now, API docs would need manual maintenance or a different approach.

### Build Performance

```text
mkdocs build --strict: ~5-10s
zensical build --strict: ~1s
```

Zensical is ~5-10x faster for the same site.

## Recommendation

Zensical is promising but **too early for production use** in this project:

1. **mkdocstrings** is critical for API docs — without it, we'd lose auto-generated API reference
2. **zh/ja/ko `lang` attribute** issue is a minor but real SEO/accessibility concern
3. **Alpha status** means potential breaking changes

**Suggested timeline:** Re-evaluate when Zensical reaches Phase 3 (feature parity with mkdocs-material plugins), likely mid-to-late 2026.

## Experiment Commands

```bash
# Install
uv add --dev zensical

# Build
uv run zensical build --strict

# Preview
uv run zensical serve
```

## Files Changed

- `mkdocs.yml` — added `extra.alternate` section (benign addition, mkdocs ignores it)
- `pyproject.toml` — added `zensical` to dev dependencies
- `uv.lock` — updated
