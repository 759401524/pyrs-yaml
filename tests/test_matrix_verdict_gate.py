"""Self-test for the matrix fan-in (`scripts/check_matrix_verdict.py`).

The point of the fan-in is that one check name can require twenty-one legs, so the thing that must not
be soft is the verdict itself. Each case below is a shape a real run produces:

* every leg green - the only pass;
* one leg `failure` - what a red matrix row looks like;
* one leg `skipped` - what the others become when a dependency of theirs died, which is precisely the
  case a naive "no failures reported" check waves through;
* one leg `cancelled` - what `concurrency: cancel-in-progress` leaves when a newer push supersedes the
  run, i.e. the leg never spoke at all;
* an empty object - a fan-in whose `needs:` list was edited away would then green-light anything.

Wiring is asserted too: a verdict no job runs is documentation, and a job whose `needs:` stops
matching the matrix is the same hole re-opened quietly.
"""

from __future__ import annotations

import importlib.util
import json
import re
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
GATE_PATH = REPO_ROOT / "scripts" / "check_matrix_verdict.py"
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci.yml"


def _load():
    spec = importlib.util.spec_from_file_location("check_matrix_verdict", GATE_PATH)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def gate():
    return _load()


def jobs_block(text: str) -> str:
    """Everything under the top-level `jobs:` key.

    Needed because the job-name pattern also matches the keys of `on:` (`push:`, `schedule:`,
    `workflow_dispatch:`) two spaces deep, and a test that read `push` as a job demanded a check
    named `push` in the fan-in. That is how this file's own baseline went red.
    """
    return text.split("\njobs:\n", 1)[1]


def job_names(text: str) -> set[str]:
    return set(re.findall(r"^  ([a-z][a-z0-9-]*):\n", jobs_block(text), re.M))


def job_body(text: str, name: str) -> str:
    """One job's own mapping, from its heading to the next job's heading."""
    match = re.search(rf"^  {name}:\n((?:.*\n)*?)(?=^  [a-z][a-z0-9-]*:\n|\Z)", jobs_block(text), re.M)
    assert match, f"no job named {name}"
    return match.group(1)


def fan_in_needs(text: str) -> set[str]:
    body = job_body(text, "matrix-verdict")
    match = re.search(r"^\s*needs: \[([^\]]+)\]", body, re.M)
    assert match, "the fan-in declares no needs"
    return {name.strip() for name in match.group(1).split(",")}


def fan_in_body(text: str) -> str:
    """The `matrix-verdict` job's own text, from its heading to the next job's."""
    return job_body(text, "matrix-verdict")


def leg(result):
    return {"result": result, "outputs": {}, "needs": []}


# ── the verdict ───────────────────────────────────────────────────────────────────


def test_all_success_is_the_only_green(gate):
    needs = {"test (ubuntu-latest, 3.8)": leg("success"), "coverage": leg("success")}
    assert gate.verdict(needs) == []


@pytest.mark.parametrize("result", ["failure", "skipped", "cancelled", "neutral", None])
def test_anything_else_is_red(gate, result):
    needs = {"a": leg("success"), "b": leg(result)}
    problems = gate.verdict(needs)
    assert problems, f"{result!r} read as a pass"
    assert "b" in problems[0], problems


def test_a_silently_empty_fan_in_is_red(gate):
    assert gate.verdict({}) == ["no jobs were reported: the fan-in lists nothing to wait for"]


def test_cli_exit_codes(gate):
    assert gate.main(["x", "--json", json.dumps({"a": leg("success")})]) == 0
    assert gate.main(["x", "--json", json.dumps({"a": leg("failure")})]) == 1
    assert gate.main(["x", "--json", "not json"]) == 1
    assert gate.main(["x", "--json", "[]"]) == 1, "a list is not the needs object"


def test_missing_environment_is_refused(gate, monkeypatch):
    monkeypatch.delenv("NEEDS", raising=False)
    assert gate.main(["x"]) == 1, "an unread verdict must not default to green"


# ── the wiring ────────────────────────────────────────────────────────────────────


def test_the_fan_in_job_exists_and_waits_on_the_test_surface():
    text = WORKFLOW.read_text(encoding="utf-8")
    assert "name: Test matrix (all legs)" in text, "no single check to make required"
    job = fan_in_body(text)
    needs = re.search(r"^\s*needs: \[([^\]]+)\]", job, re.M)
    assert needs, "the fan-in declares no needs"
    waited = {name.strip() for name in needs.group(1).split(",")}
    assert {"test", "test-freethreaded", "coverage"} <= waited, waited
    assert "toJSON(needs)" in job, "the verdict is not fed the legs' results"
    assert "check_matrix_verdict.py" in job, "the job does not run the checker"


def test_the_fan_in_waits_on_every_job_in_the_workflow():
    """The list cannot quietly fall behind the file.

    A fan-in over three of eleven names lets a red `clippy`, `no_std`, MSRV or property-tier merge just
    as easily as the matrix leg that motivated it - so the rule is exact, not a sample: every job in
    `ci.yml` must be in `needs`, except the verdict itself and `main-gate`, which deliberately runs only
    on `push` to keep the default-branch history and badge current and would otherwise never have a
    result to report on a pull request.
    """
    text = WORKFLOW.read_text(encoding="utf-8")
    waited = fan_in_needs(text)
    expected = job_names(text) - {"matrix-verdict", "main-gate"}
    assert expected, "no jobs found: the heading pattern stopped matching ci.yml"
    assert waited == expected, (
        f"not covered by the fan-in: {sorted(expected - waited)}; unknown jobs listed: {sorted(waited - expected)}"
    )
    # `skipped` is a refusal, so a job that does not run on a pull request would make the verdict
    # permanently red: every job waited on must be one that does.
    for name in sorted(waited):
        condition = re.search(r"if: \$\{\{ (.+?) \}\}", job_body(text, name))
        assert condition is None or "push" in condition.group(1) or "pull_request" in condition.group(1), (
            f"{name}: conditional on an event outside pull_request, so its absence reads as `skipped`"
        )


def test_the_verdict_only_runs_where_a_merge_decision_exists():
    """`workflow_dispatch` has no merge to gate; running red there would train people to ignore it."""
    job = fan_in_body(WORKFLOW.read_text(encoding="utf-8"))
    assert re.search(r"if: \$\{\{ always\(\) && github.event_name == 'pull_request' \}\}", job), (
        "the verdict must run on pull requests only, and must survive a red leg (`always()`)"
    )


def test_every_matrix_leg_is_reachable_from_the_fan_in():
    """A job that produces check names (a `strategy:` matrix) must be waited on.

    Bounded to each job's own body: the first version of this search let `(?:.*\n)*?` run past a job
    boundary, so every job in the file looked like a matrix producer and the assertion could only pass
    because of an `or` escape clause that hid it.
    """
    text = WORKFLOW.read_text(encoding="utf-8")
    producing = {name for name in job_names(text) if "    strategy:" in job_body(text, name)}
    assert producing, "no matrix jobs found: the probe for them stopped matching"
    waited = fan_in_needs(text)
    assert producing <= waited, f"matrix producers outside the fan-in: {sorted(producing - waited)}"


def test_the_checker_stays_loadable_on_the_supported_floor(gate):
    """The fan-in runs on 3.8; annotations that 3.8 evaluates at import time would break the gate."""
    text = GATE_PATH.read_text(encoding="utf-8")
    assert "from __future__ import annotations" in text
    assert "removeprefix" not in text and "removesuffix" not in text
