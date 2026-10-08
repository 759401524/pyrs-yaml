"""Mapping keys must resolve to exactly what the same text resolves to as a value.

The engine resolved scalars on the value side and ignored the rule on the key side:
`1: a` loaded as ``{'1': 'a'}`` while every value of that same text is the integer 1,
and `~: 1` loaded with the string key ``'~'`` where PyYAML and ruamel both give
``{None: 1}``. Not a spelling difference - two documents that mean the same thing
produced two different objects, and a config with an integer or null key was
unreachable by lookup.

The property asserted first is the one the fix is built on: key resolution == value
resolution, inside this engine, for every text, with no third party involved. Then
parity is asserted *live* against both reference libraries where they agree with each
other; where they disagree (YAML 1.1 legacy forms) the shape is recorded with what our
schema resolves and which library dissents, so a future change argues with a record
instead of a guess.

Round-trip is untouched by all of this: the AST keeps the source spelling, so `~: 1`
still emits `~: 1`. That is why the fix belongs in the conversion step and nowhere
earlier.
"""

from __future__ import annotations

import math

import pytest
import yaml  # declared test dependency; a skipped gate is the failure mode, not a fallback

import pyrs_yaml

try:
    from ruamel.yaml import YAML

    _ruamel = YAML(typ="safe")
except Exception:  # pragma: no cover - ruamel is a declared test dependency
    _ruamel = None

# Texts whose resolution both reference libraries agree on, so parity is assertable.
AGREED = [
    ("~: 1", {None: 1}),
    ("null: 1", {None: 1}),
    ("1: a", {1: "a"}),
    ("true: a", {True: "a"}),
    ("3.5: a", {3.5: "a"}),
    (".inf: a", {float("inf"): "a"}),
    ("a: {~: 1}", {"a": {None: 1}}),
    ("~: ~", {None: None}),
    # A quoted key is a string by YAML's own rule, and must stay one.
    ("'~': a", {"~": "a"}),
    ("'1': a", {"1": "a"}),
]

# Where the references disagree with each other, this engine follows its own schema
# (YAML 1.2 core by default - the resolver its values already use). Pinned with the
# disagreement named.
SCHEMA_DIVERGENT = [
    # PyYAML implements YAML 1.1's bool set; ruamel agrees with us.
    ("yes: a", {"yes": "a"}, "PyYAML gives {True: 'a'} (1.1 bool)"),
    # 1.1 read a leading-zero integer as octal; 1.2 dropped it and PyYAML kept it.
    ("0755: a", {755: "a"}, "PyYAML gives {493: 'a'} (1.1 octal)"),
    # No 1.1 form for 0o, so PyYAML leaves it a string.
    ("0o17: a", {15: "a"}, "PyYAML gives {'0o17': 'a'}"),
    ("1.5e3: a", {1500.0: "a"}, "PyYAML gives {'1.5e3': 'a'} (1.1 wants a signed exponent)"),
    # Timestamps resolve for neither keys nor values here, so there is no asymmetry -
    # which is what the invariant test checks. Both references give datetime.date.
    ("2024-01-01: a", {"2024-01-01": "a"}, "both references give datetime.date(...)"),
]

# Every text the resolver has an opinion about, for the invariant.
RESOLVED_TEXTS = [
    "~",
    "null",
    "Null",
    "1",
    "-2",
    "3.5",
    "true",
    "False",
    "yes",
    "0755",
    "0o17",
    "0x1F",
    "1.5e3",
    ".inf",
    "inf",
    "-.inf",
    ".nan",
    "y",
    "n",
    "on",
    "2024-01-01",
    "'1'",
    '"~"',
    "text",
    "''",
]


def same(left, right) -> bool:
    """Equality that survives the two values no `==` relates: None and NaN.

    `None == None` is True but a dict key `None` must not be confused with the string,
    and `nan != nan`, so a plain `==` would fail the invariant for `.nan` while the
    engine was in fact resolving it correctly.
    """
    if (left is None) != (right is None):
        return False
    if left is None:
        return True
    if isinstance(left, float) and isinstance(right, float) and (math.isnan(left) or math.isnan(right)):
        return math.isnan(left) and math.isnan(right)
    return type(left) is type(right) and left == right


def test_format_bridges_keep_their_own_key_types():
    """A TOML or JSON key is a string by that grammar, and stays one through YAML.

    The shared AST has no marker for "string, do not resolve" except quoting - the same
    mechanism TOML uses for its string values - so the bridges quote exactly the keys a
    YAML reader would re-type, and leave every other key plain. Both halves are
    asserted: the load, and the emitted text that must read back identically.
    """
    loaded = pyrs_yaml.load_toml('"1" = 2\n"" = 3\nport = 4\nyes = 5\n')
    assert loaded == {"1": 2, "": 3, "port": 4, "yes": 5}
    text = pyrs_yaml.safe_dump(loaded)
    assert text == '"1": 2\n"": 3\nport: 4\nyes: 5\n'
    assert pyrs_yaml.safe_load(text) == loaded
    assert pyrs_yaml.load_json('{"1": 2, "": 3}') == {"1": 2, "": 3}
    assert pyrs_yaml.load_json5("{null: 1, 'x': 2}") == {"null": 1, "x": 2}


@pytest.mark.parametrize(("src", "want"), AGREED)
def test_the_two_load_routes_agree_on_key_types(src, want):
    """The fast granit-event route and the AST route must not disagree about a key.

    `safe_load` materializes Python objects straight from the event stream; `parse(...)
    .to_dict()` walks the AST. They are two implementations of one contract, and the
    key rule lives in both - so the same document has to give the same object either
    way, or the fix is only half landed.
    """
    assert pyrs_yaml.safe_load(src) == want
    assert pyrs_yaml.parse(src).to_dict() == want


@pytest.mark.parametrize("text", RESOLVED_TEXTS)
def test_a_key_resolves_exactly_like_the_same_text_as_a_value(text):
    as_value = pyrs_yaml.safe_load(f"k: {text}")["k"]
    document = pyrs_yaml.safe_load(f"{text}: v")
    keys = list(document)
    assert len(keys) == 1, f"{text!r} must stay one pair, got {document!r}"
    assert same(keys[0], as_value), f"{text!r}: key {keys[0]!r} but value {as_value!r} - one text, two types"


@pytest.mark.parametrize(("src", "want"), AGREED)
def test_typed_keys_match_pyyaml(src, want):
    assert yaml.safe_load(src) == want, "the reference moved; re-measure, do not restate"
    assert pyrs_yaml.safe_load(src) == want


@pytest.mark.parametrize(("src", "want"), AGREED)
@pytest.mark.skipif(_ruamel is None, reason="ruamel.yaml not installed")
def test_typed_keys_match_ruamel(src, want):
    assert _ruamel.load(src) == want, "the reference moved; re-measure, do not restate"
    assert pyrs_yaml.safe_load(src) == want


@pytest.mark.parametrize(("src", "want", "why"), SCHEMA_DIVERGENT)
def test_schema_divergent_keys_follow_our_values(src, want, why):
    """Our answer equals our own value resolution; the difference is the schema."""
    got = pyrs_yaml.safe_load(src)
    assert got == want, f"{src!r} -> {got!r}, expected {want!r}: {why}"


@pytest.mark.parametrize(
    "src",
    [src for src, _ in AGREED] + [src for src, _, _ in SCHEMA_DIVERGENT],
)
def test_typed_keys_round_trip_with_their_source_spelling(src):
    """Resolution happens on conversion, so emission still carries what was written."""
    expected = src if src.endswith("\n") else f"{src}\n"
    once = pyrs_yaml.parse(src).to_yaml()
    assert once == expected
    assert pyrs_yaml.parse(once).to_yaml() == once
