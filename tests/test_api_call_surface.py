"""Every public callable the shipped stub declares is called by something in `tests/`.

`python/pyrs_yaml/pyrs_yaml.pyi` is the contract users type against, and the documentation gate
(`tests/test_api_doc_signatures.py`) already asks whether each of those names is *named* in every
locale's API pages. This is the other question, which no gate asked: is it ever *run*? A callable can
be exported, documented, typed, and never once executed - and then a change to it is unmeasured by
definition. Measured 2026-10-31 the gap was 2 of 74: `get_plugin` and `YamlDocument.flush_source`,
both answered by `tests/test_builtin_plugins.py` and `TestFlushSource` here rather than deleted.

Credit is a call site, not a mention. The name has to be the callee of a call node -
`parse(...)`, `doc.to_yaml()`, `YamlDocument.validate(...)` - somewhere under `tests/`. An attribute
read that never calls, a docstring that quotes, or a string that contains the name is not evidence
that anything ran it, which is the difference this gate keeps and a name-grep would not.
"""

from __future__ import annotations

import ast
import pathlib

import pytest

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
STUB_PATH = REPO_ROOT / "python" / "pyrs_yaml" / "pyrs_yaml.pyi"
TESTS_DIR = REPO_ROOT / "tests"


def public_callables(stub_text: str) -> set[str]:
    """Module functions and public methods of public classes, as `Class.method` and bare names."""
    found: set[str] = set()
    for node in ast.parse(stub_text).body:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            if not node.name.startswith("_"):
                found.add(node.name)
        elif isinstance(node, ast.ClassDef) and not node.name.startswith("_"):
            for sub in node.body:
                if isinstance(sub, (ast.FunctionDef, ast.AsyncFunctionDef)) and not sub.name.startswith("_"):
                    found.add(f"{node.name}.{sub.name}")
    return found


def call_sites(root: pathlib.Path) -> set[str]:
    """Every name used as the callee of a call somewhere under `root`, and nothing else."""
    names: set[str] = set()
    for path in sorted(root.rglob("*.py")):
        if "__pycache__" in path.parts:
            continue
        try:
            tree = ast.parse(path.read_text(encoding="utf-8", errors="replace"))
        except SyntaxError:
            continue
        for node in ast.walk(tree):
            if not isinstance(node, ast.Call):
                continue
            if isinstance(node.func, ast.Name):
                names.add(node.func.id)
            elif isinstance(node.func, ast.Attribute):
                names.add(node.func.attr)
    return names


def unreached(stub_text: str, root: pathlib.Path) -> list[str]:
    called = call_sites(root)
    return sorted(name for name in public_callables(stub_text) if name.split(".")[-1] not in called)


def test_every_public_callable_is_called_from_the_suite() -> None:
    missing = unreached(STUB_PATH.read_text(encoding="utf-8"), TESTS_DIR)
    assert not missing, (
        "public callable(s) the shipped stub declares, called by nothing in tests/: "
        f"{missing} - run them or remove them from the contract"
    )


def test_both_halves_of_the_check_are_live() -> None:
    """A gate over an empty left side or a broken right side passes forever, so both are pinned."""
    surface = public_callables(STUB_PATH.read_text(encoding="utf-8"))
    assert len(surface) >= 60, f"the stub read found {len(surface)} public callables; the parser broke"
    short = {name.split(".")[-1] for name in surface}
    assert {"parse", "safe_load", "list_plugins"} <= short, sorted(surface)
    assert "YamlDocument.flush_source" in surface, "the method half of the read stopped working"
    called = call_sites(TESTS_DIR)
    assert {"to_yaml", "parse"} <= called, sorted(called)[:20]
    # A private name is not the contract's surface, and the module's own leading-underscore helpers
    # must not be what fills the count out.
    assert not [name for name in surface if name.split(".")[-1].startswith("_")], sorted(surface)


def test_a_callable_nobody_calls_is_named(tmp_path: pathlib.Path) -> None:
    """The forcing row: the gate must be able to fail, in both directions."""
    stub = "def parse(y: str) -> str: ...\ndef never_called() -> None: ...\n"
    (tmp_path / "test_a.py").write_text("import pyrs_yaml\n\n\ndef test_x():\n    pyrs_yaml.parse('a: 1')\n")
    assert unreached(stub, tmp_path) == ["never_called"]

    (tmp_path / "test_a.py").write_text(
        "import pyrs_yaml\n\n\ndef test_x():\n    pyrs_yaml.parse('a: 1')\n    pyrs_yaml.never_called()\n"
    )
    assert unreached(stub, tmp_path) == []


def test_a_mention_that_never_calls_does_not_count(tmp_path: pathlib.Path) -> None:
    """The distinction this gate exists for: reading an attribute is not running it.

    A file that mentions the name in a docstring, compares it as a string, or binds it without
    calling it must leave the callable unreached - otherwise the gate would be satisfied by prose,
    which is the shape of every false green this repository has had to fix.
    """
    stub = "def only_bound() -> None: ...\n"
    (tmp_path / "test_b.py").write_text(
        "import pyrs_yaml\n\n\n"
        "def test_b():\n"
        '    """only_bound is documented here"""\n'
        '    assert "only_bound" in dir(pyrs_yaml)\n'
        "    handler = pyrs_yaml.only_bound\n"
        "    assert handler is not None\n"
    )
    assert unreached(stub, tmp_path) == ["only_bound"]

    (tmp_path / "test_c.py").write_text("import pyrs_yaml\n\n\ndef test_c():\n    pyrs_yaml.only_bound()\n")
    assert unreached(stub, tmp_path) == []


def test_the_stub_and_the_gate_read_the_same_file_the_wheel_ships() -> None:
    """The gate is worth nothing if it reads a different artifact than the published contract."""
    assert STUB_PATH.is_file(), STUB_PATH
    text = STUB_PATH.read_text(encoding="utf-8")
    assert "def flush_source" in text and "def get_plugin" in text, (
        "the two names this gate was written for are no longer in the stub; re-check what it covers"
    )


@pytest.mark.parametrize("name", ["get_plugin", "YamlDocument.flush_source"])
def test_the_two_names_this_gate_found_now_answer(name: str) -> None:
    """The finding, pinned as a row: these were the 2 unreached callables of 74 on 2026-10-31."""
    short = name.split(".")[-1]
    assert short in call_sites(TESTS_DIR), f"{name} is back to being called by nothing"
