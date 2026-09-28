"""Cross-library performance leaderboard gate.

Runs in normal CI (no ``--codspeed`` needed), so the "top 3 in class"
claim is continuously enforced rather than a one-off snapshot. It is a
relative-ranking check with very generous margins: pyrs-yaml beats the
pure-Python YAML libraries (PyYAML, ruamel) by tens-to-hundreds of times,
so asserting only a 5x margin and "at most 2 installed competitors are
faster" is robust against hardware/clock noise while still failing loudly
if pyrs ever regressed into the bottom of the field.

The absolute per-op timings live in CodSpeed (``test_benchmark_*`` files,
``--codspeed``); this file is the *ranking* invariant.
"""

import io
import statistics
import time

import pytest

import pyrs_yaml
from tests.data.yaml_samples import BENCHMARK_LARGE as LARGE_YAML
from tests.data.yaml_samples import BENCHMARK_MEDIUM as MEDIUM_YAML

# Competitor import detection (all optional).
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


def _median_us(fn, reps=60):
    samples = []
    for _ in range(reps):
        t0 = time.perf_counter()
        fn()
        samples.append(time.perf_counter() - t0)
    return statistics.median(samples) * 1e6


def _ruamel():
    y = _RUAMEL_YAML()
    y.preserve_quotes = True
    return y


@pytest.mark.skipif(not (HAS_PYYAML and HAS_RUAMEL), reason="competitors not installed")
def test_parse_is_top3_and_beats_python_libs():
    pyrs = _median_us(lambda: pyrs_yaml.parse(MEDIUM_YAML))
    faster = 0
    if HAS_PYYAML:
        assert pyrs * 5 < _median_us(lambda: _pyyaml.safe_load(MEDIUM_YAML)), (
            "pyrs parse should be far faster than PyYAML"
        )
        if _median_us(lambda: _pyyaml.safe_load(MEDIUM_YAML)) < pyrs:
            faster += 1
    if HAS_RUAMEL:
        ry = _ruamel()
        assert pyrs * 5 < _median_us(lambda: ry.load(MEDIUM_YAML)), "pyrs parse should be far faster than ruamel"
        if _median_us(lambda: ry.load(MEDIUM_YAML)) < pyrs:
            faster += 1
    if HAS_YAML_RS and _median_us(lambda: _yaml_rs.loads(MEDIUM_YAML)) < pyrs:
        faster += 1
    # Top-3 means at most two installed competitors beat us on this op.
    assert faster <= 2, f"pyrs parse not in top 3 (faster competitors: {faster})"


@pytest.mark.skipif(not (HAS_PYYAML and HAS_RUAMEL), reason="competitors not installed")
def test_serialize_is_top3_and_beats_python_libs():
    doc = pyrs_yaml.parse(LARGE_YAML)
    pyrs = _median_us(doc.to_yaml)
    faster = 0
    if HAS_PYYAML:
        data = _pyyaml.safe_load(LARGE_YAML)
        assert pyrs * 5 < _median_us(lambda: _pyyaml.safe_dump(data)), "pyrs serialize should be far faster than PyYAML"
        if _median_us(lambda: _pyyaml.safe_dump(data)) < pyrs:
            faster += 1
    if HAS_RUAMEL:
        ry = _ruamel()
        rdata = ry.load(LARGE_YAML)
        assert pyrs * 5 < _median_us(lambda: ry.dump(rdata, io.StringIO())), (
            "pyrs serialize should be far faster than ruamel"
        )
        if _median_us(lambda: ry.dump(rdata, io.StringIO())) < pyrs:
            faster += 1
    if HAS_YAML_RS:
        vdata = _yaml_rs.loads(LARGE_YAML)
        if _median_us(lambda: _yaml_rs.dumps(vdata)) < pyrs:
            faster += 1
    assert faster <= 2, f"pyrs serialize not in top 3 (faster competitors: {faster})"
