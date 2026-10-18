#!/usr/bin/env python3
"""Fail unless the content this push put on `main` is content the merge gate judged green.

`main` skips the heavy CI legs on purpose: every one of them is gated behind
`github.event_name != 'push'`, and the single job that runs instead - `main-gate` - said so in an
`echo`: "push sha … is the rebased tip whose identical tree already passed the full CI matrix as a
pull request; skipping the duplicate spend." The claim was true on the tree this was written against.
Measured: push `07f988fa`'s tree is `7b24405da569974fa71c427322627296b1d0ed8e`, the same object pull
request #492's head carried; and #487 landed two commits on `main` whose trees (`65467f0ba321…`,
`15acab960233…`) map one-to-one onto its two pull-request commit trees. What made it a hole was not
the claim but that nothing asked it - an `echo` cannot fail, so the only trunk job a code change
receives had converted the defence's strongest premise into prose. That is the shape
`QUALITY_MATRIX.md` already names elsewhere: a check that no decision consumes is a report, not a gate.

So this asks the three questions the sentence contains, cheapest first:

1. **Which pull request produced this commit?** `GET /commits/{sha}/pulls` answers it, and the answer
   can be *none* - a direct push to the default branch, a merge commit, or content the API cannot
   attribute - which is exactly the case the echo used to print "already verified" about.
2. **Is this commit's tree one of that pull request's commit trees?** A rebase-merge preserves trees, so
   identity is checkable rather than assumed; a conflicted or hand-edited rebase is what breaks it.
3. **Did the merge gate report `success` for the commit that carries that tree?** Identical content is a
   weaker claim than verified content: the tree can match a head whose verdict was cancelled, skipped,
   or never reported. `Test matrix (all legs)` is the one name branch protection requires, so it is the
   one conclusion that settles the question, and `scripts/check_matrix_verdict.py` is what produced it.

The question is asked of the pushed tip, which is what a `push` event carries. That distinction was
measured rather than assumed: replaying this checker over `main`'s last 18 commits refused eight of them,
and all eight turned out to be ancestors of a push tip - #483 landed a rebase of an eleven-commit branch,
and the intermediate heads it passed through carried `failure` or no verdict at the time, while the tip
`225cf373` carries the final head `c6b9e41e`'s tree and its `success`. Judging ancestors would report
reds about content that was never the state `main` was in; the tip is that state, and it verified in every
case measured here (`07f988fa`, `60f670d0`, `225cf373`).

Usage:
    python scripts/check_main_provenance.py --sha <push sha> [--repo owner/name]
    python scripts/check_main_provenance.py --selftest              # no network

Exit codes match the other harnesses: 0 the trunk tip is verified content, 1 it is not (a human decides
what an unverified `main` owes), 2 the question could not be asked (a failed or truncated `gh` call).
A refusal is not a flaky API - it means `main` moved without the gate that blocks that move having been
consulted on this exact content.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from typing import NamedTuple

#: The single check name branch protection requires (`scripts/check_matrix_verdict.py`,
#: `scripts/dependency_merge_gate.py`), so the same name is the whole contract here.
REQUIRED_CHECK = "Test matrix (all legs)"


class GateError(Exception):
    """The question could not be asked - which is not the same as asking it and being refused."""


class Candidate(NamedTuple):
    """One commit of an associated pull request, with the verdict its head carried."""

    pr: int
    sha: str
    tree: str
    verdict: str | None


def decide(push_sha: str, push_tree: str, candidates: tuple[Candidate, ...]) -> list:
    """Every reason this push must not be called verified content, as one-line messages.

    Args:
        push_sha: the commit `main` now points at.
        push_tree: the tree object that commit carries.
        candidates: the pull-request commits associated with that push, each with its tree and the
            conclusion the merge gate reached for it (`None` when it never reported).

    """
    if not candidates:
        return [
            f"push {push_sha[:8]} has no associated pull request: main received content "
            "that no merge decision was ever asked about"
        ]
    matched = [c for c in candidates if c.tree == push_tree]
    if not matched:
        seen = ", ".join(sorted({f"#{c.pr}" for c in candidates}))
        return [
            f"push {push_sha[:8]} tree {push_tree[:12]} is not the tree of any commit of the "
            f"associated pull request(s) ({seen}): the rebased result differs from what CI checked"
        ]
    if any(c.verdict == "success" for c in matched):
        return []
    return [
        f"push {push_sha[:8]} tree {push_tree[:12]} is pull request #{c.pr}'s commit {c.sha[:8]}, "
        f"whose `{REQUIRED_CHECK}` verdict is {c.verdict or 'never reported'}: the content matches, "
        "the green does not"
        for c in sorted(matched, key=lambda c: (c.pr, c.sha))
    ]


def _gh(repo: str, path: str, *extra: str) -> list:
    """One `gh api` call, its stdout as lines; a nonzero exit after both tries is a question not asked.

    Two attempts rather than one because measured on this project's host the call does fail
    intermittently - `check-runs` for `5e222b3f` died with a TLS handshake timeout on the first try and
    answered in full on the second, and this job's whole job is to distinguish "refused" from "unable to
    ask" rather than let either look like the other.
    """
    cmd = ["gh", "api", "--paginate", f"repos/{repo}/{path}", *extra]
    error = ""
    for _ in range(2):
        proc = subprocess.run(cmd, capture_output=True, text=True)
        if proc.returncode == 0:
            return [line for line in proc.stdout.splitlines() if line.strip()]
        error = proc.stderr.strip()
    raise GateError(f"{' '.join(cmd[2:4])} exited {proc.returncode}: {error}")


def collect(repo: str, push_sha: str, push_tree: str) -> list:
    """The associated pull requests' commits, with the merge gate's conclusion for the matching ones."""
    candidates = []
    for number in _gh(repo, f"commits/{push_sha}/pulls", "--jq", ".[] | .number"):
        pr = int(number)
        rows = _gh(
            repo,
            f"pulls/{pr}/commits",
            "--jq",
            ".[] | [.sha, .commit.tree.sha] | @tsv",
        )
        for row in rows:
            sha, _, tree = row.partition("\t")
            verdict = None
            if tree == push_tree:
                conclusions = _gh(
                    repo,
                    f"commits/{sha}/check-runs",
                    "--jq",
                    f'.check_runs[] | select(.name == "{REQUIRED_CHECK}") | .conclusion // "never reported"',
                )
                # The newest attempt decides, same convention as `scripts/dependency_merge_gate.py`:
                # a re-run appends rather than replaces, so reading the first entry would let an
                # attempt that has since been made green keep refusing a tip that is verified.
                verdict = conclusions[-1] if conclusions else "never reported"
            candidates.append(Candidate(pr=pr, sha=sha, tree=tree, verdict=verdict))
    return candidates


def _push_tree(repo: str, push_sha: str) -> str:
    lines = _gh(repo, f"git/commits/{push_sha}", "--jq", ".tree.sha")
    if not lines:
        raise GateError(f"git/commits/{push_sha} returned no tree")
    return lines[0]


#: The shapes the three questions must sort out. Each entry is (name, push tree, candidates,
#: how many refusals `decide` owes). #487's two-commit landing is the reason the matched-tree rule
#: is a set membership rather than an equality against the head; the `cancelled` row is the reason
#: identity alone is not enough.
SELFTEST = (
    ("tip of a rebased single-commit pull request", "a" * 40, (Candidate(492, "1" * 40, "a" * 40, "success"),), 0),
    (
        "second commit of a two-commit pull request",
        "b" * 40,
        (
            Candidate(487, "1" * 40, "a" * 40, "success"),
            Candidate(487, "2" * 40, "b" * 40, "success"),
        ),
        0,
    ),
    (
        "tip whose tree is an earlier commit because the rebase dropped an empty tail",
        "a" * 40,
        (
            Candidate(500, "1" * 40, "a" * 40, "success"),
            Candidate(500, "2" * 40, "b" * 40, "never reported"),
        ),
        0,
    ),
    ("no pull request at all", "a" * 40, (), 1),
    ("tree no pull-request commit carried", "c" * 40, (Candidate(500, "1" * 40, "a" * 40, "success"),), 1),
    ("matching tree, verdict cancelled", "a" * 40, (Candidate(500, "1" * 40, "a" * 40, "cancelled"),), 1),
    ("matching tree, verdict never reported", "a" * 40, (Candidate(500, "1" * 40, "a" * 40, None),), 1),
    (
        "one of two associated pull requests green",
        "a" * 40,
        (
            Candidate(500, "1" * 40, "a" * 40, "skipped"),
            Candidate(501, "2" * 40, "a" * 40, "success"),
        ),
        0,
    ),
)


def selftest() -> int:
    """Exercise `decide` with no network, and fail if a shape answers the wrong way."""
    failures = 0
    for name, tree, candidates, expected in SELFTEST:
        reasons = decide(f"{'0' * 40}", tree, candidates)
        ok = len(reasons) == expected
        print(f"{'ok  ' if ok else 'FAIL'} {name}: {len(reasons)} refusal(s), expected {expected}")
        for line in reasons:
            print(f"     {line}")
        failures += 0 if ok else 1
    print(f"{len(SELFTEST) - failures}/{len(SELFTEST)} shapes answered correctly")
    return 1 if failures else 0


def main(argv: list) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--sha", default=os.environ.get("GITHUB_SHA"), help="the pushed commit")
    parser.add_argument(
        "--repo",
        default=os.environ.get("GITHUB_REPOSITORY"),
        help="owner/name, defaults to $GITHUB_REPOSITORY",
    )
    parser.add_argument("--selftest", action="store_true", help="exercise decide() with no network")
    args = parser.parse_args(argv)

    if args.selftest:
        return selftest()
    if not args.sha or not args.repo:
        print("neither --sha/--repo nor $GITHUB_SHA/$GITHUB_REPOSITORY is set", file=sys.stderr)
        return 2
    try:
        tree = _push_tree(args.repo, args.sha)
        candidates = tuple(collect(args.repo, args.sha, tree))
    except (GateError, ValueError, json.JSONDecodeError) as error:
        print(f"cannot ask whether {args.sha[:8]} was verified: {error}", file=sys.stderr)
        return 2

    reasons = decide(args.sha, tree, candidates)
    if reasons:
        for line in reasons:
            print(line, file=sys.stderr)
        print(
            "\n`main` moved to content the merge gate did not judge. Either re-run the matrix on this "
            "sha, or revert - the echo that used to live here called this verified without asking.",
            file=sys.stderr,
        )
        return 1
    print(f"push {args.sha[:8]} tree {tree[:12]} was verified by `{REQUIRED_CHECK}` on the pull request")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
