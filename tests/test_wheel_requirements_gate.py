"""Gate the gate over the wheel legs' declared dependencies: which imports must bite, and which must stay quiet.

The finding this exists for is concrete. The v0.18.2 tag fired Publish and five wheel legs reddened before a
wheel uploaded, on four collection errors: `tests/test_toml_writer_suite.py`, `tests/test_toml_crosslib.py`,
`tests/test_json5_spec_table.py` and `tests/test_json_crosslib.py` import `tomlkit` and `json5` at module
scope, and `.ci/requirements-test.txt` - the only requirement file those legs install - never named them.
The pair came in 2026-10-18 with 02f7bbf7, "test(toml): judge the TOML writer by parsers we do not own", and
the PR matrix could not see it because that matrix installs the project's dev groups, which do declare both.
Nothing but a release read the thin file, and a release is the one place a missing test dependency costs a
tag instead of a red run.

So the shapes tested here are the shapes of that failure and the shapes that must stay quiet: an authority
dropped from the file is named, an import the rule cannot classify is reported rather than ignored, and the
`try:`-guarded fallback AGENTS.md asks for around optional cross-library readers is not charged - because
charging it would push every optional import toward a bare import to keep this green, which is a worse hole
than the one being closed.
"""

from __future__ import annotations

import importlib.util
import pathlib
import subprocess
import sys

import pytest

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
SCRIPT = REPO_ROOT / "scripts" / "check_wheel_requirements.py"
REQUIREMENTS = REPO_ROOT / ".ci" / "requirements-test.txt"

AUTHORITY_IMPORT = "tomlkit"


@pytest.fixture(scope="module")
def gate():
    spec = importlib.util.spec_from_file_location("check_wheel_requirements", SCRIPT)
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


def test_the_suite_imports_only_what_the_wheel_legs_install():
    """Green is a claim about the tree, so the checker runs over the real files here."""
    result = run()
    assert result.returncode == 0, result.stdout + result.stderr


def test_the_authorities_that_broke_the_release_are_declared(gate):
    """The two names the tag found missing, asserted where the release reads them from."""
    declared = gate.declared_distributions(REQUIREMENTS)
    assert {AUTHORITY_IMPORT, "json5"} <= declared, sorted(declared)


def test_the_gate_is_wired_into_the_hook_set():
    """A checker nothing invokes is a report, not a gate; naming it here keeps the reason close."""
    prek = (REPO_ROOT / "prek.toml").read_text(encoding="utf-8")
    assert "scripts/check_wheel_requirements.py" in prek


# ── the claim that must bite ─────────────────────────────────────────────────────


def test_dropping_an_authority_from_the_file_is_caught(gate, tmp_path, monkeypatch):
    """The exact regression the tag paid for, reproduced without waiting for a release."""
    thin = tmp_path / "requirements-test.txt"
    kept = [
        line
        for line in REQUIREMENTS.read_text(encoding="utf-8").splitlines()
        if not line.lower().startswith(("tomlkit", "json5"))
    ]
    thin.write_text("\n".join(kept) + "\n", encoding="utf-8")
    monkeypatch.setattr(gate, "REQUIREMENTS", thin)
    assert gate.main([]) == 1


def test_an_import_the_rule_cannot_name_is_reported_not_ignored(gate, tmp_path):
    """A new authority that is not in the map has to be named deliberately, or the gate narrows silently."""
    page = tmp_path / "test_needs_a_named_authority.py"
    page.write_text("import no_such_authority_module\n", encoding="utf-8")
    thin = tmp_path / "requirements-test.txt"
    thin.write_text("pytest>=8.0\n", encoding="utf-8")
    found = gate.findings([page], gate.declared_distributions(thin))
    assert found and "named nowhere" in found[0][2], found


# ── the claims that must stay quiet ─────────────────────────────────────────────


def test_a_stdlib_module_above_the_floor_is_not_charged(gate, tmp_path):
    """`tomllib` is 3.11+, so the 3.8 and 3.9/3.10 legs resolve it nowhere.

    Charging it would ask those legs to install a distribution that has never existed, and the verdict
    would then depend on which interpreter ran the hook - which is how this shape was first caught.
    """
    page = tmp_path / "test_needs_a_newer_stdlib.py"
    page.write_text("import tomllib\n", encoding="utf-8")
    assert gate.findings([page], gate.declared_distributions(REQUIREMENTS)) == []


def test_a_guarded_optional_import_is_not_charged(gate, tmp_path):
    """AGENTS.md asks for try/except around optional cross-library readers; charging those would delete them."""
    page = tmp_path / "test_optional_reader.py"
    page.write_text(
        "import pytest\n\ntry:\n    import yaml_rs\nexcept ImportError:\n    yaml_rs = None\n",
        encoding="utf-8",
    )
    assert gate.module_scope_imports(page) == {"pytest"}


def test_local_modules_and_the_standard_library_are_not_requirements(gate, tmp_path):
    """pyrs_yaml is the artefact under test on these legs, and the standard library ships with the interpreter."""
    page = tmp_path / "test_local_only.py"
    page.write_text("import os\nimport pathlib\nimport pyrs_yaml\nimport tests.helper\n", encoding="utf-8")
    names = gate.module_scope_imports(page)
    declared = gate.declared_distributions(REQUIREMENTS)
    assert gate.findings([page], declared) == [], names
