"""Gate the gate over a design document's "landed" claims: which headings must bite, and which stay quiet.

The finding this exists for is concrete. `docs/dev/event-stream.md` carried the heading "What the note flag
split answered (2026-10-21, decision 6 landed)" against a tree that did not have the split: the documentation
reached trunk through a docs-only commit (`0a074148`, no `.rs`), while the implementation sat in an unmerged
local change and never opened a pull request (`git log origin/main -S owns_line -- '*.rs'` is empty there, and
`pyrs-yaml-core/tests/event_notes.rs` never existed). `scripts/check_hole_claims.py` compares the instruction
files against the measured matrix and `tests/test_quality_matrix.py` compares the registry against the
measurement, so the artefact left outside the gate was the design document's own present tense - and a design
doc is what the next agent reads before touching the feature.

The shapes tested here are that failure and the shapes that must not become one: the exact unlocked heading
caught, a lock whose symbol the tree does not carry caught by name, evidence attached to a heading that does
not need any still allowed, and a scope that found no documents reporting 2 rather than passing.
"""

from __future__ import annotations

import importlib.util
import pathlib
import subprocess
import sys

import pytest

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
SCRIPT = REPO_ROOT / "scripts" / "check_landed_claims.py"

# The heading that rotted, quoted the way the live document carries it.
UNLOCKED = "## What the note flag split answered (2026-10-21, decision 6 landed)\n\nThe tree does it now.\n"

LOCKED = UNLOCKED + "<!-- landed: crates/pyrs-event/src/lib.rs::NoteOwner -->\n"


@pytest.fixture(scope="module")
def gate():
    spec = importlib.util.spec_from_file_location("check_landed_claims", SCRIPT)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def run(*argv):
    return subprocess.run(
        [sys.executable, str(SCRIPT), *argv],
        capture_output=True,
        check=False,
        text=True,
        encoding="utf-8",
        errors="replace",
    )


# ── the tree as it stands ────────────────────────────────────────────────────────


def test_every_landed_claim_in_the_live_documents_is_locked():
    """Green is a claim about the tree, so the checker runs over the real documents here."""
    result = run()
    assert result.returncode == 0, result.stdout + result.stderr


def test_the_live_scope_is_the_design_documents_not_the_dated_records(gate):
    """The dated records may describe a past state, so they must be out - and the doc that rotted must be in."""
    names = {path.name for path in gate.live_docs()}
    assert "event-stream.md" in names
    assert "quality-ledger.md" not in names


def test_the_gate_is_wired_into_the_hook_set():
    """A checker nothing invokes is a report, not a gate; naming it here keeps the reason close."""
    prek = (REPO_ROOT / "prek.toml").read_text(encoding="utf-8")
    assert "scripts/check_landed_claims.py" in prek


# ── the claim that must bite ─────────────────────────────────────────────────────


def test_the_heading_that_rotted_is_caught(gate, tmp_path):
    """`landed` with nothing under it is the exact shape trunk carried for a day."""
    page = tmp_path / "event-stream.md"
    page.write_text(UNLOCKED, encoding="utf-8")
    assert gate.main([str(page)]) == 1


def test_a_lock_whose_symbol_the_tree_does_not_carry_is_caught_by_name(gate, tmp_path):
    """The docs-only commit would have stopped here: the prose said `owns_line`, the code did not have it."""
    page = tmp_path / "event-stream.md"
    page.write_text(
        UNLOCKED + "<!-- landed: crates/pyrs-event/src/lib.rs::owns_line_that_was_never_written -->\n",
        encoding="utf-8",
    )
    found, _claims, _locks = gate.scan(page.read_text(encoding="utf-8"), REPO_ROOT)
    assert any("owns_line_that_was_never_written" in reason for _line, reason in found), found


def test_a_lock_naming_a_missing_file_is_caught(gate, tmp_path):
    """A document may point at a tier that was never merged, and `landed` would still read as proof."""
    page = tmp_path / "event-stream.md"
    page.write_text(
        UNLOCKED + "<!-- landed: crates/pyrs-yaml-core/tests/a_tier_nobody_wrote.rs -->\n",
        encoding="utf-8",
    )
    found, _claims, _locks = gate.scan(page.read_text(encoding="utf-8"), REPO_ROOT)
    assert any("not a file" in reason for _line, reason in found), found


# ── the claims that must stay quiet ─────────────────────────────────────────────


def test_evidence_under_a_heading_that_does_not_claim_is_allowed(gate, tmp_path):
    """Locks are welcome anywhere, so a document can carry proof without the verb that requires it."""
    page = tmp_path / "event-stream.md"
    page.write_text(
        "## What the replay gate answered (2026-10-16, step 4)\n"
        "<!-- landed: crates/pyrs-json/src/parser.rs::from_events -->\n\n`from_events` is public.\n",
        encoding="utf-8",
    )
    assert gate.main([str(page)]) == 0


def test_a_prose_landed_outside_a_heading_is_not_a_claim(gate, tmp_path):
    """The rule reads headings. A sentence in a body paragraph is prose, and charging it would be a tax."""
    page = tmp_path / "event-stream.md"
    page.write_text("Some notes on the design.\n\nThe change landed on 2026-10-21 without a heading.\n")
    assert gate.main([str(page)]) == 0


# ── the failure a silent pass would cause ───────────────────────────────────────


def test_an_unreadable_document_is_not_a_pass(gate, tmp_path):
    missing = tmp_path / "does-not-exist.md"
    assert gate.main([str(missing)]) == 2


def test_a_scope_that_found_no_documents_is_not_a_pass(gate, monkeypatch, tmp_path):
    """An empty `docs/dev` would wave every heading through, so the checker refuses to measure nothing."""
    monkeypatch.setattr(gate, "LIVE_DOCS_DIR", tmp_path)
    assert gate.main([]) == 2
