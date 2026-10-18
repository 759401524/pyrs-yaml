"""Every Unicode whitespace code point, in key and value position, through the JSON family.

The hub stores a decoded JSON string as a YAML scalar, and the scalar is plain unless
`needs_quotes` says otherwise; the reader then *trims* a plain scalar with Unicode rules before
resolving it. So a text that starts or ends with a non-ASCII space used to lose that character,
and a text that is nothing but one resolved to null: measured on the released engine,
`load_json5` of a document whose key is U+2028 alone answered `{None: 1}` while `load_json` kept
the string and `pyjson5` agreed with it. The rule now covers both ends of every whitespace code
point, and this table is what keeps it there.
"""

import json

import pytest

import pyrs_yaml

try:
    import pyjson5
except ImportError:  # pragma: no cover - platform without a wheel
    pyjson5 = None

# Only the one test below needs the reference reader; the rest ask pyrs alone, so a leg without a
# pyjson5 wheel (3.8 and 3.15t) skips that test rather than the whole table.
needs_pyjson5 = pytest.mark.skipif(pyjson5 is None, reason="pyjson5 (JSON5 reference) not installed")

# The set is derived from the property the engine trims on rather than transcribed, so a Unicode
# update moves this list and the Rust sweep together instead of one of them silently. A lone
# surrogate is not whitespace for Python, so none can enter this list by accident.
WHITESPACE = [chr(cp) for cp in range(0x110000) if chr(cp).strip() == ""]

QUOTES = {
    "json5": pyrs_yaml.load_json5,
    "json": pyrs_yaml.load_json,
    "jsonc": pyrs_yaml.load_jsonc,
}


def spelled(text):
    """The JSON document `{"<text>": "<text>"}` with `text` escaped the way JSON spells it."""
    return "{" + json.dumps(text) + ": " + json.dumps(text) + "}"


def test_the_whitespace_set_is_the_one_the_engine_trims_on():
    # Rust's `char::is_whitespace` is the Unicode White_Space property, 25 code points: the ASCII
    # tab-through-space run, NEL, NBSP, OGHAM SPACE MARK, U+2000-U+200A, LS, PS, NARROW NBSP,
    # MEDIUM MATHEMATICAL SPACE and IDEOGRAPHIC SPACE. Python's `strip` additionally removes the
    # four C0 separators U+001C-U+001F, which the engine already quotes as control characters -
    # so the table below covers the union, and the count names both halves.
    python_only_separators = {0x1C, 0x1D, 0x1E, 0x1F}
    extra = [c for c in WHITESPACE if ord(c) in python_only_separators]
    assert len(extra) == 4, [hex(ord(c)) for c in extra]
    assert len(WHITESPACE) - len(extra) == 25, [hex(ord(c)) for c in WHITESPACE]


@pytest.mark.parametrize("dialect", sorted(QUOTES))
@pytest.mark.parametrize("space", WHITESPACE)
def test_a_whitespace_text_keeps_its_own_spelling(dialect, space):
    for text in (space, space + "a", "a" + space):
        loaded = QUOTES[dialect](spelled(text))
        assert list(loaded.keys()) == [text], f"{dialect}: {text!r} read back as {list(loaded)}"
        assert loaded[text] == text, f"{dialect}: value {text!r} became {loaded[text]!r}"


@pytest.mark.parametrize("space", WHITESPACE)
@needs_pyjson5
def test_the_reference_agrees_that_the_key_is_a_string(space):
    # The claim being guarded is not "pyrs accepts this" but "the text is a key, not null":
    # the reference reader is what says so.
    doc = spelled(space)
    assert list(pyjson5.loads(doc)) == [space]
    assert list(json.loads(doc)) == [space]


@pytest.mark.parametrize("space", WHITESPACE)
def test_the_hub_round_trip_keeps_the_character_outside_a_null_fold(space):
    # from_json5 emits hub YAML; a plain scalar there is what used to collapse.
    text = space + "a"
    hub = pyrs_yaml.from_json5(spelled(text))
    assert pyrs_yaml.safe_load(hub) == {text: text}, hub
