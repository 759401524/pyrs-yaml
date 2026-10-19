"""Gate the known-residuals registry against the tests that pin it.

`.ci/quality-holes.json` registers what `scripts/quality_matrix.py` can *measure* as
missing, so it can only hold a hole some probe can derive. A residual that is understood
and deliberately left open - a declined engine boundary, a question about the public
error surface - has no measured kind, and an empty holes registry then reads as "no
blind spots" while the behaviour is still there.

`.ci/known-residuals.json` is that second registry. Each entry names a `repro` node id,
and the test at that id carries `@pytest.mark.known_residual(<id>)` and asserts today's
behaviour. This module is what makes the pair more than documentation:

* an entry whose repro is missing, unmarked, or marked with another id fails - a stale
  entry cannot stay as decoration;
* a marked test whose id has no entry fails - deleting an entry while its repro still
  pins the behaviour cannot pass;
* fixing a residual reddens the characterization test itself, so closing it forces the
  marker and its entry out in the same change;
* `QUALITY_MATRIX.md` has to name every id under its Known residuals heading, and no
  other id there, or the assessment is telling a different story than the registry.

Everything here is repository state plus an AST parse: no build, no network, milliseconds.
"""

from __future__ import annotations

import ast
import datetime
import json
import re
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
REGISTRY_PATH = REPO_ROOT / ".ci" / "known-residuals.json"
DOC_PATH = REPO_ROOT / "QUALITY_MATRIX.md"
MARKER = "known_residual"
ID_PATTERN = re.compile(r"\b(?:engine|api):[a-z0-9-]+\b")


def marker_id(decorator: ast.expr) -> str | None:
    """The id a `@pytest.mark.known_residual(...)` decorator names; None if it is another marker."""
    target = decorator.func if isinstance(decorator, ast.Call) else decorator
    if not (isinstance(target, ast.Attribute) and target.attr == MARKER):
        return None
    if not isinstance(decorator, ast.Call) or len(decorator.args) != 1:
        return ""
    arg = decorator.args[0]
    if isinstance(arg, ast.Constant) and isinstance(arg.value, str):
        return arg.value
    return ""


def scan_marked_tests(root: Path) -> dict[str, str]:
    """Map every marked function's pytest node id to the residual id it names."""
    marked: dict[str, str] = {}
    for path in sorted((root / "tests").rglob("test_*.py")):
        relative = path.relative_to(root).as_posix()
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
        _collect(tree.body, relative, marked)
    return marked


def _collect(body: list[ast.stmt], prefix: str, marked: dict[str, str]) -> None:
    for node in body:
        if isinstance(node, ast.ClassDef):
            _collect(node.body, f"{prefix}::{node.name}", marked)
        elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            for decorator in node.decorator_list:
                residual = marker_id(decorator)
                if residual is not None:
                    marked[f"{prefix}::{node.name}"] = residual


def staleness(entries: list[dict], marked: dict[str, str]) -> tuple[list[tuple[str, str]], list[tuple[str, str]]]:
    """The two directions: entries with no live repro of their own, and repros with no entry."""
    registered = {entry["id"]: entry["repro"] for entry in entries}
    stale = sorted((residual, repro) for residual, repro in registered.items() if marked.get(repro) != residual)
    orphaned = sorted((nodeid, residual) for nodeid, residual in marked.items() if residual not in registered)
    return stale, orphaned


@pytest.fixture(scope="module")
def registry() -> dict:
    return json.loads(REGISTRY_PATH.read_text(encoding="utf-8"))


@pytest.fixture(scope="module")
def marked() -> dict[str, str]:
    return scan_marked_tests(REPO_ROOT)


# ── the registry: every entry is actionable and owned ─────────────────────────────


def test_registry_entries_are_actionable(registry):
    seen_ids: set[str] = set()
    seen_repros: set[str] = set()
    for entry in registry["residuals"]:
        residual = entry["id"]
        assert residual.count(":") == 1, f"{residual}: id must be exactly '<layer>:<name>'"
        assert residual not in seen_ids, f"{residual}: registered twice"
        seen_ids.add(residual)
        for field in ("repro", "exit", "tier", "opened", "why"):
            assert entry.get(field), f"{residual}: {field} is empty"
        assert entry["repro"].startswith("tests/") and "::" in entry["repro"], (
            f"{residual}: {entry['repro']!r} is not a pytest node id"
        )
        assert entry["repro"] not in seen_repros, f"{residual}: the repro is pinned twice"
        seen_repros.add(entry["repro"])
        datetime.date.fromisoformat(entry["opened"])
        exit_criterion = entry["exit"]
        assert len(exit_criterion) > 40, f"{residual}: the exit criterion is not concrete"
        assert not exit_criterion.lower().startswith(("fix ", "improve ", "todo")), (
            f"{residual}: {exit_criterion!r} names an action, not a checkable state"
        )


# ── the gate: registry and marked repros agree, in both directions ────────────────


def test_every_entry_has_a_live_repro(registry, marked):
    stale, _ = staleness(registry["residuals"], marked)
    assert not stale, (
        "registered residual(s) with no live repro - the pin is gone or names another id; "
        "delete the entry only when the behaviour is gone too:\n"
        + "\n".join(f"  - {residual} -> {repro}" for residual, repro in stale)
    )


def test_every_marked_repro_is_registered(registry, marked):
    _, orphaned = staleness(registry["residuals"], marked)
    assert not orphaned, (
        "marked repro(s) with no registry entry - add .ci/known-residuals.json entry or "
        "remove the marker:\n" + "\n".join(f"  + {nodeid} ({residual})" for nodeid, residual in orphaned)
    )


def test_every_marker_names_exactly_one_id(marked):
    malformed = sorted(nodeid for nodeid, residual in marked.items() if not residual)
    assert not malformed, (
        "known_residual takes exactly one string id, e.g. "
        "@pytest.mark.known_residual('api:some-name'):\n" + "\n".join(f"  ? {nodeid}" for nodeid in malformed)
    )


def test_every_marked_repro_is_a_collected_test(marked):
    """A marker on a helper is not a repro: pytest would never run it."""
    uncollected = []
    for nodeid in sorted(marked):
        parts = nodeid.split("::")
        if not parts[-1].startswith("test") or not all(part.startswith("Test") for part in parts[1:-1]):
            uncollected.append(nodeid)
    assert not uncollected, "pytest will not collect these marked functions:\n" + "\n".join(
        f"  ? {nodeid}" for nodeid in uncollected
    )


# ── the document names exactly the registered residuals ───────────────────────────


def test_doc_lists_exactly_every_known_residual(registry):
    doc = DOC_PATH.read_text(encoding="utf-8")
    heading = "### Known residuals"
    start = doc.find(heading)
    assert start != -1, "QUALITY_MATRIX.md has no Known residuals section"
    end = doc.find("\n### ", start + len(heading))
    section = doc[start : end if end != -1 else None]
    named = set(ID_PATTERN.findall(section))
    declared = {entry["id"] for entry in registry["residuals"]}
    assert named == declared, (
        f"the Known residuals section names {sorted(named)}, the registry declares {sorted(declared)}"
    )


# ── instrument accuracy: the scanner and the diff are tested in both directions ───


def test_the_scanner_reads_module_and_class_context(tmp_path):
    tests = tmp_path / "tests"
    tests.mkdir()
    (tests / "test_demo.py").write_text(
        "import pytest\n"
        "\n"
        "@pytest.mark.known_residual('api:top-level')\n"
        "def test_top():\n"
        "    pass\n"
        "\n"
        "class TestThing:\n"
        "    @pytest.mark.known_residual('api:nested')\n"
        "    def test_nested(self):\n"
        "        pass\n"
        "\n"
        "    def test_unmarked(self):\n"
        "        pass\n",
        encoding="utf-8",
    )
    assert scan_marked_tests(tmp_path) == {
        "tests/test_demo.py::test_top": "api:top-level",
        "tests/test_demo.py::TestThing::test_nested": "api:nested",
    }


def test_deleting_an_entry_orphans_its_marker():
    """The acceptance this gate exists for: dropping the entry alone must not pass."""
    marked = {"tests/test_demo.py::test_shape": "api:demo"}
    stale, orphaned = staleness([], marked)
    assert stale == []
    assert orphaned == [("tests/test_demo.py::test_shape", "api:demo")]


def test_an_entry_whose_repro_disappeared_is_stale():
    entries = [{"id": "api:demo", "repro": "tests/test_demo.py::test_shape"}]
    stale, orphaned = staleness(entries, {})
    assert stale == [("api:demo", "tests/test_demo.py::test_shape")]
    assert orphaned == []


def test_a_repro_marked_with_another_id_leaves_both_directions_red():
    entries = [{"id": "api:demo", "repro": "tests/test_demo.py::test_shape"}]
    stale, orphaned = staleness(entries, {"tests/test_demo.py::test_shape": "api:other"})
    assert stale == [("api:demo", "tests/test_demo.py::test_shape")]
    assert orphaned == [("tests/test_demo.py::test_shape", "api:other")]
