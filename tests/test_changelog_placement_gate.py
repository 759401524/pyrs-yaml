"""Gate changelog placement and parity against the shapes that hid a real defect.

401a8057 added a Fixed entry to all five mirrors and filed none of them where a reader looks: above
the preamble in `CHANGELOG.md`, inside the `tags:` list of the en and zh frontmatter, and between the
frontmatter and the first heading in ja and ko. `scripts/check_changelog_mirrors.py` stayed green,
because comparing version headers says nothing about *placement*. It also used to say nothing about
*completeness*: the entry counts per `[Unreleased]` section drifted across mirrors (one entry behind in
`docs/en`, five net in `docs/zh`) and the checker only printed them, which is why the divergence lived
as `.ci/quality-holes.json`'s `changelog-parity:entry-counts` instead of an assertion - a red gate that
names someone else's missing translation stops being a gate. Condensing the release notes into
user-facing entries with the translations written in the same pass closed the gap, and the checker now
asserts the equality instead of reporting it, so the registered hole is gone in both directions: the
measurement finds nothing, and the registry no longer claims it does.

So the rules below are tested the way every other fix here is: each one has to fire on the injection
that names it, and the counts probe has to stay quiet while the five pages agree.
"""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
CHECKER = REPO_ROOT / "scripts" / "check_changelog_mirrors.py"
MATRIX = REPO_ROOT / "scripts" / "quality_matrix.py"
REGISTRY = REPO_ROOT / ".ci" / "quality-holes.json"

ROOT_SHAPE = "\n".join(
    [
        "# Changelog",
        "",
        "- **An entry pasted in the wrong place** — cites `some_token`",
        "  and continues here.",
        "All notable changes to this project will be documented in this file.",
        "",
        "## [Unreleased]",
        "",
        "### Fixed",
        "",
        "- **A properly filed entry** — cites `other_token`",
        "",
        "## [v0.1.0] — 2026-01-01",
        "",
        "### Fixed",
        "",
        "- **An old entry** — cites `ancient_token`",
    ]
)
DOC_SHAPE = "\n".join(
    [
        "---",
        "title: Changelog",
        "tags:",
        "",
        "- **An entry pasted into the frontmatter** — cites `some_token`",
        "  and continues here.",
        "- docs",
        "status: new",
        "---",
        "",
        "## Changelog",
        "",
        "### [Unreleased]",
        "",
        "#### Fixed",
        "",
        "- **A properly filed entry** — cites `other_token`",
    ]
)


def _load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def checker():
    return _load("check_changelog_mirrors", CHECKER)


@pytest.fixture(scope="module")
def matrix():
    return _load("quality_matrix", MATRIX)


# ── the tree ──────────────────────────────────────────────────────────────────────


def test_every_entry_in_the_tree_is_filed_inside_a_section(checker):
    """The rule the five mirrors all broke, asserted over the files it broke in."""
    for path in checker.FILES:
        text = path.read_text(encoding="utf-8")
        errors = checker.placement_errors(text, path.name)
        assert errors == [], "\n".join(errors)


def test_the_checker_still_exits_zero(checker, capsys):
    assert checker.main() == 0
    printed = capsys.readouterr().out
    assert "structurally in sync" in printed
    # The divergence is reported even while it is only registered, so a new regression in
    # completeness is visible in the log instead of being inferred later.
    assert "[Unreleased] sections [" in printed


# ── rule 1: no entry outside a version block ─────────────────────────────────────


def test_an_entry_above_the_preamble_is_a_finding(checker):
    errors = checker.placement_errors(ROOT_SHAPE, "CHANGELOG.md")
    assert len(errors) == 1, errors
    assert "before the first version heading" in errors[0], errors[0]
    assert "An entry pasted in the wrong place" in errors[0], errors[0]


def test_an_entry_inside_the_frontmatter_is_a_finding(checker):
    errors = checker.placement_errors(DOC_SHAPE, "changelog.md")
    assert len(errors) == 1, errors
    assert "before the first version heading" in errors[0], errors[0]


def test_the_same_entry_moved_into_its_section_is_clean(checker):
    """Discriminator: the finding is the position, not the entry.

    Removing the misplaced bullet from `ROOT_SHAPE` must leave no error at all, otherwise the rule
    fires on content and the fix would be to delete entries rather than file them.
    """
    dropped = ROOT_SHAPE.replace("- **An entry pasted in the wrong place** — cites `some_token`\n", "")
    dropped = dropped.replace("  and continues here.\n", "")
    assert checker.placement_errors(dropped, "CHANGELOG.md") == []


def test_a_file_with_no_version_heading_cannot_hold_entries(checker):
    errors = checker.placement_errors("# Changelog\n\n- **An entry** — text\n", "CHANGELOG.md")
    assert errors and "no version heading" in errors[0], errors


# ── rule 2: every entry is under a section heading, not a bare version ──────────


def test_an_entry_under_a_version_heading_with_no_section_is_a_finding(checker):
    text = "\n".join(
        [
            "# Changelog",
            "",
            "## [Unreleased]",
            "",
            "- **An entry filed under no category** — cites `some_token`",
        ]
    )
    errors = checker.placement_errors(text, "CHANGELOG.md")
    assert len(errors) == 1, errors
    assert "under no section heading" in errors[0], errors[0]


# ── counts: locale-independent positions ────────────────────────────────────────


def test_counts_cover_only_the_unreleased_block(checker):
    assert checker.positioned_counts(ROOT_SHAPE) == [1]
    assert checker.positioned_counts(DOC_SHAPE) == [1]


def test_the_five_pages_carry_equal_entry_counts_now(checker, matrix):
    """The condition the registered hole described is closed, so the gate asserts it instead.

    Equality is checked positionally: every mirror keeps Added/Changed/Fixed/Performance in the same
    order, so a count difference at the same position can only mean a translated entry is missing.
    """
    counts = matrix.changelog_mirror_counts()
    assert len({tuple(values) for values in counts.values()}) == 1, counts
    assert checker.count_drift(counts) == [], counts


def test_a_mirror_that_drops_a_translated_entry_fails(checker):
    """Fitness: the enforcement bites on the shape it exists for."""
    counts = {
        "CHANGELOG.md": [11, 3, 14, 3],
        "docs/en/changelog.md": [11, 3, 14, 3],
        "docs/zh/changelog.md": [11, 3, 13, 3],
    }
    found = checker.count_drift(counts)
    assert len(found) == 1 and "docs/zh/changelog.md" in found[0], found
    assert "14" in found[0] and "13" in found[0], found


def test_the_registry_is_empty_because_the_measurement_is_quiet(checker, matrix):
    """An empty registry is a measured state here, not an aspiration.

    `tests/test_quality_matrix.py` fails if the derived set and the registered set differ in either
    direction, so this asserts only what that gate does not already: the page that documented the
    divergence cites nothing now, and the checker is the reason.
    """
    assert json.loads(REGISTRY.read_text(encoding="utf-8"))["holes"] == []
    assert not any(kind == "changelog-parity" for kind, _name, _why in matrix.measure()["holes"])
