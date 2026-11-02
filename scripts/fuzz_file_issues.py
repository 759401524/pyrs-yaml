#!/usr/bin/env python3
"""Open one GitHub issue per fuzz target that surfaced crash inputs, and append to it after that.

`.github/workflows/fuzz.yml`'s aggregate job could already say what the sampler found - it writes a
table to `$GITHUB_STEP_SUMMARY` and prints `::warning::<N> crash input(s) collected`. Nothing
consumed it. Measured over the five scheduled runs from 2026-10-03 to 2026-10-31, every one a
failure and every one carrying crash inputs (39 distinct hashes across the four whose archives still
use the `collected/` layout), and every finding reached the fix queue only because a person opened the
run page and read it. This script is the consumer: a red sampler gets an assignable entry point
instead of a wall of red runs.

Why one issue per target rather than one per signature: `scripts/fuzz_rounds.sh` already says a
signature names the assertion that failed, not the root cause - several distinct bugs reach one
`assert_eq!` line, which is why that script calls its signature count a lower bound. Per-signature
issues would overcount root causes and churn their titles as panic text drifts. A per-target issue
stays open until a human says the seed tier replays clean, which is the same closure the pipeline in
`fuzz_rounds.sh` describes.

Why the computed title is also the de-duplication key: it is the only field this script controls end
to end. Signature text comes out of fuzzer output, so it goes into a fenced block in the body and
never into a search query or a shell word.

Usage (the aggregate job runs the first form):
    python3 scripts/fuzz_file_issues.py --crashes crashes --fuzz-result failure \
        --run-url https://github.com/owner/repo/actions/runs/123
    python3 scripts/fuzz_file_issues.py --crashes crashes --dry-run

Exit codes: 0 = nothing to file, or every target with findings now has an issue carrying this run;
1 = a `gh` call failed; 2 = the sampler did not succeed and no artifact arrived, so this run's bytes
are gone and the finding has nowhere to go. 2 is not a pass - it is the one shape where silence would
read as "nothing found".
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections import Counter
from pathlib import Path
from typing import Callable, List, Optional

TITLE = "fuzz: {target} has untriaged crash inputs"

# `upload-artifact` names each archive `fuzz-artifacts-<target>`; the download keeps that name as a
# directory. Sliced rather than `str.removeprefix`, which is 3.9 and this script must import on 3.8.
ARCHIVE_PREFIX = "fuzz-artifacts-"

# Four backticks so a signature carrying a fenced block of its own cannot end the fence early.
FENCE = "````"

# `(gh arguments, stdin body or None) -> stdout`. Injected by the tests so no case needs network.
# `typing.List`/`Optional` rather than `list[str]`/`str | None`: this subscript runs at import time,
# where the builtins form is a `TypeError` on the 3.8 floor.
Runner = Callable[[List[str], Optional[str]], str]


class TargetFindings:
    """One target's inputs and signature counts. Plain attributes, so an `importlib` load works."""

    __slots__ = ("inputs", "signatures", "target")

    def __init__(self, target: str, inputs: list[str], signatures: Counter) -> None:
        self.target = target
        self.inputs = inputs
        self.signatures = signatures


def _signatures(directory: Path) -> Counter:
    """Signature -> number of inputs that reached it, read from every `findings.tsv` below `directory`."""
    counts: Counter = Counter()
    for findings in directory.rglob("findings.tsv"):
        for line in findings.read_text(encoding="utf-8", errors="replace").splitlines():
            signature = line.partition("\t")[0].strip()
            if signature:
                counts[signature] += 1
    return counts


def collect(crashes: Path) -> list[TargetFindings]:
    """Every target that found anything, with its input names and signature counts.

    `merge-multiple: false` on the download gives `crashes/fuzz-artifacts-<target>/`, and each
    archive mirrors `artifacts/<target>/` under `collected/<target>/`, so inputs are counted by file
    name - the same reduction the aggregate step's `sort -u` performs, for the same reason.
    """
    found: list[TargetFindings] = []
    for directory in sorted(p for p in crashes.iterdir() if p.is_dir()):
        inputs = sorted({p.name for p in directory.rglob("crash-*") if p.is_file()})
        if not inputs:
            continue
        name = directory.name
        if name.startswith(ARCHIVE_PREFIX):
            name = name[len(ARCHIVE_PREFIX) :]
        found.append(TargetFindings(name, inputs, _signatures(directory)))
    return found


def build_body(finding: TargetFindings, run_url: str) -> str:
    target, inputs, signatures = finding.target, finding.inputs, finding.signatures
    head = f"The discovery run {run_url} surfaced {len(inputs)} crash input(s) for `{target}`"
    if signatures:
        head += f" across {len(signatures)} distinct signature(s)."
    else:
        head += "; its `findings.tsv` did not arrive, so the signatures are unknown."
    parts = [
        head,
        "",
        "| crash inputs | distinct signatures |",
        "| --- | --- |",
        f"| {len(inputs)} | {len(signatures)} |",
        "",
    ]
    if signatures:
        parts += [
            "Signatures, most frequent first. A signature names the assertion that failed, not the "
            "root cause, so this is a lower bound:",
            "",
            FENCE,
            *[f"{count}x  {signature}" for signature, count in signatures.most_common()],
            FENCE,
            "",
        ]
    parts += [
        f"Inputs, in this run's `fuzz-artifacts-{target}` artifact:",
        "",
        FENCE,
        *inputs,
        FENCE,
        "",
        f"Pipeline: minimize -> regression test in the owning crate -> seed under `fuzz/seeds/{target}/`"
        " -> fix. Close this issue when the seeds replay clean.",
        "",
    ]
    return "\n".join(parts)


def _run_gh(args: list[str], body: str | None) -> str:
    proc = subprocess.run(["gh", *args], input=body, check=True, capture_output=True, text=True)
    return proc.stdout


def find_open_issue(title: str, run_gh: Runner) -> int | None:
    """The number of the open issue titled exactly `title`, if one exists.

    Exact title match rather than a `--search` query: the point is one stable issue per target, and a
    search would also match an issue whose title merely contains this one.
    """
    listed = json.loads(
        run_gh(["issue", "list", "--state", "open", "--limit", "500", "--json", "number,title"], None) or "[]"
    )
    for item in listed:
        if item.get("title") == title:
            return int(item["number"])
    return None


def file_issues(found: list[TargetFindings], *, run_url: str, dry_run: bool, run_gh: Runner = _run_gh) -> int:
    failures = 0
    for finding in found:
        title = TITLE.format(target=finding.target)
        body = build_body(finding, run_url)
        if dry_run:
            print(f"would file: {title}")
            continue
        try:
            number = find_open_issue(title, run_gh)
            if number is None:
                run_gh(["issue", "create", "--title", title, "--body-file", "-"], body)
                print(f"opened: {title}")
            else:
                run_gh(["issue", "comment", str(number), "--body-file", "-"], body)
                print(f"appended to #{number}: {title}")
        except (subprocess.CalledProcessError, ValueError) as exc:
            failures += 1
            print(f"could not file {title}: {exc}", file=sys.stderr)
    return 1 if failures else 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--crashes",
        type=Path,
        default=Path("crashes"),
        help="directory holding the downloaded fuzz-artifacts-* archives",
    )
    parser.add_argument(
        "--fuzz-result",
        default="success",
        help="the fuzz job's own result; anything but 'success' means crash bytes "
        "were expected, so their absence is a finding about the pipeline",
    )
    parser.add_argument("--run-url", default="(run URL not supplied)")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args(argv)

    found = collect(args.crashes) if args.crashes.is_dir() else []
    if not found:
        if args.fuzz_result == "success":
            print("the sampler succeeded and no target collected a crash input; nothing to file")
            return 0
        print(
            f"the sampler result was {args.fuzz_result!r} but no crash artifact arrived, so this run's bytes are gone",
            file=sys.stderr,
        )
        return 2
    return file_issues(found, run_url=args.run_url, dry_run=args.dry_run)


if __name__ == "__main__":
    sys.exit(main())
