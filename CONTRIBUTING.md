# CONTRIBUTING.md — pyrs-yaml

## Multi-language Documentation Sync

All 4 language documentation directories must be updated in lockstep:
`docs/en/`, `docs/zh/`, `docs/ja/`, `docs/ko/`

- **信 (Faithful)**: Technical terms, numbers, and code examples must be identical across all language versions — no omissions or errors.
- **达 (Fluency)**: Each language version must read naturally and follow that language's conventions.
- **雅 (Elegance)**: Strive for professional, concise phrasing in all languages.
- **Never** commit partial updates — all languages must be modified and verified before committing.

## Changelog Mirrors

The changelog has a special structure: `docs/{en,ja,ko,zh}/changelog.md` mirrors the root `CHANGELOG.md`, but the `[Unreleased]` section is translated into each locale while historical entries remain English. The script `scripts/check_changelog_mirrors.py` enforces **structural parity** (same version headers, [Unreleased] section present) rather than verbatim text equality — this allows translation divergence while catching missing mirrors.

When adding a new `[Unreleased]` entry:

1. Write it first in `CHANGELOG.md` (English, canonical)
2. Translate the same entry into `docs/{zh,ja,ko}/changelog.md` (keeping the version header `## [Unreleased]` and any nested headers like `### Changed` translated)
3. Run `uv run python scripts/check_changelog_mirrors.py` to verify structural sync before committing
