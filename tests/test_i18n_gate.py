"""Self-test for the i18n doc gate (`scripts/check_i18n.py`), which had none.

The rule: every page under `docs/en` has its three mirrors, and the mirrors keep the source's
section shape and placeholders. `prek.toml` runs it as `i18n-check` and `ci.yml` runs it as a step,
so a change to the docs tree is judged by it on both routes.

What was missing is the refusal. Sections 1 and 2 iterate the list the `docs/en` glob returns, so a
tree that yields nothing - the directory renamed, the glob pointed at the wrong place, a checkout
from another working directory, since `DOCS_DIR` is the relative `Path("docs")` - walked five printed
OK lines to `All checks passed!` and exit 0. A gate that compared zero pages cannot report the
policy as held, which is the same convention `check_landed_claims.py` and `check_stub_types.py`
state in their own docstrings: exit 2 means nothing was measurable and is never a pass.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
GATE_PATH = REPO_ROOT / "scripts" / "check_i18n.py"


def _gate():
    spec = importlib.util.spec_from_file_location("check_i18n", GATE_PATH)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def gate():
    return _gate()


def test_the_real_tree_is_a_pass(gate):
    assert gate.main() == 0


def test_a_docs_directory_with_no_page_refuses(gate, monkeypatch, tmp_path, capsys):
    """Zero pages read is a different verdict from zero findings."""
    monkeypatch.setattr(gate, "DOCS_DIR", tmp_path)
    assert gate.main() == 2
    out = capsys.readouterr().out
    assert "cannot measure" in out, out
    assert "All checks passed" not in out, out


def test_the_gate_is_wired_on_both_routes():
    """A checker nothing runs is an `orphan-gate` hole the matrix reports, so pin the wiring here."""
    prek = (REPO_ROOT / "prek.toml").read_text(encoding="utf-8")
    assert 'entry = "python scripts/check_i18n.py"' in prek, prek
    workflow = (REPO_ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
    assert "scripts/check_i18n.py" in workflow, workflow
