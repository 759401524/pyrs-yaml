#!/usr/bin/env python3
"""Compare the test group's version caps against the wheel availability the probe recorded.

The nine counterparty libraries in the test group each carry a `python_version < 'X.Y'` cap, and the
cap is a claim about what PyPI publishes: too low and a scheduled leg silently installs no oracle and
runs fewer assertions, too high and the leg cannot install the group at all. Measured on 2026-10-28,
five of the nine still said `< '3.14'` when `pyjson5`, `numpy`, `ujson` and `python-rapidjson` all had
3.14 wheels, and the ≥3.14 legs were running 236 fewer cross-library tests than the 3.13 leg. Nothing
read that, because the claim and the fact lived in different files.

This gate closes the loop without a network call. `scripts/counterparty_wheels.py` records, per OS and
band, whether a wheel exists; the legs are derived from `scripts/ci_matrix_plan.py` and the
free-threaded matrix in `ci.yml`, so the question is always asked of the interpreters CI really runs.
Two refusals, one per direction:

- a cap that admits a scheduled band the record says has no wheel — the leg would not install;
- a cap that excludes a scheduled band the record says has a wheel *everywhere that band is
  scheduled*, including its free-threaded variant — the coverage is being thrown away, which is what
  a stale cap means. PEP 508 has no free-threading marker, so `3.14t` is admitted by the same
  `python_version` test as `3.14`, and that is why one no-wheel variant justifies the cap.

Only the boundary is asked. The highest admitted band and the lowest excluded one are what a cap
decides; the bands between them were measured when they were set and are not re-asked here. A
boundary with no recorded row is also a refusal, naming the probe command that would fill it.

Usage:
    python scripts/check_counterparty_caps.py
    python scripts/check_counterparty_caps.py --pyproject <path> --record <path> --ci <path>

Exit code 0 = every cap matches the record; 1 = a cap contradicts it; 2 = nothing was measurable
(no record, no legs, no test group), which is never allowed to read as a pass.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
OS_TAG = {"ubuntu-latest": "linux", "windows-latest": "windows", "macos-latest": "macos"}
CAP_RE = re.compile(r"python_version\s*<\s*['\"](\d+\.\d+)['\"]")
FLOOR_RE = re.compile(r"python_version\s*>=\s*['\"](\d+\.\d+)['\"]")
REQUIREMENT_LINE = re.compile(r'^\s{4}"(?P<name>[A-Za-z0-9._-]+)[^;]*;\s*(?P<marker>[^"]+)"')


def key(band: str) -> tuple:
    """`3.14t` compares as 3.14 — PEP 508 has no free-threading marker — and sorts after it."""
    digits, threaded = (band[:-1], True) if band.endswith("t") else (band, False)
    return (*[int(part) for part in digits.split(".")], 1 if threaded else 0)


def gil(band: str) -> str:
    return band[:-1] if band.endswith("t") else band


def test_group_requirements(text: str) -> dict[str, str]:
    """name -> PEP 508 marker for the `[dependency-groups] test` array, nothing else.

    A regex over the array rather than a TOML parse, because the array is the one place the file
    spells markers as the string a cap is made of, and a parse would normalise the very spacing the
    prose gate elsewhere reads. The heading is located first so the extra groups' requirements are
    not mistaken for the test group's.
    """
    heading = re.search(r"^test = \[\n", text, re.M)
    if heading is None:
        return {}
    body = text[heading.end() :].split("\n]\n", 1)[0]
    found: dict[str, str] = {}
    for line in body.splitlines():
        match = REQUIREMENT_LINE.match(line)
        if match:
            found[match.group("name").lower().replace("_", "-")] = match.group("marker")
    return found


def scheduled_legs(ci_plan_module: pathlib.Path, workflow: pathlib.Path) -> list[tuple[str, str]]:
    """(os tag, band) for every test leg CI schedules, free-threaded bands included."""
    spec = importlib.util.spec_from_file_location("ci_matrix_plan", ci_plan_module)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    legs = [(OS_TAG[leg["os"]], leg["python-version"]) for leg in module.plan("pull_request")]
    text = workflow.read_text(encoding="utf-8")
    job = re.search(r"^  test-freethreaded:\n(?:.*\n)*?(?=^  [a-z])", text, re.M)
    if job:
        runs_on = re.search(r"runs-on:\s*([a-z0-9-]+)", job.group(0))
        bands = re.search(r'python-version:\s*\[("3\.\d+t",?\s*)+\]', job.group(0))
        if runs_on and bands:
            for band in re.findall(r'"(3\.\d+t)"', bands.group(0)):
                legs.append((OS_TAG[runs_on.group(1)], band))
    return legs


def bounds(marker: str) -> tuple[str | None, str | None]:
    cap = CAP_RE.search(marker)
    floor = FLOOR_RE.search(marker)
    return (floor.group(1) if floor else None, cap.group(1) if cap else None)


def admitted(marker: str, band: str) -> bool:
    floor, cap = bounds(marker)
    version = key(gil(band))
    if floor and version < key(floor):
        return False
    return not (cap and version >= key(cap))


def check(
    requirements: dict[str, str],
    rows: dict[str, dict],
    legs: list[tuple[str, str]],
    exceptions: dict[str, str] | None = None,
) -> tuple[list[str], list[str]]:
    """(problems, notes) over the boundary of every capped requirement."""
    problems: list[str] = []
    notes: list[str] = []
    exceptions = exceptions or {}
    for name, marker in sorted(requirements.items()):
        cap = CAP_RE.search(marker)
        if cap is None:
            notes.append(f"{name} declares no upper cap, so nothing bounds it here")
            continue
        recorded = any(name in (per_package or {}) for per_os in rows.values() for per_package in per_os.values())
        if not recorded:
            reason = exceptions.get(name)
            if reason:
                notes.append(f"{name} is capped by a boundary that is not a wheel: {reason}")
            else:
                problems.append(
                    f"{name} carries a cap and the record holds no row for it: probe it with "
                    f"scripts/counterparty_wheels.py, or name it in not_wheel_bounded with the reason"
                )
            continue
        admitted_legs = [(os_name, band) for os_name, band in legs if admitted(marker, band)]
        excluded_legs = [
            (os_name, band)
            for os_name, band in legs
            if not admitted(marker, band) and key(gil(band)) >= key(cap.group(1))
        ]
        for os_name, band in admitted_legs:
            # Only the highest admitted band per OS is asked: it is the one a raised cap puts at risk.
            higher = [b for o, b in admitted_legs if o == os_name and key(b) > key(band)]
            if higher:
                continue
            row = rows.get(os_name, {}).get(band, {}).get(name)
            if row is None:
                problems.append(
                    f"{name} admits {os_name} {band} and the record has no row for it: "
                    f"run scripts/counterparty_wheels.py --host {os_name} --bands {band}"
                )
            elif row["state"] != "wheel":
                problems.append(
                    f"{name} admits {os_name} {band} but the record says {row['state']} "
                    f"({row['detail']}) - that leg cannot install the test group"
                )
        if not excluded_legs:
            continue
        # The boundary is a version, not a band: raising `< '3.14'` to `< '3.15'` admits 3.14 AND
        # 3.14t, because PEP 508 has no free-threading marker. Reading only the GIL row let a cap look
        # wasteful for orjson and rtoml, whose 3.14 wheel exists while their 3.14t wheel does not.
        lowest = min(gil(band) for _, band in excluded_legs)
        boundary = [(os_name, band) for os_name, band in excluded_legs if gil(band) == lowest]
        states = []
        for os_name, band in boundary:
            row = rows.get(os_name, {}).get(band, {}).get(name)
            states.append((os_name, band, None if row is None else row["state"]))
        if all(state == "wheel" for _, _, state in states):
            where = ", ".join(f"{os_name} {band}" for os_name, band, _ in states)
            problems.append(
                f"{name} caps at < '{cap.group(1)}' but the record says a wheel exists on every leg of "
                f"the next scheduled band ({where}) - the cap is throwing away coverage. Raise it, or "
                f"re-run the probe if the vendor withdrew the wheel."
            )
    return problems, notes


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--pyproject", type=pathlib.Path, default=REPO / "pyproject.toml")
    parser.add_argument("--record", type=pathlib.Path, default=REPO / ".ci" / "counterparty-wheels.json")
    parser.add_argument("--ci", type=pathlib.Path, default=REPO / ".github" / "workflows" / "ci.yml")
    parser.add_argument("--plan", type=pathlib.Path, default=REPO / "scripts" / "ci_matrix_plan.py")
    args = parser.parse_args(argv)

    if not args.pyproject.is_file() or not args.record.is_file():
        print("cannot measure: pyproject.toml or the wheel record is missing")
        return 2
    requirements = test_group_requirements(args.pyproject.read_text(encoding="utf-8"))
    if not requirements:
        print("cannot measure: no [dependency-groups] test array was readable")
        return 2
    payload = json.loads(args.record.read_text(encoding="utf-8"))
    rows: dict[str, dict] = payload.get("rows") or {}
    if not isinstance(rows, dict) or not rows:
        print("cannot measure: the wheel record holds no rows")
        return 2
    legs = scheduled_legs(args.plan, args.ci)
    if not legs:
        print("cannot measure: the CI plan reported no legs")
        return 2
    problems, notes = check(requirements, rows, legs, payload.get("not_wheel_bounded") or {})
    capped = sorted(name for name, marker in requirements.items() if CAP_RE.search(marker))
    if problems:
        print(f"{len(problems)} cap/record disagreement(s) across {len(capped)} capped requirement(s):")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print(
        f"OK: {len(capped)} capped requirement(s) match the record on {len({b for _, b in legs})} scheduled "
        f"band(s) x {len({o for o, _ in legs})} OS(es)" + (f"; {len(notes)} note(s)" if notes else "")
    )
    for note in notes:
        print(f"  note: {note}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
