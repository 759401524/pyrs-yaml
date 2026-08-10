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

## Version Control & Commits

These conventions govern committing, pushing, and merging. Agents and developers must adhere to them to maintain repository integrity, traceability, and engineering excellence.

### Staging and Committing

- **Explicit Staging**: Files must be staged explicitly using `git add <file>`. The use of `git add -A` or `git add .` for indiscriminate bulk staging is **strictly prohibited** to prevent the accidental inclusion of unrelated modifications or sensitive data.
- **Standardized Commits**: Commit operations automatically trigger local pre-commit hooks. Commit messages must strictly conform to conventions (e.g., Conventional Commits) for semantic clarity and structural consistency.

### Quality Gates

- **Hook Enforcement**: Pre-commit hooks executed during the commit phase encompass code formatting and static analysis tools (e.g., `fmt`, `clippy`, `ruff`).
- **Failure Resolution**: In the event of hook failures, the underlying issues must be rectified prior to re-committing. Using `git commit --no-verify` to bypass quality checks is **strictly forbidden**.

### Pushing and Merging

- **Secure Pushing**: Code may be pushed to the remote repository **only** after all local pre-commit hooks have passed successfully.
- **CI Prerequisite**: Achieving a passing (green) status across all Continuous Integration (CI) pipeline checks is a **mandatory prerequisite** for merging a Pull Request (PR). This is a necessary condition, not a sufficient one: while a merge is strictly prohibited until all CI checks are green, passing CI does not automatically authorize the merge (e.g., peer review or architectural approval may still be required).

### Commit Message Convention

Commit messages must adhere to the following standardized structure for semantic clarity and machine parsability:

```text
<type>(<scope>): <subject>
// blank line
<body>
// blank line
<footer>
```

- **type** (Mandatory): The category of the commit (e.g., `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`).
- **scope** (Optional): The specific module, component, or file affected by the commit.
- **subject** (Mandatory): A concise description of the core changes, not exceeding 50 characters.
- **body** (Optional): Detailed context regarding the motivation for the change and a comparison with previous behavior.
- **footer** (Optional): Used for referencing issues (e.g., `Closes #123`) or denoting breaking changes (`BREAKING CHANGE`).
