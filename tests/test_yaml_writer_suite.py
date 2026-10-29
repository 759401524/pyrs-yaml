"""Every YAML writer in the hub, judged by readers this repository does not own.

The tiers that touch the YAML write side compare it with our own reader. That is a fixed point: an
implementation that is wrong twice in the same way satisfies it as happily as one that is right
twice. So the documents below are emitted and handed to PyYAML and to ruamel.yaml - neither of
which has ever seen this writer - and what comes back has to be the value that went in.

Measured before it was pinned, on the trunk of 2026-10-18: 26 spellings leaked per route in value
position and the same 26 in key position. Every leak was a spelling the YAML 1.1 type repository
types while the 1.2 core schema leaves a string - `yes`/`on`/`off` and `y`/`n`, `1_000` and `8_0`,
`0b101`, `+0x1f`, the date-only timestamp `1990-06-23`. PyYAML answered `True`, `1000`, `5`, `31`
and `datetime.date(1990, 6, 23)`; ruamel agreed on all of it except the legacy bool words, which
its safe loader leaves strings. Our own `schema="1.1"` profile resolved `y`, `yes` and `on` too, so
the writer and one of the engine's own profiles disagreed about the same token.

The edit layer leaked the same way through `rename` and `create_missing`, and it is held to a
narrower rule than the data routes: issue #336 decided that a path segment *addresses* the key a
reader resolves, so `$.~` is the null key and `$.42` the integer one and both must stay plain.
Only the spellings that lose their type between the two profiles are quoted there, which is what
the two tables below are for.

Temporals are the family this gate was most needed for and the one the forest cannot carry. Until
the `!!timestamp` registration, `safe_dump({"v": date(2026, 10, 18)})` emitted the local `!date`
tag, and a SafeLoader has no constructor for a local tag, so both strangers refused the document
outright - not a mistyped value, an unrecoverable one. The rows at the bottom of this file emit the
temporal objects and read them back through both strangers. `!time` is the deliberate exception:
YAML defines no tag for a bare time of day, so that one stays a local extension.
"""

import datetime as dt
import io
import json
import random

import pytest
import ruamel.yaml
import yaml

import pyrs_yaml

# Spelled by code point rather than by glyph: a raw non-breaking space in this file is what ruff's
# RUF001 calls ambiguous, and a character no reviewer can see is a weak way to name a case.
NBSP = chr(0x00A0)
E_ACUTE = chr(0x00E9)

# A string going in; a bool, a number or a date coming out, for any reader on the 1.1 table.
LEGACY_LEXEMES = [
    "y",
    "Y",
    "yes",
    "Yes",
    "YES",
    "n",
    "N",
    "no",
    "No",
    "NO",
    "on",
    "On",
    "ON",
    "off",
    "Off",
    "OFF",
    "1_000",
    "0_0",
    "1_",
    "1__0",
    "8_0",
    "1_0.0",
    "0b101",
    "0b1",
    "+0b1",
    "+0x1f",
    "-0x1f",
    "+0o17",
    "1990-06-23",
    "2026-10-18",
    "0001-01-01",
]

# Spellings the core schema already types. The data routes have quoted them all along; the route
# that addresses a key by name must keep them plain, which is what the last test below pins.
CORE_LEXEMES = ["42", "true", "null", "~", ".inf", "0x1f", "0o17", "1e5"]

# A 1.1 reader makes these sexagesimal integers. They are not in the guard: a plain scalar holding
# a colon is refused one layer below, and that is what the emitted document is asked here.
SEXAGESIMAL = ["12:00", "190:20:30", "1:30"]

# Text no reader types. Quoting these would still be correct, and a wall of quotes is its own
# regression, so plainness is pinned for the spellings a widening guard might swallow.
STAYS_PLAIN = ["plain", "database", "a_b", "_1", "0x", "0b", "2026-10", "2026-10-18x", "no1", "often"]

TEXTS = [
    "plain",
    "a b",
    "line1\nline2",
    "tab\there",
    "# hash",
    "- dash",
    "{ brace",
    "' quote",
    '"dq"',
    "back\\slash",
    E_ACUTE,
    NBSP,
    "\x07",
    "  padded  ",
    "",
    *LEGACY_LEXEMES,
]
NUMBERS = [0, 1, -1, 3.5, -2.5, 1_000_000, -7]
CHOICES = [True, False, None]

RUAMEL = ruamel.yaml.YAML(typ="safe")
READERS = {"PyYAML": yaml.safe_load, "ruamel": RUAMEL.load}
OWN_PROFILES = {
    "ours core": lambda text: pyrs_yaml.safe_load(text),
    "ours 1.1": lambda text: pyrs_yaml.safe_load(text, schema="1.1"),
}


def _edit(source, apply):
    doc = pyrs_yaml.parse(source)
    apply(doc)
    return doc.to_yaml()


# Each route takes the text the caller handed it and answers with the document that writer emitted
# and the value every reader has to return for it. `None` means the route cannot express that
# spelling, which the test below reports as a skip.
def _dump_value(text):
    return pyrs_yaml.safe_dump({"v": text}), {"v": text}


def _dump_key(text):
    return pyrs_yaml.safe_dump({text: 1}), {text: 1}


def _json_value(text):
    return pyrs_yaml.from_json(json.dumps({"v": text})), {"v": text}


def _json_key(text):
    return pyrs_yaml.from_json(json.dumps({text: 1})), {text: 1}


def _set_value(text):
    return _edit("v: 1\n", lambda d: d.set("$.v", text)), {"v": text}


def _rename_key(text):
    return _edit("v: 1\n", lambda d: d.rename("$.v", text)), {text: 1}


def _created_key(text):
    if "." in text:
        # The path syntax splits a dotted name into two segments, its own declared behaviour; the
        # dotted lexeme is exercised by the routes that take a Python key instead of a path.
        return None
    return _edit("root: 1\n", lambda d: d.set(f"$.{text}", 2, True)), {"root": 1, text: 2}


# Routes that carry a data string: the caller means the string, whatever the spelling looks like.
DATA_ROUTES = {
    "safe_dump value": _dump_value,
    "safe_dump key": _dump_key,
    "from_json value": _json_value,
    "from_json key": _json_key,
    "set value": _set_value,
}

# Routes that address a key by name: issue #336 gives the name a type, so only the spellings that
# lose their type between the two profiles are quoted here.
NAME_ROUTES = {"rename key": _rename_key, "create_missing key": _created_key}

FORESTS = {
    "safe_dump": lambda value: pyrs_yaml.safe_dump({"v": value}),
    "from_json": lambda value: pyrs_yaml.from_json(json.dumps({"v": value})),
    "set": lambda value: _edit("v: 1\n", lambda d: d.set("$.v", value)),
}


def gen(depth, rng):
    if depth <= 0 or rng.random() < 0.35:
        kind = rng.random()
        if kind < 0.6:
            return rng.choice(TEXTS)
        if kind < 0.9:
            return rng.choice(NUMBERS)
        return rng.choice(CHOICES)
    if rng.random() < 0.5:
        return [gen(depth - 1, rng) for _ in range(rng.randint(0, 3))]
    return {rng.choice(TEXTS): gen(depth - 1, rng) for _ in range(rng.randint(0, 3))}


TREES = [gen(3, random.Random(seed)) for seed in range(1, 401)]


@pytest.mark.parametrize("route", list(DATA_ROUTES))
@pytest.mark.parametrize("text", LEGACY_LEXEMES + CORE_LEXEMES + SEXAGESIMAL)
def test_a_stranger_reads_the_spelling_back_as_the_same_string(route, text):
    answer = DATA_ROUTES[route](text)
    if answer is None:
        pytest.skip(f"{route} cannot name a key that contains a dot")
    document, expected = answer
    for name, read in READERS.items():
        back = read(document)
        assert back == expected, f"{route} / {name}: {document!r} read as {back!r}, not {expected!r}"


@pytest.mark.parametrize("route", list(NAME_ROUTES))
@pytest.mark.parametrize("text", LEGACY_LEXEMES + SEXAGESIMAL)
def test_a_key_named_by_a_path_keeps_its_legacy_name_a_string(route, text):
    answer = NAME_ROUTES[route](text)
    if answer is None:
        pytest.skip(f"{route} cannot name a key that contains a dot")
    document, expected = answer
    for name, read in READERS.items():
        back = read(document)
        assert back == expected, f"{route} / {name}: {document!r} read as {back!r}, not {expected!r}"


@pytest.mark.parametrize("text", CORE_LEXEMES)
def test_a_key_named_by_a_path_keeps_the_fold_that_name_addresses(text):
    # The other half of #336, pinned against this file's own fix: `$.42` asks for the integer key
    # and `$.~` for the null one, so widening the guard must not reach those spellings here. The
    # route that carries the name is `rename`, whose argument is the name itself.
    document, _ = NAME_ROUTES["rename key"](text)
    assert document.startswith(f"{text}: "), document


@pytest.mark.parametrize("route", list(FORESTS))
def test_a_stranger_reads_what_the_writer_emitted(route):
    emit = FORESTS[route]
    for tree in TREES:
        document = emit(tree)
        for name, read in READERS.items():
            back = read(document)
            assert back == {"v": tree}, f"{route} / {name}: {document!r} read as {back!r}"


@pytest.mark.parametrize("text", LEGACY_LEXEMES + CORE_LEXEMES)
def test_our_own_two_profiles_read_the_document_as_the_strangers_do(text):
    # The guard is written from the reader's side, so the pair has to agree: what we emit reads back
    # as the same string under the core profile and under the 1.1 profile. Before the fix the 1.1
    # profile resolved `y`, `yes` and `on` out of a document this writer had just produced.
    document = pyrs_yaml.safe_dump({"v": text})
    for name, read in OWN_PROFILES.items():
        assert read(document) == {"v": text}, f"{name}: {document!r} is not the string we sent"


@pytest.mark.parametrize("text", STAYS_PLAIN)
def test_the_guard_does_not_quote_what_no_reader_types(text):
    assert pyrs_yaml.safe_dump({"v": text}) == f"v: {text}\n"
    assert pyrs_yaml.safe_dump({text: 1}) == f"{text}: 1\n"


TEMPORALS = [
    dt.date(2026, 10, 18),
    dt.datetime(2026, 10, 18, 9, 30),
    dt.datetime(2026, 10, 18, 9, 30, 0, 123456),
    dt.datetime(2026, 10, 18, 9, 30, tzinfo=dt.timezone(dt.timedelta(hours=8))),
    dt.datetime(2026, 10, 18, 9, 30, tzinfo=dt.timezone.utc),
]

STRANGERS = {
    "PyYAML": lambda text: yaml.safe_load(text),
    "ruamel.yaml": lambda text: ruamel.yaml.YAML(typ="safe").load(io.StringIO(text)),
}


@pytest.mark.parametrize("value", TEMPORALS)
@pytest.mark.parametrize("reader", list(STRANGERS))
def test_a_stranger_reads_the_temporal_we_emitted(reader, value):
    # A tag is the author's claim about the type of the text under it, so the tag we invent for a
    # Python temporal has to be the one every other reader already knows. Measured before this row
    # existed: both strangers answered `safe_dump({"v": date(...)})` with ConstructorError, because
    # the local `!date` spelling has no constructor outside this library - the value was not
    # mistyped, it was unrecoverable.
    document = pyrs_yaml.safe_dump({"v": value})
    assert "!!timestamp" in document, document
    back = STRANGERS[reader](document)
    assert back == {"v": value}, f"{reader}: {document!r} came back as {back!r}, not {value!r}"


def test_a_toml_document_written_back_as_yaml_is_still_readable_by_a_stranger():
    # The hub invents a temporal tag in two places: from a Python object, and from a TOML date or
    # date-time. Both have to land on the standard spelling, or a document that merely passes through
    # on its way to YAML is unreadable again. A bare time is left out of the source: it has no
    # standard tag to aim at, and it is the one shape that still makes both strangers refuse.
    source = "d = 2026-10-18\ndt = 2026-10-18T09:30:00Z\n"
    document = pyrs_yaml.safe_dump(pyrs_yaml.load_toml(source))
    expected = {
        "d": dt.date(2026, 10, 18),
        "dt": dt.datetime(2026, 10, 18, 9, 30, tzinfo=dt.timezone.utc),
    }
    assert "!!timestamp" in document, document
    for name, read in STRANGERS.items():
        assert read(document) == expected, f"{name}: {document!r} came back as {read(document)!r}"


def test_the_bare_time_stays_the_local_extension_it_admits_being():
    # YAML defines no tag for a time of day - PyYAML cannot even represent one, so `!time` is written
    # as what it is instead of dressed up as a standard tag no reader would resolve. The text now
    # rides the key's line, because the direct writer asks the handler before it lays a value out
    # (`docs/dev/quality-ledger.md` (hr)); the AST route puts the same text in quotes, so what these
    # two rows hold is the one thing that must not move - both spellings re-read as the same time.
    document = pyrs_yaml.safe_dump({"v": dt.time(9, 30)})
    assert document == "v: !time 09:30:00\n"
    assert pyrs_yaml.safe_load(document) == {"v": dt.time(9, 30)}
    doc = pyrs_yaml.parse("v: 0\n")
    doc.set("$.v", dt.time(9, 30))
    assert pyrs_yaml.safe_load(doc.to_yaml()) == {"v": dt.time(9, 30)}


def test_a_tag_a_document_arrived_with_is_not_rewritten_on_the_way_out():
    # This change gives a tag to objects that had no text to preserve. A document that came in with
    # the legacy spelling keeps it, so the fix cannot quietly restyle somebody's file.
    text = "v: !date 2026-10-18\n"
    assert pyrs_yaml.parse(text).to_yaml() == text


def test_the_forest_carries_the_lexemes_this_gate_names():
    # The count and the members are what the module docstring measured; a silent shrink here would
    # turn that paragraph into a claim nobody checked.
    assert len(TREES) == 400
    assert {NBSP, E_ACUTE, "\x07"} <= set(TEXTS)
    assert set(LEGACY_LEXEMES) <= set(TEXTS)
    assert len(LEGACY_LEXEMES) == 31
    assert any(isinstance(t, dict) for t in TREES)
    assert any(isinstance(t, list) for t in TREES)
