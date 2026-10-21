"""The JSON5 grammar, production by production, against the reference implementation.

Runs in normal CI. This tier exists because the property and fuzz tiers cannot ask the question
this one asks: their oracles compare *paths that exist* - a round trip, a fixed point, a replay - so
a syntax the reader never accepted is invisible to them. It rejects, no test claimed it should work,
and nothing goes red. The gaps this table was written to keep finding are exactly that shape:
`load_json5` rejected the spec's `BS x HH` escape, and an escaped character in an unquoted key, and a
backslash before CRLF inside a single-quoted string, while the shipped stub said it "Accepts the full
JSON5 grammar".

So each row is one production, and the authority is not this repository: it is the pure-Python
`json5` package, which every tested leg installs. `pyjson5` is a second opinion where its wheel
exists - it is a C implementation and used to be the only reference here, but its markers keep it
off the 3.8 floor and off 3.14+, so a file that asked it alone judged those legs by nothing.
`ACCEPTS` rows assert that pyrs and `json5` take the document and decode it to the same value;
`REJECTS` rows assert that both refuse it - pyrs with its own typed error, so a refusal cannot
quietly become a panic. The `BS`/`chr` spelling is deliberate: the sequences under test must
reach the reader as source text rather than being decoded by this file first.

Two behaviours are measured divergences, not shared rows: `json5` keeps a `\ud83d\ude00` pair as
two UTF-16 code units where pyrs and `pyjson5` combine it into one character, and `json5` accepts a
lone surrogate as an identifier escape where pyrs and `pyjson5` refuse it. Both are pinned by their
own tests below, so the shared tables stay exact and the difference cannot be mistaken for a skip.

Where the hub deliberately represents a value differently from the reference - the non-finite words -
that is pinned by its own test below instead of being hidden in a skip.
"""

import math

import json5
import pytest

import pyrs_yaml

try:
    import pyjson5
except ImportError:  # pragma: no cover - platform without a wheel
    pyjson5 = None

needs_pyjson5 = pytest.mark.skipif(pyjson5 is None, reason="pyjson5 (second opinion) not installed")

BS = chr(92)  # a backslash
DQ = chr(34)  # a double quote, so a row can spell its own delimiters without nesting escapes
UESC = BS + "u"  # the two characters that open a JSON5 unicode escape
LS = chr(0x2028)
PS = chr(0x2029)
NBSP = chr(0x00A0)
EM_SPACE = chr(0x2003)
THIN_SPACE = chr(0x2009)
E_ACUTE = chr(0x00E9)

ACCEPTS = [
    # --- identifiers, and the escape productions of ES5 IdentifierName ---
    ("bare key", "{a: 1}"),
    ("dollar and underscore keys", "{$: 1, _: 2}"),
    ("reserved word as a key", "{if: 1}"),
    ("ID_Start key", "{" + E_ACUTE + ": 1}"),
    ("escaped key at the start", "{" + UESC + "0061: 1}"),
    ("escaped key mid-name", "{a" + UESC + "0062: 1}"),
    ("escaped digit as a key start", "{" + UESC + "0031: 1}"),
    ("escaped dot as a key", "{" + UESC + "002E: 1}"),
    ("escaped uppercase then a literal", "{" + UESC + "0041b: 1}"),
    # --- numbers ---
    ("hex lower and upper prefix", "{a: 0x1f, b: 0XFF}"),
    ("signed hex", "{a: +0x1F, b: -0x1F}"),
    ("leading plus, leading and trailing dot", "{a: +7, b: .5, c: 5.}"),
    ("exponent forms", "{a: 1e+3, b: 1E-3}"),
    ("negative zero", "{a: -0}"),
    # --- strings and their escapes ---
    ("single-quoted string", "{a: 'x'}"),
    ("escaped single quote", "{a: '" + BS + "''}"),
    ("escaped double quote in a single-quoted string", "{a: '" + BS + "\"'}"),
    ("escaped single quote in a double-quoted string", "{a: " + DQ + BS + "'" + DQ + "}"),
    ("vertical tab and NUL escapes", "{a: '" + BS + "v" + BS + "0'}"),
    ("two-digit hex escape", "{a: '" + BS + "x41'}"),
    ("hex escape in a double-quoted string", '{a: "' + BS + 'x7e"}'),
    ("hex escape at the top of the range", "{a: '" + BS + "xFF'}"),
    ("four-digit unicode escape", '{a: "' + UESC + '00e9"}'),
    ("line separator as raw string content", "{a: 'x" + LS + "y'}"),
    # --- the continuation family: one production, five terminators, both quote styles ---
    ("continuation before LF", "{a: '" + BS + "\ny'}"),
    ("continuation before CR", "{a: '" + BS + "\ry'}"),
    ("continuation before CRLF", "{a: '" + BS + "\r\ny'}"),
    ("continuation before LS", "{a: '" + BS + LS + "y'}"),
    ("continuation before PS in a double-quoted string", '{a: "' + BS + PS + 'y"}'),
    ("continuation before a digit does not eat it", "{a: '" + BS + "\n1y'}"),
    # --- structure ---
    ("trailing comma in an object", "{a: 1,}"),
    ("trailing comma in an array", "[1, 2,]"),
    ("comments in every key position", "{/*h*/ a /*k*/ : /*v*/ 1 /*t*/,}"),
    ("line comment after the document", "[1] //t"),
    ("block comment closing the document", "[1 /*x*/]"),
    ("JSON5 whitespace set", "{" + NBSP + "a" + EM_SPACE + ":" + THIN_SPACE + "1" + PS + "}"),
    ("bare top-level string", "'hi'"),
    ("bare top-level number", "42"),
    ("duplicate key keeps the last value", "{a: 1, a: 2}"),
    ("empty object and array in a document", "{a: [], b: {}}"),
]

# A raw control byte is string content in JSON5: the dialect forbids only a line terminator inside
# a quote, and `pyjson5` answers the same for either spelling. The table already credits the line
# separator as raw content, and this repository's single-quoted scanner has always copied these
# bytes through - it was the double-quoted one that stopped on every byte below 0x20, which is RFC
# 8259 §7's rule rather than JSON5's. Each C0 byte is asked in both quote spellings, because one
# dialect must not answer a string's content twice depending on which quote opened it.
C0_CONTENT = [chr(cp) for cp in range(0x20) if cp not in (0x0A, 0x0D)]

ACCEPTS += [
    (
        f"raw U+{ord(ch):04X} as {name} content",
        "{a: " + quote + "x" + ch + "y" + quote + "}",
    )
    for name, quote in (("double-quoted", DQ), ("single-quoted", "'"))
    for ch in C0_CONTENT
]

REJECTS = [
    ("leading-zero decimal", "{a: 01}"),
    ("two trailing commas", "[1, 2,,]"),
    ("unterminated block comment", "{a: 1 /*x}"),
    ("raw newline in a string", "{a: 'x\ny'}"),
    ("octal-style escape in a string", '{a: "' + BS + '41"}'),
    ("curly unicode escape is ES6, not JSON5", "{a: '" + UESC + "{41}'}"),
    ("x escape is not an identifier escape", "{" + BS + "x41: 1}"),
    ("truncated identifier escape", "{" + UESC + "00: 1}"),
    ("bare lowercase nan", "{a: nan}"),
    ("unquoted key starting with a digit", "{1a: 2}"),
    # The two line terminators a JSON5 string cannot hold: JSON5's own rule, and the reason the
    # row above is a list of 30 rather than of 32.
    ("raw LF inside a double-quoted string", "{a: " + DQ + "x\ny" + DQ + "}"),
    ("raw CR inside a double-quoted string", "{a: " + DQ + "x\ry" + DQ + "}"),
]


@pytest.mark.parametrize("label, doc", ACCEPTS)
def test_the_reference_accepts_and_pyrs_agrees(label, doc):
    assert pyrs_yaml.load_json5(doc) == json5.loads(doc), label


@needs_pyjson5
@pytest.mark.parametrize("label, doc", ACCEPTS)
def test_the_second_opinion_agrees_on_the_same_rows(label, doc):
    assert pyrs_yaml.load_json5(doc) == pyjson5.loads(doc), label


@pytest.mark.parametrize("label, doc", REJECTS)
def test_the_reference_rejects_and_pyrs_refuses_too(label, doc):
    with pytest.raises(ValueError):
        json5.loads(doc)
    with pytest.raises(pyrs_yaml.YamlParseError):
        pyrs_yaml.load_json5(doc)


def test_a_surrogate_pair_is_one_character_here_and_two_utf16_units_in_the_pure_python_reference():
    # ES5 strings are UTF-16, and the pure-Python `json5` package leaves `\uD83D\uDE00` as the two
    # code units the source spells; pyrs (like `pyjson5`) follows RFC 8259 and combines them. Measured
    # from the shared table: this is the only one of its 101 accept rows where the two disagree.
    doc = '{a: "' + UESC + "D83D" + UESC + 'DE00"}'
    assert pyrs_yaml.load_json5(doc) == {"a": "\U0001f600"}
    assert json5.loads(doc) == {"a": "\ud83d\ude00"}
    if pyjson5 is not None:
        assert pyjson5.loads(doc) == {"a": "\U0001f600"}


def test_a_lone_surrogate_is_not_an_identifier_start_and_only_the_pure_python_reference_accepts_it():
    # The one reject row the two references disagree on: a `\uD800` escape decodes to a surrogate,
    # which is not a valid IdentifierStart, so pyrs and `pyjson5` refuse the key while pure `json5`
    # carries the lone code unit into the mapping.
    doc = "{" + UESC + "D800: 1}"
    with pytest.raises(pyrs_yaml.YamlParseError):
        pyrs_yaml.load_json5(doc)
    assert json5.loads(doc) == {"\ud800": 1}
    if pyjson5 is not None:
        with pytest.raises(pyjson5.Json5Exception):
            pyjson5.loads(doc)


@pytest.mark.parametrize("loader", [pyrs_yaml.load_json, pyrs_yaml.load_jsonc])
def test_the_strict_dialects_keep_every_c0_byte_forbidden(loader):
    # The widening is a JSON5 rule, so it must not reach the other two: RFC 8259 §7 forbids every
    # unescaped C0 byte, and all 32 of them - the two line terminators and the 30 the JSON5
    # reader now takes - stay refused here.
    for cp in range(0x20):
        doc = '{"a": "x' + chr(cp) + 'y"}'
        with pytest.raises(pyrs_yaml.YamlParseError):
            loader(doc)


def test_a_raw_control_byte_survives_the_tree_route_that_reads_it():
    # `load_json5` decodes on the AST-free fast path and `from_json5` projects the same bytes into
    # the hub's YAML, where a control character has to be quoted to survive. A dialect rule that
    # reaches only one of the two readers is the drift this repository has been bitten by before.
    doc = "{a: " + DQ + "x\ty" + DQ + "}"
    assert pyrs_yaml.load_json5(doc) == {"a": "x\ty"}
    assert pyrs_yaml.safe_load(pyrs_yaml.from_json5(doc)) == {"a": "x\ty"}


def test_the_two_readers_of_one_dialect_agree_on_escaped_keys():
    # `load_json5` builds Python objects on the AST-free fast path, `from_json5` builds the hub's
    # tree; both decode the same key. A dialect rule that reaches only one of them is the drift this
    # repository has been bitten by before, so it is asserted rather than assumed.
    doc = "{" + UESC + "0061: 1}"
    assert pyrs_yaml.load_json5(doc) == {"a": 1}
    assert pyrs_yaml.from_json5(doc) == pyrs_yaml.from_json5("{a: 1}")


def test_non_finite_words_agree_on_value_and_the_tree_route_spells_them_the_hubs_way():
    # JSON5 has `Infinity` / `NaN` and JSON does not, so the reference yields floats and the fast path
    # matches it exactly. The tree route projects the same value into YAML, where the hub spells a
    # non-finite float `.inf` / `.nan` - a representation choice, not a different number. Measured,
    # not inferred: `NaN` never equals itself, so it is compared by predicate on both sides.
    assert pyrs_yaml.load_json5("{a: Infinity}") == json5.loads("{a: Infinity}")
    assert pyrs_yaml.from_json5("{a: Infinity}") == "a: .inf\n"
    assert pyrs_yaml.from_json5("{a: NaN}") == "a: .nan\n"
    assert math.isnan(pyrs_yaml.load_json5("{a: NaN}")["a"])
    assert math.isnan(json5.loads("{a: NaN}")["a"])
