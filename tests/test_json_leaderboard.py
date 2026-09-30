"""JSON fast-path floors — the native paths must beat the in-process paths they replace.

Runs in normal CI. These gate the *structural* wins of the native JSON kernel
against a baseline computed **in the same process on the same runner**, so the
ratio is stable under shared-CI contention (unlike a cross-library timing
ranking, which flips when a runner is loaded - see the #139/#141 flake history
and why absolute rankings live in CodSpeed, not a blocking pytest assert):

- `load_jsonc` (single-pass parse straight into Python objects) vs the AST route
  it bypasses (`parse(doc).to_dict()`: build a `CustomNode`, then convert).
- `to_json` (single-pass AST -> text) vs the double conversion it replaced
  (`json.dumps(doc.to_dict())`: materialise Python objects, then re-walk them).

Both native paths are strictly less work than their baselines (~22x parse, ~5x
serialize locally), so the floor holds on every platform with a wide margin and
still fails loudly if the fast path regresses back through the slow route. Sizes
are medium/large: a tiny document is dominated by fixed call overhead.
"""

import json
import statistics
import time

import pytest

import pyrs_yaml


def _payload(items):
    return {
        "server": {"host": "0.0.0.0", "port": 8080, "ssl": True, "workers": 4},
        "items": [{"name": f"item_{i}", "value": i * 10} for i in range(items)],
    }


def _doc(items):
    return json.dumps(_payload(items))


_SIZES = {"medium": _doc(300), "large": _doc(1200)}


def _median_us(fn, reps=50):
    samples = []
    for _ in range(reps):
        t0 = time.perf_counter()
        fn()
        samples.append(time.perf_counter() - t0)
    return statistics.median(samples) * 1e6


@pytest.mark.parametrize("size", sorted(_SIZES))
def test_json_parse_beats_ast_route(size):
    doc = _SIZES[size]
    # Parity: fast path, AST route, and the reference parser must all agree.
    assert pyrs_yaml.load_jsonc(doc) == pyrs_yaml.parse(doc).to_dict() == json.loads(doc)
    fast = _median_us(lambda: pyrs_yaml.load_jsonc(doc))
    ast_route = _median_us(lambda: pyrs_yaml.parse(doc).to_dict())
    assert fast < ast_route, (
        f"parse/{size}: native fast path not faster than the AST route it bypasses ({fast:.1f}us vs {ast_route:.1f}us)"
    )


_SERIALIZE_ITEMS = (300, 1200)


@pytest.mark.parametrize("items", _SERIALIZE_ITEMS)
def test_json_serialize_beats_old_round_trip(items):
    data = _payload(items)
    doc = pyrs_yaml.parse(pyrs_yaml.from_jsonc(json.dumps(data)))
    # Parity + byte-stability: native compact output must parse back to the data.
    assert json.loads(doc.to_json(0)) == data
    native = _median_us(lambda: doc.to_json(0))
    round_trip = _median_us(lambda: json.dumps(doc.to_dict()))
    assert native < round_trip, (
        f"serialize/{items}: native to_json not faster than the old "
        f"to_dict()+json.dumps round-trip it replaced ({native:.1f}us vs {round_trip:.1f}us)"
    )
