"""Assert root CHANGELOG.md mirrors are structurally in sync across locales.

Instead of comparing text verbatim (which breaks translation), this script
checks structural parity: every locale must declare the same set of version
headers (## [X.Y.Z]) and must contain a [Unreleased] section. This catches
common mistakes — adding an entry to root but forgetting a mirror — without
requiring translated content to match the English text byte-for-byte.

It also checks *placement*, which the version-header comparison cannot see: an entry
bullet has to sit under a section heading (`### Fixed` and friends) inside a version
block. 401a8057 put one hash-fidelity entry above the preamble in root, inside the
`tags:` list of the en and zh frontmatter, and between the frontmatter and the first
heading in ja and ko - invisible in the changelog body in all five files, and green
under every gate that existed, because "the same version headers exist" says nothing
about where a reader looks. `placement_errors` is the rule that would have caught it.

Exit code 0 = structurally in sync and every entry in place; 1 = drift detected.

Importing this module on Python 3.8 used to raise `TypeError: 'type' object is not
subscriptable` - `def _versions(text: str) -> set[str]` evaluates its annotation at import time
without the future import below. The package supports 3.8, so the annotations must stay lazy;
`tests/test_scripts_import_on_supported_python.py` now imports every checker under the running
interpreter. The 3.8 leg of the pytest matrix found this one by importing the checker from
`tests/test_changelog_coupling_gate.py`; that leg is also why a general guard exists now, since
nothing else ran a checker on the supported floor - hooks and the other jobs use 3.12/3.14.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FILES = [
    ROOT / "CHANGELOG.md",
    ROOT / "docs" / "en" / "changelog.md",
    ROOT / "docs" / "ja" / "changelog.md",
    ROOT / "docs" / "ko" / "changelog.md",
    ROOT / "docs" / "zh" / "changelog.md",
]

_VERSION_RE = re.compile(r"^#{2,3} \[(v?\d+\.\d+\.\d+)\](?:[ —][^\]]+)?\s*$", re.M)
_UNRELEASED_RE = re.compile(r"^#{2,3} \[Unreleased\]", re.M)

# A release-note entry, a version heading (any locale's `[Unreleased]` or `[X.Y.Z]`), and any
# heading at all. Entry bullets are `**bold lead-in**` by this changelog's convention, which is what
# distinguishes them from the `>` quotes and `- ` lists that appear in prose.
_ENTRY_RE = re.compile(r"^- \*\*")
_HEADING_RE = re.compile(r"^#{1,6} ")
_VERSION_HEADING_RE = re.compile(r"^#{2,4} \[(?:Unreleased|v?\d+\.\d+\.\d+)\]")


def _versions(text: str) -> set[str]:
    return set(_VERSION_RE.findall(text))


def _has_unreleased(text: str) -> bool:
    return bool(_UNRELEASED_RE.search(text))


def placement_errors(text: str, name: str) -> list[str]:
    """Every entry bullet in this file that no reader of the changelog would ever find.

    Two shapes, both measured on the tree before being asserted: a bullet before the first version
    heading (the 401a8057 damage - above the preamble, or inside YAML front matter), and a bullet
    whose nearest heading is a version heading rather than a section heading, i.e. an entry filed
    under no category. All five mirrors are clean on the second rule today, so enforcing it costs
    nothing now and catches the next paste that lands in the wrong nesting.
    """
    lines = text.splitlines()
    errors = []
    versions = [i for i, line in enumerate(lines) if _VERSION_HEADING_RE.match(line)]
    if not versions:
        return [f"{name}: no version heading at all, so no entry can be filed under one"]
    heading = None
    for index, line in enumerate(lines):
        if _HEADING_RE.match(line):
            heading = line
        elif not _ENTRY_RE.match(line):
            continue
        elif index < versions[0]:
            errors.append(
                f"{name} line {index + 1}: entry bullet {line[:40]!r} sits before the first version "
                f"heading ({lines[versions[0]]!r}) - invisible in the changelog body"
            )
        elif heading is None or _VERSION_HEADING_RE.match(heading):
            errors.append(
                f"{name} line {index + 1}: entry bullet {line[:40]!r} is under no section heading "
                f"(nearest is {heading!r}) - it is not filed as Added/Changed/Fixed/Performance"
            )
    return errors


def unreleased_counts(text: str) -> dict[str, int]:
    """Entry bullets per section heading inside [Unreleased], keyed by the heading as written.

    Section names are translated (`#### 修正`, `#### 수정`, `#### 修复`), so the keys differ per
    locale and the values are what is comparable: the count at a given position in the section order.
    `scripts/quality_matrix.py` compares those positions across mirrors and registers a blind spot
    when they diverge; this script prints them so the divergence is visible wherever the hook runs.
    """
    inside = False
    section = None
    out = {}
    for line in text.splitlines():
        if _VERSION_HEADING_RE.match(line):
            inside = bool(re.match(r"^#{2,4} \[Unreleased\]", line))
            section = None
            continue
        if not inside:
            continue
        if re.match(r"^#{3,4} ", line):
            section = line.lstrip("#").strip()
        elif _ENTRY_RE.match(line) and section:
            out[section] = out.get(section, 0) + 1
    return out


def positioned_counts(text: str) -> list[int]:
    """The [Unreleased] bullets per section, as a position-keyed list (locale-independent).

    Public because `scripts/quality_matrix.py` calls it rather than re-implementing "what counts as
    an entry": two parsers of the same rule is how a parity check ends up disagreeing with the hook
    that enforces it.
    """
    counts = unreleased_counts(text)
    return [counts[key] for key in sorted(counts, key=lambda k: list(counts).index(k))]


def count_drift(counts: dict) -> list[str]:
    """Findings for mirrors whose [Unreleased] per-section counts differ from the canonical page.

    Split out of `main` so a test can fire it on injected counts: a rule that can only be exercised by
    editing five real changelogs is a rule nobody proves.
    """
    items = list(counts.items())
    if len(items) < 2:
        return []
    canonical = items[0][1]
    return [
        f"{name}: [Unreleased] sections {got} do not match the canonical {canonical}"
        for name, got in items[1:]
        if got != canonical
    ]


def main() -> int:
    errors: list[str] = []
    root_text = FILES[0].read_text(encoding="utf-8")
    root_versions = _versions(root_text)
    texts = {path: path.read_text(encoding="utf-8") for path in FILES}

    for path, text in texts.items():
        errors.extend(placement_errors(text, path.name))

    for path in FILES[1:]:
        text = texts[path]
        versions = _versions(text)
        unreleased = _has_unreleased(text)

        missing = root_versions - versions
        if missing:
            errors.append(f"{path.name}: missing versions {sorted(missing)}")
        extra = versions - root_versions
        if extra:
            errors.append(f"{path.name}: has extra versions {sorted(extra)}")
        if not unreleased:
            errors.append(f"{path.name}: missing [Unreleased] section")

    # Also verify root has all versions the mirrors do (catches root missing
    # entries after mirrors are updated first, which sometimes happens).
    mirror_versions = set()
    for path in FILES[1:]:
        mirror_versions |= _versions(texts[path])
    root_missing = mirror_versions - root_versions
    if root_missing:
        errors.append(f"root CHANGELOG.md: missing versions {sorted(root_missing)}")

    if errors:
        print("changelog structural drift detected:")
        print("\n".join(errors))
        return 1
    # The [Unreleased] entry counts per section are the measurable form of "AGENTS.md forbids partial
    # changelog updates": section order is locale-independent (every mirror keeps Added/Changed/Fixed/
    # Performance), so an entry added to root and not translated shows up as a count difference at the
    # same position. This used to be reported rather than asserted, which is what let 401a8057's entry
    # sit invisible in four of five files; the divergence is now a failure.
    counts = {path.name: positioned_counts(texts[path]) for path in FILES}
    drift = count_drift(counts)
    if drift:
        print("changelog entry-count drift detected:")
        print("\n".join(drift))
        return 1
    print("OK: all 5 changelogs structurally in sync, every entry filed under a section, counts equal")
    for path in FILES:
        print(f"  {path.relative_to(ROOT).as_posix():24} [Unreleased] sections {counts[path.name]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
