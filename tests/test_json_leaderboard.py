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

import pytest

import pyrs_yaml
from tests.timing import compare, majority


def _payload(items):
    return {
        "server": {"host": "0.0.0.0", "port": 8080, "ssl": True, "workers": 4},
        "items": [{"name": f"item_{i}", "value": i * 10} for i in range(items)],
    }


def _doc(items):
    return json.dumps(_payload(items))


_SIZES = {"medium": _doc(300), "large": _doc(1200)}


@pytest.mark.parametrize("size", sorted(_SIZES))
def test_json_parse_beats_ast_route(size):
    """Native load_jsonc must beat the AST route it replaces, sampled in pairs.

    Measured locally at 11-17x, so the margin is not the concern; the estimator was. Both sides used
    to be timed in one block each, back to back, which hands the verdict to whatever the scheduler
    did during those two blocks. `tests/timing.py` measures the pair adjacently in five blocks, judges
    the minima with a floor of two pair wins, and quotes every pair when it fails.
    """
    doc = _SIZES[size]
    # Parity: fast path, AST route, and the reference parser must all agree.
    assert pyrs_yaml.load_jsonc(doc) == pyrs_yaml.parse(doc).to_dict() == json.loads(doc)
    result = compare(lambda: pyrs_yaml.load_jsonc(doc), lambda: pyrs_yaml.parse(doc).to_dict())
    assert majority(result), f"json parse/{size}: " + result.verdict("native load_jsonc vs the AST route it bypasses")


_SERIALIZE_ITEMS = (300, 1200)


@pytest.mark.parametrize("items", _SERIALIZE_ITEMS)
def test_json_serialize_beats_old_round_trip(items):
    """Native compact to_json must beat the to_dict()+json.dumps it replaced, in pairs."""
    data = _payload(items)
    doc = pyrs_yaml.parse(pyrs_yaml.from_jsonc(json.dumps(data)))
    # Parity + byte-stability: native compact output must parse back to the data.
    assert json.loads(doc.to_json(0)) == data
    result = compare(lambda: doc.to_json(0), lambda: json.dumps(doc.to_dict()))
    assert majority(result), f"json serialize/{items}: " + result.verdict(
        "native to_json vs the to_dict()+json.dumps round trip"
    )
