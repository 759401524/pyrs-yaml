"""toml-test conformance runner (official language-agnostic TOML corpus).

Handled the same way as ``test_yaml_suite.py``: the corpus is an untracked local
dev artifact (``Reference/toml-test``, excluded via ``.git/info/exclude``), so
every test ``skipif`` the directory is absent. toml-test is a *decode* conformance
suite - each ``valid/<dir>/<name>.toml`` pairs with a ``<name>.json`` describing
the decoded data using TOML type tags (``{"type": "integer", "value": "42"}``,
``datetime``/``datetime-local``/``date-local``/``time-local``), and each
``invalid/<...>.toml`` must be rejected. The adapter below maps ``load_toml``
output to that tagged form for comparison.

The threshold gates are honest floors pinned to the *measured* compliance of the
current parser, not aspirational 100% - they exist to stop regression, and the
known gap categories (TOML 1.1 super-table re-opening, control-character and
float-underscore strictness, inline-table duplicate keys) are tracked on the
ROADMAP rather than silently excused.
"""

import json
from datetime import date, datetime, time
from pathlib import Path

import pytest

import pyrs_yaml

SUITE_DIR = Path("Reference/toml-test/tests")

pytestmark = pytest.mark.skipif(
    not SUITE_DIR.exists(),
    reason="toml-test corpus not found (clone https://github.com/toml-lang/toml-test into Reference/)",
)


def _iter_valid():
    for toml in sorted((SUITE_DIR / "valid").rglob("*.toml")):
        js = toml.with_suffix(".json")
        if js.exists():
            yield toml, js


def _iter_invalid():
    yield from sorted((SUITE_DIR / "invalid").rglob("*.toml"))


def _decode_expected(node):
    """toml-test expected JSON -> canonical comparable form."""
    if isinstance(node, dict):
        if set(node) == {"type", "value"}:
            return (node["type"], node["value"])
        return {k: _decode_expected(v) for k, v in node.items()}
    if isinstance(node, list):
        return [_decode_expected(v) for v in node]
    return node  # plain str / bool


def _encode_actual(val):
    """pyrs load_toml output -> the same tagged canonical form."""
    if isinstance(val, bool):
        return ("bool", "true" if val else "false")
    if isinstance(val, int):
        return ("integer", str(val))
    if isinstance(val, float):
        return ("float", val)
    if isinstance(val, datetime):
        kind = "datetime" if val.tzinfo is not None else "datetime-local"
        return (kind, val.isoformat())
    if isinstance(val, date):
        return ("date-local", val.isoformat())
    if isinstance(val, time):
        return ("time-local", val.isoformat())
    if isinstance(val, str):
        return ("string", val)
    if isinstance(val, list):
        return [_encode_actual(v) for v in val]
    if isinstance(val, dict):
        return {k: _encode_actual(v) for k, v in val.items()}
    return val


def _canon_temporal(v):
    """Normalize RFC 3339 spelling: 'Z'/'z' -> '+00:00', strip trailing micro 0s."""
    s = str(v).replace("Z", "+00:00").replace("z", "+00:00")
    if "." in s:
        head, _, rest = s.partition(".")
        frac, tz = rest, ""
        for i, ch in enumerate(rest):
            if ch in "+-":
                frac, tz = rest[:i], rest[i:]
                break
        frac = frac.rstrip("0")
        s = head + ("." + frac if frac else "") + tz
    return s


def _matches(expected, actual):
    if isinstance(expected, tuple) and isinstance(actual, tuple):
        te, ve = expected
        ta, va = actual
        if te in ("integer", "float"):
            try:
                return float(ve) == float(va)
            except (ValueError, TypeError):
                pass
        if te in ("datetime", "datetime-local", "date-local", "time-local"):
            return te == ta and _canon_temporal(ve) == _canon_temporal(va)
        return te == ta and ve == va
    if isinstance(expected, float) or isinstance(actual, float):
        try:
            return float(expected) == float(actual)
        except (ValueError, TypeError):
            return False
    if isinstance(expected, dict) and isinstance(actual, dict):
        return set(expected) == set(actual) and all(_matches(expected[k], actual[k]) for k in expected)
    if isinstance(expected, list) and isinstance(actual, list):
        return len(expected) == len(actual) and all(_matches(a, b) for a, b in zip(expected, actual))
    return expected == actual


def test_toml_test_valid_decodes_match():
    """Valid TOML parses and decodes to the expected tagged values (>=85%)."""
    total = 0
    matched = 0
    for toml, js in _iter_valid():
        total += 1
        try:
            expected = _decode_expected(json.loads(js.read_text(encoding="utf-8")))
            actual = _encode_actual(pyrs_yaml.load_toml(toml.read_text(encoding="utf-8")))
        except Exception:
            continue
        if _matches(expected, actual):
            matched += 1
    assert total > 0, "no valid toml-test cases found"
    rate = matched / total
    assert rate >= 0.85, f"valid decode-match rate {rate:.1%} ({matched}/{total}) below floor 85%"


def test_toml_test_invalid_rejected():
    """Invalid TOML is rejected rather than silently accepted (>=80%).

    A document is "rejected" when loading raises - either a typed
    ``YamlParseError`` or any conversion-time error (a bare ``ValueError`` from
    the temporal plugin is still a rejection, not a silent accept).
    """
    total = 0
    rejected = 0
    for toml in _iter_invalid():
        total += 1
        try:
            pyrs_yaml.load_toml(toml.read_text(encoding="utf-8"))
        except Exception:
            rejected += 1
    assert total > 0, "no invalid toml-test cases found"
    rate = rejected / total
    assert rate >= 0.80, f"invalid rejection rate {rate:.1%} ({rejected}/{total}) below floor 80%"


def test_toml_test_datetime_never_crashes():
    """Regression: temporal values decode via typed plugins, never a leaked error.

    toml-test surfaced that local-time (``07:32:00``) and lowercase-delimiter
    offset datetimes (``1987-07-05t17:45:00z``) reached ``datetime.fromisoformat``
    and raised a raw ``ValueError`` - a crash on *valid* TOML. A clean
    ``YamlParseError`` on a genuinely unsupported shape (e.g. TOML 1.1
    super-table re-opening) is a typed limitation, not a crash, so it is allowed.
    """
    for toml, _js in _iter_valid():
        try:
            pyrs_yaml.load_toml(toml.read_text(encoding="utf-8"))
        except pyrs_yaml.YamlParseError:
            continue  # typed parse error on an unsupported-but-finite grammar
        except ValueError as exc:  # the plugin-crash class we pinned
            pytest.fail(f"valid TOML {toml.relative_to(SUITE_DIR)} raised {type(exc).__name__}: {exc}")
