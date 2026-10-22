#!/usr/bin/env python3
"""Decide whether a finished CI run should be re-run because a leg was cancelled, not failed.

Why this exists, named: `ci.yml` fans every matrix leg into one required check,
`Test matrix (all legs)` (`scripts/check_matrix_verdict.py`), and that gate accepts `success`
only - `cancelled` is refused, deliberately, because a leg that did not report owes the merge a
result. But a `cancelled` leg is often not the change's fault: `concurrency.cancel-in-progress`
supersedes an in-flight run when a newer push lands, and a runner can be reclaimed mid-leg. That
leaves a pull request fully green except for one cancelled wheel or test leg, and a human has to
press Re-run before anything can merge.

The rule, kept as small as the problem:

1. **Only a cancelled leg on an otherwise-green run is retried.** The run must have at least one
   `cancelled` job and at least one `success` job, and no `failure` job. The `success` floor is
   what separates this from a whole run cancelled by supersession - a superseded run has no
   successful jobs left and its newer sibling already runs, so re-running it wastes a machine and
   races the head. The `failure` veto is the safe direction: a leg that failed for a real reason
   must stay red, and the verdict is exactly what says so.
2. **Bounded, so it cannot loop.** A run is retried only while its attempt is below the cap. The
   re-run bumps the attempt, and the trigger fires again on the new attempt, so without the cap a
   leg cancelled every time would re-fire forever. Past the cap the answer is "leave it to a human",
   which is a normal outcome, not an error.
3. **The decision is a pure function.** `decide(conclusions, attempt)` is what
   `tests/test_ci_rerun_gate.py` pins in both directions - the shape this protects against is a
   guard that reads green for the wrong reason, the same failure the matrix's own probes were
   rewritten to refuse.

Usage:
    python scripts/ci_rerun_gate.py --run 1234567           # CI: decide from the run's jobs
    python scripts/ci_rerun_gate.py --run 1234567 --explain # report, dispatch nothing
    python scripts/ci_rerun_gate.py --selftest              # exercise decide() with no network

Exit codes match `dependency_merge_gate.py`: 0 = re-run dispatched (or reported as eligible),
1 = not eligible (a refusal, and a green step - most finished runs are not cancelled),
2 = could not decide (the run's jobs could not be read).
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from collections import Counter
from typing import NamedTuple

#: A run is retried at most this many times before it is left for a human. Attempt 1 is the first
#: run; with a cap of 2 the re-run happens at attempt 1 and is not repeated at attempt 2.
MAX_ATTEMPTS = 2


class Decision(NamedTuple):
    """Whether this run should be re-run, and why in one line."""

    rerun: bool
    reason: str


def decide(conclusions: list[str], attempt: int, max_attempts: int = MAX_ATTEMPTS) -> Decision:
    """The whole rule, on data the caller already read - so it can be tested without a run."""
    counts = Counter(conclusions)
    if counts["failure"]:
        return Decision(False, f"{counts['failure']} job(s) failed for real; the red verdict is the point")
    if not counts["cancelled"]:
        return Decision(False, "no cancelled job; nothing this bot owns")
    if not counts["success"]:
        return Decision(False, "cancelled with no successful job; the run was superseded, not flaked")
    if attempt >= max_attempts:
        return Decision(False, f"already re-run to attempt {attempt}; leaving it for a human")
    return Decision(True, f"{counts['cancelled']} cancelled leg(s) on a run with {counts['success']} green; re-running")


# ── the CLI ──────────────────────────────────────────────────────────────────────────


def gh(*args: str) -> object:
    proc = subprocess.run(["gh", *args], capture_output=True, text=True, encoding="utf-8", errors="replace")
    if proc.returncode != 0:
        raise RuntimeError(f"gh {' '.join(args)} exited {proc.returncode}: {proc.stderr.strip()}")
    return json.loads(proc.stdout or "null")


def repo_slug() -> str:
    return os.environ["GITHUB_REPOSITORY"]


def run_jobs(run_id: str) -> list[str]:
    """Every job conclusion on the run.

    Not paginated: the grid is a few dozen legs and the endpoint returns up to 256 jobs on one page.
    The attempt number is not here - it lives on the run, and the trigger hands it in as `--attempt`.
    """
    page = gh("api", f"repos/{repo_slug()}/actions/runs/{run_id}/jobs")
    if not isinstance(page, dict):
        raise RuntimeError("the jobs endpoint returned no object")
    return [job.get("conclusion") or "" for job in page.get("jobs", [])]


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--run", help="the workflow_run id whose jobs to read")
    parser.add_argument("--attempt", type=int, default=1, help="the run's attempt number, from the trigger event")
    parser.add_argument("--explain", action="store_true", help="report only, dispatch nothing")
    parser.add_argument("--selftest", action="store_true", help="run the built-in fixtures")
    args = parser.parse_args(argv)
    if args.selftest:
        return selftest()
    if not args.run:
        parser.error("--run is required unless --selftest")
    try:
        conclusions = run_jobs(args.run)
        attempt = args.attempt
    except (RuntimeError, json.JSONDecodeError, KeyError) as error:
        print(f"could not read run {args.run}: {error}", file=sys.stderr)
        return 2
    decision = decide(conclusions, attempt)
    print(f"run {args.run} (attempt {attempt}): {'re-run' if decision.rerun else 'leave'} - {decision.reason}")
    if not decision.rerun:
        return 1
    if args.explain:
        return 0
    try:
        gh("api", "-X", "POST", f"repos/{repo_slug()}/actions/runs/{args.run}/rerun-failed-jobs")
    except (RuntimeError, json.JSONDecodeError) as error:
        print(f"the re-run call failed: {error}", file=sys.stderr)
        return 2
    return 0


def selftest() -> int:
    """The fixtures a reviewer can read in one screen, without a run or the network."""
    cases = [
        ("a cancelled wheel on a green run", decide(["success", "success", "cancelled"], 1), True, "1 cancelled"),
        ("a real failure is left red", decide(["success", "failure"], 1), False, "failed for real"),
        ("fully cancelled (superseded)", decide(["cancelled", "cancelled"], 1), False, "superseded"),
        ("nothing cancelled", decide(["success", "success"], 1), False, "nothing this bot owns"),
        ("attempt past the cap", decide(["success", "cancelled"], MAX_ATTEMPTS), False, "leaving it for a human"),
        ("a skipped sibling is not a cancel", decide(["success", "skipped"], 1), False, "nothing this bot owns"),
    ]
    failed = []
    for label, decision, want, want_text in cases:
        good = decision.rerun is want and want_text in decision.reason
        if not good:
            failed.append(label)
        print(f"{'ok  ' if good else 'FAIL'} {label}: {decision.reason}")
    print(f"\nselftest: {len(cases) - len(failed)}/{len(cases)} cases as specified")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
