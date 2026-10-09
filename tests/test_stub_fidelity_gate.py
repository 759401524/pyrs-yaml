"""The shipped type stub has to be readable by a Python parser, and today's was not.

`python/pyrs_yaml/pyrs_yaml.pyi` is machine output: maturin 1.14.1 imports the freshly built extension and
writes each `__doc__` into the stub *verbatim*. Two of this project's doc comments contain a backslash - the
`to_json` note about `json.dumps` escaping and the JSON5 loader's remark about exotic spellings - so both
landed in the artifact unescaped, and a triple-quoted string holding a lone `\\u` is not Python: `ast.parse`
and `compile` both fail with `'unicodeescape' codec can't decode bytes … truncated unicode escape`.

That is a shipped defect, not a build hiccup. The stub is the public typing contract inside every wheel and is
marked by `py.typed`, so mypy and pyright read exactly that file, and mkdocstrings could not reach the API
through it either - which is the reason a documentation-toolchain upgrade had nothing to render. Every gate
that existed stayed green, because the drift checker compared the tracked file against generator output that
carried the same bug: two copies of a broken artifact agreeing is not a check.

These tests hold the two properties the repair turns on. The declared fidelity transform escapes backslashes
inside docstrings only, reports its site count, and fails loud when that count moves, so a third unescaped
site or an upstream fix is a review rather than a silent change. And the committed artifact parses while its
docstrings still *say* the same thing - the escape is Python source syntax, not content.
"""

from __future__ import annotations

import ast
import importlib.util
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
CHECKER = REPO_ROOT / "scripts" / "check_stub_drift.py"
STUB = REPO_ROOT / "python" / "pyrs_yaml" / "pyrs_yaml.pyi"

# Generator output with both shapes present: a docstring that carries a backslash, a docstring that does
# not, and a backslash in code (an annotation) that must be left exactly alone.
GENERATED = '''
class Document:
    """Round-trip document."""

    def to_json(self, /, indent: "int" = 2) -> "str":
        """
        Emit raw UTF-8 rather than `json.dumps`' \\uXXXX escapes.
        """

    def path(self) -> "Node | None":
        """
        Return the address as `a\\.b` segments.
        """
'''


@pytest.fixture(scope="module")
def checker():
    spec = importlib.util.spec_from_file_location("check_stub_drift", CHECKER)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_only_docstring_bodies_are_candidates(checker):
    """The scanner reports docstring content lines, so a backslash in code is never a site to escape."""
    bodies = checker.docstring_body_lines(GENERATED)
    content = [GENERATED.splitlines()[number - 1] for number in sorted(bodies)]
    assert any("json.dumps" in line for line in content), content
    assert any("segments" in line for line in content), content
    assert not any("indent:" in line for line in content), content


def test_escaping_preserves_the_text_and_counts_the_sites(checker):
    """Doubling the backslash is Python source syntax: the string value stays what `help()` prints."""
    escaped, sites = checker.escape_docstring_backslashes(GENERATED)
    assert sites == 2
    tree = ast.parse(escaped)
    values = [
        node.value.value
        for node in ast.walk(tree)
        if isinstance(node, ast.Expr) and isinstance(node.value, ast.Constant) and isinstance(node.value.value, str)
    ]
    joined = "\n".join(values)
    assert "`json.dumps`' \\uXXXX escapes" in joined, values
    assert "`a\\.b` segments" in joined, values


def test_the_count_is_a_tripwire_not_a_preference(checker, monkeypatch):
    """An unexpected number of sites fails the gate instead of quietly rewriting more or less than declared.

    The declared `__next__` fixes are switched off for the moment: this test is about the escape count, and
    a synthetic snippet carrying two iterator signatures would otherwise report a problem for the wrong rule.
    """
    monkeypatch.setattr(checker, "FIDELITY_FIXES", ())
    _, problems = checker.derived_text(GENERATED)
    assert not problems, problems
    three_sites = (
        GENERATED + '\nclass Extra:\n    def m(self) -> None:\n        """\n        A `\\u` note.\n        """\n'
    )
    _, problems = checker.derived_text(three_sites)
    assert any("expected 2" in problem for problem in problems), problems


def test_an_unparseable_stub_is_named_by_line(checker):
    """The message has to point at the line, because the file it names is one nobody may hand-edit."""
    problem = checker.verify_parses(GENERATED, "the derived stub")
    assert problem is not None
    assert "is not valid Python" in problem
    assert "declare it in" in problem, problem
    assert checker.verify_parses(checker.escape_docstring_backslashes(GENERATED)[0], "the derived stub") is None


def test_the_committed_stub_is_valid_python():
    """The artifact in the tree, not a synthetic one: this is the assertion that was missing.

    Reading the docstring back out of the parsed tree also proves the escape did not change what the
    documentation says - a type checker and a reader of `help()` now agree on the same sentence.
    """
    source = STUB.read_text(encoding="utf-8").replace("\r\n", "\n")
    tree = ast.parse(source)
    sentences = [
        line
        for node in ast.walk(tree)
        if isinstance(node, ast.Expr) and isinstance(node.value, ast.Constant) and isinstance(node.value.value, str)
        for line in node.value.value.splitlines()
    ]
    assert any("`json.dumps`' `\\uXXXX` escapes" in line for line in sentences), len(sentences)
    assert any("`\\u` escapes" in line for line in sentences), len(sentences)
