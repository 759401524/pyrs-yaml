"""Property-based fuzzing (Hypothesis) of the four non-YAML spokes.

Pillar 2.4 of the objective asks for Hypothesis (Python) *and* proptest (Rust)
fuzzing across all five formats. The Rust side fuzzes the dialects in
``fmt_pbt.rs``; until now the Python-side property tests only exercised the
YAML pipeline (``safe_dump`` / ``safe_load``). This module fuzzes the binding's
dialect surface over random JSON-compatible structures and cross-checks with
external oracles: stdlib ``json`` (JSON/JSONC/JSON5 parsing), ``tomllib``
(TOML parsing, 3.11+) and ``pyjson5`` (JSON5 parsing, optional).

Comparisons are **type-strict**: ``True == 1`` and ``1 == 1.0`` in Python, so a
plain ``==`` would hide bool/int and int/float drift introduced by a writer or
schema resolver. ``strict_eq`` pins the scalar *kind* at every position.
"""

import json
import math

import pytest
from hypothesis import HealthCheck, given, settings
from hypothesis import strategies as st

import pyrs_yaml
from tests.strategies import roundtrip_safe_text

pytestmark = pytest.mark.slow

try:  # optional reference parser, same guard style as test_json_crosslib
    import pyjson5
except ImportError:  # pragma: no cover
    pyjson5 = None

# ── strategies ───────────────────────────────────────────────────────────────

json_leaf = st.one_of(
    st.none(),
    st.booleans(),
    st.integers(min_value=-(10**12), max_value=10**12),
    st.floats(allow_nan=False, allow_infinity=False, width=64),
    roundtrip_safe_text,
)


def _bare_risk(s):
    """Strings a JSON/JSON5 writer may emit *bare* even when the AST holds a
    string, because the spelling is number-grammar in JSON but string under
    the YAML core schema (which caps plain ints at i64 and does not read
    `Infinity`/`NaN` as floats). Two documented fidelity contracts collide:
    `from_json -> to_json` keeps big-digit spellings verbatim (precision
    preservation), and the JSON5 writer keeps JSON5-only spellings verbatim
    (PR #120/#121 idempotency). An identically-spelled *string* passing
    through the hub therefore re-reads as a number: a known, pinned
    limitation (see test_json5_ambiguous_spellings_are_bare and
    test_big_digit_string_spelling_is_verbatim). Disambiguating needs an
    AST-level marker on JSON5/overflow number literals — a representation
    change requiring sign-off — so fuzz strategies exclude these spellings
    from string positions instead of asserting cross-library equality on
    them.
    """
    if s in ("Infinity", "-Infinity", "+Infinity", "NaN"):
        return True
    body = s[1:] if s[:1] in "+-" else s
    if body[:2] in ("0x", "0X") and len(body) > 2 and all(c in "0123456789abcdefABCDEF" for c in body[2:]):
        return True
    if body.startswith(".") and body[1:].isdigit():
        return True
    if body.endswith(".") and body[:-1].isdigit():
        return True
    try:
        complex(s)  # cheap superset check: rejects words, accepts number grammars
        return True
    except ValueError:
        return False


def _safe_text():
    # `<<` keys join the `_bare_risk` carve-out: a mapping-valued `<<` is
    # reserved merge-extension syntax the YAML hub consumes by design
    # (PR #187 contract), so cross-library equality domains must not let
    # hypothesis generate it as a dictionary key.
    return roundtrip_safe_text.filter(lambda v: not (isinstance(v, str) and (_bare_risk(v) or v == "<<")))


json_leaf = json_leaf.filter(lambda v: not (isinstance(v, str) and _bare_risk(v)))

# Container-rooted documents: load_jsonc/load_json5 return dict|list, and
# to_toml requires a mapping root.
json_doc = st.recursive(
    json_leaf,
    lambda children: st.one_of(
        st.lists(children, max_size=6),
        st.dictionaries(_safe_text(), children, max_size=6),
    ),
    max_leaves=30,
).filter(lambda v: isinstance(v, (dict, list)))

# TOML has no null: exclude None from every position of the dialect strategy.
toml_leaf = st.one_of(
    st.booleans(),
    st.integers(min_value=-(2**63), max_value=2**63 - 1),
    st.floats(allow_nan=False, allow_infinity=False, width=64),
    roundtrip_safe_text,
)

toml_doc = st.dictionaries(
    _safe_text(),
    st.recursive(
        toml_leaf.filter(lambda v: not (isinstance(v, str) and _bare_risk(v))),
        lambda children: st.one_of(
            st.lists(children, max_size=5),
            st.dictionaries(_safe_text(), children, max_size=5),
        ),
        max_leaves=25,
    ),
    min_size=1,
    max_size=6,
)

# ── type-strict comparison ───────────────────────────────────────────────────


def _kind(v):
    if isinstance(v, bool):
        return "bool"
    if isinstance(v, int):
        return "int"
    if isinstance(v, float):
        return "float"
    if isinstance(v, str):
        return "str"
    if v is None:
        return "null"
    if isinstance(v, list):
        return "list"
    if isinstance(v, dict):
        return "dict"
    return type(v).__name__


def strict_eq(a, b, order_matters=True):
    """Compare two decoded values without letting True==1 / 1==1.0 hide drift."""
    ka, kb = _kind(a), _kind(b)
    if ka != kb:
        return False
    if ka == "dict":
        if len(a) != len(b):
            return False
        if order_matters:
            if list(a.keys()) != list(b.keys()):
                return False
            return all(strict_eq(x, y, True) for x, y in zip(a.values(), b.values()))
        if set(a.keys()) != set(b.keys()):
            return False
        return all(strict_eq(a[k], b[k], False) for k in a)
    if ka == "list":
        return len(a) == len(b) and all(strict_eq(x, y, order_matters) for x, y in zip(a, b))
    return a == b or (ka == "float" and math.isnan(a) and math.isnan(b))


# ── JSON / JSONC: stdlib json as the external oracle ─────────────────────────

MAX = dict(max_examples=200, deadline=None, suppress_health_check=[HealthCheck.too_slow])


@settings(**MAX)
@given(json_doc)
def test_load_jsonc_matches_stdlib_json(value):
    # The strict-JSON fast path must decode exactly what the stdlib decodes.
    text = json.dumps(value)
    assert strict_eq(pyrs_yaml.load_jsonc(text), json.loads(text))


@settings(**MAX)
@given(json_doc)
def test_load_json_matches_stdlib_json(value):
    # Same contract as load_jsonc but through the STRICT RFC 8259 loader.
    # `json.dumps` only emits canonical strict JSON (no comments, no trailing
    # commas, no NaN/Infinity — `allow_nan=False` behaviour is what we get by
    # using the default encoder on finite-value strategies), so the strict
    # loader must produce the identical value to the stdlib decoder. This
    # pins the fast path + AST fallback equivalence for the whole `json_doc`
    # domain, guarding against silent drift between the two loaders.
    text = json.dumps(value)
    assert strict_eq(pyrs_yaml.load_json(text), json.loads(text))


@settings(**MAX)
@given(json_doc)
def test_load_json_matches_load_jsonc_on_strict_domain(value):
    # Both loaders must agree on every strict-JSON document (the widening
    # surface of `load_jsonc` is only visible on JSONC/JSON5-only inputs,
    # which `json.dumps` never emits).
    text = json.dumps(value)
    assert strict_eq(pyrs_yaml.load_json(text), pyrs_yaml.load_jsonc(text))


@settings(**MAX)
@given(json_doc)
def test_from_json_hub_fidelity(value):
    # JSON -> YAML hub conversion must preserve every typed value.
    text = json.dumps(value)
    yaml_text = pyrs_yaml.from_json(text)
    assert strict_eq(pyrs_yaml.safe_load(yaml_text), json.loads(text))


@settings(**MAX)
@given(json_doc)
def test_to_json_writes_what_stdlib_can_read(value):
    # Our strict-JSON writer re-read by the stdlib oracle must equal the input.
    text = pyrs_yaml.parse(pyrs_yaml.safe_dump(value)).to_json()
    assert strict_eq(json.loads(text), value)


@settings(**MAX)
@given(json_doc)
def test_jsonc_writer_roundtrip(value):
    text = pyrs_yaml.parse(pyrs_yaml.safe_dump(value)).to_jsonc()
    assert strict_eq(pyrs_yaml.load_jsonc(text), value)


@settings(**MAX)
@given(json_doc)
def test_json5_writer_roundtrip(value):
    text = pyrs_yaml.parse(pyrs_yaml.safe_dump(value)).to_json5()
    assert strict_eq(pyrs_yaml.load_json5(text), value)


@settings(**MAX)
@given(json_doc)
def test_json5_matches_pyjson5(value):
    if pyjson5 is None:
        pytest.skip("pyjson5 not installed")
    text = pyrs_yaml.parse(pyrs_yaml.safe_dump(value)).to_json5()
    assert strict_eq(pyjson5.loads(text), value)


def test_json5_ambiguous_spellings_are_bare():
    """Pin the documented JSON5 string/number ambiguity (known limitation).

    A string that spells a JSON5-only number (`Infinity` / `NaN`) survives
    `safe_dump` as a *plain* scalar, which the JSON5 writer emits bare: the
    AST cannot distinguish it from a JSON5 number literal. `-Infinity`,
    `+Infinity` and `NaN` are quoted by the YAML dumper itself, so only the
    exact `Infinity` spelling corrupts through the hub. If the disambiguation
    (tagged JSON5 number literals) ever lands, update this test's expectation
    and drop the strategy filter above.
    """
    text = pyrs_yaml.parse(pyrs_yaml.safe_dump(["Infinity"])).to_json5()
    assert "Infinity" in text and '"Infinity"' not in text
    # Known consequence: reading it back yields the number, not the string.
    assert pyrs_yaml.load_json5(text) == [float("inf")]


def test_big_digit_string_spelling_is_verbatim():
    """Pin the documented big-integer fidelity contract (known limitation).

    Plain scalars cap ints at i64 under the schema, so a 19-digit string is
    a string and the YAML dumper leaves it unquoted. The JSON writer emits
    number-grammar text verbatim (the documented `from_json -> to_json`
    precision-preserving contract), so the bare spelling round-trips through
    the hub. Our own loader keeps the overflow as a string; stdlib json
    reads it as an arbitrary-precision int — the oracle divergence is why
    the fuzz strategies skip these spellings (see `_bare_risk`).
    """
    big = "9223372036854775808"
    text = pyrs_yaml.parse(pyrs_yaml.safe_dump([big])).to_json()
    assert str(big) in text and f'"{big}"' not in text
    assert pyrs_yaml.load_jsonc(text) == [big]  # our loader: i64 overflow -> str
    assert json.loads(text) == [int(big)]  # stdlib oracle: arbitrary-precision int


# ── TOML: tomllib as the external parse oracle (3.11+) ───────────────────────


@settings(**MAX)
@given(toml_doc)
def test_toml_roundtrip_via_pyrs(value):
    out = pyrs_yaml.to_toml(pyrs_yaml.safe_dump(value))
    # TOML may regroup scalars before sub-tables, so key order is not asserted.
    assert strict_eq(pyrs_yaml.load_toml(out), value, order_matters=False)


@settings(**MAX)
@given(toml_doc)
def test_toml_matches_tomllib(value):
    tomllib = pytest.importorskip("tomllib")
    out = pyrs_yaml.to_toml(pyrs_yaml.safe_dump(value))
    assert strict_eq(tomllib.loads(out), value, order_matters=False)
