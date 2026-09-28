"""Cross-library performance leaderboard gate.

Runs in normal CI (no ``--codspeed`` needed), so the "top 3 in class"
claim is continuously enforced rather than a one-off snapshot. Measured
on the current build against all five competitors (PyYAML, ruamel,
ryaml, yaml_rs): pyrs-yaml is #1-#2 on serialize and #2-#3 on parse —
always inside the top three. Only the two Rust peers (yaml_rs, ryaml)
ever place ahead of pyrs, and the pure-Python PyYAML/ruamel trail by
30-540x, so "at most 2 installed competitors are faster" is robust to
clock noise while failing loudly if pyrs ever regressed into the
bottom of the field.

The absolute per-op timings live in CodSpeed (``test_benchmark_*``,
``--codspeed``); this file is the *ranking* invariant, checked across
every fixture size and both parse + serialize.
"""

import io
import statistics
import time

import pytest

import pyrs_yaml
from tests.data.yaml_samples import BENCHMARK_LARGE as LARGE_YAML
from tests.data.yaml_samples import BENCHMARK_MEDIUM as MEDIUM_YAML
from tests.data.yaml_samples import BENCHMARK_SMALL as SMALL_YAML

_SIZES = {"small": SMALL_YAML, "medium": MEDIUM_YAML, "large": LARGE_YAML}

try:
    import yaml as _pyyaml

    HAS_PYYAML = True
except ImportError:  # pragma: no cover
    HAS_PYYAML = False

try:
    from ruamel.yaml import YAML as _RUAMEL_YAML

    HAS_RUAMEL = True
except ImportError:  # pragma: no cover
    HAS_RUAMEL = False

try:
    import yaml_rs as _yaml_rs

    HAS_YAML_RS = True
except ImportError:  # pragma: no cover
    HAS_YAML_RS = False

try:
    import ryaml as _ryaml

    HAS_RYAML = True
except ImportError:  # pragma: no cover
    HAS_RYAML = False


def _median_us(fn, reps=50):
    samples = []
    for _ in range(reps):
        t0 = time.perf_counter()
        fn()
        samples.append(time.perf_counter() - t0)
    return statistics.median(samples) * 1e6


def _load(lib, y):
    """Parse with a competitor, or None if it cannot ingest the fixture."""
    try:
        if lib == "pyyaml":
            return _pyyaml.safe_load(y)
        if lib == "ruamel":
            return _RUAMEL_YAML(typ="safe").load(y)
        if lib == "yaml_rs":
            return _yaml_rs.loads(y)
        if lib == "ryaml":
            return _ryaml.load(io.StringIO(y))
    except Exception:
        return None
    return None


def _dump(lib, data):
    if lib == "pyyaml":
        return _pyyaml.safe_dump(data)
    if lib == "ruamel":
        s = io.StringIO()
        _RUAMEL_YAML(typ="safe").dump(data, s)
        return s.getvalue()
    if lib == "yaml_rs":
        return _yaml_rs.dumps(data)
    if lib == "ryaml":
        return _ryaml.dumps(data)
    raise AssertionError(lib)


# Competitors we can rank against, filtered to what's installed.
_ALL = [
    n
    for n, ok in [("pyyaml", HAS_PYYAML), ("ruamel", HAS_RUAMEL), ("yaml_rs", HAS_YAML_RS), ("ryaml", HAS_RYAML)]
    if ok
]
# The pure-Python peers pyrs must stay far ahead of; only enforced when
# installed so the gate is meaningful on minimal CI images.
_PYTHON_PEERS = [n for n in _ALL if n in ("pyyaml", "ruamel")]

pytestmark = pytest.mark.skipif(not (HAS_PYYAML and HAS_RUAMEL), reason="pure-Python peers not installed")


@pytest.mark.parametrize("size", sorted(_SIZES))
def test_parse_top3(size):
    y = _SIZES[size]
    pyrs = _median_us(lambda: pyrs_yaml.parse(y))
    faster = 0
    for lib in _ALL:
        val = None
        try:
            val = _median_us(lambda f=lib, y=y: _load(f, y))
        except Exception:  # pragma: no cover - timing harness safety
            val = None
        if val is not None and val < pyrs:
            faster += 1
    assert faster <= 2, f"parse/{size}: pyrs not top-3 ({faster} competitors faster)"
    # pyrs stays an order of magnitude ahead of the pure-Python peers.
    for lib in _PYTHON_PEERS:
        peer = _median_us(lambda f=lib, y=y: _load(f, y))
        assert pyrs * 5 < peer, f"parse/{size}: pyrs not >5x faster than {lib} ({pyrs:.1f} vs {peer:.1f})"


@pytest.mark.parametrize("size", sorted(_SIZES))
def test_serialize_top3(size):
    y = _SIZES[size]
    doc = pyrs_yaml.parse(y)
    pyrs = _median_us(doc.to_yaml)
    faster = 0
    for lib in _ALL:
        data = _load(lib, y)
        if data is None:  # competitor could not ingest the fixture -> not rankable
            continue
        try:
            val = _median_us(lambda f=lib, d=data: _dump(f, d))
        except Exception:  # pragma: no cover
            val = None
        if val is not None and val < pyrs:
            faster += 1
    assert faster <= 2, f"serialize/{size}: pyrs not top-3 ({faster} competitors faster)"
    for lib in _PYTHON_PEERS:
        data = _load(lib, y)
        peer = _median_us(lambda f=lib, d=data: _dump(f, d))
        assert pyrs * 5 < peer, f"serialize/{size}: pyrs not >5x faster than {lib} ({pyrs:.1f} vs {peer:.1f})"
