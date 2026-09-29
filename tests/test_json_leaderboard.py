"""JSON parse leaderboard gate — pyrs is top-3 against the real field.

Runs in normal CI. Ranks `load_jsonc` (the direct-parse fast path) against every
installed JSON library. On canonical strict-JSON payloads the fast path builds
Python objects in a single tight pass (no intermediate AST, no per-string UTF-8
re-validation, no `str::parse` per integer), so pyrs lands behind at most the two
fastest of {orjson, ujson} and ahead of the C `json` scanner and `rapidjson`.
"At most 2 installed competitors finish faster" is the top-3 invariant; it fails
loudly if pyrs regresses out of the top three while tolerating clock noise.

Sizes are medium/large: a tiny document is dominated by fixed call overhead where
any library can edge another, so the ranking would be unstable. Non-canonical
shapes fall back to the AST path and are out of scope for this ranking gate.
"""

import json
import statistics
import time

import pytest

import pyrs_yaml

try:
    import orjson

    HAS_ORJSON = True
except ImportError:
    HAS_ORJSON = False

try:
    import ujson

    HAS_UJSON = True
except ImportError:
    HAS_UJSON = False

try:
    import rapidjson

    HAS_RAPIDJSON = True
except ImportError:
    HAS_RAPIDJSON = False


def _payload(items):
    return {
        "server": {"host": "0.0.0.0", "port": 8080, "ssl": True, "workers": 4},
        "items": [{"name": f"item_{i}", "value": i * 10} for i in range(items)],
    }


def _doc(items):
    return json.dumps(_payload(items))


_SIZES = {"medium": _doc(300), "large": _doc(1200)}

# The competitor field, filtered to what is importable on this matrix cell.
_PEERS = []
if HAS_ORJSON:
    _PEERS.append(lambda s: orjson.loads(s))
if HAS_UJSON:
    _PEERS.append(lambda s: ujson.loads(s))
_PEERS.append(json.loads)
if HAS_RAPIDJSON:
    _PEERS.append(lambda s: rapidjson.loads(s))


def _median_us(fn, reps=50):
    samples = []
    for _ in range(reps):
        t0 = time.perf_counter()
        fn()
        samples.append(time.perf_counter() - t0)
    return statistics.median(samples) * 1e6


@pytest.mark.parametrize("size", sorted(_SIZES))
def test_json_parse_top3(size):
    doc = _SIZES[size]
    # Parity: the fast path must agree with the reference parser on canonical input.
    assert pyrs_yaml.load_jsonc(doc) == json.loads(doc)
    pyrs = _median_us(lambda: pyrs_yaml.load_jsonc(doc))
    faster = sum(1 for fn in _PEERS if _median_us(lambda f=fn: f(doc)) < pyrs)
    assert faster <= 2, f"json parse/{size}: pyrs not top-3 ({faster} of {len(_PEERS)} competitors faster)"


# Serialize: native single-pass to_json(0) vs the field's compact dumps.
_SERIALIZE_ITEMS = (300, 1200)


def _serialize_peers(data):
    peers = []
    if HAS_ORJSON:
        peers.append(lambda: orjson.dumps(data))
    if HAS_UJSON:
        peers.append(lambda: ujson.dumps(data))
    peers.append(lambda: json.dumps(data))
    if HAS_RAPIDJSON:
        peers.append(lambda: rapidjson.dumps(data))
    return peers


@pytest.mark.parametrize("items", _SERIALIZE_ITEMS)
def test_json_serialize_top3(items):
    data = _payload(items)
    doc = pyrs_yaml.parse(pyrs_yaml.from_jsonc(json.dumps(data)))
    # Parity + byte-stability: native compact output must parse back to the data.
    assert json.loads(doc.to_json(0)) == data
    pyrs = _median_us(lambda: doc.to_json(0))
    peers = _serialize_peers(data)
    faster = sum(1 for peer in peers if _median_us(peer) < pyrs)
    assert faster <= 2, f"serialize/{items}: {faster} of {len(peers)} competitors beat pyrs"
