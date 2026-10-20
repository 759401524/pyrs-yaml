"""Cross-library validation of the native JSONC/JSON5 loaders.

The JSONC/JSON5 engines previously had no *external* comparison (the leaderboard
gates cover stdlib `json`; `orjson`/`ujson`/`rapidjson` do not parse JSONC/JSON5).
`pyjson5` is a C-accelerated JSON5 reference (a superset of JSONC), so it fills
the "跨库对比验证" gap for the two dialects that had none: pyrs and pyjson5 must
agree on the *value* produced for every construct the dialects allow.

Known, intentional pyrs divergences from a reference parser are asserted
explicitly (not hidden): out-of-i64 integers are preserved as their source text.

The guards are per comparison, not per file, because the file-level one was a blind spot: `orjson` and
`pyjson5` are both declared `python_version >= '3.9' and python_version < '3.14'`, so a blanket
`pytestmark` skipped the strict-JSON comparisons on the 3.8 floor this package promises and on 3.14+,
where nothing about them needs either reader. Where `pyjson5` has no wheel the JSON5 and JSONC
comparisons still skip cleanly, each carrying its own `needs_pyjson5`; `json` from the standard library
- an RFC 8259 reader this repository does not own and every leg has - keeps the strict dialect compared
everywhere. The two guarded dialects are what `scripts/quality_matrix.py` still reports as restricted.
"""

import json
import math

import pytest

import pyrs_yaml

try:
    import pyjson5
except ImportError:  # pragma: no cover - platform without a wheel
    pyjson5 = None

try:
    import orjson
except ImportError:  # pragma: no cover - optional dev dep
    orjson = None

needs_pyjson5 = pytest.mark.skipif(pyjson5 is None, reason="pyjson5 (JSON5 reference) not installed")

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
@needs_pyjson5
def test_load_json5_matches_pyjson5(doc):
    assert pyrs_yaml.load_json5(doc) == pyjson5.loads(doc)


@pytest.mark.parametrize("doc", JSONC_DOCS)
@needs_pyjson5
def test_load_jsonc_matches_pyjson5(doc):
    assert pyrs_yaml.load_jsonc(doc) == pyjson5.loads(doc)


@needs_pyjson5
def test_special_floats_match_pyjson5():
    # Infinity / -Infinity are IEEE doubles both libraries agree on.
    assert pyrs_yaml.load_json5("{i: Infinity, j: -Infinity}") == pyjson5.loads("{i: Infinity, j: -Infinity}")
    # NaN never equals itself; compare by predicate, not ==.
    a = pyrs_yaml.load_json5("{n: NaN}")["n"]
    b = pyjson5.loads("{n: NaN}")["n"]
    assert math.isnan(a) and math.isnan(b)


@needs_pyjson5
def test_out_of_i64_int_is_a_documented_divergence():
    # Reference parser yields an arbitrary-precision int; pyrs deliberately
    # preserves the source text. This pins the known divergence, not a bug.
    doc = '{"big": 123456789012345678901234567890}'
    assert isinstance(pyjson5.loads(doc)["big"], int)
    assert pyrs_yaml.load_json5(doc)["big"] == "123456789012345678901234567890"


@needs_pyjson5
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


@needs_pyjson5
def test_json5_hex_escape_is_accepted_and_does_not_leak_into_the_narrower_dialects():
    # The JSON5 escape set includes `\x` followed by exactly two hex digits. The reader had `\v`,
    # `\0` and the line continuation but not this one, so `load_json5` rejected a document the
    # reference accepts while its own docstring promised "the full JSON5 grammar".
    doc = r"{a: '\x41', b: " + r'"c\x7ed"}'
    assert pyrs_yaml.load_json5(doc) == pyjson5.loads(doc) == {"a": "A", "b": "c~d"}
    # Strict JSON and JSONC share the scanner and must keep rejecting it.
    with pytest.raises(pyrs_yaml.YamlParseError):
        pyrs_yaml.load_jsonc(r'{"a": "\x41"}')
    with pytest.raises(pyrs_yaml.YamlParseError):
        pyrs_yaml.load_json(r'{"a": "\x41"}')


# ── orjson as a strict-JSON oracle (better than json.loads) ────────────────
# The stdlib `json.loads` is NOT a perfect RFC 8259 oracle — under its default
# `allow_nan=True` it accepts the three non-standard literals `NaN` /
# `Infinity` / `-Infinity`. orjson, being RFC-strict, rejects all three with
# the same signal shape (JSONDecodeError) that a strict parser should. This
# section uses orjson as the external authority for the *strict* loader
# `load_json` — objective §测试覆盖 3 「逐位比对」 is now enforced against
# the strongest available oracle, not just stdlib json.
#
# The two corpora below are asked of both readers: orjson where it is installed, and stdlib `json`
# with the non-standard literals switched off everywhere. The second is what keeps the strict dialect
# compared on the legs `python_version >= '3.9' and python_version < '3.14'` excludes.


def _refuse_non_rfc8259(name):
    """The `parse_constant` hook that makes stdlib `json` a strict reader.

    `json.loads` accepts `NaN`, `Infinity` and `-Infinity` under its default `allow_nan=True`, which is
    a json-py behaviour and not RFC 8259, so the reader is not a strict oracle as imported.
    `parse_constant` is called for exactly those three names, and refusing one is what makes the
    stdlib agree with orjson about the grammar without a dependency a floor interpreter cannot install.
    """
    raise ValueError(f"not RFC 8259: {name}")


STRICT_JSON_DOCS = [
    '{"a":1,"b":[1,2,3]}',
    "[[1,2],[3,4]]",
    '"hello"',
    "null",
    "true",
    "false",
    "123",
    "-0",
    "1.5e10",
    "-0.25",
    r'{"k":"a\nb"}',
    r'{"k":"a\tb"}',
    r'{"k":"\u00e9"}',
    r'{"k":"\ud83d\ude00"}',
    '{"k":"café 🌄"}',
    '{"a": 1, "b": 2, "a": 3}',  # duplicate key: last wins, first position
]

STRICT_JSON_REJECTIONS = [
    "NaN",
    "Infinity",
    "-Infinity",
    '{"a":1 // hi\n}',
    '{"a":1 /* hi */}',
    '{"a":1,}',
    "[1,2,]",
    "{'a':1}",
    "0x1F",
    "+.5",
    "5.",
    "[01]",
    r'{"a": "\x41"}',  # a JSON5 escape, not a JSON one
]


@pytest.mark.skipif(orjson is None, reason="orjson not installed")
@pytest.mark.parametrize("doc", STRICT_JSON_DOCS)
def test_load_json_matches_orjson(doc):
    # Byte-for-byte value parity with orjson.loads on every strict-JSON
    # construct in the corpus. `orjson.loads` accepts `str | bytes`, so we
    # pass the same text to both sides.
    assert pyrs_yaml.load_json(doc) == orjson.loads(doc)


@pytest.mark.parametrize("doc", STRICT_JSON_DOCS)
def test_load_json_matches_the_stdlib_reader(doc):
    assert pyrs_yaml.load_json(doc) == json.loads(doc, parse_constant=_refuse_non_rfc8259)


@pytest.mark.skipif(orjson is None, reason="orjson not installed")
@pytest.mark.parametrize("doc", STRICT_JSON_REJECTIONS)
def test_load_json_rejects_what_orjson_rejects(doc):
    # Every non-strict form both loaders reject. orjson raises JSONDecodeError
    # (subclass of ValueError) which is what json.loads raises too; pyrs
    # surfaces a typed YamlParseError carrying the same rejection signal.
    with pytest.raises(orjson.JSONDecodeError):
        orjson.loads(doc)
    with pytest.raises(pyrs_yaml.YamlParseError):
        pyrs_yaml.load_json(doc)


@pytest.mark.parametrize("doc", STRICT_JSON_REJECTIONS)
def test_load_json_rejects_what_the_stdlib_reader_rejects(doc):
    with pytest.raises(ValueError):
        json.loads(doc, parse_constant=_refuse_non_rfc8259)
    with pytest.raises(pyrs_yaml.YamlParseError):
        pyrs_yaml.load_json(doc)


@pytest.mark.skipif(orjson is None, reason="orjson not installed")
def test_orjson_is_stricter_than_stdlib_json_on_bare_literals():
    # Documents why `load_json` uses orjson (not json.loads) as the strictness
    # oracle: the stdlib accepts `NaN` / `Infinity` / `-Infinity` under its
    # default `allow_nan=True`, which is a json-py historical behaviour and
    # NOT RFC 8259. pyrs's loader sides with the spec, matching orjson.
    for tok in ("NaN", "Infinity", "-Infinity"):
        json.loads(tok)  # stdlib accepts
        with pytest.raises(orjson.JSONDecodeError):
            orjson.loads(tok)
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.load_json(tok)


@pytest.mark.skipif(orjson is None, reason="orjson not installed")
def test_load_jsonc_widens_only_over_orjson_domain():
    # `load_jsonc` accepts comments (orjson rejects them) — the widening is
    # JSONC's raison d'etre; strict `load_json` continues to reject.
    doc = '{"a": 1 // note\n}'
    assert pyrs_yaml.load_jsonc(doc) == {"a": 1}
    with pytest.raises(pyrs_yaml.YamlParseError):
        pyrs_yaml.load_json(doc)
    with pytest.raises(orjson.JSONDecodeError):
        orjson.loads(doc)
