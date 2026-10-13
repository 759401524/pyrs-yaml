"""Every `scripts/*.py` must import on the interpreter that runs this test.

The matrix in `ci.yml` installs Python 3.8 through 3.14 and runs `tests/` on each, but until now no
test *imported* a checker, so the checkers' own compatibility was only ever exercised on the
interpreters that happen to execute them - the hook environment and two CI jobs on 3.12/3.14. That
let a real defect sit in `scripts/check_changelog_mirrors.py` since the file was written: a
`-> set[str]` annotation without `from __future__ import annotations` raises
`TypeError: 'type' object is not subscriptable` at import time on 3.8, and the module still looked
fine to every hook and job that ran a newer Python.

What caught it was the 3.8 leg of the pytest matrix, and only because
`tests/test_changelog_coupling_gate.py` happens to import one checker with `importlib`. This file
generalises that accident to the whole directory, so the class cannot recur silently for a script
that no gate's test happens to load.

Runtime calls into a checker used to be this module's blind spot, named here as unfinished work: an
API that exists only on a newer Python (`str.removeprefix`, 3.9) fails only when the function runs,
which is why the coupling gate's own tests had to keep executing its functions - they are what failed
on 3.8 for the second bug in this pair. It cost a full matrix run again in October 2026, when three
tests written against 3.13 reached for `Path.write_text(..., newline=)`, a keyword 3.10 added, and
`TypeError`ed on the two oldest legs before testing anything. `TOO_NEW_FOR_THE_FLOOR` below closes the
class statically: the scan needs no interpreter older than the one running it.
"""

from __future__ import annotations

import importlib.util
import re
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
SCRIPTS = sorted((REPO_ROOT / "scripts").glob("*.py"))
FLOOR = (3, 8)

# Call shapes that raise at runtime - not import time - on an interpreter older than the version
# listed. Syntax is out of scope: a `match` statement or a `X | Y` annotation already fails to
# compile or import on the floor, so the legs that run this file catch that for free. Each entry
# names one API with a documented first release; every one of them is reachable in this
# repository's own history or in the standard library changelog.
#
# Two entries were rejected as written, and the reason is the lesson of this list: a pattern that
# collides with the code it polices is worse than no pattern, because the fix is an exemption and the
# exemptions outlive the API. Measured here, `\.walk\(\)` answered seven findings for this project's
# own `walk()` verb on a document node (so `Path.walk`, 3.12, is absent), and `@override\b` found the
# `typing_extensions.override` shim this repository standardises on - legal on 3.8, so the entry
# below names the import that actually breaks instead of the identifier that is everywhere.
TOO_NEW_FOR_THE_FLOOR = [
    (r"\.write_text\([^)]*\bnewline=", "Path.write_text with the newline keyword", (3, 10)),
    (r"\.read_text\([^)]*\bnewline=", "Path.read_text with the newline keyword", (3, 13)),
    (r"\.removeprefix\(|\.removesuffix\(", "str.removeprefix / str.removesuffix", (3, 9)),
    (r"\bis_relative_to\(", "Path.is_relative_to", (3, 9)),
    (r"\brandbytes\(", "random.randbytes", (3, 9)),
    (r"\bfunctools\.cache\b", "the functools.cache decorator", (3, 9)),
    (r"\bzip\([^)]*\bstrict=", "zip with the strict keyword", (3, 10)),
    (r"\.bit_count\(", "int.bit_count", (3, 10)),
    (r"\bdatetime\.UTC\b", "the datetime.UTC alias", (3, 11)),
    (r"\bStrEnum\b", "enum.StrEnum", (3, 11)),
    (r"\bExceptionGroup\b", "ExceptionGroup", (3, 11)),
    (r"\bfrom typing import [^\n]*\boverride\b", "typing.override, not the typing_extensions shim", (3, 12)),
]


def _code_only(text: str) -> str:
    """Strip docstrings, string literals, then comments: naming an API is not the same as calling it.

    The order is the point. Literals go before comments so that a `#` inside a string cannot delete
    the code behind it, and comments go last so a trailing `#` is never mistaken for an operator.
    What this cannot see is a call assembled at runtime (`getattr(obj, "removeprefix")`) - a
    deliberate limit, because chasing it would mean executing the corpus from the scan.
    """
    text = re.sub(r'"""(?:.|\n)*?"""|\'\'\'(?:.|\n)*?\'\'\'', "", text)
    text = re.sub(r'"(?:[^"\\\n]|\\.)*"|\'(?:[^\'\\\n]|\\.)*\'', "", text)
    return re.sub(r"#.*", "", text)


def _floor_violations(text: str) -> list:
    """Every catalogue entry this code calls, as (what, version) pairs."""
    code = _code_only(text)
    return [
        (what, version)
        for pattern, what, version in TOO_NEW_FOR_THE_FLOOR
        if version > FLOOR and re.search(pattern, code)
    ]


def _is_importable(path: Path) -> bool:
    """Skip anything that runs work at import time rather than defining a module.

    A script whose top level opens the network, a subprocess or a file rewrite is not safe to import
    from a test; `if __name__ == "__main__"` guards are the signal that it defers its work.
    """
    text = path.read_text(encoding="utf-8")
    return bool(re.search(r"^if __name__ == [\"']__main__[\"']:$", text, re.M))


def test_the_directory_is_not_empty():
    """An empty glob would make every other test here pass over nothing."""
    assert SCRIPTS, f"no scripts found under {REPO_ROOT / 'scripts'}"


@pytest.mark.parametrize("path", SCRIPTS, ids=lambda p: p.name)
def test_checkers_declare_their_annotations_lazy(path: Path):
    """`X | Y` and `list[T]` in a signature need the future import to be 3.8-safe.

    Checked statically as well as by import, because a file that only annotates inside function
    bodies would import cleanly and still explode on the first call.
    """
    text = path.read_text(encoding="utf-8")
    body = re.sub(r'"""(?:.|\n)*?"""|\'\'\'(?:.|\n)*?\'\'\'', "", text)
    annotated = re.findall(
        r"^\s*def .*?:\s*(?:list|dict|set|tuple)\[|^\s*def .*->\s*(?:list|dict|set|tuple)\[|^\s*def .*: *[A-Za-z]+ \| ",
        body,
        re.M,
    )
    if annotated and "from __future__ import annotations" not in text:
        raise AssertionError(f"{path.name}: builtin-generic annotations without the future import: {annotated[:3]}")


@pytest.mark.parametrize("path", SCRIPTS, ids=lambda p: p.name)
def test_script_imports_on_this_interpreter(path: Path):
    if not _is_importable(path):
        pytest.skip(f"{path.name}: no `if __name__ == '__main__'` guard, importing would run it")
    spec = importlib.util.spec_from_file_location(path.stem, path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)  # a 3.8-incompatible annotation raises here
    assert hasattr(module, "__file__")


def test_the_supported_floor_is_actually_3_8():
    """The premise of the two tests above, taken from the packaging metadata rather than memory."""
    pyproject = (REPO_ROOT / "pyproject.toml").read_text(encoding="utf-8")
    match = re.search(r'requires-python\s*=\s*"([^"]+)"', pyproject)
    assert match, "no requires-python in pyproject.toml"
    assert "3.8" in match.group(1), f"this test's premise changed: {match.group(1)}"
    declared = tuple(int(part) for part in re.search(r"(\d+)\.(\d+)", match.group(1)).groups())
    assert declared == FLOOR, f"FLOOR says {FLOOR}, the metadata says {declared}"


# The runtime half of the same question, which the two tests above cannot see: an API the floor's
# interpreter has never heard of raises when the function runs, not when the module is imported.
# Scanned over the shipped package as well as the scripts, because a 3.10-only keyword argument in
# `python/pyrs_yaml/` is a crash a user on 3.8 would hit, not a red test.

CORPUS = sorted(list(SCRIPTS) + list((REPO_ROOT / "tests").glob("*.py")) + list((REPO_ROOT / "python").rglob("*.py")))


@pytest.mark.parametrize("path", CORPUS, ids=lambda p: f"{p.parent.name}/{p.name}")
def test_no_call_needs_an_interpreter_newer_than_the_floor(path: Path):
    """No file the matrix or a user on the floor runs may call an API younger than the floor."""
    findings = _floor_violations(path.read_text(encoding="utf-8"))
    assert not findings, f"{path.relative_to(REPO_ROOT).as_posix()} calls {findings}"


def test_the_scan_fires_on_every_shape_it_is_about():
    """Control: one fixture exercising every catalogue entry, so a dead pattern cannot hide.

    A list of regexes that never matches is the characteristic failure of a static gate, and the only
    way to tell it apart from a clean tree is to hand the scan something dirty. Compared for set
    equality, so an entry nobody wrote a reachable shape for is reported the moment it is added.
    """
    fixture = "\n".join(
        [
            'path.write_text(body, encoding="utf-8", newline="\\n")',
            'path.read_text(encoding="utf-8", newline="")',
            'name.removeprefix("x")',
            'name.removesuffix("y")',
            "path.is_relative_to(root)",
            "random.randbytes(8)",
            "@functools.cache",
            "value.bit_count()",
            "zip(left, right, strict=True)",
            "datetime.UTC",
            "enum.StrEnum",
            "except* as ExceptionGroup:",
            "from typing import Any, override",
        ]
    )
    found = {what for what, _ in _floor_violations(fixture)}
    expected = {what for _, what, _ in TOO_NEW_FOR_THE_FLOOR}
    assert found == expected, f"missed {sorted(expected - found)}, invented {sorted(found - expected)}"


def test_the_corpus_reaches_beyond_the_scripts_it_was_built_for():
    """Without this, a renamed directory would shrink CORPUS and every scan above would still pass."""
    parents = {path.parent for path in CORPUS}
    assert (REPO_ROOT / "python" / "pyrs_yaml") in parents or any("pyrs_yaml" in path.as_posix() for path in CORPUS), (
        f"the shipped package is absent from the corpus: {sorted(parents)}"
    )
    assert any(path.parent.name == "tests" for path in CORPUS), CORPUS[:5]
