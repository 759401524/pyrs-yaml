"""Every CI job that runs the whole Python suite must fetch every corpus that suite reads.

The corpora live in `Reference/`, which is untracked, and the tests that read them `skipif` the
directory is absent. That combination has a specific failure mode which is easy to miss and expensive
to keep: a job that never clones a corpus reports a green run, and the run says nothing about
conformance. It was measured, not imagined - `coverage` ran `pytest tests/` with no clone step at all
while `test` and `test-freethreaded` had three, so three conformance suites skipped there and the job
still passed.

The rule is derived rather than listed: the corpora come from whatever `tests/*.py` references under
`Reference/`, and the jobs come from whatever runs the whole suite. Adding a corpus-reading test
without teaching CI to fetch it reddens this file, which is the only moment the omission is cheap to
fix. The clones now live in one composite action shared by the whole-suite jobs, so the derivation
follows a local `uses:` into that action's steps - otherwise the shared form of the fix would itself
be the hole this file was written to close.
"""

from __future__ import annotations

import re
from pathlib import Path

import yaml

REPO_ROOT = Path(__file__).resolve().parent.parent

CORPUS_RE = re.compile(r"Reference/([A-Za-z0-9][A-Za-z0-9._-]*)")
WHOLE_SUITE_RE = re.compile(r"pytest tests/?([\s$]|$)")


def corpus_names() -> set[str]:
    """Directory names the Python tests expect under `Reference/`."""
    found: set[str] = set()
    for test in sorted((REPO_ROOT / "tests").glob("test_*.py")):
        found.update(CORPUS_RE.findall(test.read_text(encoding="utf-8")))
    return found


def _step_names(step: dict) -> list[str]:
    """The names a step contributes to the job's record, following a local action to its own steps.

    A composite action is the shared form of the three clone steps, and `uses:` hides them: read
    literally, a job that fetches every corpus through one looks identical to a job that fetches
    nothing. Local paths (`./...`) are the only ones resolved - a third-party action's internals are
    not this repository's to parse.
    """
    name = str(step.get("name", ""))
    uses = str(step.get("uses", ""))
    if not uses.startswith("./"):
        return [name]
    # Slicing rather than `removeprefix`, which is 3.9: this file runs on the 3.8 leg of the matrix.
    action = REPO_ROOT / uses[2:] / "action.yml"
    if not action.is_file():
        return [name, uses]  # an unresolvable path is reported as itself, never as nothing
    document = yaml.safe_load(action.read_text(encoding="utf-8"))
    inner = [str(sub.get("name", "")) for sub in (document.get("runs", {}).get("steps") or [])]
    return [name, *inner]


def suite_jobs(workflow: str) -> list[tuple[str, list[str]]]:
    """`(job name, step names)` for every job in the workflow that runs the whole `tests/` suite."""
    text = (REPO_ROOT / ".github/workflows" / workflow).read_text(encoding="utf-8")
    document = yaml.safe_load(text)
    jobs: list[tuple[str, list[str]]] = []
    for name, job in (document.get("jobs") or {}).items():
        runs_whole_suite = any(WHOLE_SUITE_RE.search(str(step.get("run", ""))) for step in job.get("steps", []))
        if runs_whole_suite:
            names: list[str] = []
            for step in job.get("steps", []):
                names.extend(_step_names(step))
            jobs.append((name, names))
    return jobs


def _key(text: str) -> str:
    """Comparison key: case-folded and stripped of separators, so `yaml-test-suite`,
    `YAML Test Suite` and `JSONTestSuite` all line up regardless of how a step is worded.
    """
    return re.sub(r"[^a-z0-9]", "", text.lower())


def missing_clones(step_names: list[str], corpora: set[str]) -> list[str]:
    """Corpora a job runs tests for but never downloads.

    Kept as a pure function so the negative control can delete a step and watch the report change,
    without touching the committed workflow.
    """
    haystack = _key(" ".join(step_names))
    return sorted(name for name in corpora if _key(name) not in haystack)


def test_the_workflow_declares_whole_suite_jobs_at_all() -> None:
    """Guard against the check becoming vacuous by renaming or dropping jobs."""
    assert corpus_names(), "no test references Reference/ - the corpus suites vanished, re-check this"
    jobs = suite_jobs("ci.yml")
    assert len(jobs) >= 2, f"only {len(jobs)} job(s) run the whole suite; this gate is watching nothing"


def test_a_local_action_step_reports_the_steps_it_hides() -> None:
    """Following `uses:` is load-bearing, not decoration.

    Without the resolution, a job that fetches every corpus through the shared action looks exactly
    like one that fetches nothing - so the shared form of the fix would re-open the hole this file
    closes. The check here is that the action's own step names reach the job's record.
    """
    names = _step_names({"name": "Fetch conformance corpora", "uses": "./.github/actions/fetch-corpora"})
    corpora = corpus_names()
    assert missing_clones(names, corpora) == [], f"{names} does not account for {sorted(corpora)}"
    # And a job that really has nothing is still caught: an unresolvable path contributes only
    # itself, so every corpus comes back missing rather than the job reading as fully covered.
    broken = _step_names({"name": "Fetch conformance corpora", "uses": "./.github/actions/renamed-away"})
    assert missing_clones(broken, corpora) == sorted(corpora), broken


def test_every_whole_suite_job_fetches_every_corpus() -> None:
    """The claim itself: no job runs the suite over a corpus it did not download."""
    corpora = corpus_names()
    problems = [
        f"{name}: no step downloads {corpus}"
        for name, steps in suite_jobs("ci.yml")
        for corpus in missing_clones(steps, corpora)
    ]
    assert not problems, "corpus-blind jobs in ci.yml:\n  " + "\n  ".join(problems)


def test_the_report_names_a_corpus_when_a_download_step_is_removed() -> None:
    """The control: a check that cannot fail is not a check.

    Takes a real whole-suite job's steps, drops the one that downloads a corpus, and requires
    `missing_clones` to name it. Without this, a matcher that always returned "nothing missing" would
    pass the test above on every workflow, including one with no download step at all.
    """
    corpora = corpus_names()
    jobs = suite_jobs("ci.yml")
    assert jobs, "no whole-suite job found to test against"
    victims_reported: list[str] = []
    for name, steps in jobs:
        complete = missing_clones(steps, corpora)
        assert not complete, f"{name} is already missing {complete}"
        downloads = [step for step in steps if any(_key(corpus) in _key(step) for corpus in corpora)]
        assert downloads, f"{name} has no download step to remove"
        for victim in downloads:
            flagged = missing_clones([step for step in steps if step != victim], corpora)
            if not flagged:
                continue
            victims_reported.append(f"{name}: -{victim} -> {flagged}")
            break
        assert victims_reported, "no single removal changed the report in any job"
    assert victims_reported, (
        "removing a corpus download step never changes the report - the matcher cannot see a gap and "
        "the gate above is decoration"
    )
