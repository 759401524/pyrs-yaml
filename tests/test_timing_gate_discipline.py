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
tests fire that measurement on injected text. They also pin the sampler's two properties directly:
phases alternate inside a block, and one lost pair is tolerated while two are not.
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


def test_one_lost_pair_is_noise_and_two_are_a_verdict():
    """The tolerance is deliberate and bounded, so a spike cannot decide but a slowdown can."""
    won_every_pair = Paired([(10.0, 20.0), (11.0, 21.0), (10.5, 20.5), (12.0, 22.0), (10.0, 19.0)])
    assert majority(won_every_pair)
    assert won_every_pair.candidate_wins == 5

    one_bad_block = Paired([(10.0, 20.0), (99.0, 20.5), (10.2, 20.2), (10.1, 20.1), (10.4, 20.4)])
    assert one_bad_block.candidate_wins == 4, one_bad_block.pairs
    assert majority(one_bad_block), "a single inflated pair must not redden the gate"
    assert one_bad_block.candidate_us == 10.0 and one_bad_block.reference_us == 20.0

    two_bad_blocks = Paired([(99.0, 20.0), (98.0, 20.5), (10.2, 20.2), (10.1, 20.1), (10.4, 20.4)])
    assert two_bad_blocks.candidate_wins == 3, two_bad_blocks.pairs
    assert not majority(two_bad_blocks), "three of five is not a claim that the path is faster"


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
