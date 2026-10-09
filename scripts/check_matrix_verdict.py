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

The second input is the change classification. `ci.yml` now triggers on every pull request, because a
required check that never reports is a deadlock - measured on PR #319, where a docs-only pull request had
every check it produced passing and the merge still refused with `Required status check "Test matrix (all
legs)" is expected`. The heavy legs are therefore *skipped on purpose* for a prose-only changeset, and the
verdict tolerates a skipped leg only while `CODE_CHANGED=false`, which is the one condition that makes the
tolerance safe: the classification job itself, and the docs gates that replace the skipped legs for that shape
of change, must succeed whatever the answer says.

Usage:
    python scripts/check_matrix_verdict.py            # reads $NEEDS and $CODE_CHANGED
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

# Jobs that run for every pull request regardless of what it touches: the classifier, and the gates that
# verify the prose this repository publishes. Their being skipped is never explainable by a changeset.
ALWAYS_RUNNING = ("changes", "docs-gates")


def verdict(needs: dict[str, object], code_changed: bool = True) -> list[str]:
    """Every reason this run must not be called green, as one-line messages.

    Args:
        needs: the `toJSON(needs)` object the fan-in job receives.
        code_changed: what the `changes` job classified. When false, a skipped heavy leg is the designed
            outcome of a prose-only pull request; when true, a skip means a leg that owed the merge decision
            a result did not deliver one.

    """
    problems: list[str] = []
    if not needs:
        return ["no jobs were reported: the fan-in lists nothing to wait for"]
    for job in sorted(needs):
        entry = needs[job]
        result = entry.get("result") if isinstance(entry, dict) else None
        if not isinstance(result, str):
            problems.append(f"{job}: no result recorded ({result!r})")
        elif result in ACCEPTED:
            continue
        elif result == "skipped" and not code_changed and job not in ALWAYS_RUNNING:
            continue  # prose-only: this leg was owed nothing, and the classifier said so on the record
        else:
            problems.append(f"{job}: {result}")
    return problems


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--json", dest="payload", default=None, help="the needs object, for tests")
    parser.add_argument(
        "--code-changed",
        dest="code_changed",
        default=None,
        help="'true' or 'false'; defaults to the $CODE_CHANGED the fan-in job exports",
    )
    args = parser.parse_args(argv[1:])

    raw = args.payload if args.payload is not None else os.environ.get("NEEDS", "")
    if not raw.strip():
        print("NEEDS is empty: the job must pass `toJSON(needs)` in its environment")
        return 1
    # Anything but an explicit `false` is treated as "code changed", so a missing or garbled classification
    # demands the full matrix instead of quietly excusing it. The failure this protects against is the
    # tolerance itself becoming the hole: a skipped leg has to be *justified*, not merely present.
    declared = args.code_changed if args.code_changed is not None else os.environ.get("CODE_CHANGED", "")
    code_changed = declared.strip().lower() != "false"
    try:
        needs = json.loads(raw)
    except json.JSONDecodeError as error:  # a malformed payload must not read as a pass
        print(f"could not parse the needs object: {error}")
        return 1
    if not isinstance(needs, dict):
        print(f"expected an object of jobs, got {type(needs).__name__}")
        return 1

    problems = verdict(needs, code_changed=code_changed)
    if problems:
        print(f"matrix verdict: RED ({len(problems)} leg(s) not successful)")
        for line in problems:
            print(f"  - {line}")
        print(
            "\n`Test matrix (all legs)` is the single check to make required in branch protection;\n"
            "if it is not required, a leg can still fail and the pull request still merge."
        )
        return 1
    print(f"matrix verdict: GREEN ({len(needs)} job(s) accounted for, code_changed={str(code_changed).lower()})")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
