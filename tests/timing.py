"""Paired sampling for the in-process timing floors, and a verdict that explains itself.

Three test files gate a *structural* property with wall-clock: a native fast path must beat the
route it replaces, measured against a baseline computed in the same process on the same runner
(cross-library ranking belongs to CodSpeed, per `test_toml_leaderboard.py`). The gates disagreed on
how to sample: the TOML parse gate ran three blocks of the fast path and then three blocks of the
baseline, the TOML serialize and both JSON gates ran a single block per side, and the leaderboard
peers were timed once per side in a loop. Every one of them therefore compares phases that never
share an environment: a scheduler spike, a cold arena, or a GC pause landing on one phase inflates
one side only, and the assertion cannot tell that apart from a regression.

Two properties fix that, and they are the reason this module exists:

- **Pairs, not phases.** Each block measures the candidate and the reference back to back, so a block
  is an internally consistent comparison. The verdict is a majority over blocks: one inverted pair is
  noise, four of five is a fact. Measured locally the margin is ~2.2x for `load_toml` and ~4.5x for
  `to_toml`, but the same parse gate recorded 336us vs 369us (1.10x) on a macos-latest runner - a
  small shared cell leaves a minima-comparison with nothing to spend, while five pairs each seeing
  the same contention stay comparable.
- **A red must be attributable.** The result carries every pair, so a failure message shows whether
  one side was inflated in a single block (measurement) or the two are genuinely close (product).

`tests/test_timing_gate_discipline.py` pins both properties: that the sampler alternates phases
inside a block, that a slowed fast path turns the gate red, and that `scripts/quality_matrix.py`
refuses a comparison gate that bypasses this module.
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
    single scheduler spike decide the outcome. `majority` checks the candidate wins all but one pair,
    which five blocks make a deliberate tolerance for one bad pair rather than a shrug.

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


def majority(result, allow_one_loss=True):
    """Whether `result` won enough pairs to be called faster.

    Tolerating one lost pair is what makes a spike survivable; `allow_one_loss=False` is for margins
    that are supposed to be decisive rather than merely ordered.
    """
    needed = result.blocks - 1 if allow_one_loss else result.blocks
    return result.candidate_wins >= needed and result.candidate_us < result.reference_us
