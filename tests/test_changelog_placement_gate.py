"""Gate changelog placement and parity against the shapes that hid a real defect.

401a8057 added a Fixed entry to all five mirrors and filed none of them where a reader looks: above
the preamble in `CHANGELOG.md`, inside the `tags:` list of the en and zh frontmatter, and between the
frontmatter and the first heading in ja and ko. `scripts/check_changelog_mirrors.py` stayed green,
because comparing version headers says nothing about *placement*. And it still says nothing about
*completeness* - the entry counts per `[Unreleased]` section differ across mirrors today (against root,
`docs/en` is one entry behind and `docs/zh` five net), which is
`.ci/quality-holes.json`'s `changelog-parity:entry-counts` rather than an assertion, because a red gate
that names someone else's missing translation stops being a gate.

So the rules below are tested the way every other fix here is: each one has to fire on the injection
that names it, the counts probe has to go quiet when the mirrors agree, and the gap written into the
registry has to equal the gap the measurement computes (one entry behind in `docs/en`, five net in
`docs/zh` against root, which is the shape of the divergence, not a snapshot that rots).
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


def _gaps(matrix):
    """How far each mirror sits behind root, per [Unreleased] section position.

    Differences rather than absolute counts on purpose: a commit that adds an entry to all five
    mirrors moves every count and no difference, so the registered statement below stays true while
    the backfill is outstanding, and goes false exactly when it should - when a mirror is added to or
    catches up.
    """
    counts = matrix.changelog_mirror_counts()
    root = counts["CHANGELOG.md"]
    return {
        name: [behind - ahead for behind, ahead in zip(values, root)]
        for name, values in sorted(counts.items())
        if values != root
    }


def test_the_probe_reports_the_measured_divergence(matrix):
    counts = matrix.changelog_mirror_counts()
    assert len({tuple(values) for values in counts.values()}) > 1, (
        f"the mirrors agree today, so this test's premise moved: {counts}"
    )
    assert _gaps(matrix) == {
        "docs/en/changelog.md": [0, 0, -1, 0],
        "docs/zh/changelog.md": [-1, 1, -5, 0],
    }, counts
    holes = {(kind, name) for kind, name, _why in matrix.measure()["holes"]}
    assert ("changelog-parity", "entry-counts") in holes, sorted(holes)


def test_the_counts_probe_goes_quiet_when_the_mirrors_agree(matrix, monkeypatch):
    """Both directions, or the probe is a comment about the mirrors rather than a measurement."""
    counts = matrix.changelog_mirror_counts()
    agreed = {name: [18, 3, 80, 3] for name in counts}
    monkeypatch.setattr(matrix, "changelog_mirror_counts", lambda: agreed)
    holes = {(kind, name) for kind, name, _why in matrix.measure()["holes"]}
    assert ("changelog-parity", "entry-counts") not in holes, sorted(holes)


def test_the_registry_quotes_the_divergence_the_measurement_reports(matrix):
    """The registered hole cites numbers, so a test has to cite them back.

    A wrong figure inside a registered blind spot is worse than a missing one, because it gets quoted
    (#303 measured exactly that). The comparison is against the per-section gaps, which is what the
    registry stores - absolute counts would go stale on the next entry any one mirror receives.
    """
    entry = next(
        e
        for e in json.loads(REGISTRY.read_text(encoding="utf-8"))["holes"]
        if e["id"] == "changelog-parity:entry-counts"
    )
    assert entry["measured"] == _gaps(matrix), entry["measured"]
    assert "docs/en" in entry["why"] and "docs/zh" in entry["why"], entry["why"]
