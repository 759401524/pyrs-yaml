"""Cross-library validation of the native TOML loader against tomlkit.

Objective §测试覆盖 3 names `tomlkit` as an oracle target. Previously
tomlkit only appeared in `test_benchmark_crosslib.py` and
`test_toml_leaderboard.py` -- never as a correctness parity check. The
existing `test_toml_matches_tomllib` covers stdlib tomllib; this module
adds the tomlkit dimension, mirroring the pattern from
`test_json_crosslib.py` for JSON/JSONC/JSON5.

tomlkit's parse result is a style-preserving `TOMLDocument` whose items
(`Integer` / `String` / `Float` / `Array` / ...) compare equal to their
plain-Python counterparts under `==`, so `dict(tomlkit.parse(src))`
interops directly with `pyrs_yaml.load_toml(src)`.

Two deliberate pyrs divergences from the tomlkit/tomllib pair are
asserted explicitly rather than hidden:

1. **Out-of-i64 integers.** TOML v1.0 spec §Integers: "Unless specified
   otherwise, integers are 64-bit signed". pyrs rejects with a typed
   `YamlParseError`; tomlkit/tomllib silently yield an arbitrary-
   precision int (technically out-of-spec). Pinned as spec-correct
   strictness, not a bug.

2. **Binary `0b…` integers.** pyrs accepts and emits decimal on
   serialize (YAML-hub limitation, ROADMAP §Known Engine Boundaries);
   on the load side, the value is the same int, so parity holds here
   even though source-text style is not preserved through the round trip.

Deps are optional (`skipif(tomlkit is None)`) so the suite runs cleanly
where tomlkit is not installed.
"""

from __future__ import annotations

import datetime as dt

import pytest

import pyrs_yaml

try:
    import tomlkit
except ImportError:  # pragma: no cover - optional dev dep
    tomlkit = None

pytestmark = pytest.mark.skipif(tomlkit is None, reason="tomlkit not installed")


TOML_PARITY_DOCS: list[tuple[str, dict]] = [
    # primitives, arrays, tables, datetimes, radix integers, escapes
    ('a = 1\nb = "x"\nc = [1, 2, 3]\nd = true\ne = 1.5\n', {"a": 1, "b": "x", "c": [1, 2, 3], "d": True, "e": 1.5}),
    ('[s]\nk = "v"\n', {"s": {"k": "v"}}),
    (
        "when = 2024-01-01T12:00:00Z\n",
        {"when": dt.datetime(2024, 1, 1, 12, 0, 0, tzinfo=dt.timezone.utc)},
    ),
    ("date_only = 2024-01-01\n", {"date_only": dt.date(2024, 1, 1)}),
    ("time_only = 07:32:00\n", {"time_only": dt.time(7, 32)}),
    ("hex = 0xFF\noct = 0o755\nbin = 0b1010\n", {"hex": 255, "oct": 493, "bin": 10}),
    ('esc = "a\\tb\\u00e9"\n', {"esc": "a\tb\u00e9"}),
    ('multi = """line1\nline2"""\n', {"multi": "line1\nline2"}),
    (
        "arrs = [[1, 2], [3, 4]]\nnested = { x = 1, y = 2 }\n",
        {"arrs": [[1, 2], [3, 4]], "nested": {"x": 1, "y": 2}},
    ),
    ("under = 1_000_000\n", {"under": 1_000_000}),
    ("exp = 5e+22\nneg = -2.5e-10\n", {"exp": 5e22, "neg": -2.5e-10}),
]


@pytest.mark.parametrize("src,expected", TOML_PARITY_DOCS)
def test_load_toml_matches_tomlkit(src, expected):
    """Byte-for-byte value parity with tomlkit.parse() on every documented
    construct in the corpus. tomlkit wraps values in style-preserving
    subclasses that compare equal to plain Python primitives under ==."""
    got = pyrs_yaml.load_toml(src)
    assert got == expected
    assert dict(tomlkit.parse(src)) == expected
    assert got == dict(tomlkit.parse(src))


@pytest.mark.parametrize("src,expected", TOML_PARITY_DOCS)
def test_tomlkit_and_tomllib_agree_with_pyrs(src, expected):
    """Three-way parity: pyrs, tomlkit, and stdlib tomllib (3.11+) all
    agree on the canonical TOML domain."""
    tomllib = pytest.importorskip("tomllib")
    assert tomllib.loads(src) == expected
    assert dict(tomlkit.parse(src)) == expected
    assert pyrs_yaml.load_toml(src) == expected


def test_out_of_i64_int_is_a_documented_divergence():
    """TOML v1.0 spec §Integers: 64-bit signed. pyrs rejects out-of-range
    integers as spec-strict; tomlkit/tomllib accept them as arbitrary-
    precision int (a documented leniency). Pin the divergence rather
    than silently drift toward the lenient side."""
    src = "big = 99999999999999999999999\n"
    with pytest.raises(pyrs_yaml.YamlParseError):
        pyrs_yaml.load_toml(src)
    # Lenient references accept (out of spec).
    assert dict(tomlkit.parse(src)) == {"big": 99999999999999999999999}


def test_negative_i64_boundary_is_accepted():
    """The legal minimum i64 (-2^63) must be accepted, matching the fix
    in PR #174 (`fix(toml): accept the legal minimum i64 integer`)."""
    src = "m = -9223372036854775808\n"
    got = pyrs_yaml.load_toml(src)
    assert got == {"m": -(2**63)}
    assert dict(tomlkit.parse(src)) == got
