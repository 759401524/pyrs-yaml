"""The cancelled-leg re-run bot, checked both as a decision and as a wiring contract.

`ci.yml` fans every matrix leg into the one required check, `Test matrix (all legs)`
(`scripts/check_matrix_verdict.py`), and that gate refuses a `cancelled` leg on purpose. The re-run
bot in `.github/workflows/ci-rerun-cancelled.yml` exists for the one shape where the refusal costs a
human a button press rather than catching a defect: a leg cancelled by infra or supersession on a run
whose other legs are green. Two things can go wrong and both are pinned here:

* the decision (`scripts/ci_rerun_gate.py::decide`) re-running a run that should stay red, or refusing
  to re-run the one shape it was written for. The failure this guards is a guard that reads green for
  the wrong reason - so the positive case and every negative case are asserted, not just the happy one;
* the wiring: the workflow must actually invoke the script (or the gate is a file no consumer runs),
  must scope to pull requests (a re-run bot that touches `main` is not what was ruled), and must not
  weaken the verdict's accept-set - the verdict script is required to still refuse a cancelled leg.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
SCRIPT = REPO_ROOT / "scripts" / "ci_rerun_gate.py"
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci-rerun-cancelled.yml"
VERDICT = REPO_ROOT / "scripts" / "check_matrix_verdict.py"


def _load():
    spec = importlib.util.spec_from_file_location("ci_rerun_gate", SCRIPT)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def gate():
    return _load()


# ── the decision, in both directions ────────────────────────────────────────────────

DECISION_CASES = [
    (["success", "success", "cancelled"], 1, True, "the one shape the bot owns"),
    (["success", "cancelled", "cancelled"], 1, True, "several cancelled legs, still one retry"),
    (["success", "failure"], 1, False, "a real failure must stay red"),
    (["success", "failure", "cancelled"], 1, False, "a failure outranks a cancel - never mask it"),
    (["cancelled", "cancelled"], 1, False, "fully cancelled is supersession, not a flake"),
    (["success", "skipped"], 1, False, "a skipped sibling is not a cancellation"),
    (["success", "success"], 1, False, "nothing was cancelled"),
    (["success", "cancelled"], 2, False, "attempt at the cap stops the loop"),
]


@pytest.mark.parametrize("conclusions,attempt,want,why", DECISION_CASES, ids=[c[3] for c in DECISION_CASES])
def test_decide_re_runs_only_a_cancelled_leg_on_a_green_run(gate, conclusions, attempt, want, why):
    decision = gate.decide(conclusions, attempt)
    assert decision.rerun is want, why
    assert decision.reason


def test_the_cap_is_read_from_the_module_not_hardcoded(gate):
    """The attempt bound is one knob; a test that retypes it would pass after the knob moved."""
    assert gate.decide(["success", "cancelled"], gate.MAX_ATTEMPTS).rerun is False
    assert gate.decide(["success", "cancelled"], gate.MAX_ATTEMPTS - 1).rerun is True


# ── the wiring ───────────────────────────────────────────────────────────────────────


def test_the_workflow_invokes_the_gate_and_is_scoped_to_pull_requests():
    text = WORKFLOW.read_text(encoding="utf-8")
    assert "ci_rerun_gate.py" in text, "the decision file is never run, so it is dead prose"
    assert "github.event.workflow_run.event == 'pull_request'" in text, "a re-run bot must not touch main"
    assert "cancel-in-progress: false" in text, "cancelling a re-run mid-flight would leave a half-retried run"
    # The dispatch is in the script, not the workflow - that is the point of a testable decision.
    script = SCRIPT.read_text(encoding="utf-8")
    assert "rerun-failed-jobs" in script, "the run must be re-run, not only reported"
    assert "--attempt" in script, "the attempt cap is the loop guard and has to reach decide()"


def test_the_verdict_still_refuses_a_cancelled_leg():
    """The bot re-runs the flake; it does not teach the merge gate to ignore one. The accept-set is the boundary."""
    verdict = VERDICT.read_text(encoding="utf-8")
    assert 'ACCEPTED = ("success",)' in verdict, "excusing a cancelled leg in the verdict is the wrong lever"
    spec = importlib.util.spec_from_file_location("check_matrix_verdict", VERDICT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    assert module.verdict({"wheel smoke (macos-latest)": {"result": "cancelled"}}, code_changed=True), (
        "a cancelled leg must remain a refusal; only a re-run that turns it green satisfies the merge"
    )
