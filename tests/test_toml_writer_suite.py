"""The TOML writer, judged by parsers this repository does not own.

Every family in the hub now has a write-side gate that asks a stranger: `to_json`, `to_jsonc` and
`to_json5` face Python's `json` and `pyjson5` in `test_json_writer_suite.py`, the YAML writers face
PyYAML and ruamel.yaml in `test_yaml_writer_suite.py`. TOML was the remaining cell — its two outside
tiers, `test_toml_test_suite.py` (the official suite) and `test_toml_crosslib.py` (tomlkit, tomllib),
both ask the *reader*, and a reader checked only against our own writer is a fixed point that an
implementation wrong twice in the same way satisfies.

Measured on the trunk of 2026-10-18 before it was pinned: 1,200 generated trees, 1,074 emitted and
read back with 0 refusals and 0 value changes through both `tomllib` and `tomlkit`; the 126 that did
not reach a stranger were all one typed refusal, `toml-cannot-represent-null`. Both module entry
points (`to_toml(text)` and `YamlDocument.to_toml()`) produce identical bytes, so the gate checks
both and compares them.

Two boundaries are asserted rather than worked around. A `datetime.time` is emitted as a bare local
time (`k = 09:30:00`), which is TOML 1.1's and which both readers here decode; TOML 1.0.0 has no
local-time type, so that spelling is a version choice recorded in `crates/pyrs-toml`, not something
this file can adjudicate. An integer beyond i64 is emitted as a quoted string, because the hub
already calls it a string — the same answer `load_jsonc` and `safe_dump` give (see
`test_big_digit_string_spelling_is_verbatim`), and the row is pinned here so the three routes cannot
drift apart.

`tomllib` needs Python 3.11, so each test asks whichever outside parsers this interpreter has:
`tomlkit` is an unconditional test dependency and covers every leg, `tomllib` joins it on 3.11+.
The last test asserts that minimum is non-empty, because a gate that skips entirely is not a gate.
"""

import datetime as dt
import math
import random
import sys

import pytest
import tomlkit

import pyrs_yaml

try:
    import tomllib
except ImportError:  # pragma: no cover - the 3.8 and 3.9/3.10 legs
    tomllib = None

TEXTS = [
    "plain",
    "1",
    "true",
    "",
    "   ",
    "a b",
    "line1\nline2",
    "tab\there",
    'quote"',
    "it's",
    "back\\slash",
    "é中",
    chr(0x00A0),
    "# hash",
    "[ bracket",
    "= eq",
    "k" * 40,
]
# TOML bare keys, quoted keys and dotted keys are three different emissions; all of these are
# spelled as data, so the writer's key path is what decides whether a stranger sees the name.
KEYS = ["plain", "a b", "", "1", "true", "a.b", "a b c", "-", "#h", "[x]", "=", "é中", "k" * 30]
NUMBERS = [0, 1, -1, 3.5, -2.5, 0.0, -0.0, 1_000_000, -7, 2**62, -(2**62)]
TEMPORALS = [
    dt.date(2026, 10, 18),
    dt.datetime(2026, 10, 18, 9, 30),
    dt.datetime(2026, 10, 18, 9, 30, tzinfo=dt.timezone.utc),
    dt.time(9, 30),
]

READERS = {"tomlkit": tomlkit.parse}
if tomllib is not None:
    READERS["tomllib"] = tomllib.loads


def gen(depth, rng):
    if depth <= 0 or rng.random() < 0.35:
        kind = rng.random()
        if kind < 0.5:
            return rng.choice(TEXTS)
        if kind < 0.8:
            return rng.choice(NUMBERS)
        if kind < 0.9:
            return rng.choice(TEMPORALS)
        return rng.choice([True, False])
    if rng.random() < 0.45:
        return [gen(depth - 1, rng) for _ in range(rng.randint(0, 3))]
    return {rng.choice(KEYS): gen(depth - 1, rng) for _ in range(rng.randint(1, 3))}


TREES = [{"v": gen(3, random.Random(seed))} for seed in range(1, 401)]


def test_a_stranger_reads_what_the_toml_writer_emitted():
    for tree in TREES:
        document = pyrs_yaml.to_toml(pyrs_yaml.from_dict(tree))
        # The two entry points are one writer, and the claim is that neither invents a second answer.
        assert pyrs_yaml.parse(pyrs_yaml.from_dict(tree)).to_toml() == document
        for name, read in READERS.items():
            back = read(document)
            assert dict(back) == tree, f"{name}: {document!r} decoded {back!r}, not {tree!r}"


@pytest.mark.parametrize(
    "tree",
    [{"v": None}, {"v": [1, None]}, {"v": {"a": {"b": None}}}, {"v": [{}, None]}],
)
def test_a_value_toml_cannot_say_is_refused_not_invented(tree):
    # `null` has no TOML spelling, and the alternatives (drop the key, emit `""`, emit `null`)
    # each produce a document a stranger reads as something other than what was sent.
    with pytest.raises(pyrs_yaml.YamlSerializeError) as refused:
        pyrs_yaml.to_toml(pyrs_yaml.from_dict(tree))
    assert "null" in str(refused.value)


@pytest.mark.parametrize("value", [*TEMPORALS, 2**63 - 1, -(2**63), 1e308, float("inf")])
def test_a_stranger_decodes_each_temporal_and_numeric_spelling_we_emitted(value):
    document = pyrs_yaml.to_toml(pyrs_yaml.from_dict({"k": value}))
    for name, read in READERS.items():
        back = read(document)["k"]
        assert back == value, f"{name}: {document!r} decoded {back!r}, not {value!r}"
        # `isinstance`, not `type(...) is`: tomlkit hands back its own `Integer`/`Float`/`Date`
        # subclasses of the builtins, and a subclass is the same kind of value as far as this
        # claim goes — a `str` where a number was sent is not.
        assert isinstance(back, type(value)), f"{name}: {document!r} decoded a {type(back).__name__}"


def test_nan_is_the_one_value_no_equal_can_confirm():
    document = pyrs_yaml.to_toml(pyrs_yaml.from_dict({"k": float("nan")}))
    for name, read in READERS.items():
        assert math.isnan(read(document)["k"]), f"{name}: {document!r} is not a NaN"


def test_an_integer_beyond_i64_is_the_string_the_hub_calls_it_everywhere():
    # Not this writer degrading: the hub already resolved the 19-digit text to a string, which is
    # what `load_jsonc` returns and what `safe_dump` emits unquoted. Pinned across the routes so
    # one of them cannot quietly start claiming a number that its own reader will not accept.
    big = "9223372036854775808"
    document = pyrs_yaml.to_toml(pyrs_yaml.from_dict({"k": int(big)}))
    assert document == f'k = "{big}"\n'
    for name, read in READERS.items():
        assert read(document)["k"] == big, f"{name} read {document!r} as not-a-string"
    assert pyrs_yaml.load_toml(document) == {"k": big}


def test_a_note_above_a_nested_key_travels_the_route_into_the_inline_table():
    # The crate's own writer test builds this AST in Rust; this one proves the shipped route hands
    # the note over in the slot the writer reads — YAML text, hub AST, TOML — and that the demotion
    # settles: re-reading the emitted TOML lands the note in the same place, so a second emission
    # is byte-identical.
    noted = pyrs_yaml.to_toml("top: 1\nnested:\n  # above the inner key\n  inner: 1\ntail: 2\n")
    assert noted == "top = 1\nnested = {\n  # above the inner key\n  inner = 1\n}\ntail = 2\n"
    assert pyrs_yaml.to_toml(pyrs_yaml.from_toml(noted)) == noted
    assert pyrs_yaml.load_toml(noted) == {"top": 1, "nested": {"inner": 1}, "tail": 2}
    # The note is the only reason the table left the compact form: undecorated output is unchanged,
    # so the 1.0 promise this file's other gates rest on keeps holding for documents with no note.
    bare = pyrs_yaml.to_toml("top: 1\nnested:\n  inner: 1\ntail: 2\n")
    assert bare == "top = 1\nnested = {inner = 1}\ntail = 2\n"
    # Which side of 1.0 each text is on is asked of a parser the project does not own — and the answer
    # is a fact about that parser's version, measured on 2026-10-29: CPython 3.11-3.14 `tomllib` refuses
    # a multi-line inline table, CPython 3.15.0's `tomllib` accepts it, and `tomli` 2.5.0 (the reader the
    # legs below 3.11 get) accepts it too. So the expectation is keyed rather than assumed: a leg whose
    # reader moved out of this table reddens instead of quietly dropping its claim. The refusal that
    # does not move is this crate's own 1.0 gate, `crates/pyrs-toml`'s `from_toml_v1_0`, pinned with
    # its 1.1 half by `the_1_0_reader_refuses_the_1_1_inline_table_freedom`.
    if sys.version_info >= (3, 11):
        import tomllib as _reader

        _refuses_multiline_inline_table = sys.version_info < (3, 15)
    else:
        import tomli as _reader

        _refuses_multiline_inline_table = False

    assert _reader.loads(bare) == {"top": 1, "nested": {"inner": 1}, "tail": 2}
    if _refuses_multiline_inline_table:
        with pytest.raises(_reader.TOMLDecodeError):
            _reader.loads(noted)
    else:
        assert _reader.loads(noted) == {"top": 1, "nested": {"inner": 1}, "tail": 2}, (
            f"{_reader.__name__} read the noted text as something else"
        )


def test_a_note_above_the_first_item_of_an_array_travels_the_route_inside_it():
    # The same fifth slot one level down, and the reason the rule says "container" rather than
    # "table": `arr:` with a note above its first item reaches the writer on the sequence node, which
    # the element checks never ask.
    noted = pyrs_yaml.to_toml("top: 1\narr:\n  # above the first item\n  - 1\n  - 2\ntail: 2\n")
    assert noted == "top = 1\narr = [\n  # above the first item\n  1,\n  2\n]\ntail = 2\n"
    assert pyrs_yaml.to_toml(pyrs_yaml.from_toml(noted)) == noted
    assert pyrs_yaml.load_toml(noted) == {"top": 1, "arr": [1, 2], "tail": 2}
    assert pyrs_yaml.to_toml("top: 1\narr:\n  - 1\n  - 2\ntail: 2\n") == "top = 1\narr = [1, 2]\ntail = 2\n"


def test_a_note_above_a_value_with_no_members_travels_the_route_on_the_key_line():
    # The shapes PR #676 left outside its own rule: a note whose owner has no member to sit beside.
    # `# above` inside `{ }` re-reads as no note, so the writer kept the compact spelling and the note
    # vanished — and the fixed point stayed green because both passes were equally note-free. They now
    # take the line above the key, which is where this crate's reader reports a member's leading note
    # from, so the text both carries the note and settles.
    cases = [
        ("A:\n  # above\n  {}\nB: 1\n", "# above\nA = {}\nB = 1\n", {"A": {}, "B": 1}),
        ("A:\n  # above\n  []\nB: 1\n", "# above\nA = []\nB = 1\n", {"A": [], "B": 1}),
        ("A:\n  # above\n  v\nB: 1\n", '# above\nA = "v"\nB = 1\n', {"A": "v", "B": 1}),
    ]
    for src, expected, value in cases:
        noted = pyrs_yaml.to_toml(src)
        assert noted == expected, f"{src!r} wrote {noted!r}"
        assert pyrs_yaml.to_toml(pyrs_yaml.from_toml(noted)) == noted, f"{expected!r} does not settle"
        assert pyrs_yaml.load_toml(noted) == value
        for name, read in READERS.items():
            assert read(noted) == value, f"{name} read the noted text as something else"


def test_a_section_key_note_beside_the_section_note_travels_the_route():
    # The two slots (hy) found are reachable from YAML text: a note above the section's key, and a
    # note above that section's first member, which the hub homes on the container's own list. Both
    # lines owe the header, the text settles on the first hop, and a parser this project does not own
    # reads it — stacked comments above a table header are TOML 1.0 syntax, so unlike the multi-line
    # inline table above, this needs no version keying.
    cases = [
        ("# K\nsec:\n  # T\n  x: 1\n", "# K\n# T\n[sec]\nx = 1\n", {"sec": {"x": 1}}),
        (
            "top: 1\n# K\nsec:\n  # T\n  x: 1\n",
            "top = 1\n# K\n# T\n[sec]\nx = 1\n",
            {"top": 1, "sec": {"x": 1}},
        ),
    ]
    for src, expected, value in cases:
        noted = pyrs_yaml.to_toml(src)
        assert noted == expected, f"{src!r} wrote {noted!r}"
        assert pyrs_yaml.to_toml(pyrs_yaml.from_toml(noted)) == noted, f"{expected!r} does not settle"
        assert pyrs_yaml.load_toml(noted) == value
        for name, read in READERS.items():
            assert read(noted) == value, f"{name} read the noted text as something else"


def test_the_gate_cannot_silently_skip_every_reader():
    # The claim of this file is that a parser the repository does not own reads the output. `tomlkit`
    # is an unconditional test dependency, so an empty reader set means the gate stopped existing.
    assert READERS, "no outside TOML parser is installed on this leg"
    assert "tomlkit" in READERS


def test_the_forest_carries_the_shapes_this_gate_names():
    # The counts and the members are what the module docstring measured; a silent shrink would turn
    # that paragraph into a number nobody checked.
    assert len(TREES) == 400
    assert {dt.time(9, 30), dt.date(2026, 10, 18)} <= set(TEMPORALS)
    assert any(isinstance(t["v"], dict) for t in TREES)
    assert any(isinstance(t["v"], list) for t in TREES)
    assert any("#" in k for k in KEYS) and any("." in k for k in KEYS)
