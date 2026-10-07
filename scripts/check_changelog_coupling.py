#!/usr/bin/env python3
"""Assert that a changeset which moves the product also moves the release notes, in all five mirrors.

Two rules, one reason:

1. **Coupling.** If a changeset touches anything the release notes are about - engine sources, the
   Python package, the fuzz harness, the CI gate set, the checkers, the manifests - then it must
   touch `CHANGELOG.md`. Otherwise the change ships unannounced.
2. **Completeness.** If a changeset touches *any* of the five changelogs it must touch *all* five.
   `AGENTS.md`: "update root first, then translate into each locale. Never commit partial updates."

Neither rule was enforceable before this file. Measured consequence, one PR old: #296 landed
`fuzz/fuzz_targets/*.rs`, two `roundtrip_corpus.rs` gates, a `fuzz.yml` matrix row and
`.ci/quality-holes.json` - a change to the defence tier, the same class as #293's
"hygiene hooks run in CI" entry, which *did* get an entry - with no changelog edit in any of the
five files, and `ROADMAP.md` recorded the decision as deliberate ("a fuzz target and two corpus
tests ship no behaviour"). The reversal of that judgement is the point of rule 1: the tier is part
of what the release notes describe, and either way the gate, not a reader's memory, decides.

`check_changelog_mirrors.py` could not catch it because it compares *version headers*, which are
unchanged until a release, and it only runs when a changelog is already in the diff - a changeset
that omits the changelog never triggers it. Both halves of that blind spot are why this checker
takes the path list as its input instead of the files' contents.

Usage:
    python scripts/check_changelog_coupling.py --base <ref>        # CI: the pull request's diff
    python scripts/check_changelog_coupling.py path1 path2 ...      # prek: the staged set
    python scripts/check_changelog_coupling.py --paths a b c       # explicit, for tests

CI evaluates at **pull-request** scope, not commit scope, because the convention lives on the pull
request: a PR may be split into commits and one of those commits legitimately carries no changelog
(measured: `0713267a` adds the stub-drift gate; its release note arrived in a sibling commit).

Exit code 0 = coupled and complete; 1 = a finding.
"""

from __future__ import annotations

import subprocess
import sys

MIRRORS = (
    "CHANGELOG.md",
    "docs/en/changelog.md",
    "docs/ja/changelog.md",
    "docs/ko/changelog.md",
    "docs/zh/changelog.md",
)

# Prefixes whose change is what a release note describes: the engines, the package, the fuzz
# harness, the checkers and the Python test tier. `docs/` and the root `*.md` are excluded because
# translating a page is not a product change.
TRIGGER_PREFIXES = (
    "crates/",
    "python/pyrs_yaml/",
    "fuzz/",
    "scripts/",
    "tests/",
)

# Manifests that carry a shipped version. Deliberately **not** `.github/workflows/**` or
# `prek.toml`: replaying this rule over the last 40 commits of `main` reddened two
# dependency-version bumps among them - dependabot's `docker/setup-qemu-action` in `publish.yml`,
# and the ruff/rumdl hook bump - neither release-note-worthy, and a gate that blocks dependabot
# gets bypassed. Dropping them costs no real coverage: a pull request that adds a CI job also adds
# a checker or a test under `scripts/` / `tests/`, which are triggers. Measured, at commit scope,
# 8 of those 40 commits redden, and every one is a change of the class this repository's own notes
# had already described (#293's "note survival is a gate now", "weekly fuzz schedule in CI",
# "type stub drift gate").
TRIGGER_EXACT = (
    "Cargo.toml",
    "Cargo.lock",
    "pyproject.toml",
    "uv.lock",
)

# A changeset that only *removes* notes-bearing files still announces it, so there is no deletion
# exemption. The ledger files (`QUALITY_MATRIX.md`, `ROADMAP.md`, `.ci/quality-holes.json`) are
# deliberately absent from the trigger list: they are this gate's own bookkeeping, and a gate that
# demands a release note every time the ledger of the ledger changes gets bypassed.


def normalise(path: str) -> str:
    return path.replace("\\", "/").removeprefix("./")


def is_trigger(path: str) -> bool:
    path = normalise(path)
    return path.startswith(TRIGGER_PREFIXES) or path in TRIGGER_EXACT


def findings(paths: list[str]) -> list[str]:
    """Every way this path set violates the two rules, as one-line messages."""
    seen = {normalise(p) for p in paths}
    triggers = sorted(p for p in seen if is_trigger(p))
    present = [m for m in MIRRORS if m in seen]

    if triggers and not present:
        return [
            "changeset moves the product but no release note: "
            f"{len(triggers)} trigger path(s) (first: {triggers[0]}), "
            f"and none of {len(MIRRORS)} changelogs"
        ]
    if present and len(present) < len(MIRRORS):
        missing = [m for m in MIRRORS if m not in seen]
        return [f"partial changelog update: missing {', '.join(missing)}"]
    return []


def staged_or_base(argv: list[str]) -> tuple[list[str], str]:
    if "--base" in argv:
        base = argv[argv.index("--base") + 1]
        proc = subprocess.run(
            ["git", "diff", "--name-only", f"{base}...HEAD"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=True,
        )
        return [ln.strip() for ln in proc.stdout.splitlines() if ln.strip()], f"--base {base}"
    if "--paths" in argv:
        head = argv.index("--paths")
        return argv[head + 1 :], "--paths"
    return [a for a in argv if not a.startswith("--")], "staged files"


def main(argv: list[str]) -> int:
    paths, how = staged_or_base(argv)
    if not paths:
        print(f"OK: {how} listed no files, nothing to couple")
        return 0
    found = findings(paths)
    if found:
        print(f"changelog coupling violated ({how}, {len(paths)} path(s)):")
        for line in found:
            print(f"  - {line}")
        print(
            "\nAdd the entry to CHANGELOG.md and to docs/{en,ja,ko,zh}/changelog.md in the same\n"
            "changeset. scripts/check_changelog_mirrors.py compares version headers, and only runs\n"
            "when a changelog is already in the diff, so it cannot see this."
        )
        return 1
    print(f"OK: changelog coupling holds ({how}, {len(paths)} path(s))")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
