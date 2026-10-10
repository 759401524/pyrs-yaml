"""Gate the gate over published signatures: what must bite, and what must keep quiet.

The defect measured: `docs/*/api/reference.md` types `read_markdown(path, schema, max_depth)` and the shipped
object accepted `(content, schema, max_depth)` - the *documentation was right and the library was wrong*, which is
the direction that matters, because a checker that only trusts the docs would have approved it and a checker that
only trusts the stub would have blamed the wrong artefact. Three contradictions of a different kind were found in
the same sweep: `register_schema(name, schema: str | dict)` and `register_type(tag, type_handler, priority)` in
three locales - names neither layer has, and `priority` a call refuses with `TypeError`.

So the tests here are: the shipped pages pass; the exact shapes that were wrong are caught; a legitimate
abbreviation of an optional tail is not; and a source of truth that cannot be read exits 2 rather than passing.
"""

from __future__ import annotations

import importlib.util
import pathlib
import subprocess
import sys

import pytest

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
SCRIPT = REPO_ROOT / "scripts" / "check_doc_signatures.py"


@pytest.fixture(scope="module")
def gate():
    spec = importlib.util.spec_from_file_location("check_doc_signatures", SCRIPT)
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


# ── the tree as it stands ───────────────────────────────────────────────────────


def test_every_published_signature_matches_the_shipped_object():
    result = run()
    assert result.returncode == 0, result.stdout + result.stderr


def test_the_checker_knows_the_names_the_publication_uses(gate):
    """The inventory is the running package, so `Node` and `MergedView` are in it and a typo is not."""
    table = gate.inventory()
    for name in ("read_markdown", "register_schema", "YamlDocument.set", "Node.find", "MergedView.get"):
        assert name in table, name
    assert "read_markdwon" not in table, "a misspelling resolved, so a typo could never be caught"


# ── the shapes that must bite ───────────────────────────────────────────────────


@pytest.mark.parametrize(
    ("block", "needle"),
    [
        # the mislabel this change fixed: the parameter really is a path
        ("read_markdown(content, schema, max_depth)", "read_markdown"),
        # names the localized pages invented; neither layer has them
        ("register_schema(name, schema)", "register_schema"),
        ("register_type(tag, type_handler, priority)", "register_type"),
        # a required parameter dropped is not an abbreviation, it is a different function
        ("validate_against_schema(data)", "validate_against_schema"),
    ],
)
def test_a_signature_the_library_does_not_offer_is_a_finding(tmp_path, block, needle):
    page = tmp_path / "reference.md"
    page.write_text(f"### `x()`\n\n```python\n{block} -> None\n```\n", encoding="utf-8")
    result = run(str(page))
    assert result.returncode == 1, result.stdout + result.stderr
    assert needle in result.stderr


def test_reordering_parameters_is_a_finding_even_when_all_names_exist(tmp_path):
    """`f(b, a)` with a real `f(a, b)` is a call no caller can make by keyword."""
    page = tmp_path / "reference.md"
    page.write_text("```python\nread_markdown(schema, path) -> None\n```", encoding="utf-8")
    assert run(str(page)).returncode == 1


# ── the shapes that must stay quiet ────────────────────────────────────────────


@pytest.mark.parametrize(
    "block",
    [
        # a required prefix with the optional tail dropped is how a reference page reads
        "parse(yaml: str | bytes) -> YamlDocument",
        "safe_load(yaml: str) -> dict",
        "read_markdown(path: str) -> tuple[dict | None, str]",
        # an example call with values is documentation of usage, not a claimed signature
        'register_type("!timestamp", TimestampType())',
        # a mention inside prose, not a fenced signature
        "see pyrs_yaml.parse for the details",
    ],
)
def test_a_faithful_or_uncomparable_block_is_not_a_finding(tmp_path, block):
    page = tmp_path / "reference.md"
    page.write_text(f"```python\n{block}\n```", encoding="utf-8")
    assert run(str(page)).returncode == 0, page.read_text(encoding="utf-8")


# ── the failure a silent pass would cause ──────────────────────────────────────


def test_an_unreadable_source_of_truth_is_not_a_pass(gate, monkeypatch, tmp_path):
    """Importing the package is the measurement; if it fails, "no findings" means nothing."""

    def boom():
        raise RuntimeError("the extension is not importable")

    monkeypatch.setattr(gate, "inventory", boom)
    page = tmp_path / "reference.md"
    page.write_text("```python\nparse(yaml)\n```", encoding="utf-8")
    assert gate.main([str(page)]) == 2


def test_the_gate_is_invoked_by_a_job_that_can_see_the_extension():
    """A checker nothing runs is an `orphan-gate` hole the matrix reports.

    It is a CI step rather than a commit hook on purpose: the hook set is stdlib-only so a commit never depends on
    a built extension, and this rule needs one. The pytest tier above is what covers a change that touches no docs.
    """
    workflow = (REPO_ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
    assert "scripts/check_doc_signatures.py" in workflow
    prek = (REPO_ROOT / "prek.toml").read_text(encoding="utf-8")
    assert "check_doc_signatures" not in prek, "a hook would fail wherever the extension is not built"
