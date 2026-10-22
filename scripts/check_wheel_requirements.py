#!/usr/bin/env python3
"""Assert that the wheel legs' declared test dependencies cover what the suite imports to collect.

Why this exists, named: the v0.18.2 tag fired Publish and five wheel legs reddened before a single wheel was
uploaded. Not a build failure - collection. `.ci/requirements-test.txt` is the only requirement file the
release workflow installs (publish.yml runs `uv pip install -r .ci/requirements-test.txt`, and so does
`.ci/pytest-wheel.sh`), and it had never been told about `tomlkit` or `json5`, while four test modules import
one of them at module scope: `tests/test_toml_writer_suite.py` and `tests/test_toml_crosslib.py` (added 2026-10-18
by 02f7bbf7, "test(toml): judge the TOML writer by parsers we do not own") and `tests/test_json5_spec_table.py`
and `tests/test_json_crosslib.py`. The PR matrix installs the project's dev groups, which do declare both, so
every gate before the tag was green and only the release read the thin file. Measured by blocking the two
names in `sys.modules`: the same four collection errors publish.yml printed.

The rule: every module the suite imports at module scope must be either the standard library, a module of this
repository, or a distribution named in `.ci/requirements-test.txt`. A name that resolves outside the standard
library and is not in the map below is reported rather than ignored, so adding an authority without declaring
it on the wheel legs is a red run here, not a red release.

Usage:
    python scripts/check_wheel_requirements.py                 # the whole suite
    python scripts/check_wheel_requirements.py path [...]      # explicit, for tests

Exit code 0 = the thin file covers the suite; 1 = an import is undeclared; 2 = the checker could not measure,
which is never allowed to read as a pass.
"""

from __future__ import annotations

import argparse
import ast
import importlib.util
import pathlib
import re
import sys
import sysconfig

REPO = pathlib.Path(__file__).resolve().parent.parent
REQUIREMENTS = REPO / ".ci" / "requirements-test.txt"
TESTS = REPO / "tests"

# Modules this repository owns. pyrs_yaml is the artefact under test on these legs, so it is never a
# requirement of the file that lists them.
LOCAL_MODULES = {"conftest", "pyrs_yaml", "scripts", "tests"}

# Import name to distribution name, for the cases where they differ. Kept explicit because a guessed
# mapping is how a gate starts passing for the wrong reason; an import that is not listed here is a
# finding, so a new authority has to be named deliberately.
AUTHORITY_MODULES = {
    "_pytest": "pytest",
    "hypothesis": "hypothesis",
    "json5": "json5",
    "jsonschema": "jsonschema",
    "numpy": "numpy",
    "pydantic": "pydantic",
    "pytest": "pytest",
    "ruamel": "ruamel-yaml",
    "tomli": "tomli",
    "tomli_w": "tomli-w",
    "tomlkit": "tomlkit",
    "yaml": "pyyaml",
}

DECLARATION = re.compile(r"^\s*(?P<name>[A-Za-z0-9][A-Za-z0-9._-]*)")
STDLIB_ROOT = pathlib.Path(sysconfig.get_paths()["stdlib"]).resolve()

# Standard library on some interpreter this project still supports, so an older one cannot locate it and
# the gate must not ask for a distribution that does not exist. `tests/test_docs_config_gate.py` imports
# `tomllib` behind its own version guard, which is the shape this list covers; a name missing here is a
# finding rather than a silent pass, so widening the suite's stdlib assumptions stays a deliberate act.
STDLIB_ABOVE_FLOOR = {"tomllib"}


def normalise(name: str) -> str:
    """PEP 503-ish: the form both a requirement line and a distribution name agree on."""
    return re.sub(r"[-_.]+", "-", name).lower()


def short(path: pathlib.Path) -> str:
    """The path as a reader expects it, or as given when it sits outside the repository."""
    try:
        return str(path.relative_to(REPO))
    except ValueError:
        return str(path)


def declared_distributions(path: pathlib.Path) -> set[str]:
    """Every distribution the wheel legs install, marker text ignored."""
    found = set()
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        match = DECLARATION.match(line)
        if match:
            found.add(normalise(match.group("name")))
    return found


def module_scope_imports(path: pathlib.Path) -> set[str]:
    """Top-level import names that a collection failure would hit.

    Only module scope counts, and an import inside `try:` is the suite's own declared fallback, which is
    the pattern AGENTS.md asks for around optional cross-library readers - charging those would push
    every optional import toward a bare import to keep this green.
    """
    tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    names: set[str] = set()
    for node in tree.body:
        if isinstance(node, ast.Import):
            names.update(alias.name.split(".")[0] for alias in node.names)
        elif isinstance(node, ast.ImportFrom) and node.module and node.level == 0:
            names.add(node.module.split(".")[0])
    return names


def is_standard_library(name: str) -> bool:
    """True when the interpreter resolves the name inside its own standard-library tree.

    A name that resolves nowhere is not standard library either: that is the wheel leg's own view of an
    authority it was never told to install, which is the finding this file exists to make. The exception is
    `STDLIB_ABOVE_FLOOR`, because asking for a distribution that has never existed is not a fix.
    """
    if name in sys.builtin_module_names or name in STDLIB_ABOVE_FLOOR:
        return True
    spec = importlib.util.find_spec(name)
    if spec is None:
        return False
    origin = spec.origin or ""
    if origin == "frozen":
        return True
    roots = spec.submodule_search_locations or []
    candidates = [pathlib.Path(origin).resolve()] if origin else [pathlib.Path(p).resolve() for p in roots]
    return any(STDLIB_ROOT in candidate.parents or candidate == STDLIB_ROOT for candidate in candidates)


def findings(paths: list[pathlib.Path], declared: set[str]) -> list[tuple[str, int, str]]:
    """Every `(file, line-ish, reason)` where the suite needs something the wheel legs do not install."""
    out: list[tuple[str, int, str]] = []
    for path in paths:
        for name in sorted(module_scope_imports(path)):
            if name in LOCAL_MODULES or is_standard_library(name):
                continue
            dist = AUTHORITY_MODULES.get(name)
            if dist is None:
                out.append(
                    (path.name, 0, f"{name} is imported at module scope but named nowhere, so the rule cannot judge it")
                )
            elif normalise(dist) not in declared:
                out.append(
                    (
                        path.name,
                        0,
                        f"{name} is imported at module scope, but {dist} is not declared in {REQUIREMENTS.name}",
                    )
                )
    return out


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="*", help="test files to judge (default: the whole suite)")
    args = parser.parse_args(argv)
    try:
        declared = declared_distributions(REQUIREMENTS)
    except OSError as exc:
        print(f"cannot read {REQUIREMENTS}: {exc}", file=sys.stderr)
        return 2
    if args.paths:
        # A caller may pass the changed file that tripped the hook, and that file is not always a test;
        # with nothing left to judge, the honest reading is the whole suite rather than a smaller one.
        paths = [pathlib.Path(arg) for arg in args.paths if arg.endswith(".py")]
        if not paths:
            paths = sorted(TESTS.glob("*.py"))
    else:
        paths = sorted(TESTS.glob("*.py"))
    if not paths:
        print(f"no test files under {TESTS}, so there is nothing to measure", file=sys.stderr)
        return 2
    problems: list[tuple[str, int, str]] = []
    for path in paths:
        try:
            problems.extend(findings([path], declared))
        except (OSError, SyntaxError) as exc:
            print(f"cannot parse {path} ({exc})", file=sys.stderr)
            return 2
    if problems:
        print(f"{len(problems)} undeclared test import(s):", file=sys.stderr)
        for name, _line, reason in problems:
            print(f"  {name}: {reason}", file=sys.stderr)
        print(
            f"Remedy: declare the distribution in {short(REQUIREMENTS)} - the PR matrix installs "
            "the dev groups, so only the wheel legs read that file, and a release failure there is the same "
            "collection error this suite would print.",
            file=sys.stderr,
        )
        return 1
    print(
        f"OK: {len(paths)} test file(s), {len(declared)} declared distribution(s) on the wheel legs, "
        "every module-scope import is covered"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
