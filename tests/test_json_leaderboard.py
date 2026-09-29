"""JSON parse leaderboard gate — pyrs is top-3 against the real field.

Runs in normal CI. Ranks `load_jsonc` (the direct parse fast path, #137)
against every installed JSON library. On canonical strict-JSON payloads the
fast path builds Python objects in a single pass, so pyrs lands behind only the
SIMD `orjson` and typically `ujson`, staying clearly ahead of the C `json`
scanner and `rapidjson`. "At most 2 installed competitors finish faster" is the
top-3 invariant; it fails loudly if pyrs regresses out of the top three while
tolerating clock noise.

Sizes are medium/large only: a tiny document is dominated by fixed call
overhead where any library can edge another, so the ranking would be unstable.
Non-canonical shapes are out of scope here (they fall back to the AST path);
the gate uses integer/string/bool/null documents, which the fast path owns.
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


def _doc(items):
    payload = {
        "server": {"host": "0.0.0.0", "port": 8080, "ssl": True, "workers": 4},
        "items": [{"name": f"item_{i}", "value": i * 10} for i in range(items)],
    }
    return json.dumps(payload)


_SIZES = {"medium": _doc(300), "large": _doc(1200)}

# The competitor field, filtered to what is importable on this matrix cell.
_PEERS = []
if HAS_ORJSON:
    _PEERS.append(("orjson", lambda s: orjson.loads(s)))
if HAS_UJSON:
    _PEERS.append(("ujson", lambda s: ujson.loads(s)))
_PEERS.append(("json", lambda s: json.loads(s)))
if HAS_RAPIDJSON:
    _PEERS.append(("rapidjson", lambda s: rapidjson.loads(s)))


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
    # Parity: the fast path must agree with the reference parser.
    assert pyrs_yaml.load_jsonc(doc) == json.loads(doc)
    pyrs = _median_us(lambda: pyrs_yaml.load_jsonc(doc))
    faster = sum(1 for _name, fn in _PEERS if _median_us(lambda f=fn: f(doc)) < pyrs)
    assert faster <= 2, f"json parse/{size}: pyrs not top-3 ({faster} of {len(_PEERS)} competitors faster)"
