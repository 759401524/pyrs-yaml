"""Cross-library validation of the native JSONC/JSON5 loaders.

The JSONC/JSON5 engines previously had no *external* comparison (the leaderboard
gates cover stdlib `json`; `orjson`/`ujson`/`rapidjson` do not parse JSONC/JSON5).
`pyjson5` is a C-accelerated JSON5 reference (a superset of JSONC), so it fills
the "跨库对比验证" gap for the two dialects that had none: pyrs and pyjson5 must
agree on the *value* produced for every construct the dialects allow.

Known, intentional pyrs divergences from a reference parser are asserted
explicitly (not hidden): out-of-i64 integers are preserved as their source text.
Deps are optional and platform/version-guarded in pyproject, so the whole module
skips cleanly where `pyjson5` has no wheel.
"""

import math

import pytest

import pyrs_yaml

try:
    import pyjson5
except ImportError:  # pragma: no cover - platform without a wheel
    pyjson5 = None

pytestmark = pytest.mark.skipif(pyjson5 is None, reason="pyjson5 (JSON5 reference) not installed")

JSON5_DOCS = [
    "{a: 'x', b: 1,}",  # unquoted key, single quotes, trailing comma
    "{h: 0x1F, e: 1e3, f: .5, g: 5., p: +7}",  # hex / exponent / leading+trailing dot / plus
    "{nested: {list: [1, 'two', 3.5, true, null]}}",
    "[1, 2, 3,]",  # top-level array with trailing comma
    r"{u: 'caf\u00e9 \ud83d\ude00'}",  # \uXXXX + surrogate pair in a string
    r"{multi: 'a\nb\tc'}",  # escape sequences inside single quotes (escaped, not raw)
]

JSONC_DOCS = [
    '{"a": 1, // line comment\n "b": 2}',
    '{"a": 1 /* trailing block */}',
    '{\n  /* header */\n  "x": [1, 2]\n}',
]


@pytest.mark.parametrize("doc", JSON5_DOCS)
def test_load_json5_matches_pyjson5(doc):
    assert pyrs_yaml.load_json5(doc) == pyjson5.loads(doc)


@pytest.mark.parametrize("doc", JSONC_DOCS)
def test_load_jsonc_matches_pyjson5(doc):
    assert pyrs_yaml.load_jsonc(doc) == pyjson5.loads(doc)


def test_special_floats_match_pyjson5():
    # Infinity / -Infinity are IEEE doubles both libraries agree on.
    assert pyrs_yaml.load_json5("{i: Infinity, j: -Infinity}") == pyjson5.loads("{i: Infinity, j: -Infinity}")
    # NaN never equals itself; compare by predicate, not ==.
    a = pyrs_yaml.load_json5("{n: NaN}")["n"]
    b = pyjson5.loads("{n: NaN}")["n"]
    assert math.isnan(a) and math.isnan(b)


def test_out_of_i64_int_is_a_documented_divergence():
    # Reference parser yields an arbitrary-precision int; pyrs deliberately
    # preserves the source text. This pins the known divergence, not a bug.
    doc = '{"big": 123456789012345678901234567890}'
    assert isinstance(pyjson5.loads(doc)["big"], int)
    assert pyrs_yaml.load_json5(doc)["big"] == "123456789012345678901234567890"


def test_jsonc_is_stricter_than_json5_about_single_quoted_keys():
    # A single-quoted key is valid JSON5 but NOT valid JSON/JSONC. pyrs enforces
    # the JSONC grammar (rejects it) where the lenient JSON5 reference accepts it
    # - a deliberate spec-correctness divergence, pinned so it never regresses
    # into silent acceptance.
    doc = "{'x': 1}"
    assert pyjson5.loads(doc) == {"x": 1}  # JSON5 reference accepts
    with pytest.raises(pyrs_yaml.YamlParseError):
        pyrs_yaml.load_jsonc(doc)  # pyrs JSONC rejects, per the stricter grammar
    # ...while the JSON5 loader does accept it, matching the reference.
    assert pyrs_yaml.load_json5(doc) == pyjson5.loads(doc)
