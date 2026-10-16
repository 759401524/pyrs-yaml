#!/usr/bin/env python3
"""Decide whether a dependency pull request may merge itself, and only when CI says so.

Dependabot is configured in `.github/dependabot.yml`; this file is the half that decides what
happens to its pull requests. Three rules, one reason each:

1. **The verdict is the gate.** Branch protection on `main` requires exactly one check name,
   `Test matrix (all legs)` (`scripts/check_matrix_verdict.py`), and that fan-in job reads every
   leg - the test grid, the docs gates, the fuzz replay, the Ir floor. A dependency pull request is
   merged when that verdict says `success` and never on a partial set, because "the build was green"
   is not the same claim as "the gate that can block a merge was green".
2. **Only a resolution move merges itself.** A lockfile bump changes which version satisfies the
   declared range; a manifest bump (`Cargo.toml`, `pyproject.toml`) changes the range, and `AGENTS.md`
   makes moving a pin a deliberate act with its own obligations - regenerate the stub, update
   `scripts/check_stub_drift.py`'s reconciliation rules, measure a parser major's migration. The bot
   cannot discharge those, so it may not merge them. The same refusal covers the committed type stub
   and any interpreter or MSRV floor, because dropping a supported Python is a product decision:
   pyo3 0.30+ requires 3.9, and `abi3-py38` is the floor this project ships against.
3. **A human can always stop it.** Any of the refusal labels on the pull request ends the automerge,
   and nothing here uses `--admin`: `enforce_admins` is on, so an admin merge would be the one way to
   get an unverified dependency onto `main`, and the repository's rule is that a gate is not
   bypassed to make a day easier.

The rule about release notes lives in `check_changelog_coupling.py`, which this script's allowlist is
deliberately wider-or-equal to: a lockfile-only bump is not a change the notes describe, the same
judgement that already excludes `.github/workflows/**` and `prek.toml` there (measured: including
them reddened dependabot for no reason and the gate got bypassed).

Usage:
    python scripts/dependency_merge_gate.py --pr 447              # CI: decide, then merge if allowed
    python scripts/dependency_merge_gate.py --pr 447 --explain    # report, merge nothing
    python scripts/dependency_merge_gate.py --selftest            # exercise decide() with no network

Exit codes match the other harnesses: 0 merged or reported-allowed, 1 refused (a human takes over),
2 could not decide (missing data, failed call) - a refusal is a normal outcome, an inability is not.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from typing import NamedTuple

#: The single check name branch protection requires (see `scripts/check_matrix_verdict.py`).
REQUIRED_CHECK = "Test matrix (all legs)"

#: The bot's login. Anything else is a human pull request and none of this applies.
DEPENDABOT = "dependabot[bot]"

#: Paths whose change is a resolved-version move and nothing more.
AUTOMERGE_ALLOWLIST = (
    "Cargo.lock",
    "uv.lock",
    "fuzz/Cargo.lock",
    "prek.toml",
)

#: Action pins live in the workflow directory and are the class `check_changelog_coupling.py`
#: already treats as out of scope for release notes.
AUTOMERGE_ALLOW_PREFIXES = (".github/workflows/",)

#: Names that make a bump a decision rather than a resolution: a declared range, a supported floor,
#: or the shipped typing contract. Matched on the path's last segment so workspace members
#: (`crates/pyrs-yaml/Cargo.toml`) are covered by the same rule as the root manifest.
REFUSED_NAMES = (
    "Cargo.toml",
    "pyproject.toml",
    "rust-toolchain",
    "rust-toolchain.toml",
    "pyrs_yaml.pyi",
)

#: Refusal labels. `needs-human` is the general stop; `python-floor` names the specific decision the
#: repository has not made yet (retire Python 3.8), and `.github/dependabot.yml` puts every pyo3
#: pull request under it so the floor is never dropped as a side effect of a dependency bump.
REFUSED_LABELS = ("needs-human", "python-floor", "do-not-merge", "blocked")


class Decision(NamedTuple):
    """Whether this pull request may merge itself, and why in one line."""

    merge: bool
    reason: str


def name_of(path: str) -> str:
    path = path.replace("\\", "/")
    return path.rsplit("/", 1)[-1]


def is_allowed_path(path: str) -> bool:
    path = path.replace("\\", "/")
    if name_of(path) in REFUSED_NAMES or path.endswith(".pyi"):
        return False
    return path in AUTOMERGE_ALLOWLIST or path.startswith(AUTOMERGE_ALLOW_PREFIXES)


def decide(pr: dict, files: list[str], verdict: str) -> Decision:
    """The whole rule, on data the caller already has - so it can be tested without a repository."""
    author = (pr.get("user") or {}).get("login", "")
    if author != DEPENDABOT:
        return Decision(False, f"{author or 'no author'} is not the dependency bot")
    if pr.get("state") != "open":
        return Decision(False, f"pull request is {pr.get('state')}, not open")
    if not pr.get("mergeable", True):
        return Decision(False, "the branch does not merge cleanly onto its base")

    labels = {str(item.get("name", "")) for item in pr.get("labels", [])}
    stopped = sorted(labels & set(REFUSED_LABELS))
    if stopped:
        return Decision(False, f"refused by label(s): {', '.join(stopped)}")

    if verdict != "success":
        return Decision(False, f"{REQUIRED_CHECK} reported {verdict or 'nothing'}, not success")

    refused = sorted(p for p in files if not is_allowed_path(p))
    if refused:
        return Decision(
            False,
            "moves a declared range, a supported floor or the shipped stub, so a human owns it: "
            + ", ".join(refused[:5]),
        )
    if not files:
        return Decision(False, "the pull request changed no files")
    return Decision(True, f"{len(files)} resolution-only file(s) and {REQUIRED_CHECK} is success")


# ── the CLI ──────────────────────────────────────────────────────────────────────────


def gh(*args: str) -> object:
    proc = subprocess.run(["gh", *args], capture_output=True, text=True, encoding="utf-8", errors="replace")
    if proc.returncode != 0:
        raise RuntimeError(f"gh {' '.join(args)} exited {proc.returncode}: {proc.stderr.strip()}")
    return json.loads(proc.stdout or "null")


def repo_slug() -> str:
    """The repository the decision is about: the runner's context, else the checked-out remote."""
    slug = os.environ.get("GITHUB_REPOSITORY", "")
    if slug:
        return slug
    return str(gh("repo", "view", "--json", "nameWithOwner")["nameWithOwner"])


def read_pr(number: str) -> tuple[dict, list[str], str]:
    pr = gh(
        "pr",
        "view",
        number,
        "--json",
        "number,state,author,labels,mergeable,headRefOid",
    )
    pr["user"] = pr.pop("author", None) or {}
    files = [f["path"] for f in gh("pr", "view", number, "--json", "files")["files"]]
    return pr, files, verdict_for(str(pr["headRefOid"]))


def verdict_for(sha: str) -> str:
    """The conclusion of the one check name that can block a merge, read from the API.

    Deliberately not handed in by the caller: the workflow that triggers this knows only that *its*
    own run finished, while `skipped` and `cancelled` are refusals and a fan-in job that never
    reported has to read as "nothing reported" here rather than as success.
    """
    lines = gh_lines(
        "api",
        "--paginate",
        f"repos/{repo_slug()}/commits/{sha}/check-runs",
        "--jq",
        f'[.check_runs[] | select(.name == "{REQUIRED_CHECK}")] | (last | .conclusion) // "none"',
    )
    return lines[-1] if lines else "none"


def gh_lines(*args: str) -> list[str]:
    """`gh` with `--jq -r`: one raw line per page, which is not a JSON document."""
    proc = subprocess.run(["gh", *args], capture_output=True, text=True, encoding="utf-8", errors="replace")
    if proc.returncode != 0:
        raise RuntimeError(f"gh {' '.join(args)} exited {proc.returncode}: {proc.stderr.strip()}")
    return [line.strip() for line in proc.stdout.splitlines() if line.strip()]


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--pr", help="pull request number to decide")
    parser.add_argument("--explain", action="store_true", help="report only, never merge")
    parser.add_argument("--selftest", action="store_true", help="run the built-in fixtures")
    args = parser.parse_args(argv)
    if args.selftest:
        return selftest()
    if not args.pr:
        parser.error("--pr is required unless --selftest")
    try:
        pr, files, verdict = read_pr(args.pr)
    except (RuntimeError, json.JSONDecodeError, KeyError) as error:
        print(f"could not decide #{args.pr}: {error}", file=sys.stderr)
        return 2
    decision = decide(pr, files, verdict)
    print(f"#{args.pr}: {'merge' if decision.merge else 'refused'} - {decision.reason}")
    if not decision.merge:
        return 1
    if args.explain:
        return 0
    merged = subprocess.run(
        ["gh", "pr", "merge", args.pr, "--rebase"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    if merged.returncode != 0:
        print(f"gh refused the merge: {merged.stderr.strip()}", file=sys.stderr)
        return 2
    return 0


def selftest() -> int:
    """The fixtures a reviewer can read in one screen, without a repository or the network."""
    ok = {
        "user": {"login": "someone"},
        "state": "open",
        "mergeable": True,
        "labels": [],
    }
    bot = {**ok, "user": {"login": DEPENDABOT}}
    cases = [
        ("not the dependency bot", decide(ok, ["Cargo.lock"], "success"), False, "not the dependency bot"),
        ("verdict failure", decide(bot, ["Cargo.lock"], "failure"), False, "reported failure"),
        ("verdict absent", decide(bot, ["Cargo.lock"], ""), False, "reported nothing"),
        ("skipped is a refusal", decide(bot, ["Cargo.lock"], "skipped"), False, "reported skipped"),
        ("one lockfile merges", decide(bot, ["Cargo.lock"], "success"), True, "resolution-only"),
        ("both lockfiles merge", decide(bot, ["uv.lock", "Cargo.lock"], "success"), True, "2 resolution-only"),
        ("an action pin merges", decide(bot, [".github/workflows/ci.yml"], "success"), True, "resolution-only"),
        ("a declared range waits", decide(bot, ["Cargo.toml"], "success"), False, "Cargo.toml"),
        (
            "a member manifest waits",
            decide(bot, ["crates/pyrs-yaml/Cargo.toml"], "success"),
            False,
            "crates/pyrs-yaml/Cargo.toml",
        ),
        ("the shipped stub waits", decide(bot, ["python/pyrs_yaml/pyrs_yaml.pyi"], "success"), False, "pyrs_yaml.pyi"),
        (
            "a source file smuggled alongside a lockfile waits",
            decide(bot, ["Cargo.lock", "crates/x/src/lib.rs"], "success"),
            False,
            "crates/x/src/lib.rs",
        ),
        (
            "the floor label stops it",
            decide({**bot, "labels": [{"name": "python-floor"}]}, ["Cargo.lock"], "success"),
            False,
            "refused by label(s): python-floor",
        ),
        (
            "a conflicted base waits",
            decide({**bot, "mergeable": False}, ["Cargo.lock"], "success"),
            False,
            "does not merge cleanly",
        ),
        (
            "an already closed pr waits",
            decide({**bot, "state": "closed"}, ["Cargo.lock"], "success"),
            False,
            "not open",
        ),
        ("no files is not a merge", decide(bot, [], "success"), False, "changed no files"),
    ]
    failed = []
    for label, decision, want_merge, want in cases:
        good = decision.merge is want_merge and want in decision.reason
        if not good:
            failed.append(label)
        print(f"{'ok  ' if good else 'FAIL'} {label}: {decision.reason}")
    print(f"\nselftest: {len(cases) - len(failed)}/{len(cases)} cases as specified")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
