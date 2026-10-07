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

Runtime calls into a checker are still this module's blind spot and is named in the test below: an
API that exists only on a newer Python (`str.removeprefix`, 3.9) fails only when the function runs,
which is why the coupling gate's own tests have to keep executing its functions - they are what
failed on 3.8 for the second bug in this pair.
"""

from __future__ import annotations

import importlib.util
import re
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
SCRIPTS = sorted((REPO_ROOT / "scripts").glob("*.py"))


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
