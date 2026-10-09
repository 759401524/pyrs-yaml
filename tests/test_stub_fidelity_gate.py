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
import re
import sys
import types
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


def exported_exceptions() -> list[str]:
    """The error names `python/pyrs_yaml/__init__.py` re-exports, read out of its syntax tree.

    Static on purpose: the assertion this feeds has to hold on any machine, including one where the
    extension is not built, because "the contract omits the exceptions" is exactly the state a machine
    without the build would otherwise bless.
    """
    tree = ast.parse((REPO_ROOT / "python" / "pyrs_yaml" / "__init__.py").read_text(encoding="utf-8"))
    names = []
    for node in ast.walk(tree):
        if isinstance(node, ast.ImportFrom) and node.module and node.module.endswith("pyrs_yaml"):
            names.extend(alias.name for alias in node.names if alias.name.endswith(("Error", "Exception")))
            names.extend(alias.name for alias in node.names if alias.name == "YamlTagSkip")
    return sorted(set(names))


def test_the_shipped_stub_declares_every_exported_exception():
    """A wheel whose `py.typed` contract names no error type makes `except pyrs_yaml.YamlParseError` invisible.

    maturin 1.14.1 introspects the built module and emits nothing for PyO3 exception classes, so all ten
    were missing until the route appended them. This is the assertion that keeps them there: the names come
    from the package's own re-export list, and each has to appear as a `class` declaration in the artifact
    that ships.
    """
    source = STUB.read_text(encoding="utf-8")
    names = exported_exceptions()
    assert len(names) >= 10, names
    missing = [name for name in names if not re.search(rf"^class {name}\(", source, re.M)]
    assert not missing, f"python/pyrs_yaml/pyrs_yaml.pyi declares no such class: {missing}"


def test_exception_declarations_are_derived_in_base_first_order(checker, monkeypatch):
    """The declarations come from the built classes, in an order the file can parse, docstrings included.

    A fake module stands in for the extension so the real derivation runs end to end: sorting the names
    alphabetically puts the subclass first, which is exactly the case the ordering exists to fix.
    """
    tag_error = type("YamlTagError", (ValueError,), {"__module__": "pyrs_yaml", "__doc__": "Raised on a bad tag."})
    parse_error = type("ZyabError", (ValueError,), {"__module__": "pyrs_yaml", "__doc__": None})
    skip = type("AaefError", (tag_error,), {"__module__": "pyrs_yaml", "__doc__": None})
    fake = types.ModuleType("pyrs_yaml")
    fake.YamlTagError, fake.ZyabError, fake.AaefError = tag_error, parse_error, skip
    monkeypatch.setitem(sys.modules, "pyrs_yaml", fake)

    text, count = checker.exception_block("def f() -> None: ...\n")
    assert count == 3, text
    assert text.index("class YamlTagError") < text.index("class AaefError"), text
    assert "Raised on a bad tag." in text, text
    ast.parse(text)


def test_an_unexpected_exception_count_is_a_problem_not_a_rewrite(checker, monkeypatch):
    """The declared count is a tripwire: an exception appearing or disappearing must be looked at."""
    monkeypatch.setattr(checker, "exception_block", lambda text: (text, 11))
    monkeypatch.setattr(checker, "EXPECTED_DOCSTRING_ESCAPES", 0)
    _, problems = checker.derived_text("x: int = 0\n")
    assert any("expected 10" in problem for problem in problems), problems


def test_order_by_base_refuses_a_cycle_between_declared_exceptions(checker):
    """An ordering the route cannot satisfy is reported, not emitted as a file that will not parse.

    A base outside the set is not an error - `ValueError` and `TypeError` are the caller's builtins - so the
    refusal is specifically the shape Python source cannot express: two declared classes each needing the
    other first.
    """
    with pytest.raises(checker.StubInputError):
        checker.order_by_base([("A", "B", ""), ("B", "A", "")])
    ordered = checker.order_by_base([("Sub", "Base", ""), ("Base", "ValueError", "")])
    assert [name for name, _b, _d in ordered] == ["Base", "Sub"]


def test_an_unimportable_extension_stops_the_derivation_rather_than_thinning_it(checker, monkeypatch, tmp_path):
    """Skipping the exception block quietly would ship a contract smaller than the bindings.

    Exit 2 is the code for "the derivation could not be done", deliberately different from the 1 that reports
    drift: a green run must never be able to mean "the exception declarations were skipped today".
    """
    generated = tmp_path / "gen.pyi"
    generated.write_text("def f() -> None: ...\n", encoding="utf-8")
    monkeypatch.setitem(sys.modules, "pyrs_yaml", None)
    with pytest.raises(checker.StubInputError):
        checker.exception_block("x: int = 0\n")
    monkeypatch.setattr(sys, "argv", ["check_stub_drift", "--generated", str(generated), "--tracked", str(generated)])
    assert checker.main() == 2, "an unusable derivation must not read as in-sync or as drift"


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
