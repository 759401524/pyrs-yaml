#!/usr/bin/env python3
"""Fail unless every job a pull request's merge gate depends on actually succeeded.

Why this exists, measured rather than theorised: branch protection matches check *names*, and a
matrix contributes one name per leg - `test (windows-latest, 3.8)`, `test (macos-latest, 3.14)`,
twenty-one of them. Two pull requests in one hour got through that:

* #298 was rebase-merged while `test (windows-latest, 3.8)` had never been consulted; the leg carried
  a real defect - two checkers could not run on the floor `pyproject.toml` promises, one of them
  unimportable since it was written - and it surfaced only on the next pull request.
* #299 went red on `Hygiene` for a formatter's fix that sat unsquashed next to the commit CI was
  checking, which is the same shape: a check that no merge decision consumes is a report, not a gate.

So one job here is the thing to make required: it `needs` the whole matrix and turns 21 names into 1.
The job feeds it `toJSON(needs)` - GitHub's own record of each leg's result - and the verdict is a
pure function so `tests/test_matrix_verdict_gate.py` can hand it the shapes that must bite
(a skipped leg, a cancelled leg, a success) instead of waiting for a real runner to misbehave.

Usage:
    python scripts/check_matrix_verdict.py            # reads $NEEDS, the job's environment
    python scripts/check_matrix_verdict.py --json '{}' # explicit input, for tests and poking
"""

from __future__ import annotations

import argparse
import json
import os
import sys

# A leg that never ran is not a pass. `skipped` is what a matrix leg reports when a sibling it
# depends on failed, and `cancelled` is what `concurrency: cancel-in-progress` leaves behind when a
# newer push supersedes the run - both have to be refused, or the verdict inherits the hole it is
# meant to close.
ACCEPTED = ("success",)


def verdict(needs: dict[str, object]) -> list[str]:
    """Every reason this run must not be called green, as one-line messages."""
    problems: list[str] = []
    if not needs:
        return ["no jobs were reported: the fan-in lists nothing to wait for"]
    for job in sorted(needs):
        entry = needs[job]
        result = entry.get("result") if isinstance(entry, dict) else None
        if not isinstance(result, str):
            problems.append(f"{job}: no result recorded ({result!r})")
        elif result not in ACCEPTED:
            problems.append(f"{job}: {result}")
    return problems


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", dest="payload", default=None, help="the needs object, for tests")
    args = parser.parse_args(argv[1:])

    raw = args.payload if args.payload is not None else os.environ.get("NEEDS", "")
    if not raw.strip():
        print("NEEDS is empty: the job must pass `toJSON(needs)` in its environment")
        return 1
    try:
        needs = json.loads(raw)
    except json.JSONDecodeError as error:  # a malformed payload must not read as a pass
        print(f"could not parse the needs object: {error}")
        return 1
    if not isinstance(needs, dict):
        print(f"expected an object of jobs, got {type(needs).__name__}")
        return 1

    problems = verdict(needs)
    if problems:
        print(f"matrix verdict: RED ({len(problems)} leg(s) not successful)")
        for line in problems:
            print(f"  - {line}")
        print(
            "\n`Test matrix (all legs)` is the single check to make required in branch protection;\n"
            "if it is not required, a leg can still fail and the pull request still merge."
        )
        return 1
    print(f"matrix verdict: GREEN ({len(needs)} job(s) all succeeded)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
