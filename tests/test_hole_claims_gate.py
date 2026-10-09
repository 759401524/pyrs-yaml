"""Gate the gate over the instruction files: which sentences must bite, and which must not.

The finding this exists for is concrete. `AGENTS.md` stated that `mkdocstrings` is "configured, installed and
invoked by the build - and renders nothing", naming `docs-generation:plugin-unused` as an open hole. #321 closed
that gap - the API pages in four locales carry `:::` directives, the registry is empty - and the sentence stayed
for a whole milestone, still telling an agent that API prose is a hand-typed content decision. Nothing was
wrong with the measurement: `scripts/quality_matrix.py` compares the registry with the derived document in both
directions. Nothing reads the file an agent trusts most.

So the tests here are the shapes of that failure and the shapes that must stay quiet: the exact stale claim
caught, a true claim allowed, a colon that is not a hole claim ignored, and a vocabulary the checker cannot
trust reporting 2 rather than passing.
"""

from __future__ import annotations

import importlib.util
import pathlib
import subprocess
import sys

import pytest

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
SCRIPT = REPO_ROOT / "scripts" / "check_hole_claims.py"

# The sentence that rotted, quoted from the file as it stood after the gap it described had closed.
STALE = "- mkdocstrings renders nothing; registered as `docs-generation:plugin-unused` for follow-up\n"


@pytest.fixture(scope="module")
def gate():
    spec = importlib.util.spec_from_file_location("check_hole_claims", SCRIPT)
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


def test_the_instruction_files_name_only_measured_holes():
    """Green is a claim about the tree, so the checker runs over the real files here."""
    result = run()
    assert result.returncode == 0, result.stdout + result.stderr


def test_the_gate_is_wired_into_the_hook_set():
    """A checker nothing invokes is a hole the matrix reports itself; naming it here keeps the reason close."""
    prek = (REPO_ROOT / "prek.toml").read_text(encoding="utf-8")
    assert "scripts/check_hole_claims.py" in prek


# ── the claim that must bite ─────────────────────────────────────────────────────


def test_the_sentence_that_rotted_is_caught(tmp_path):
    page = tmp_path / "AGENTS.md"
    page.write_text(STALE, encoding="utf-8")
    result = run(str(page))
    assert result.returncode == 1, result.stdout + result.stderr
    assert "docs-generation:plugin-unused" in result.stderr


def test_every_hole_kind_the_probe_can_emit_is_in_the_vocabulary(gate, tmp_path):
    """Any emittable kind named without being measured must be caught, not just the one that rotted.

    The sample is derived from the probe's own reporting sites, so a new kind is covered the day it lands -
    which is the reason the vocabulary is read off the source instead of kept in a list beside it.
    """
    kinds = gate.emittable_kinds()
    assert kinds, "the vocabulary came back empty, which would wave every sentence through"
    for kind in sorted(kinds):
        page = tmp_path / f"{kind}.md"
        page.write_text(f"- a hole called `{kind}:some-name` is open\n", encoding="utf-8")
        assert gate.stale_claims(page.read_text(encoding="utf-8"), kinds, set()), kind


# ── the claims that must stay quiet ─────────────────────────────────────────────


def test_an_id_the_matrix_actually_measures_is_allowed(gate, monkeypatch, tmp_path):
    """The rule has to permit the true sentence, or the only way to satisfy it is to say nothing."""
    kind = sorted(gate.emittable_kinds())[0]
    hole = f"{kind}:demo"
    monkeypatch.setattr(gate, "measured_ids", lambda: {hole})
    page = tmp_path / "AGENTS.md"
    page.write_text(f"- the defence still cannot see this: `{hole}`\n", encoding="utf-8")
    assert gate.main([str(page)]) == 0


def test_a_colon_that_is_not_a_hole_claim_is_ignored(gate, tmp_path):
    """`line:column` names two fields, and a URL names a scheme; neither is a hole id, so neither is a claim."""
    page = tmp_path / "AGENTS.md"
    page.write_text(
        "- errors print `line:column: message` (see https://example.com:8080/docs and `key: value`)\n",
        encoding="utf-8",
    )
    assert gate.main([str(page)]) == 0


# ── the failure a silent pass would cause ───────────────────────────────────────


def test_a_vocabulary_that_cannot_be_trusted_reports_loudly(gate, monkeypatch, tmp_path):
    """A reporting site whose kind is not a literal makes the set shorter, and a short set catches nothing.

    So the mismatch is an error rather than a smaller job: the guard refuses to be the way this file went
    quietly green.
    """
    broken = tmp_path / "quality_matrix.py"
    broken.write_text(
        'holes.append(["real-kind", "a", "why"])\nname = computed()\nholes.append([name, kind, why])\n',
        encoding="utf-8",
    )
    monkeypatch.setattr(gate, "PROBE", broken)
    with pytest.raises(RuntimeError, match="cannot be trusted"):
        gate.emittable_kinds()
    page = tmp_path / "AGENTS.md"
    page.write_text("- nothing named here\n", encoding="utf-8")
    assert gate.main([str(page)]) == 2


def test_an_unreadable_instruction_file_is_not_a_pass(gate, tmp_path):
    missing = tmp_path / "does-not-exist.md"
    assert gate.main([str(missing)]) == 2
