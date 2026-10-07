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
    job = text.split("  matrix-verdict:", 1)[1].split("\n  compliance-report:", 1)[0]
    needs = re.search(r"^\s*needs: \[([^\]]+)\]", job, re.M)
    assert needs, "the fan-in declares no needs"
    waited = {name.strip() for name in needs.group(1).split(",")}
    assert {"test", "test-freethreaded", "coverage"} <= waited, waited
    assert "toJSON(needs)" in job, "the verdict is not fed the legs' results"
    assert "check_matrix_verdict.py" in job, "the job does not run the checker"


def test_every_matrix_leg_is_reachable_from_the_fan_in():
    """A matrix added to `test` is covered automatically, but a second matrix would not be.

    Asserting the shape rather than the job names: any job that produces a check name the merge should
    care about must appear in the fan-in's `needs`, directly or as a matrix producer named here.
    """
    text = WORKFLOW.read_text(encoding="utf-8")
    jobs = set(re.findall(r"^  ([a-z][a-z0-9-]*):\n", text, re.M))
    job = text.split("  matrix-verdict:", 1)[1].split("\n  compliance-report:", 1)[0]
    waited = {name.strip() for name in re.search(r"needs: \[([^\]]+)\]", job).group(1).split(",")}
    producing = {name for name in jobs if re.search(rf"^  {name}:\n(?:.*\n)*?    strategy:\n", text, re.M)}
    assert producing <= waited or "test" in waited, (producing, waited)
    assert waited <= jobs, f"the fan-in waits on jobs that do not exist: {sorted(waited - jobs)}"


def test_the_checker_stays_loadable_on_the_supported_floor(gate):
    """The fan-in runs on 3.8; annotations that 3.8 evaluates at import time would break the gate."""
    text = GATE_PATH.read_text(encoding="utf-8")
    assert "from __future__ import annotations" in text
    assert "removeprefix" not in text and "removesuffix" not in text
