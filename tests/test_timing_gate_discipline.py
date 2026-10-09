"""Gate the *methodology* of the in-process timing floors, not just their verdicts.

Three leaderboard files asserted "this path does less work" from wall-clock, each with a different
estimator, and none of them sampled the two phases under the same conditions: the TOML parse gate ran
every candidate block before every reference block, the TOML serialize gate and both JSON gates took
one block per side. A block is only internally comparable, so a scheduler spike on one phase decided
the outcome - which is how a step measured locally at 2.2x showed up as 336us vs 369us (1.10x) on a
macos-latest runner, and how a red arrived with two numbers in the message and no way to tell a
measurement artefact from a regression.

Fixing the files is half of it. The other half is that nothing said *why*: a convention written in a
docstring is a convention someone deletes under pressure, so `scripts/quality_matrix.py` now measures
any ordinary-CI wall-clock gate that bypasses the shared sampler (`timing-floor-unpaired`), and these
tests fire that measurement on injected text. They also pin the sampler itself: phases alternate
inside a block, the verdict is the block minima with a two-win floor, and the macOS log that refuted
the stricter majority rule this module first shipped with is replayed as a passing case rather than
argued about.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
SCRIPT_PATH = REPO_ROOT / "scripts" / "quality_matrix.py"

sys.path.insert(0, str(REPO_ROOT))

from tests.timing import Paired, compare, majority, median_us  # noqa: E402


def _load_matrix():
    spec = importlib.util.spec_from_file_location("quality_matrix", SCRIPT_PATH)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def matrix():
    return _load_matrix()


# ── the sampler ─────────────────────────────────────────────────────────────────


def test_the_sampler_alternates_phases_within_each_block():
    """The claim the old comment made and the code did not do: A and B are measured adjacently.

    Asserted as the number of phase switches, because each phase is called once for the discarded
    warm-up plus `reps` times. Three sequential blocks of the old design produce exactly one switch
    (all candidate calls, then all reference calls); three alternating blocks produce five. If
    `compare` is ever folded back into phases, the count drops and this fails.
    """
    order = []
    blocks = 3
    result = compare(
        lambda: order.append("candidate"),
        lambda: order.append("reference"),
        blocks=blocks,
        reps=1,
    )
    switches = sum(1 for left, right in zip(order, order[1:]) if left != right)
    assert switches == 2 * blocks - 1, order
    assert order.count("candidate") == order.count("reference") == 2 * blocks, order
    assert result.blocks == blocks


def test_the_reference_can_be_sampled_fewer_times_than_the_candidate():
    """Cross-library peers are milliseconds slow; repetition count is not what cancels drift."""
    counts = {"ours": 0, "peer": 0}

    def ours():
        counts["ours"] += 1

    def peer():
        counts["peer"] += 1

    compare(ours, peer, blocks=3, reps=9, reference_reps=2)
    # Each side also gets one discarded warm-up call per block.
    assert counts["ours"] == 3 * (9 + 1), counts
    assert counts["peer"] == 3 * (2 + 1), counts


def test_the_macos_log_that_broke_the_first_rule_now_passes():
    """Replays the numbers a CI job printed, because the argument is that log, not a preference.

    The first verdict rule demanded four of five pair wins. This run produced three while the document
    was 2.58x cheaper than its baseline, because bursts of roughly three times the floor landed on
    three of the five candidate blocks. Per-pair signs are not stable when a disturbance outlives a
    pair; the minima are, because both sides are sampled in every block.
    """
    ci_log = Paired([(139.4, 1001.3), (145.9, 385.5), (405.8, 359.6), (379.1, 1006.8), (437.2, 375.8)])
    assert ci_log.candidate_wins == 3, ci_log.pairs
    assert ci_log.ratio == pytest.approx(359.6 / 139.4, rel=1e-6), ci_log.verdict("replayed")
    assert majority(ci_log), ci_log.verdict("replayed macos run")


def test_one_lucky_block_does_not_carry_a_verdict():
    """The floor still bites: minima alone would accept a single undisturbed pair and four losses."""
    lucky = Paired([(10.0, 400.0), (410.0, 405.0), (420.0, 415.0), (430.0, 425.0), (440.0, 435.0)])
    assert lucky.candidate_us < lucky.reference_us, lucky.pairs
    assert lucky.candidate_wins == 1, lucky.pairs
    assert not majority(lucky), lucky.verdict("one lucky block")


def test_a_real_regression_is_still_red():
    """A candidate that is actually slower fails, whatever the estimator's tolerance is.

    Without this, "the verdict is a minimum" is indistinguishable from a gate that cannot go red.
    """
    inverted = Paired([(90.0, 30.0), (95.0, 31.0), (88.0, 29.0), (92.0, 30.5), (91.0, 30.2)])
    assert not majority(inverted), inverted.verdict("slower candidate")


def test_a_genuinely_slower_candidate_goes_red():
    """Fitness: the gate still fails loudly when the product really regresses.

    Without this, "tolerate one lost pair" is indistinguishable from a gate that cannot fail.
    """
    import time as _time

    def fast():
        _time.sleep(0)

    def slow():
        _time.sleep(0.002)

    result = compare(slow, fast, blocks=3, reps=3)
    assert not majority(result), result.verdict("slow candidate")
    assert result.candidate_wins == 0, result.pairs


def test_a_failure_message_names_every_pair():
    """A red has to say which side moved, or the next one is guessed at again."""
    result = Paired([(30.0, 20.0), (31.0, 21.0), (29.0, 19.0)])
    verdict = result.verdict("load_toml vs the AST route")
    assert "won 0/3" in verdict, verdict
    assert "30.0/20.0" in verdict and "31.0/21.0" in verdict and "29.0/19.0" in verdict, verdict
    # Best candidate block (29) against best reference block (19): the ratio says the pair inverted.
    assert "0.66x" in verdict, verdict


def test_median_discards_a_warm_up_round():
    """The first iterations ride cold caches; the macos incident traced to that, not to the code."""
    calls = []
    median_us(lambda: calls.append(1), reps=3)
    assert len(calls) == 4, "one warm-up call plus three timed repetitions"


# ── the measurement that keeps the convention alive ─────────────────────────────


def test_no_ordinary_ci_gate_times_wall_clock_outside_the_shared_sampler(matrix):
    """Every in-process timing floor routes through `tests/timing.py`.

    This is what stops the methodology from decaying back into three different estimators, which is
    how it started.
    """
    assert matrix.timing_floors_unpaired() == []


def test_the_measurement_fires_on_an_unpaired_gate(matrix, tmp_path):
    """Injected, not asserted: a file that times two phases separately is named.

    The root is a parameter for exactly this reason - the alternative was a scratch tree built with
    symlinks, which measures the test's patience rather than the rule.
    """
    tests = tmp_path / "tests"
    tests.mkdir()
    (tests / "test_synthetic_leaderboard.py").write_text(
        "import time\n\n"
        "def test_floor():\n"
        "    a = time.perf_counter()\n"
        "    b = time.perf_counter()\n"
        "    assert a < b\n",
        encoding="utf-8",
    )
    assert matrix.timing_floors_unpaired(tmp_path) == ["tests/test_synthetic_leaderboard.py"]


def test_a_paired_gate_is_not_reported(matrix, tmp_path):
    """The rule is about the estimator, not about timing at all: importing the sampler clears it."""
    tests = tmp_path / "tests"
    tests.mkdir()
    (tests / "test_synthetic_paired.py").write_text(
        "import time\n\nfrom tests.timing import compare\n\ndef test_floor():\n    assert time\n",
        encoding="utf-8",
    )
    assert matrix.timing_floors_unpaired(tmp_path) == []


def test_codspeed_files_are_not_this_rules_business(matrix, tmp_path):
    """Benchmark files measure the product on a pinned runner; a paired estimator is not the point."""
    tests = tmp_path / "tests"
    tests.mkdir()
    (tests / "test_synthetic_benchmark.py").write_text(
        "import pytest\nimport time\n\n"
        "@pytest.mark.benchmark\ndef test_throughput():\n"
        "    start = time.perf_counter()\n"
        "    assert time.perf_counter() > start\n",
        encoding="utf-8",
    )
    assert matrix.timing_floors_unpaired(tmp_path) == []
