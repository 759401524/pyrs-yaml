"""A job that installs an artifact must wait on its producer and on nothing else that is slower.

The wheel the Python legs test is not the wheel the release builds. Measured on run 39424748440 (44
jobs, 856 s from first runner to verdict): the abi3 wheel is uploaded at +134 s (Windows), +135 s
(macOS) and +151 s (ubuntu), while the `--find-interpreter` recipe that shared that job needed +285 s
more on macOS and +346 s on Windows. `build` therefore completed at +485 s, and all 24 legs of the
test matrix and the wheel smoke started 355-471 s *after* the only artifact they read already existed.

That is a graph fact, so it is guarded as one: the families are derived from each job's own
`upload-artifact` and `download-artifact` inputs rather than from job names, which means the rule
survives a rename and dies on a re-merge. Re-merging is the regression this file refuses, because it
is invisible in the diff - both steps stay, only the queue time moves.
"""

from __future__ import annotations

import re
from pathlib import Path

import pytest
import yaml

REPO_ROOT = Path(__file__).resolve().parent.parent
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci.yml"

FAMILIES = {
    "wheel-": "test wheel",
    "wheels-": "release recipe",
}


def family(artifact: str) -> str:
    """Which consumer family an artifact name belongs to, or ``""`` for anything else.

    Prefix matching on the name the workflow itself writes: `wheel-<os>` is what the suite legs
    install and `wheels-<os>` is the release-recipe family read by the compliance report.
    """
    for prefix, label in FAMILIES.items():
        if artifact.startswith(prefix):
            return label
    return ""


def steps(job: dict) -> list[dict]:
    return list(job.get("steps") or [])


def needed(job: dict) -> set[str]:
    needs = job.get("needs")
    if needs is None:
        return set()
    return {needs} if isinstance(needs, str) else set(needs)


def artifacts(job: dict, action: str) -> set[str]:
    """Artifact names this job's `action` steps name, following `with:` rather than step titles."""
    found = set()
    for step in steps(job):
        if str(step.get("uses", "")).startswith(action):
            found.add(str((step.get("with") or {}).get("name", "")))
    return {name for name in found if family(name)}


def producers(document: dict) -> dict[str, set[str]]:
    """Each job's uploaded families - only jobs that upload a wheel are part of this graph."""
    out = {}
    for name, job in (document.get("jobs") or {}).items():
        families = {family(a) for a in artifacts(job, "actions/upload-artifact")}
        if families:
            out[name] = families
    return out


def wiring_problems(document: dict) -> list[str]:
    """Every way this workflow's wheel graph can make a leg wait for work it does not consume."""
    jobs = document.get("jobs") or {}
    uploads = producers(document)
    problems = []
    for name, families in sorted(uploads.items()):
        if len(families) > 1:
            problems.append(
                f"{name}: uploads {sorted(families)} in one job, so every consumer of the faster "
                f"family waits for the slower build too"
            )
    for name, job in sorted(jobs.items()):
        wanted = {family(a) for a in artifacts(job, "actions/download-artifact")}
        if not wanted:
            continue
        waited = {other for other in needed(job) if other in uploads}
        have = set().union(*[uploads[other] for other in waited]) if waited else set()
        for label in sorted(wanted - have):
            problems.append(f"{name}: installs the {label} without waiting on a job that uploads it")
        for other in sorted(waited - {n for n in uploads if uploads[n] & wanted}):
            problems.append(f"{name}: waits on {other}, which uploads none of the artifacts this job installs")
    return problems


def test_the_declared_wheels_have_exactly_two_producers_and_each_uploads_one_family() -> None:
    document = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
    uploads = producers(document)
    assert uploads, "no wheel-uploading job found: the probe for them stopped matching ci.yml"
    assert all(len(families) == 1 for families in uploads.values()), uploads
    families = sorted({next(iter(f)) for f in uploads.values()})
    assert families == sorted(FAMILIES.values()), families


def test_the_shipped_workflow_waits_only_on_the_wheels_it_installs() -> None:
    document = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
    assert wiring_problems(document) == []


def test_the_producer_of_both_families_is_named_as_the_hole() -> None:
    """The control: the shape run 39424748440 actually had must be reported, not waved through.

    One job uploading both families is the pre-split `build`, and a consumer that reads only the fast
    one still needs it; a guard that passes on that graph guards nothing.
    """
    merged = {
        "jobs": {
            "build": {
                "steps": [
                    {"uses": "actions/upload-artifact@v7", "with": {"name": "wheel-ubuntu-latest"}},
                    {"uses": "actions/upload-artifact@v7", "with": {"name": "wheels-ubuntu-latest"}},
                ],
            },
            "test": {
                "needs": ["build"],
                "steps": [{"uses": "actions/download-artifact@v8", "with": {"name": "wheel-ubuntu-latest"}}],
            },
        }
    }
    problems = wiring_problems(merged)
    assert len(problems) == 1, problems
    assert problems[0].startswith("build: uploads"), problems


@pytest.mark.parametrize("needs", [["build"], "build"])
def test_a_consumer_that_waits_on_the_wrong_producer_is_named(needs) -> None:
    """The other direction: `needs` pointing at the job whose artifact this leg never reads."""
    document = {
        "jobs": {
            "build": {
                "steps": [{"uses": "actions/upload-artifact@v7", "with": {"name": "wheel-ubuntu-latest"}}],
            },
            "release-wheels": {
                "steps": [{"uses": "actions/upload-artifact@v7", "with": {"name": "wheels-ubuntu-latest"}}],
            },
            "compliance-report": {
                "needs": needs,
                "steps": [{"uses": "actions/download-artifact@v8", "with": {"name": "wheels-ubuntu-latest"}}],
            },
        }
    }
    problems = wiring_problems(document)
    assert len(problems) == 2, problems
    assert re.match(r"compliance-report: installs the release recipe", problems[0]), problems
    assert "waits on build" in problems[1], problems
