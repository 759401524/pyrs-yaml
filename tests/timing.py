"""Paired sampling for the in-process timing floors, and a verdict that explains itself.

Three test files gate a *structural* property with wall-clock: a native fast path must beat the
route it replaces, measured against a baseline computed in the same process on the same runner
(cross-library ranking belongs to CodSpeed, per `test_toml_leaderboard.py`). The gates disagreed on
how to sample: the TOML parse gate ran three blocks of the fast path and then three blocks of the
baseline, the TOML serialize and both JSON gates ran a single block per side, and the leaderboard
peers were timed once per side in a loop. Every one of them therefore compares phases that never
share an environment: a scheduler spike, a cold arena, or a GC pause landing on one phase inflates
one side only, and the assertion cannot tell that apart from a regression.

Two properties carry this module, and the second one is a correction the CI wrote for us:

- **A red must be attributable.** Each block measures the candidate and the reference back to back, and
  the result keeps every pair, so a failure message shows whether one side was inflated in a few blocks
  (measurement) or the two are genuinely close (product). Without it, a red on this gate is a rumour.
- **The verdict is the minima, not a majority of pair signs.** The first version asked the candidate to
  win all but one pair, on the theory that a burst inside a pair cancels. It does not: bursts outlast
  pairs. A macos-latest run reported `2.58x` of real headroom and failed that rule because three of five
  candidate blocks sat at ~3x their own floor - `139.4/1001.3, 145.9/385.5, 405.8/359.6, 379.1/1006.8,
  437.2/375.8`. An undisturbed block exists on both sides precisely because both sides are measured in
  every block, so `min` is the stable statistic and `majority()` keeps a two-win floor so one lucky
  block cannot carry a verdict alone.

`tests/test_timing_gate_discipline.py` pins the properties: that the sampler alternates phases inside a
block, that the CI log above passes while a genuinely slower candidate still goes red, and that
`scripts/quality_matrix.py` refuses a comparison gate that bypasses this module.
"""

from __future__ import annotations

import statistics
import time


def median_us(action, reps=25):
    """Median microseconds of `action`, after one discarded warm-up round.

    The first timed iterations ride cold caches - import machinery, allocator arenas - which skews
    the median on shared runners; the macos flake recorded in `test_toml_leaderboard.py` traced to
    this rather than to the code under test.
    """
    action()
    samples = []
    for _ in range(reps):
        start = time.perf_counter()
        action()
        samples.append(time.perf_counter() - start)
    return statistics.median(samples) * 1e6


class Paired:
    """The outcome of comparing a candidate against a reference, pair by pair.

    `candidate_us` and `reference_us` are the best block of each side, which is what an assertion
    about "less work" should quote. `candidate_wins` counts the pairs where the candidate was faster,
    and `pairs` is the raw material for the failure message.
    """

    def __init__(self, pairs):
        self.pairs = pairs
        self.candidate_us = min(candidate for candidate, _ in pairs)
        self.reference_us = min(reference for _, reference in pairs)
        self.candidate_wins = sum(1 for candidate, reference in pairs if candidate < reference)

    @property
    def blocks(self):
        return len(self.pairs)

    @property
    def ratio(self):
        return self.reference_us / self.candidate_us if self.candidate_us else float("inf")

    def verdict(self, name):
        """One line carrying the pairs, so a red says which side moved."""
        cells = ", ".join(f"{candidate:.1f}/{reference:.1f}" for candidate, reference in self.pairs)
        return (
            f"{name}: candidate {self.candidate_us:.1f}us vs reference {self.reference_us:.1f}us "
            f"({self.ratio:.2f}x), won {self.candidate_wins}/{self.blocks} pairs. per-pair us: {cells}"
        )


def compare(candidate, reference, blocks=5, reps=25, reference_reps=None):
    """Sample `candidate` and `reference` in adjacent blocks and return a `Paired`.

    The alternation is the point: measuring all candidate blocks before all reference blocks lets a
    single scheduler spike decide the outcome. The verdict that `majority` reaches is the minima of
    those blocks, with a floor on how many blocks the candidate has to win.

    `reference_reps` exists for cross-library pairs, where the reference is a pure-Python parser and
    its own repetitions dominate the cost: five blocks of it would make the suite slower without
    making the verdict better, because the pair - not the repetition count - is what cancels drift.
    """
    if reference_reps is None:
        reference_reps = reps
    pairs = []
    for _ in range(blocks):
        first = median_us(candidate, reps)
        second = median_us(reference, reference_reps)
        pairs.append((first, second))
    return Paired(pairs)


def majority(result, minimum_wins=2):
    """Whether `result` is faster, judged on block minima with a floor on pair wins.

    The first version of this rule asked the candidate to win all but one pair, on the theory that a
    pair is a same-environment comparison and therefore a sign flip is noise. A macOS runner said
    otherwise, and the log is the argument:

        candidate 139.4us vs reference 359.6us (2.58x), won 3/5 pairs.
        per-pair us: 139.4/1001.3, 145.9/385.5, 405.8/359.6, 379.1/1006.8, 437.2/375.8

    The document really is 2.58x cheaper, and three of five candidate blocks were inflated to roughly
    three times their own floor by scheduler bursts that lasted longer than a block. Per-pair signs are
    therefore not a stable verdict on a shared cell: the burst does not respect the pair boundary. The
    minima are - a block that is undisturbed exists on both sides, because both sides are measured in
    every block, which is what the alternation buys. So the verdict is the minima comparison, and
    `minimum_wins` is the floor that stops one lucky block from carrying it alone.

    Keeping the pairs in the message is not decoration: it is how a reader tells this shape (a few
    inflated blocks, ratio intact) from a genuine narrowing (every pair close, ratio near 1).
    """
    return result.candidate_wins >= minimum_wins and result.candidate_us < result.reference_us
