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

### PR Description Convention

- **No manual line breaks**: Each paragraph and each list item in a PR body must be written as a single unwrapped line; line wrapping for display is GitHub's responsibility, not the author's.
- **Never reuse the commit message as the PR body**: Commit messages follow the 72-column git wrapping convention while PR bodies must remain unwrapped - the two formats are incompatible by design, so a `git commit -F` message file must not be piped into PR creation.

### Jujutsu (jj) Workflow

This repository is a colocated jj/Git repository (`jj git init --colocate`): `.git` remains the object store, so other developers keep using plain Git and `gh`/`prek`/`git status` keep working in the same directory.

**The gate moves, deliberately.** jj 0.45 does not execute `.git/hooks/pre-commit` and has no commit-hook mechanism of its own (verified: `jj commit` returns instantly where `prek run --all-files` takes about a minute, and the only `hooks.*` keys present were inert ones). `prek run --all-files` is therefore a required step **before** `jj git push`, not an optional one - the same checks re-run in CI, so a forgotten local pass surfaces there rather than silently.

| Task | Command |
| --- | --- |
| See your own work + trunk | `jj log -r 'author(MuLong) \| trunk()'` |
| Full history, all branches/tags | `jj log --all` |
| Start a change off trunk | `jj new main@origin` |
| See what is in the current change | `jj diff --stat` |
| Name a change | `jj describe -m "fix(yaml): ..."` (message rules unchanged) |
| Retarget a branch pointer for pushing | `jj bookmark set fix/x -r @` |
| Push (after prek) | `prek run --all-files; jj git push --allow-new` |
| Fold a change into its parent | `jj squash` |
| Rebase onto moved trunk | `jj rebase -d main@origin` |
| Undo the last jj operation | `jj undo` |
| Sync remote refs without pushing | `jj git fetch` |

Colocated jj keeps git `HEAD` detached (it points at the working-copy commit), so read-only Git is fine - `git log`, `git diff`, `git status`, `gh` - while `git commit`/`git checkout`/`git switch` in this directory fight jj: use `jj edit`, `jj new` and `jj bookmark set` instead. Plain Git is unaffected in everyone else's clone.

Gotchas that cost real time here: `jj` re-snapshots the working copy before every command, so keep a Git-only index workflow out of a jj directory (colocate stages untracked paths into the `.git` index as a side effect); `jj` ignores nothing Git ignores plus nothing else, so path patterns must match the actual type - `.venv` is a directory **junction** here and `.venv/` did not match it, which is why `/.venv` is in `.gitignore`; and a `git revert`-style fix to an older change needs `jj edit <change>` first, since `@` snapshots the working tree, not the change you meant. A pushed bookmark is not frozen either: a later `jj edit` or an automatic re-snapshot rewrites the same change, so immediately before `jj git push` re-check the pointer (`jj bookmark list --all-remotes`, then `git diff --stat <remote-sha> <local-sha>`) - in this repository a branch that had just been pushed silently grew four files belonging to the other branch within minutes, and only that comparison caught it.

## Dependency Updates

Dependabot opens the pull requests (`.github/dependabot.yml`); nothing about a merge is decided in
the GitHub UI, so the policy is in two reviewable files: `scripts/dependency_merge_gate.py` holds the
rules and `.github/workflows/deps-automerge.yml` is only the trigger that runs them once CI has
reported.

**Scope.** Three ecosystems, because those are the three places this repository pins versions:
`cargo` (the workspace and the `fuzz/` workspace, each with its own lockfile), `uv`
(`pyproject.toml` + `uv.lock`), and `github-actions`. Every entry targets `main`, runs weekly, and is
capped by `open-pull-requests-limit` so a bad Friday rebuild cannot fill the review queue. Security
advisories are not suppressed by any `ignore` entry: the ignores are named pins and one floor.

**The line a bump may not cross.** A lockfile change picks a different version that the declared
range already admits; a manifest change rewrites what is supported. Only the first merges itself:
`Cargo.lock`, `uv.lock`, `fuzz/Cargo.lock`, `prek.toml` and the action pins in
`.github/workflows/**`. `Cargo.toml`, `pyproject.toml`, `rust-toolchain*` and the committed
`pyrs_yaml.pyi` always wait for a human, because moving one carries obligations no bot can discharge -
regenerating the stub and updating `scripts/check_stub_drift.py`'s reconciliation rules for maturin
(`AGENTS.md` locks that version for exactly that reason), re-measuring a parser major's migration,
or deciding that a supported interpreter floor may rise. pyo3 0.30+ requires Python 3.9 while the
shipped wheels are `abi3-py38`, so its updates arrive under a `python-floor` label that the merge gate
refuses: the floor stays a decision, not a side effect of a green check.

**The gate is the same one that blocks a human.** A merge happens when `Test matrix (all legs)` -
the single required check, and the fan-in over the test grid, the docs gates, the fuzz replay and the
Ir floor - reports `success` for that head commit. `skipped` and `cancelled` are read as refusals, and
a check that never reported is read as "nothing reported". This is the one bounded exception to
*"passing CI does not automatically authorize the merge"* above: it applies to the bot's resolution
bumps, and a human author never gets the exemption. Nothing here uses `--admin`; with
`enforce_admins` on, an admin merge is the only way an unverified dependency can reach `main`.

**Conflicts.** Nothing merges a pull request whose head does not apply onto its base: an unmergeable
head is read as a refusal, so a bump that collides with a moved `main` waits for its conflict to be
resolved rather than being merged through it. Resolving it by hand is a human act with a rule
attached - re-resolve the lockfile (`cargo update -p <name>`, `uv lock --upgrade-package <name>`)
instead of editing it, because a hand-merged `Cargo.lock` under a moved manifest is exactly what the
Ir baseline cannot be read against. Refusal labels (`needs-human`, `python-floor`, `do-not-merge`,
`blocked`) stop a merge at any point, and adding one needs no deploy.

**Keys.** Every key in `.github/dependabot.yml` is one the hosted schema documents, and no
`versioning-strategy` is set: a rejected value does not fail loudly, it stops updates for that
ecosystem, which is the same silent failure this repository treats as a refusal everywhere else. The
boundary that key would express - move the resolution, not the declared range - is enforced by the
merge gate's path allowlist instead, where `tests/test_dependency_merge_gate.py` can see it.

**Release notes.** `scripts/check_changelog_coupling.py --allow-lockfile-bumps` is the exemption a
lockfile-only bump needs to go green, and CI grants it on `github.actor == 'dependabot[bot]'` only, so
the commit hook stays strict and a human bumping the same file still owes the note. The file set behind
that exemption and the auto-merge allowlist are asserted to agree
(`tests/test_changelog_coupling_gate.py`, `tests/test_dependency_merge_gate.py`), because if they
drift a bump is either unable to go green or lands unannounced.
