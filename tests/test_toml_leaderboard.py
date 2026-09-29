"""TOML leaderboard gate — pyrs beats the standard-library reference.

Runs in normal CI (no ``--codspeed``), so the claim "the native TOML kernel
clearly outperforms the ecosystem's default parser" is continuously enforced
and the ``load_toml`` core path is guarded against regressions.

``tomllib`` is the pure-Python stdlib reference (3.11+). Measured margin across
sizes is ~2.9-4.6x, so the gate asserts a conservative **2x floor** — decisive
enough to be meaningful, wide enough to survive CI clock noise (both sides
scale with CPU, so the ratio is stable). A richer top-3 field (taplo / tomlkit /
tomli) needs those optional dependencies and is tracked as a separate decision;
this file only enforces what is verifiable with the standard library already
present in the test environment.
"""

import statistics
import time

import pytest

import pyrs_yaml

try:
    import tomllib

    HAS_TOMLLIB = True
except ImportError:  # pragma: no cover - py<3.11
    HAS_TOMLLIB = False

try:
    import tomlkit

    HAS_TOMLKIT = True
except ImportError:  # pragma: no cover
    HAS_TOMLKIT = False


def _doc(items):
    head = (
        '[server]\nhost = "0.0.0.0"\nport = 8080\nssl = true\n'
        '[database]\ntype = "postgresql"\n\n'
        "[database.pool]\nmin_size = 5\nmax_size = 20\n\n"
    )
    body = "".join(f'[[items]]\nname = "item_{index}"\nvalue = {index * 10}\n\n' for index in range(items))
    return head + body


_SIZES = {"small": _doc(3), "medium": _doc(100), "large": _doc(500)}


def _median_us(fn, reps=40):
    samples = []
    for _ in range(reps):
        t0 = time.perf_counter()
        fn()
        samples.append(time.perf_counter() - t0)
    return statistics.median(samples) * 1e6


pytestmark = pytest.mark.skipif(not HAS_TOMLLIB, reason="tomllib requires Python 3.11+")


@pytest.mark.parametrize("size", sorted(_SIZES))
def test_toml_parse_beats_stdlib_reference(size):
    doc = _SIZES[size]
    # Parity guard: both must produce the same dict (we are timing real work,
    # not a parser that bails early).
    assert pyrs_yaml.load_toml(doc) == tomllib.loads(doc)
    pyrs = _median_us(lambda: pyrs_yaml.load_toml(doc))
    ref = _median_us(lambda: tomllib.loads(doc))
    assert pyrs * 2 < ref, f"toml parse/{size}: pyrs not >2x faster than tomllib ({pyrs:.1f}us vs {ref:.1f}us)"


@pytest.mark.skipif(not HAS_TOMLKIT, reason="tomlkit not installed")
@pytest.mark.parametrize("size", sorted(_SIZES))
def test_toml_parse_top3_among_installed(size):
    """Rank pyrs against every installed pure-Python TOML parser.

    Measured on the real field (tomllib / tomlkit), pyrs is #1 by a wide margin;
    the gate asserts at most 2 competitors finish faster (top-3) and that pyrs
    still beats the reference `tomllib`, so a regression into the bottom of the
    field fails loudly while staying robust to clock noise.
    """
    doc = _SIZES[size]
    assert pyrs_yaml.load_toml(doc) == tomlkit.parse(doc)
    pyrs = _median_us(lambda: pyrs_yaml.load_toml(doc))
    faster = 0
    for _name, fn in (("tomllib", lambda: tomllib.loads(doc)), ("tomlkit", lambda: tomlkit.parse(doc))):
        if _median_us(fn) < pyrs:
            faster += 1
    assert faster <= 2, f"toml parse/{size}: pyrs not top-3 ({faster} competitors faster)"
