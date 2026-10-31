"""A note arriving in one format survives every comment-carrying export, or the export says why.

`tests/test_cross_format_note_slots.py` pins the outbound half of each bridge: eleven YAML documents,
one note each, four writers. The inbound half - a note that arrives in JSONC, JSON5 or TOML text and
then leaves through a writer - was asked by nothing. Measured 2026-10-31 by an AST pass over every
test function body in all 134 modules of `tests/`, five (reader format, writer format) pairs had no
call site at all: `json -> jsonc`, `jsonc -> toml`, `json5 -> json`, `json5 -> jsonc`,
`json5 -> toml`. JSON5 - the dialect with the most own literal spellings and two comment syntaxes -
never left toward strict JSON, JSONC or TOML anywhere in the suite, and JSONC never left toward TOML.

So this file is that table. Every row is one note, written the way that format writes it, entering
through that format's reader and leaving through every export that has comment syntax. A row passes
when the note's token reaches the text, or when the export refuses with a reason that is true about
the target format. The outcome that is never allowed is the one the closed defects in the sibling
file were: text in, note silently gone.
"""

from __future__ import annotations

import itertools
import re

import pytest

import pyrs_yaml

#: How each of the four formats spells the same four note slots. `yaml` enters with no conversion
#: because YAML text is the hub; the other three names are the bridge readers, which return YAML.
ENTRANCES = {
    "yaml": lambda text: text,
    "jsonc": pyrs_yaml.from_jsonc,
    "json5": pyrs_yaml.from_json5,
    "toml": pyrs_yaml.from_toml,
}

#: The exports with comment syntax. Strict `to_json` is deliberately absent: JSON has no comment to
#: write, so omitting one is the only correct answer there and asking for a refusal would be a lie
#: about the format. The sibling file draws the same line for the same reason.
EXPORTS = {
    "to_yaml": lambda hub: pyrs_yaml.parse(hub).to_yaml(),
    "to_toml": lambda hub: pyrs_yaml.to_toml(hub),
    "to_jsonc": lambda hub: pyrs_yaml.parse(hub).to_jsonc(),
    "to_json5": lambda hub: pyrs_yaml.parse(hub).to_json5(),
}

#: (slot, {format: source text}, token). The slot is the position a note can sit in; the spelling is
#: what that format's own grammar requires, including the forms JSON5 allows and JSON does not.
SLOTS = [
    (
        "leading above a root key",
        {
            "yaml": "# noteL\na: 1\n",
            "jsonc": '// noteL\n{"a": 1}',
            "json5": "// noteL\n{a: 1}",
            "toml": "# noteL\na = 1\n",
        },
        "noteL",
    ),
    (
        "inline after a value",
        {
            "yaml": "a: 1 # noteT\n",
            "jsonc": '{"a": 1 /* noteT */}',
            "json5": "{a: 1 // noteT\n}",
            "toml": "a = 1 # noteT\n",
        },
        "noteT",
    ),
    (
        "key line heading a section",
        {
            "yaml": "m: # noteK\n  x: 1\n",
            "jsonc": '{/* noteK */ "m": {"x": 1}}',
            "json5": "{/* noteK */ m: {x: 1}}",
            "toml": "# noteK\n[m]\nx = 1\n",
        },
        "noteK",
    ),
    (
        "inside a sequence item",
        {
            "yaml": "s:\n  - 1 # noteS\n",
            "jsonc": '{"s": [1 /* noteS */]}',
            "json5": "{s: [1 // noteS\n]}",
            "toml": "s = [1] # noteS\n",
        },
        "noteS",
    ),
]

ROWS = [(slot, entry, export) for (slot, _docs, _token), entry, export in itertools.product(SLOTS, ENTRANCES, EXPORTS)]

#: Each export's own comment grammar. Stripping it is how the row tells a carried note from a note
#: that became data: a token that survives only because it landed inside a string would pass
#: `token in out` and fail `token not in stripped(out)`, and all 64 cells pass both.
COMMENT_SPANS = {
    "to_yaml": re.compile(r"#.*"),
    "to_toml": re.compile(r"#.*"),
    "to_jsonc": re.compile(r"//.*|/\*.*?\*/", re.DOTALL),
    "to_json5": re.compile(r"//.*|/\*.*?\*/", re.DOTALL),
}

#: Refusals that say something true about the target format rather than about notes. Every row above
#: is representable in all four exports, so these stay empty on purpose: a refusal here is the
#: blind-slot bug showing up, exactly as in the sibling file.
HONEST_REFUSALS: dict[str, tuple[str, ...]] = {
    "to_yaml": (),
    "to_toml": (),
    "to_jsonc": (),
    "to_json5": (),
}

#: The five bridges the inventory found with no call site, each entering with the literals that
#: dialect owns alone and leaving with the value the reader resolved. A comment is not the claim in
#: these rows; the number spellings are - `0xFF`, `.5`, `+7`, single quotes, a trailing comma.
BRIDGES = [
    ("json", "jsonc", '{"a": 1, "b": [1, 2]}', {"a": 1, "b": [1, 2]}),
    ("jsonc", "toml", '{\n  // head\n  "a": 1, /* tail */\n  "b": [1, 2]\n}', {"a": 1, "b": [1, 2]}),
    ("json5", "json", "{ a: 0xFF, b: [1, .5, +7], }", {"a": 255, "b": [1, 0.5, 7]}),
    ("json5", "jsonc", "{ a: 0xFF, b: [1, .5, +7], }", {"a": 255, "b": [1, 0.5, 7]}),
    ("json5", "toml", "{ a: 0xFF, b: [1, .5, +7], }", {"a": 255, "b": [1, 0.5, 7]}),
]

#: The same entrances plus strict JSON, which the note table above cannot use: JSON has no comment
#: syntax to carry a token, but it is one of the five formats a value bridge starts from.
BRIDGE_IN = {**ENTRANCES, "json": pyrs_yaml.from_json}

READ_IN = {
    "json": pyrs_yaml.load_json,
    "jsonc": pyrs_yaml.load_jsonc,
    "json5": pyrs_yaml.load_json5,
    "toml": pyrs_yaml.load_toml,
    "yaml": pyrs_yaml.safe_load,
}
WRITE_OUT = {
    "json": lambda hub: pyrs_yaml.parse(hub).to_json(),
    "jsonc": lambda hub: pyrs_yaml.parse(hub).to_jsonc(),
    "json5": lambda hub: pyrs_yaml.parse(hub).to_json5(),
    "toml": lambda hub: pyrs_yaml.to_toml(hub),
    "yaml": lambda hub: pyrs_yaml.parse(hub).to_yaml(),
}


@pytest.mark.parametrize("slot,entry,export", ROWS, ids=[f"{e}->{x}:{s}" for s, e, x in ROWS])
def test_a_note_that_arrives_in_one_format_leaves_in_another(slot, entry, export):
    documents = dict((s, docs) for s, docs, _token in SLOTS)[slot]
    token = next(tok for s, _docs, tok in SLOTS if s == slot)
    hub = ENTRANCES[entry](documents[entry])
    try:
        out = EXPORTS[export](hub)
    except pyrs_yaml.YamlSerializeError as error:
        reason = str(error)
        allowed = HONEST_REFUSALS[export]
        assert reason in allowed, (
            f"{token!r} entering as {entry} and leaving as {export} refused with {reason!r}; this "
            f"row is representable, so the only accepted refusals are {allowed or 'none'}"
        )
        return
    assert token in out, (
        f"the note in slot {slot!r} vanished on the way from {entry} to {export}: "
        f"input {documents[entry]!r} wrote {out!r} (hub {hub!r})"
    )
    assert token not in COMMENT_SPANS[export].sub("", out), (
        f"{token!r} reached {export} text but not as a comment, so it is data, not the note: {out!r}"
    )


def test_the_table_reaches_every_entrance_and_export() -> None:
    """The shape of the table, checked where a mistake could actually hide.

    `ROWS` is derived, so comparing it with its own product would prove nothing. What can drift is
    the *text*: a format added to `ENTRANCES` without a spelling in every slot, or strict JSON
    creeping into `EXPORTS` where it would refuse every note truthfully. Both are named here, and
    the value bridges have to reach all five formats while the note table reaches only the four that
    own comment syntax.
    """
    for slot, docs, _token in SLOTS:
        assert set(docs) == set(ENTRANCES), f"{slot}: {sorted(set(docs) ^ set(ENTRANCES))}"
    assert "to_json" not in EXPORTS, "strict JSON has no comment to write; this row is a decision"
    assert set(BRIDGE_IN) == set(READ_IN) == set(WRITE_OUT) == {"json", "jsonc", "json5", "toml", "yaml"}
    assert {(src, dst) for src, dst, _, _ in BRIDGES} == {
        ("json", "jsonc"),
        ("jsonc", "toml"),
        ("json5", "json"),
        ("json5", "jsonc"),
        ("json5", "toml"),
    }, sorted(BRIDGES)


@pytest.mark.parametrize("src,dst,text,_expected", BRIDGES, ids=[f"{s}->{d}" for s, d, _, _ in BRIDGES])
def test_the_bridge_carries_the_value_it_read(src, dst, text, _expected) -> None:
    """The five one-way gaps cross with their literals intact and their entries in order."""
    hub = BRIDGE_IN[src](text)
    out = WRITE_OUT[dst](hub)
    assert READ_IN[dst](out) == _expected, f"{src} -> {dst}: {text!r} wrote {out!r}"


def test_a_note_on_an_inline_array_moves_in_toml_and_then_settles() -> None:
    """The one placement the TOML writer does not hold still, reached from the JSONC door.

    Documented in `docs/dev/boundaries.md` for a YAML document: the inline array's own line keeps the
    note on the first round, the re-read writes the array block-style with the note on its last
    element, and that second form is the fixed point. Reached through `from_jsonc` the same shape was
    the only row in this table that moved, so it is pinned as a relocation - the note never leaves.
    """
    once = pyrs_yaml.to_toml(pyrs_yaml.from_jsonc('{\n  "b": [1, 2] /* tail */\n}'))
    assert "tail" in once, once
    twice = pyrs_yaml.to_toml(pyrs_yaml.from_toml(once))
    assert "tail" in twice, twice
    assert pyrs_yaml.to_toml(pyrs_yaml.from_toml(twice)) == twice, (once, twice)
    assert pyrs_yaml.load_toml(once) == pyrs_yaml.load_toml(twice) == {"b": [1, 2]}
