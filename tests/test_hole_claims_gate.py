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


# ── the [Unreleased] block of a changelog mirror ────────────────────────────────

HOLE = "perf-coverage:binding-layer"

# The sentence that rotted in five mirrors at once, placed where each of the two scopes begins.
NOTE = f"- the binding layer has no instruction floor: `{HOLE}`"
RELEASED = """## [v0.18.3] - 2026-09-30

### Performance

- a note that names no hole id
"""
UNRELEASED_STALE = f"""## [Unreleased]

### Performance

{NOTE}

{RELEASED}"""
RELEASED_STALE = f"""## [Unreleased]

### Performance

- a note that names no hole id

## [v0.18.3] - 2026-09-30

### Performance

{NOTE}
"""


def mirror(tmp_path, text):
    """A changelog by name, so the `[Unreleased]` slice applies to the fixture exactly as to the five."""
    page = tmp_path / "changelog.md"
    page.write_text(text, encoding="utf-8")
    return page


def test_an_unreleased_note_that_names_a_closed_hole_is_caught(tmp_path):
    """One `[Unreleased]` block is present tense, so it carries the same obligation as `AGENTS.md`.

    The block is the text the next release cut turns into a dated record: a stale id inside it is the
    defect this gate was written for, and eight places said it across two pull requests while the
    instruction files were the only thing anyone read.
    """
    page = mirror(tmp_path, UNRELEASED_STALE)
    result = run(str(page))
    assert result.returncode == 1, result.stdout + result.stderr
    assert f"changelog.md:5 names {HOLE!r}" in result.stderr
    assert "a note that names no hole id" not in result.stderr


def test_a_released_section_that_names_a_closed_hole_stays_exempt(gate, tmp_path):
    """History may name a hole that has since closed, so the slice has to stop at the next version.

    The fixture keeps the file, the id and the sentence and moves only the section, so a pass here is
    evidence about the scope rather than about the id.
    """
    page = mirror(tmp_path, RELEASED_STALE)
    scoped = gate.scope_lines(page)
    assert any("no hole id" in line for _number, line in scoped), scoped
    assert all(number < 7 for number, _line in scoped), "the [Unreleased] slice reached the released section"
    assert run(str(page)).returncode == 0


def test_an_unreleased_note_that_names_a_measured_hole_is_allowed(gate, monkeypatch, tmp_path):
    """The rule must let a true release note through, or the only way to satisfy it is to say nothing."""
    monkeypatch.setattr(gate, "measured_ids", lambda: {HOLE})
    assert gate.main([str(mirror(tmp_path, UNRELEASED_STALE))]) == 0


def test_a_changelog_with_no_unreleased_body_is_not_a_pass(gate, tmp_path):
    """A heading spelled `## Unreleased` (no brackets) slices to nothing, and nothing names no stale id.

    An empty slice is therefore a refusal: the gate is silent about the file rather than green for it,
    because a scope that quietly stops covering anything is how this defect came back twice.
    """
    page = mirror(tmp_path, "## Unreleased\n\n" + NOTE + "\n")
    assert gate.main([str(page)]) == 2


def test_the_default_scope_covers_every_changelog_mirror(gate):
    """The parity checker owns the mirror list, so a hole gate reading some other list is a second parser."""
    mirrors = gate.load_module(gate.MIRRORS, "check_changelog_mirrors").FILES
    scope = {path.resolve() for path in gate.default_paths()}
    missing = [str(path) for path in mirrors if path.resolve() not in scope]
    assert not missing, f"changelog mirrors outside the hole-claim scope: {missing}"
