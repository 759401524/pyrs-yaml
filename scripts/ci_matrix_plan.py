#!/usr/bin/env python3
"""Plan the CI interpreter matrix, so the grid is derived from what the package *claims* to support.

Why a planner and not a literal `os x python-version` cross product. Measured over six recent CI runs
(144 matrix legs): a leg costs 138 s on ubuntu, 186 s on macOS and 251 s on Windows, of which the
suite itself is 116 / 141 / 183 s. So the 8 x 3 = 24-cell cross product spent 78.2 minutes of runner
time per pull request, and a cell in the middle of the version range cost 1.8x on Windows what the same
cell cost on ubuntu *for the same assertions*. Evaluating every interpreter on every OS is the
expensive axis; evaluating every interpreter *somewhere* and every OS *at the boundaries* is the part
that carries information - which is what the last 30 runs said too, having produced no failure confined
to one interpreter version and none confined to one OS.

The rules, all of them derived:

* the version list is read from `pyproject.toml`'s trove classifiers, so the matrix cannot drift away
  from the range the package publishes, and cannot silently drop a version added there;
* every supported version runs the suite on the cheapest runner (ubuntu) on every pull request;
* the first supported version and the last run the suite on every OS, because that is where an
  interpreter and a platform can disagree — the floor is what the package promises and what
  `check_matrix_verdict.py` itself runs on, the top is where the abi3 boundary claims break;
* the coverage carrier runs the suite on ubuntu alone. Its assertions are the ones ubuntu already makes
  for that version, re-run on a runner that bills 10x (macOS) or 2x (Windows) for the same document, and
  the mid-range is the part this file's own measurement says yields: no failure in the last 30 runs was
  confined to one interpreter version or one OS;
* the remaining (OS, version) pairs are not dropped but demoted to a wheel smoke: install the abi3
  wheel that platform built, import it, and round-trip one document through every dialect. That is the
  cross product kept, at the price of a setup rather than the price of a suite;
* the weekly schedule runs the full grid with the suite, which is what makes the demotion reviewable
  instead of a one-way ratchet.

Two gates used to be selected by *cell* - `--cov-fail-under=78` on ubuntu/3.12 and the compliance
summary on ubuntu/3.14, both behind `matrix.os == ... && matrix.python-version == ...`. Delete one of
those cells and both conditions silently go false, the suite still passes, and the run is green. Here
they are roles attached to legs, and `--check` refuses any plan that does not carry each of them
exactly once, so the same edit fails the plan instead of switching a gate off.

`--event` chooses the tier. The output is a JSON array of matrix `include` entries, one per leg, and
`tests/test_ci_matrix_plan.py` is what keeps these rules from rotting.

Python 3.8 is the floor of the supported range and `tests/test_scripts_import_on_supported_python.py`
imports this file on it, so the annotations stay lazy and no 3.9+ stdlib call appears.

Usage:
    python scripts/ci_matrix_plan.py --check
    python scripts/ci_matrix_plan.py --event pull_request
    python scripts/ci_matrix_plan.py --event schedule --smoke
"""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
PYPROJECT = REPO / "pyproject.toml"

OS_ALL = ("ubuntu-latest", "windows-latest", "macos-latest")
CHEAP_OS = "ubuntu-latest"

# The roles the matrix must carry exactly one leg each. They used to be written as conditionals on
# `matrix.os && matrix.python-version` inside ci.yml, where removing a cell silently switches the
# gate off; here they are inputs to the plan, and a role naming an unsupported version is an error.
COVERAGE_VERSION = "3.12"
COMPLIANCE_VERSION = "3.14"

CLASSIFIER = re.compile(r'^\s*"Programming Language :: Python :: (3\.\d+)",\s*$')
MIN_VERSION = "3.8"


def _key(version: str) -> tuple:
    return tuple(int(part) for part in version.split("."))


def supported_versions(text: str) -> list:
    """Every `3.x` this package declares, in ascending order.

    Sorted on parsed integers rather than as strings, because `"3.10" < "3.8"` as text, and a
    string-sorted plan would put the floor version in the middle of the grid.
    """
    found = set()
    in_classifiers = False
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("classifiers"):
            in_classifiers = True
            continue
        if in_classifiers and stripped == "]":
            break
        if not in_classifiers:
            continue
        match = CLASSIFIER.match(line)
        if match:
            found.add(match.group(1))
    if not found:
        raise SystemExit("no 'Programming Language :: Python :: 3.x' classifiers found in pyproject.toml")
    return sorted(found, key=_key)


def boundary_versions(versions: list) -> list:
    """First, last, and the coverage carrier - the versions the *roles* of this plan name.

    Kept as the validator it always was: a role pointing at a version `pyproject.toml` does not declare
    is an error here rather than a plan that quietly carries no gate. Which of these a runner that bills
    more than ubuntu must run the suite on is the narrower `expensive_suite_versions`.
    """
    wanted = [versions[0], versions[-1], COVERAGE_VERSION]
    out = []
    for version in wanted:
        if version not in versions:
            raise SystemExit(
                f"matrix role wants {version!r}, which pyproject.toml does not declare as supported "
                f"(declared: {', '.join(versions)})"
            )
        if version not in out:
            out.append(version)
    return out


def expensive_suite_versions(versions: list) -> list:
    """The two versions a runner that bills more than ubuntu must run the suite on: floor and top.

    Derived from the declared range, never from a literal, and deliberately narrower than
    `boundary_versions`: the coverage carrier is a *gate role* attached to one ubuntu leg, and
    re-running ~2 500 assertions on macOS (10x) or Windows (2x) for the same document buys an
    interpreter-platform interaction that the floor and the top are the recorded places to find.
    """
    out = []
    for version in (versions[0], versions[-1]):
        if version not in out:
            out.append(version)
    return out


def _leg(os_name: str, version: str, versions: list, event: str = "pull_request") -> dict:
    full = event in ("schedule", "workflow_dispatch")
    return {
        "os": os_name,
        "python-version": version,
        "coverage": os_name == CHEAP_OS and version == COVERAGE_VERSION,
        "compliance": os_name == CHEAP_OS and version == COMPLIANCE_VERSION,
        # The suite runs in full on ubuntu at every version and on the floor and top elsewhere. A
        # leg with `suite: false` is the smoke shape: the same artefact, imported and exercised,
        # without ~2 500 assertions re-run per OS. The scheduled tier asks for the full grid, so there
        # every cell runs the suite and the distinction is switched off rather than re-derived.
        "suite": full or os_name == CHEAP_OS or version in expensive_suite_versions(versions),
    }


def plan(event: str, versions: list | None = None) -> list:
    """The matrix legs for one event. `schedule` and manual dispatch get the full cross product."""
    if versions is None:
        versions = supported_versions(PYPROJECT.read_text(encoding="utf-8"))
    if event in ("schedule", "workflow_dispatch"):
        return [_leg(os_name, version, versions, event) for os_name in OS_ALL for version in versions]
    legs = [_leg(CHEAP_OS, version, versions, event) for version in versions]
    for os_name in OS_ALL:
        if os_name == CHEAP_OS:
            continue
        for version in expensive_suite_versions(versions):
            legs.append(_leg(os_name, version, versions, event))
    order = {name: index for index, name in enumerate(OS_ALL)}
    return sorted(legs, key=lambda leg: (order[leg["os"]], _key(leg["python-version"])))


def smoke_plan(versions: list) -> list:
    """Every (OS, version) pair the suite tier does not run: the cross product, kept cheap.

    The legs carry the same keys as a suite leg with `suite` false, so `ci.yml` can feed either tier
    through one step shape and the difference is a flag rather than the record's shape.
    """
    covered = {(leg["os"], leg["python-version"]) for leg in plan("pull_request", versions)}
    return [
        {
            "os": os_name,
            "python-version": version,
            "coverage": False,
            "compliance": False,
            "suite": False,
        }
        for os_name in OS_ALL
        for version in versions
        if (os_name, version) not in covered
    ]


def check(versions: list, legs: list | None = None, smoke: list | None = None) -> list:
    """The invariants the ci.yml wiring depends on, as problems rather than assertions.

    `legs` and `smoke` default to the live plan. They are parameters so a test can hand this function
    a doctored plan and require it to name the hole - a validator that only ever sees the committed
    shape cannot be shown to be capable of failing, which is the defect class the planner exists to
    close.
    """
    if legs is None:
        legs = plan("pull_request", versions)
    if smoke is None:
        smoke = smoke_plan(versions)
    full = plan("schedule", versions)
    problems = []
    roles = {"coverage": 0, "compliance": 0}
    for leg in legs:
        for role in roles:
            if leg[role]:
                roles[role] += 1
    for role, count in sorted(roles.items()):
        if count != 1:
            problems.append(f"exactly one leg must carry the {role!r} role; the plan has {count}")
    for role in roles:
        carriers = [leg for leg in legs if leg[role]]
        if len(carriers) == 1 and carriers[0]["os"] != CHEAP_OS:
            problems.append(
                f"the {role!r} role sits on {carriers[0]['os']}, not on {CHEAP_OS}: "
                f"the gate would run on a runner that bills more for the same assertions"
            )
    for version in versions:
        if not any(leg["python-version"] == version and leg["suite"] for leg in legs):
            problems.append(f"{version} runs no suite on any OS of the pull-request tier")
    for os_name in OS_ALL:
        if os_name == CHEAP_OS:
            continue
        for version in expensive_suite_versions(versions):
            if not any(leg["os"] == os_name and leg["python-version"] == version and leg["suite"] for leg in legs):
                problems.append(f"{os_name} does not run the suite on the {version} it owes")
    if len(full) != len(OS_ALL) * len(versions):
        problems.append(f"the scheduled tier should be the full cross product, got {len(full)} legs")
    if len(legs) + len(smoke) != len(full):
        problems.append(
            f"pull-request suite legs ({len(legs)}) plus smoke legs ({len(smoke)}) must equal the "
            f"full grid ({len(full)})"
        )
    pr_cells = {(leg["os"], leg["python-version"]) for leg in legs}
    smoke_cells = {(leg["os"], leg["python-version"]) for leg in smoke}
    if pr_cells & smoke_cells:
        problems.append(f"a cell is in both tiers: {sorted(pr_cells & smoke_cells)}")
    if any(not leg["suite"] for leg in legs):
        problems.append("every leg of the pull-request suite tier must run the suite")
    if any(leg["suite"] or leg["coverage"] or leg["compliance"] for leg in smoke):
        problems.append("a smoke leg must carry no suite and no gate role")
    if any(not leg["suite"] for leg in full):
        problems.append("the scheduled tier must run the suite in every cell, not just the PR tier")
    if _key(MIN_VERSION) > _key(versions[0]):
        problems.append(f"the floor declared in requires-python ({MIN_VERSION}) is above {versions[0]}")
    return problems


def main(argv: list) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--event", default="pull_request", help="the GitHub event name to plan for")
    parser.add_argument("--check", action="store_true", help="validate the plan instead of printing it")
    parser.add_argument("--smoke", action="store_true", help="print the cheap cross-product tier")
    args = parser.parse_args(argv[1:])

    versions = supported_versions(PYPROJECT.read_text(encoding="utf-8"))
    if args.check:
        problems = check(versions)
        if problems:
            print("matrix plan is not sound:")
            for line in problems:
                print(f"  - {line}")
            return 1
        legs = plan("pull_request", versions)
        print(
            f"OK: {len(legs)} suite legs of {len(OS_ALL) * len(versions)} cells on a pull request, "
            f"{len(smoke_plan(versions))} smoke legs, {len(versions)} declared versions "
            f"({versions[0]}-{versions[-1]}), one coverage leg, one compliance leg"
        )
        return 0
    if args.smoke:
        if args.event in ("schedule", "workflow_dispatch"):
            print(json.dumps([]))  # the whole grid already runs the suite; nothing to demote
        else:
            print(json.dumps(smoke_plan(versions)))
        return 0
    print(json.dumps(plan(args.event, versions)))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
