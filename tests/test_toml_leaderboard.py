"""TOML fast-path floors — the native paths must beat the in-process paths they replace.

Runs in normal CI (no ``--codspeed``). These gate the *structural* wins of the
native TOML kernel against a baseline computed **in the same process on the same
runner**, so the ratio is stable under shared-CI contention.

**Cross-library ranking does not belong here.** The "pyrs beats tomllib"
comparison is already owned by CodSpeed (``tests/test_benchmark_crosslib.py``:
``test_pyrs_load_toml`` vs ``test_tomllib_load``, groups ``pyrs-toml`` vs
``tomllib``, isolated WallTime on a fixed runner). It was additionally asserted
here with a 2x floor, which is not a property of the code:

- The floor is **platform-dependent, not a code property**. Measured ratio vs
  ``tomllib`` on a 100-item document: ~4.5x on Windows x86-64, ~1.8x on the
  macOS runner. A pure-Python stdlib parser simply fares relatively better on
  Apple Silicon, so *any* floor that holds on one OS can break on another while
  the Rust kernel is unchanged - the CI matrix runs this file on 3 OS x 7 Python,
  so one slow cell blocks every unrelated PR.
- The previous 2x floor failed exactly that way: ``toml parse/medium: pyrs not
  >2x faster than tomllib (381.1us vs 688.4us)`` on macos-latest while passing
  on the same commit's other 20 cells. This file's own ``small`` fixture had
  already been dropped for the same reason (#142).

What is a real invariant, and is gated below, is *less work per unit of input*:

- ``load_toml`` (single-pass TOML -> Python objects) vs the AST route it
  bypasses (``parse(from_toml(doc)).to_dict()``: TOML -> YAML text -> CustomNode
  tree -> Python objects). Measured ~2.2x / ~1.9x on medium / large.
- ``to_toml`` (single-pass AST -> TOML text) vs the serialize-to-YAML-then-
  reparse round-trip the method eliminates (#142).

Both native paths do strictly less work than their baselines on every platform,
so the floors hold with a wide margin and still fail loudly if a fast path
regresses back through the slow route. Sizes are medium/large: a tiny document is
dominated by fixed call overhead, and both baselines are timed end-to-end in the
same process so shared-CI contention moves the two sides together.
"""

import statistics
import time

import pytest

import pyrs_yaml


def _doc(items):
    head = (
        '[server]\nhost = "0.0.0.0"\nport = 8080\nssl = true\n'
        '[database]\ntype = "postgresql"\n\n'
        "[database.pool]\nmin_size = 5\nmax_size = 20\n\n"
    )
    body = "".join(f'[[items]]\nname = "item_{index}"\nvalue = {index * 10}\n\n' for index in range(items))
    return head + body


_SIZES = {"medium": _doc(100), "large": _doc(500)}


def _median_us(fn, reps=40):
    # One discarded warm-up round: the first timed iterations ride cold caches
    # (import machinery, allocator arenas), which skews the median on shared
    # runners — the macos flake of the parse/medium gate (336 vs 369 us,
    # CI #219) traced to this, not to the code under test.
    fn()
    samples = []
    for _ in range(reps):
        t0 = time.perf_counter()
        fn()
        samples.append(time.perf_counter() - t0)
    return statistics.median(samples) * 1e6


@pytest.mark.parametrize("size", sorted(_SIZES))
def test_toml_parse_beats_ast_route(size):
    """load_toml must beat the to-YAML-then-reparse AST route it bypasses.

    Parity guard: both routes must produce the same dict, so we are timing real
    work rather than a path that bails early.
    """
    doc = _SIZES[size]
    # Interleave short A/B blocks (3 rounds): runner contention that inflates
    # one side alone inflates the whole block pair, keeping the ratio honest.
    native = min(
        _median_us(lambda: pyrs_yaml.load_toml(doc), reps=15),
        _median_us(lambda: pyrs_yaml.load_toml(doc), reps=15),
        _median_us(lambda: pyrs_yaml.load_toml(doc), reps=15),
    )
    # The AST route is timed end-to-end, from_toml included: it converts TOML to
    # YAML text, builds a CustomNode tree, then converts that tree to Python
    # objects. Hoisting from_toml into setup would compare against a different
    # pipeline (YAML -> dict only) and can measure the native path as *slower*.
    ast_route = min(
        _median_us(lambda: pyrs_yaml.parse(pyrs_yaml.from_toml(doc)).to_dict(), reps=15),
        _median_us(lambda: pyrs_yaml.parse(pyrs_yaml.from_toml(doc)).to_dict(), reps=15),
        _median_us(lambda: pyrs_yaml.parse(pyrs_yaml.from_toml(doc)).to_dict(), reps=15),
    )
    # Parity: both routes must produce the same dict, so we are timing real work
    # rather than a path that bails early.
    assert pyrs_yaml.load_toml(doc) == pyrs_yaml.parse(pyrs_yaml.from_toml(doc)).to_dict()
    assert native < ast_route, (
        f"parse/{size}: native load_toml not faster than the AST route it bypasses ({native:.1f}us vs {ast_route:.1f}us)"
    )


@pytest.mark.parametrize("size", sorted(_SIZES))
def test_toml_serialize_beats_yaml_round_trip(size):
    """Native doc.to_toml() must beat the to_yaml()+to_toml round-trip it replaces.

    The earlier version gated a cross-library margin vs tomli_w at 2x; calibrated
    on a local ~4.3x it flaked to 1.97x on a loaded macOS runner (the #142 lesson:
    do not put a cross-implementation timing floor in a blocking assert). This
    in-process self-relative floor compares doc.to_toml() (single native AST pass,
    #142) against pyrs_yaml.to_toml(doc.to_yaml()) - the serialize-to-YAML-then-
    reparse round-trip the method eliminates. Both run on the same contended CPU,
    so the ratio is stable and it fails loudly if a round-trip creeps back in.
    """
    doc_text = _SIZES[size]
    parsed = pyrs_yaml.parse(pyrs_yaml.from_toml(doc_text))  # setup: get a document
    # Parity: the writer's output reloads to the same data.
    assert pyrs_yaml.load_toml(parsed.to_toml()) == pyrs_yaml.load_toml(doc_text)
    native = _median_us(lambda: parsed.to_toml())
    round_trip = _median_us(lambda: pyrs_yaml.to_toml(parsed.to_yaml()))
    assert native < round_trip, (
        f"toml serialize/{size}: native to_toml not faster than the "
        f"to_yaml()+to_toml round-trip it replaces ({native:.1f}us vs {round_trip:.1f}us)"
    )
