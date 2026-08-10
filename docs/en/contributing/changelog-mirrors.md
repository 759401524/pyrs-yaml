# Changelog Mirrors

The changelog has a special structure: `docs/{en,ja,ko,zh}/changelog.md` mirrors the root `CHANGELOG.md`, but the `[Unreleased]` section is **translated per locale** while historical entries remain English.

## Structural Parity

The guard script `scripts/check_changelog_mirrors.py` checks **structural parity** (same version headers, `[Unreleased]` section present) rather than verbatim text equality. This allows translation divergence while catching missing mirrors.

## Workflow

1. Write the entry first in root `CHANGELOG.md` (English, canonical)
2. Translate the same `[Unreleased]` entry into `docs/{zh,ja,ko}/changelog.md` (keep version headers like `## [Unreleased]` and `### Added` translated)
3. Verify:

```bash
uv run python scripts/check_changelog_mirrors.py
```

## Rules

| Rule | Description |
|------|-------------|
| **Root is canonical** | `CHANGELOG.md` is the primary English source |
| **Unreleased is translated** | Only the `[Unreleased]` section differs per locale |
| **Historical is English** | All past version entries (`[v0.x.y]`) remain English in all mirrors |
| **Never partial** | All 4 locales must be updated together before committing |
| **Headers stay** | Version headers (`## [Unreleased]`, `### Added`, etc.) must exist in every locale |
