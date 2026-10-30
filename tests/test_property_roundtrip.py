"""Property-based tests (Hypothesis) for round-trip fidelity.

These fuzz the dump/parse pipeline with randomly generated structures to catch (a) panics/crashes on
arbitrary input and (b) structural or scalar drift introduced by refactors. The exact-equality
strategies stay JSON-shaped; the fuzz domain (`arbitrary_json`) also draws the families a
JSON-shaped generator cannot name — an integer past i64, a NaN or an infinity, a tuple, bytes, the
temporals, `Decimal`, and four types the binding refuses with its own `YamlTypeError` — because that
is where the number protocol and the type registry live.

The round-trip invariant now holds end-to-end: quoted scalars load as strings
(YAML 1.2), lone-quote keys and empty collections round-trip exactly, and only
plain scalars are schema-resolved. The only strings still excluded from the
*exact-equality* strategy are those the YAML plain-scalar grammar cannot
represent losslessly (leading/trailing whitespace, control characters) — a
serializer syntax concern, not a type-resolution one. ``test_dump_never_raises``
fuzzes *arbitrary* input to assert the dump step itself never panics.

Strategies are shared via ``tests/strategies.py``.
"""

import datetime as dt
from decimal import Decimal

import pytest
from hypothesis import HealthCheck, given, settings

import pyrs_yaml
from tests.strategies import (
    arbitrary_json,
    roundtrip_safe_json,
    roundtrip_safe_leaf,
    rt,
)

pytestmark = pytest.mark.slow


@pytest.fixture(autouse=True)
def _ensure_builtin_plugins():
    """Re-register the built-in plugins, because other suites clear the shared registry.

    The same guard `tests/test_toml.py` and `tests/test_standard_tags.py` carry, and this tier is the
    one that needs it most: `bytes`, `date`, `time` and `datetime` reach their tags through a
    registered plugin, so a run ordered after a `clear_type_handlers()` — `test_concurrency.py` ends
    with one — measures the registry's absence instead of the route. Measured on 2026-10-30 with
    `tests/test_concurrency.py` first: the `bytes`, `date`, `time` and `datetime` rows raised
    `YamlTypeError`, the `Decimal` row silently wrote `1.5\n` and read it back as a `float`, and the
    `arbitrary_json` property still passed, because a refusal is what its weakened form accepts.
    Re-registration is idempotent, which makes the module order-independent.
    """
    from pyrs_yaml.plugins import _builtin

    _builtin._register_builtins()


@settings(max_examples=300, deadline=5000, suppress_health_check=[HealthCheck.too_slow])
@given(roundtrip_safe_leaf)
def test_safe_scalar_roundtrip_exact(value):
    assert rt(value) == value


@settings(max_examples=300, deadline=5000, suppress_health_check=[HealthCheck.too_slow])
@given(roundtrip_safe_json)
def test_safe_structure_roundtrip_exact(value):
    assert rt(value) == value


@settings(max_examples=300, deadline=5000, suppress_health_check=[HealthCheck.too_slow])
@given(roundtrip_safe_json)
def test_roundtrip_idempotent(value):
    once = rt(value)
    twice = rt(once)
    assert twice == once


@settings(max_examples=300, deadline=5000, suppress_health_check=[HealthCheck.too_slow])
@given(arbitrary_json)
def test_dump_never_raises(value):
    # The dump step must not panic on any structure, and the only refusal it may make is the
    # binding's own typed one: a value with no YAML spelling raises `YamlTypeError`, and anything
    # else escaping is a crash wearing a different name. The leaves now reach the families the capped
    # strategy could not draw — integers past i64, non-finite floats, tuples, bytes, the temporals,
    # `Decimal`, and four types no spelling carries — so the claim covers the routes rather than the
    # subset of them that happens to be JSON.
    try:
        out = pyrs_yaml.safe_dump(value)
    except pyrs_yaml.YamlTypeError:
        return
    assert isinstance(out, str)


# What each widened family does at the binding, measured on 2026-10-30 rather than reasoned: the
# type the reader hands back after one dump/load pair, or `None` for the value shapes that carry no
# YAML spelling and must be refused. A tuple is the list the sequence form reads back as; an integer
# past i64 is the string the hub already calls it; `Decimal` and the temporals keep their tags.
HARD_FAMILIES = [
    ("int past i64", 2**63, str),
    ("int below -i64", -(2**64), str),
    ("NaN", float("nan"), float),
    ("infinity", float("inf"), float),
    ("negative infinity", float("-inf"), float),
    ("negative zero", -0.0, float),
    ("tuple", (1, 2), list),
    ("nested tuple", (1, (2, 3)), list),
    ("bytes", b"abc\x00\xff", bytes),
    ("date", dt.date(2026, 10, 30), dt.date),
    ("time", dt.time(9, 30, 0), dt.time),
    ("datetime", dt.datetime(2026, 10, 30, 9, 30, 3), dt.datetime),
    ("Decimal", Decimal("1.5"), Decimal),
    ("set", {1, 2}, None),
    ("frozenset", frozenset([1]), None),
    ("complex", 1 + 2j, None),
    ("unregistered object", type("Point", (), {"x": 1})(), None),
]

# One row is measured as not settling, and says so instead of leaving the claim: `safe_dump(2**63)`
# and `safe_dump(-(2**64))` both write bare digits that read back as a string, but re-dumping the
# positive one repeats the text while the negative one gains quotes — the two spellings of one hub
# decision disagree on the sign. Asserting the measured pair keeps the disagreement from changing
# silently while it is open.
NON_SETTLING = {"int below -i64": '"-18446744073709551616"\n'}


@pytest.mark.parametrize(
    ["label", "value", "read_back"],
    HARD_FAMILIES,
    ids=[case[0] for case in HARD_FAMILIES],
)
def test_each_widened_family_is_text_or_the_typed_refusal(label, value, read_back):
    if read_back is None:
        with pytest.raises(pyrs_yaml.YamlTypeError):
            pyrs_yaml.safe_dump(value)
        return
    text = pyrs_yaml.safe_dump(value)
    assert type(pyrs_yaml.safe_load(text)) is read_back, label
    # The value shape the reader hands back writes the same text again: a family that only settles on
    # its second emission is a round trip that moves, whatever its type says.
    again = pyrs_yaml.safe_dump(pyrs_yaml.safe_load(text))
    if label in NON_SETTLING:
        assert again == NON_SETTLING[label], (text, again)
    else:
        assert again == text, label


def test_the_widened_leaves_reach_every_family_they_were_added_for():
    # A strategy nobody draws from is decoration: the same generator, 400 examples, and a count of
    # which widened families actually arrived. `arbitrary_json` is the only consumer, so if a leaf
    # ever drops out of the mix this reddens instead of the fuzz tier quietly narrowing again. The
    # two shapes that are rare *inside* an unbounded integer or float strategy get their own branch in
    # `hard_leaf` for exactly this reason: the first version asked for them inside 400 draws and two
    # interpreter legs reddened on a different family each, which is a coin flip wearing a test name.
    seen: set[str] = set()

    def record(value):
        if isinstance(value, bool) or value is None or isinstance(value, (str, int, float, list, dict)):
            if isinstance(value, int) and abs(value) > 2**63 - 1:
                seen.add("int past i64")
            if isinstance(value, float) and (value != value or value in (float("inf"), float("-inf"))):
                seen.add("non-finite float")
            if isinstance(value, (list, dict)):
                for item in value if isinstance(value, list) else value.values():
                    record(item)
            return
        for name, probe in (
            ("tuple", lambda v: isinstance(v, tuple)),
            ("bytes", lambda v: isinstance(v, bytes)),
            ("date", lambda v: isinstance(v, dt.date) and not isinstance(v, dt.datetime)),
            ("time", lambda v: isinstance(v, dt.time)),
            ("datetime", lambda v: isinstance(v, dt.datetime)),
            ("Decimal", lambda v: isinstance(v, Decimal)),
            ("set", lambda v: isinstance(v, set) and not isinstance(v, frozenset)),
            ("frozenset", lambda v: isinstance(v, frozenset)),
            ("complex", lambda v: isinstance(v, complex)),
            ("unregistered object", lambda v: type(v).__name__ == "Point"),
        ):
            if probe(value):
                seen.add(name)
                return

    @settings(max_examples=400, deadline=None, suppress_health_check=[HealthCheck.too_slow])
    @given(arbitrary_json)
    def collect(value):
        record(value)

    collect()

    expected = {
        "int past i64",
        "non-finite float",
        "tuple",
        "bytes",
        "date",
        "time",
        "datetime",
        "Decimal",
        "set",
        "frozenset",
        "complex",
        "unregistered object",
    }
    missing = sorted(expected - seen)
    assert not missing, f"the widened leaves never drew {missing} in 400 examples"
