"""`main`'s one job for a code change must measure the claim it used to print.

`.github/workflows/ci.yml` gates every heavy leg behind `github.event_name != 'push'`, so the only thing
trunk runs after a merge was `main-gate`, and it asserted in an `echo` that the pushed tip's "identical
tree already passed the full CI matrix". The assertion was true on every tip measured — `07f988fa`,
`60f670d0` and `225cf373` each carry a tree whose pull-request head reported `success` for the merge gate
— and that is exactly what made it a hole rather than an incident: an `echo` cannot fail, so the defence
rested on a premise nothing asked. `scripts/check_main_provenance.py` now asks it against the API.

What is pinned here is the wiring, in both directions: the job has to call the script (the `echo` must not
come back as the whole of the job), the check name the script requires has to be the same single name the
rest of the merge path uses, and the shapes the script distinguishes have to keep answering correctly.
"""

from __future__ import annotations

import ast
import importlib.util
import sys
from pathlib import Path

import pytest
import yaml

REPO_ROOT = Path(__file__).resolve().parent.parent
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci.yml"
SCRIPT = REPO_ROOT / "scripts" / "check_main_provenance.py"
VERDICT = REPO_ROOT / "scripts" / "check_matrix_verdict.py"
AUTOMERGE = REPO_ROOT / "scripts" / "dependency_merge_gate.py"


def _load(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def gate():
    return _load(SCRIPT, "check_main_provenance")


@pytest.fixture(scope="module")
def job():
    text = WORKFLOW.read_text(encoding="utf-8")
    document = yaml.safe_load(text)
    return document["jobs"]["main-gate"]


def _candidate(gate, pr=492, sha="1" * 40, tree="a" * 40, verdict="success"):
    return gate.Candidate(pr=pr, sha=sha, tree=tree, verdict=verdict)


def test_the_job_calls_the_script_rather_than_describing_itself(job):
    """A job whose only step is an `echo` is a report; the measurement has to be a command."""
    runs = [step.get("run", "") for step in job["steps"] if "run" in step]
    assert any("check_main_provenance.py" in line for line in runs), runs
    asserted = [line for line in runs if "already passed the full CI matrix" in line]
    assert not asserted, f"the echo is back as an executable statement: {asserted}"


def test_the_job_runs_on_a_push_to_main_and_can_be_asked_by_hand(job):
    """Push-only could not be tested before the merge that first runs it; dispatch is the test channel."""
    assert job["if"] == (
        "${{ (github.event_name == 'push' && github.ref == 'refs/heads/main')"
        " || github.event_name == 'workflow_dispatch' }}"
    )


def test_a_refusal_here_cannot_refuse_a_pull_request():
    """The dispatch channel is only honest if it cannot become a deadlock or a hidden merge gate.

    `tests/test_matrix_verdict_gate.py` keeps `main-gate` out of the fan-in for a different reason (a job
    with no result on a pull request would stall the verdict). Both reasons are load-bearing here, so the
    claim is asserted from this file too: a hand-dispatched refusal is a report to whoever asked, never a
    pull request that cannot be merged.
    """
    jobs = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))["jobs"]
    assert "main-gate" not in jobs["matrix-verdict"]["needs"]


def test_the_job_asks_the_api_for_the_least_it_can_read(job):
    """`pull-requests: read` is the new need; nothing here writes, so nothing here may hold a write token."""
    permissions = job.get("permissions", {})
    assert permissions.get("pull-requests") == "read", permissions
    assert permissions.get("contents") == "read", permissions
    assert not any(scope.endswith("write") for scope in permissions), permissions


def test_the_step_hands_the_gh_cli_a_token(job):
    """A job's `permissions:` mints `GITHUB_TOKEN`; `gh` reads `GH_TOKEN`, and says so by exiting 4.

    Found by dispatching this job rather than reasoning about it: on the runner the call reported
    "To use GitHub CLI in a GitHub Actions workflow, set the GH_TOKEN environment variable", and the
    script answered honestly with exit 2 - "could not ask". Without the dispatch arm that reading would
    have arrived as `main` going red on its first trunk run.
    """
    steps = [step for step in job["steps"] if "check_main_provenance.py" in str(step.get("run", ""))]
    assert steps, f"no provenance step among {job['steps']}"
    env = steps[0].get("env") or {}
    assert env.get("GH_TOKEN"), f"gh has no token to read: {env}"


def test_the_verdict_name_is_one_name_across_the_merge_path(gate):
    """Three files name the required check; a rename that reaches two of them silently unlifts the gate."""
    for source in (VERDICT, AUTOMERGE):
        text = source.read_text(encoding="utf-8")
        assert gate.REQUIRED_CHECK in text, f"{source.name} no longer names {gate.REQUIRED_CHECK!r}"
    assert gate.REQUIRED_CHECK == "Test matrix (all legs)"


def test_a_green_head_on_the_pushed_tip_verifies(gate):
    tree = "a" * 40
    assert gate.decide("0" * 40, tree, (_candidate(gate, tree=tree),)) == []


@pytest.mark.parametrize(
    "name",
    [
        "no pull request at all",
        "tree no pull-request commit carried",
        "matching tree, verdict cancelled",
        "matching tree, verdict never reported",
    ],
)
def test_the_three_unverified_shapes_refuse(gate, name):
    """Each refusal is a different hole: unattributed content, a rebased result that changed, a red gate."""
    shape = next(row for row in gate.SELFTEST if row[0] == name)
    _, tree, candidates, expected = shape
    reasons = gate.decide("0" * 40, tree, candidates)
    assert len(reasons) == expected == 1, reasons
    assert reasons[0], "a refusal has to say which shape it is"


def test_ancestor_trees_are_not_the_tip_and_are_not_judged(gate):
    """#483 landed an eleven-commit rebase; only the pushed tip is the state `main` is in.

    Measured: seven of its ancestors matched pull-request heads whose verdict was `failure`, and the tip
    matched the final head's tree with `success`. So the rule is membership of the associated pull
    request's *set* of commit trees, not equality against its head - and the job is handed `github.sha`,
    the tip. The set rule is what keeps a rebase that drops an empty tail commit green: the tip's tree is
    then an earlier commit's, and only a head-only comparison would refuse it.
    """
    two = next(row for row in gate.SELFTEST if row[0] == "second commit of a two-commit pull request")
    assert gate.decide("0" * 40, two[1], two[2]) == []
    tail = next(row for row in gate.SELFTEST if "dropped an empty tail" in row[0])
    assert gate.decide("0" * 40, tail[1], tail[2]) == []
    assert tail[2][-1].verdict == "never reported", "the matched commit must not be the last one"


def test_one_green_among_two_associated_pull_requests_is_enough(gate):
    """A commit shared by two pull requests is verified if either of them was judged green for it."""
    shape = next(row for row in gate.SELFTEST if row[0] == "one of two associated pull requests green")
    _, tree, candidates, _expected = shape
    assert gate.decide("0" * 40, tree, candidates) == []


def test_the_selftest_covers_the_shapes_without_network(gate, capsys):
    assert gate.selftest() == 0
    out = capsys.readouterr().out
    assert f"{len(gate.SELFTEST)}/{len(gate.SELFTEST)} shapes answered correctly" in out


def test_the_script_stays_loadable_on_the_interpreter_floor_it_runs_on():
    """`main-gate` installs Python 3.8, so this is the claim that has to hold in 3.8's grammar.

    Checked as a parse under `feature_version=(3, 8)` plus the future import that keeps the
    `str | None` and `tuple[...]` annotations ruff asks for from ever being evaluated at runtime - not
    by running the script on this box's interpreter, which would prove nothing about 3.8.
    """
    text = SCRIPT.read_text(encoding="utf-8")
    tree = ast.parse(text, feature_version=(3, 8))
    assert tree.body, "the script does not parse as 3.8"
    assert "from __future__ import annotations" in text


def test_the_newest_attempt_of_the_required_check_decides(gate, monkeypatch):
    """A re-run appends a check-run rather than replacing it, so the last row is the verdict.

    Read the first instead and a tip whose red attempt was later made green keeps being refused - a
    false red on `main`, the one thing a trunk gate may not produce. Stubbed at `_gh`, the single place
    this module speaks to the API, so the ordering rule is tested without a network call.
    """

    def fake_gh(repo, path, *extra):
        if path.endswith("/pulls"):
            return ["500"]
        if path.startswith("pulls/500/commits"):
            return [f"{'1' * 40}\t{'a' * 40}"]
        if "check-runs" in path:
            return ["failure", "success"]
        raise AssertionError(f"unexpected api path {path}")

    monkeypatch.setattr(gate, "_gh", fake_gh)
    candidates = gate.collect("owner/repo", "0" * 40, "a" * 40)
    assert [c.verdict for c in candidates] == ["success"], candidates
    assert gate.decide("0" * 40, "a" * 40, tuple(candidates)) == []
