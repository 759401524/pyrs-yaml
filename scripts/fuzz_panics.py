#!/usr/bin/env python
"""High-volume Hypothesis fuzz for pyrs-yaml panic hunting.

Runs the same invariants as the ``tests/`` property suite but bypasses pytest
so ``max_examples`` is unbounded and hostile inputs (control characters, NBSP,
backslashes, wide characters — all excluded by ``roundtrip_safe_text``) are
included. Any failure prints the offending input and exits non-zero.

Usage:
  uv run python scripts/fuzz_panics.py --examples 50000 --trials 5
  uv run python scripts/fuzz_panics.py --examples 100000 --trials 1 --seed 0
"""

import argparse
import sys
from pathlib import Path

# Allow running as a plain script (`uv run python scripts/fuzz_panics.py`)
# by making the repo root importable so `tests.strategies` resolves.
_REPO_ROOT = str(Path(__file__).resolve().parent.parent)
if _REPO_ROOT not in sys.path:
    sys.path.insert(0, _REPO_ROOT)

from hypothesis import HealthCheck, given, settings  # noqa: E402
from hypothesis import strategies as st  # noqa: E402

import pyrs_yaml  # noqa: E402
from tests.strategies import arbitrary_json, roundtrip_safe_json, rt  # noqa: E402

# ── hostile strategies ───────────────────────────────────────────────────────
# roundtrip_safe_text blacklists Cc (control) + Cs (surrogate) + Cn; real
# panics lurk near control chars, NBSP, backslashes, and wide characters.
_hostile_char = st.characters(blacklist_categories=("Cs",), max_codepoint=0x10FFFF)
hostile_text = st.text(_hostile_char, min_size=0, max_size=80)
hostile_leaf = st.one_of(
    st.none(),
    st.booleans(),
    st.integers(min_value=-(10**12), max_value=10**12),
    st.floats(allow_nan=False, allow_infinity=False, width=64),
    hostile_text,
)
hostile_json = st.recursive(
    hostile_leaf,
    lambda children: st.one_of(
        st.lists(children, min_size=0, max_size=8),
        st.dictionaries(hostile_text, children, min_size=0, max_size=8),
    ),
    max_leaves=40,
)

# ── invariants ───────────────────────────────────────────────────────────────


def check_roundtrip_exact(value):
    assert rt(value) == value


def check_roundtrip_idempotent(value):
    once = rt(value)
    twice = rt(once)
    assert twice == once


def check_dump_never_raises(value):
    out = pyrs_yaml.safe_dump(value)
    assert isinstance(out, str)


def check_dump_reparses(value):
    """safe_dump output must always be re-parseable (never emits invalid YAML)."""
    out = pyrs_yaml.safe_dump(value)
    doc = pyrs_yaml.parse(out)
    doc.to_dict()


def check_edit_root_replace(value):
    doc = pyrs_yaml.parse(pyrs_yaml.safe_dump(value))
    doc.set("$", {"__prop__": 42})
    assert doc.to_dict() == {"__prop__": 42}


def check_edit_to_scalar(value):
    doc = pyrs_yaml.parse(pyrs_yaml.safe_dump(value))
    if doc.root_type() in ("mapping", "seq"):
        doc.set("$", 42)
        assert doc.to_dict() == 42


# ── harness ──────────────────────────────────────────────────────────────────


def make_target_test(strategy, fn, name, trial, examples):
    """Build a Hypothesis-decorated test bound to a specific target/trial.

    Returns a zero-arg callable suitable for direct invocation (outside pytest).
    """

    @given(strategy)
    @settings(
        max_examples=examples,
        deadline=None,
        suppress_health_check=[HealthCheck.too_slow, HealthCheck.data_too_large],
    )
    def run(value):
        try:
            fn(value)
        except BaseException as exc:
            print(f"[FAIL] target={name} trial={trial} example={value!r} exc={exc!r}")
            raise

    return run


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--examples", type=int, default=50_000)
    ap.add_argument("--trials", type=int, default=5)
    ap.add_argument("--seed", type=int, default=None)
    ap.add_argument("--quiet", action="store_true")
    args = ap.parse_args()

    targets = [
        # Exact round-trip equality is only guaranteed for ``roundtrip_safe``
        # inputs (plain scalars cannot losslessly carry control characters).
        ("roundtrip_exact", roundtrip_safe_json, check_roundtrip_exact),
        ("idempotent", roundtrip_safe_json, check_roundtrip_idempotent),
        # No-panic + re-parseability hold for ANY input, including hostile ones.
        ("dump_never_raises", arbitrary_json, check_dump_never_raises),
        ("dump_never_raises_hostile", hostile_json, check_dump_never_raises),
        ("dump_reparses_hostile", hostile_json, check_dump_reparses),
        ("edit_root_replace", roundtrip_safe_json, check_edit_root_replace),
        ("edit_root_replace_hostile", hostile_json, check_edit_root_replace),
        ("edit_to_scalar", roundtrip_safe_json, check_edit_to_scalar),
        ("edit_to_scalar_hostile", hostile_json, check_edit_to_scalar),
    ]

    trials = range(args.trials) if args.seed is None else [args.seed]
    for trial in trials:
        for name, strategy, fn in targets:
            make_target_test(strategy, fn, name, trial, args.examples)()
            if not args.quiet:
                print(f"ok  {name}  trial={trial}  ({args.examples} examples)")

    print(f"\nAll targets passed for {len(trials)} trial(s) x {len(targets)} targets ({args.examples} examples each).")


if __name__ == "__main__":
    main()
