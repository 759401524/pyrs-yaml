"""Fail when the docs configuration names an option the installed handler does not accept.

Measured, not assumed: adding `this_option_does_not_exist = true` under
`[project.plugins.mkdocstrings.handlers.python.options]` leaves `zensical build --strict` exiting 0 with
output byte-identical to without it. A configuration can therefore outlive a handler version silently -
the same class of failure as a page whose metadata the generator tolerates while publishing the wrong
description, which this repository has now been bitten by once. A *duplicated* key, by contrast, does fail
the build: the parser is strict about TOML and lax about names, so the only place to catch a renamed or
removed option is against the handler's own option fields.

Skipped below Python 3.11, where the docs group is not installed at all (`zensical` and
`mkdocstrings-python` both require it); the CI matrix covers 3.11 through 3.14 on three OSes, so the
check runs where it can.
"""

from __future__ import annotations

import dataclasses
import re
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
MINIMUM = (3, 11)

if sys.version_info < MINIMUM:
    pytest.skip("the docs group needs Python >= 3.11", allow_module_level=True)

# Imported after the version guard rather than with the rest: `tomllib` is 3.11+, and a module-level
# import of it would raise ImportError on 3.8-3.10 before the skip line could be reached - a green
# suite that is actually an uncollected one.
import tomllib  # noqa: E402


def handler_option_fields() -> set[str]:
    """Every option name the installed mkdocstrings-python accepts."""
    module = pytest.importorskip("mkdocstrings_handlers.python")
    options = module.PythonOptions
    assert dataclasses.is_dataclass(options), f"PythonOptions stopped being a dataclass: {options!r}"
    return {field.name for field in dataclasses.fields(options)}


def configured_options() -> tuple[list[str], dict]:
    config = tomllib.loads((REPO_ROOT / "zensical.toml").read_text(encoding="utf-8"))
    python = config["project"]["plugins"]["mkdocstrings"]["handlers"]["python"]
    return sorted(python.get("options", {})), python


def locked_versions() -> dict:
    """`name -> version` from `uv.lock`.

    The `version` key is absent for entries that have none - the root package is
    `source = { editable = "." }` - so reading it unguarded raises KeyError inside a guard, which is the
    worst possible failure for a check whose whole job is to explain a mismatch.
    """
    lock = tomllib.loads((REPO_ROOT / "uv.lock").read_text(encoding="utf-8"))
    return {package["name"]: package.get("version", "") for package in lock["package"]}


def declared_docs_requirements() -> dict:
    """`package name -> requirement string` for the docs dependency group.

    The name is parsed out of the requirement rather than used as the key: an entry reads
    `zensical>=0.0.69; python_version >= '3.11'`, and keying on that string is how the first version of
    this helper raised KeyError while looking for `zensical`.
    """
    pyproject = tomllib.loads((REPO_ROOT / "pyproject.toml").read_text(encoding="utf-8"))
    out = {}
    for requirement in pyproject["dependency-groups"]["docs"]:
        match = re.match(r"^([A-Za-z0-9_.-]+)\s*([<>=~!].*?)?\s*(?:;.*)?$", requirement)
        if match:
            out[match.group(1).lower()] = requirement
    return out


def test_every_configured_option_is_accepted_by_the_handler():
    configured, python = configured_options()
    known = handler_option_fields()
    unknown = sorted(set(configured) - known)
    assert not unknown, (
        "zensical.toml sets option(s) the installed handler does not accept, and the build ignores "
        f"them silently: {unknown}. Known fields: {len(known)}"
    )
    assert "paths" in python, "the handler has no `paths`, so `:::` blocks resolve nothing"


def test_the_paths_the_handler_is_told_to_read_exist():
    _configured, python = configured_options()
    for relative in python["paths"]:
        assert (REPO_ROOT / relative).is_dir(), f"{relative} is not a package directory"


def test_the_locked_tools_are_the_ones_the_configuration_targets():
    """The version floor the config was written against has to be the version that runs.

    Read from `uv.lock`, not from whatever is installed, because CI resolves the lock: a manifest that
    moved and a lock that did not is a half-upgraded state, and this repository already shipped one
    through an unpinned `uv run`.
    """
    versions = locked_versions()
    requirements = declared_docs_requirements()
    for name in ("zensical", "mkdocstrings-python"):
        assert name in requirements, f"{name} is not in the docs group: {sorted(requirements)}"
        floor = re.search(r">=\s*([0-9.]+)", requirements[name]).group(1)
        assert name in versions, f"{name} is missing from the lock"
        installed = tuple(int(part) for part in versions[name].split("."))
        assert installed >= tuple(int(part) for part in floor.split(".")), (
            f"{name} is locked at {versions[name]} but the manifest asks for >={floor}"
        )


def test_the_upgrade_did_not_move_the_stub_generator():
    """`maturin` belongs to the release route, not to the docs group.

    The committed `.pyi` is generated by a specific maturin, and `scripts/check_stub_drift.py` carries
    the rules that reconcile that generator's output with this repository's declarations. A dependency
    bump that silently moves maturin moves the expected stub too, which shows up as a code change
    inside a documentation pull request - the failure this test exists to make loud instead of quiet.
    """
    versions = locked_versions()
    assert versions["maturin"] == "1.14.1", (
        f"maturin moved to {versions['maturin']}; regenerate python/pyrs_yaml/pyrs_yaml.pyi on the "
        "route AGENTS.md declares and update check_stub_drift.py's reconciliation rules in the same "
        "change"
    )
