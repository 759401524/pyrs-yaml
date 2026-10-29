"""Test Community Plugins (Spiral 3)."""

import decimal
import io
from datetime import datetime, timezone

import pytest

import pyrs_yaml


class TimestampType(pyrs_yaml.CustomType):
    python_type = datetime

    def from_yaml(self, value):
        return datetime.fromisoformat(value)

    def to_yaml(self, obj):
        return obj.isoformat()


class TestCommunityPlugins:
    def setup_method(self):
        pyrs_yaml.clear_type_handlers()

    def test_from_yaml_custom_type(self):
        """Load a tagged scalar and convert via CustomType.from_yaml."""
        pyrs_yaml.register_type("!ts", TimestampType())
        doc = pyrs_yaml.parse("when: !ts 2026-08-11T10:30:00")
        val = doc.get("when")
        assert isinstance(val, datetime), f"got {type(val)}"
        assert val.isoformat() == "2026-08-11T10:30:00"

    def test_register_type_decorator(self):
        """Decorator form of register_type."""

        @pyrs_yaml.register_type("!ts")
        class _DT(pyrs_yaml.CustomType):
            python_type = datetime

            def from_yaml(self, value):
                return datetime.fromisoformat(value)

            def to_yaml(self, obj):
                return obj.isoformat()

        doc = pyrs_yaml.parse("when: !ts 2026-08-11T10:30:00")
        val = doc.get("when")
        assert isinstance(val, datetime)

    def test_to_yaml_custom_type(self):
        """Serialize a Python object via CustomType.to_yaml."""
        pyrs_yaml.register_type("!ts", TimestampType())
        out = pyrs_yaml.safe_dump({"t": datetime(2026, 8, 11, 10, 30, tzinfo=timezone.utc)})
        assert "2026-08-11T10:30:00" in out, f"missing timestamp in {out!r}"

    def test_custom_type_round_trip(self):
        """Full round-trip: load custom type, then dump back."""
        pyrs_yaml.register_type("!ts", TimestampType())
        doc = pyrs_yaml.parse("when: !ts 2026-08-11T10:30:00")
        val = doc.get("when")
        assert isinstance(val, datetime)
        # Re-serialize
        out = pyrs_yaml.safe_dump({"when": val})
        # The serialized output should contain the tag and value
        assert "!ts" in out, f"missing tag in {out!r}"
        assert "2026-08-11T10:30:00" in out, f"missing value in {out!r}"

    def test_clear_type_handlers(self):
        """clear_type_handlers clears registered custom types."""
        pyrs_yaml.register_type("!ts", TimestampType())
        # After clearing, the tagged scalar falls through to default resolution
        # (plain string) instead of being converted to datetime.
        pyrs_yaml.clear_type_handlers()
        doc = pyrs_yaml.parse("when: !ts 2026-08-11T10:30:00")
        val = doc.get("when")
        assert not isinstance(val, datetime), f"expected plain value, got {type(val)}"

    def test_validate_custom_types_dict(self):
        """validate_custom_types validates dicts recursively."""
        pyrs_yaml.register_type("!ts", TimestampType())
        # Valid datetime passes
        pyrs_yaml.validate_custom_types({"a": datetime(2026, 8, 11, 10, 30), "b": {"c": datetime(2026, 8, 11)}})
        # Non-container value passes (no registered type match means ok)
        pyrs_yaml.validate_custom_types({"a": 42, "b": "hello"})

    def test_validate_custom_types_list_tuple_set(self):
        """validate_custom_types handles list, tuple, and set containers."""
        pyrs_yaml.register_type("!ts", TimestampType())
        pyrs_yaml.validate_custom_types([datetime(2026, 8, 11, 10, 30), 42, "x"])
        pyrs_yaml.validate_custom_types((datetime(2026, 8, 11, 10, 30), {"nested": 1}))
        pyrs_yaml.validate_custom_types({datetime(2026, 8, 11, 10, 30), datetime(2026, 8, 12)})
        pyrs_yaml.validate_custom_types(frozenset({1, 2, 3}))

    def test_validate_custom_types_failure(self):
        """validate_custom_types raises when a custom type's validate returns False."""

        class AlwaysInvalid(pyrs_yaml.CustomType):
            python_type = str

            def validate(self, obj):
                return False

        pyrs_yaml.register_type("!bad", AlwaysInvalid())
        import pytest

        with pytest.raises(ValueError, match="bad"):
            pyrs_yaml.validate_custom_types("anything")


class _FloatWrap(pyrs_yaml.CustomType):
    python_type = float

    def to_yaml(self, obj):
        return f"float<{obj}>"


class _M1(pyrs_yaml.CustomType):
    python_type = str

    def can_parse(self, value):
        return True

    def from_yaml(self, value):
        return "M1:" + value


class _M2(pyrs_yaml.CustomType):
    python_type = str

    def can_parse(self, value):
        return True

    def from_yaml(self, value):
        return "M2:" + value


class TestBindingCoverageGaps:
    """Boundary coverage for direct_dump / type_registry branches (audit §15)."""

    def setup_method(self):
        pyrs_yaml.clear_type_handlers()

    def test_custom_type_on_float_scalar_dump(self):
        # direct_dump write_scalar_node → type_registry branch for scalar floats.
        pyrs_yaml.register_type("!fw", _FloatWrap())
        out = pyrs_yaml.safe_dump(3.5)
        assert "float<3.5>" in out, out
        # Tagged output round-trips; the wrapped text is not a float so it
        # falls back to the raw string.
        assert pyrs_yaml.safe_load(out) == "float<3.5>"

    def test_custom_type_on_float_value_in_mapping(self):
        pyrs_yaml.register_type("!fw", _FloatWrap())
        out = pyrs_yaml.safe_dump({"r": 2.5})
        assert "float<2.5>" in out, out

    def test_duplicate_type_register_last_wins(self):
        # type_registry keeps insertion order; a later registration replaces
        # the earlier one for the same tag name.
        pyrs_yaml.register_type("!m", _M1())
        pyrs_yaml.register_type("!m", _M2())
        assert pyrs_yaml.safe_load("x: !m v\n")["x"] == "M2:v"

    def test_remove_type_falls_back(self):
        pyrs_yaml.register_type("!m", _M1())
        assert pyrs_yaml.safe_load("x: !m v\n")["x"] == "M1:v"
        pyrs_yaml.remove_type("!m")
        # After removal the tagged scalar is no longer converted.
        assert pyrs_yaml.safe_load("x: !m v\n")["x"] == "v"


class _IndexMoney:
    """A value type that answers the number protocol without being an integer."""

    def __init__(self, cents):
        self.cents = cents

    def __index__(self):
        return self.cents

    def __int__(self):
        return self.cents

    def __hash__(self):
        return hash(self.cents)

    def __eq__(self, other):
        return isinstance(other, _IndexMoney) and other.cents == self.cents


class _SubMoney(int):
    """The other side of that boundary: an object that really is an integer."""

    def __new__(cls, cents):
        return super().__new__(cls, cents)


class _MoneyType(pyrs_yaml.CustomType):
    python_type = _IndexMoney

    def to_yaml(self, obj):
        return f"{obj.cents}c"

    def from_yaml(self, value):
        return _IndexMoney(int(value[:-1]))


class _SubMoneyType(pyrs_yaml.CustomType):
    python_type = _SubMoney

    def to_yaml(self, obj):
        return f"{obj.cents}c"


class _DecimalType(pyrs_yaml.CustomType):
    python_type = decimal.Decimal

    def to_yaml(self, obj):
        return str(obj)


class _BoolType(pyrs_yaml.CustomType):
    python_type = bool

    def to_yaml(self, obj):
        return "yes"


def _node_route(value):
    """The same object carried by `pyobject_to_node` and written by the node serializer."""
    doc = pyrs_yaml.parse("holder: 0\n")
    doc.set("$.holder", value)
    return doc.to_yaml()


def _node_route_sorted(value):
    doc = pyrs_yaml.parse("holder: 0\n")
    doc.set("$.holder", value)
    doc.sort_keys("$.holder")
    return doc.to_yaml()


def _direct_route_sorted(value):
    buf = io.StringIO()
    pyrs_yaml.YAML("safe").dump_stream(buf, [value], sort_keys=True)
    return buf.getvalue()


# (position, the object the direct writer is given, its text, the object the node writer is given,
# its text) — every text here is measured output, and the `holder:` wrapper is the only difference
# between the two halves of a row.
POSITIONS = [
    ("value", {"m": _IndexMoney(500)}, "m: !money 500c\n", _IndexMoney(500), "holder: !money 500c\n"),
    ("key", {_IndexMoney(500): 1}, "!money 500c: 1\n", {_IndexMoney(500): 1}, "holder:\n  !money 500c: 1\n"),
    ("item", [_IndexMoney(500)], "- !money 500c\n", [_IndexMoney(500)], "holder:\n  - !money 500c\n"),
]


class TestHandlerBeatsTheNumberProtocol:
    """A registered `CustomType` owns its class, so it is asked before `__index__`/`__float__`.

    Both writers carried the ordering defect, in different shapes: `direct_dump` asked the registry
    only inside the branch the `f64` extraction landed in, and `pyobject_to_node` asked it after
    `extract::<i64>()` had already answered for the object. A class carrying `__index__` therefore
    left as the integer its protocol handed back - which is how `Decimal` reached the float-shaped
    check in the first place, and why that check never covered the integer one.
    """

    def setup_method(self):
        pyrs_yaml.clear_type_handlers()

    @pytest.mark.parametrize(
        "position,direct_obj,direct_text,node_obj,node_text",
        POSITIONS,
        ids=[row[0] for row in POSITIONS],
    )
    def test_the_handler_wins_in_every_position(self, position, direct_obj, direct_text, node_obj, node_text):
        pyrs_yaml.register_type("!money", _MoneyType())
        assert pyrs_yaml.safe_dump(direct_obj) == direct_text
        assert _node_route(node_obj) == node_text

    def test_the_tagged_key_round_trips_back_through_the_handler(self):
        pyrs_yaml.register_type("!money", _MoneyType())
        once = pyrs_yaml.safe_dump({_IndexMoney(500): 1})
        assert pyrs_yaml.safe_load(once) == {_IndexMoney(500): 1}
        assert pyrs_yaml.safe_dump(pyrs_yaml.safe_load(once)) == once

    def test_an_integer_and_a_bool_keep_their_own_spellings(self):
        """The guard is "int or int subclass", and it is the same guard on both routes.

        A registered handler for `int` or `bool` therefore does not re-spell a native integer, which
        is what keeps the common value in a real document off the registry path. `_SubMoney` is the
        boundary stated so it cannot move unnoticed: an `int` subclass is read as the integer a
        reader resolves, not as its handler's text.
        """
        pyrs_yaml.register_type("!smoney", _SubMoneyType())
        pyrs_yaml.register_type("!b", _BoolType())
        assert pyrs_yaml.safe_dump({"m": _SubMoney(500)}) == "m: 500\n"
        assert _node_route(_SubMoney(500)) == "holder: 500\n"
        assert pyrs_yaml.safe_dump({_SubMoney(500): 1}) == "500: 1\n"
        assert _node_route({_SubMoney(500): 1}) == "holder:\n  500: 1\n"
        assert pyrs_yaml.safe_dump({"m": True}) == "m: true\n"
        assert _node_route(True) == "holder: true\n"
        assert pyrs_yaml.safe_dump({True: 1}) == "true: 1\n"

    def test_decimal_keeps_its_own_text_on_both_routes(self):
        """The shape the float-only check used to be the only answer for, still honoured."""
        pyrs_yaml.register_type("!dec", _DecimalType())
        value = decimal.Decimal("1.100")
        assert pyrs_yaml.safe_dump({"m": value}) == "m: !dec 1.100\n"
        assert _node_route(value) == "holder: !dec 1.100\n"

    def test_without_a_handler_the_protocol_answer_is_unchanged(self):
        assert pyrs_yaml.safe_dump({"m": _IndexMoney(500)}) == "m: 500\n"
        assert _node_route(_IndexMoney(500)) == "holder: 500\n"

    def test_sorted_keys_order_on_the_handlers_text_on_both_routes(self):
        """`Money(5)`/`Money(50)` is the pair where the two readings of "sort the key" fork.

        The serializer's comparator reads the key node's scalar value, which is the handler's text
        once a registered type owns the key, so `50c` sorts before `5c`. Sorting the direct writer's
        keys on the protocol's integer instead would order the same mapping the other way.
        """
        pyrs_yaml.register_type("!money", _MoneyType())
        obj = {_IndexMoney(5): "a", _IndexMoney(50): "b"}
        assert _direct_route_sorted(obj) == "!money 50c: b\n!money 5c: a\n"
        assert _node_route_sorted(obj) == "holder:\n  !money 50c: b\n  !money 5c: a\n"

    def test_a_native_integer_still_sorts_on_its_digits(self):
        assert _direct_route_sorted({5: "a", 50: "b"}) == "5: a\n50: b\n"
        assert _node_route_sorted({5: "a", 50: "b"}) == "holder:\n  5: a\n  50: b\n"

    def test_a_tagged_key_and_a_plain_key_sort_in_the_same_order_on_both_routes(self):
        pyrs_yaml.register_type("!dec", _DecimalType())
        obj = {True: 1, decimal.Decimal("2"): 3}
        assert _direct_route_sorted(obj) == "!dec 2: 3\ntrue: 1\n"
        assert _node_route_sorted(obj) == "holder:\n  !dec 2: 3\n  true: 1\n"


class _Booly:
    """No `__index__` and no `__float__`: only a handler has anything to say about this one."""

    def __bool__(self):
        return True


class _MyStr(str):
    pass


class _BoolyType(pyrs_yaml.CustomType):
    python_type = _Booly

    def to_yaml(self, obj):
        return "B"

    def from_yaml(self, value):
        return _Booly()


class _MyStrType(pyrs_yaml.CustomType):
    python_type = _MyStr

    def to_yaml(self, obj):
        return "S"


class _WholeStrType(pyrs_yaml.CustomType):
    python_type = str

    def to_yaml(self, obj):
        return f"<{obj}>"


class TestHandlerOwnsTheScalarNotTheString:
    """Where the registry sits against the shapes the language already spells (#298).

    A handler owns its scalar - so a value only a handler can spell is written inline, on the line
    its key or dash already opened, and a class whose buffer the array route would read is the
    handler's business too. What it does not own is a value with a YAML-native spelling: a `str`
    (or a `str` subclass) is written as a string, and a container as a container, because that is
    what the caller's data says and what a reader resolves.
    """

    def setup_method(self):
        pyrs_yaml.clear_type_handlers()

    def teardown_method(self):
        pyrs_yaml.clear_type_handlers()

    def test_a_value_only_a_handler_can_spell_stays_on_its_line(self):
        pyrs_yaml.register_type("!booly", _BoolyType())
        assert pyrs_yaml.safe_dump({"m": _Booly()}) == "m: !booly B\n"
        assert _node_route(_Booly()) == "holder: !booly B\n"
        assert pyrs_yaml.safe_dump([_Booly()]) == "- !booly B\n"
        assert _node_route([_Booly()]) == "holder:\n  - !booly B\n"

    def test_a_compact_item_survives_a_handler_spelled_value(self):
        """The dash line is decided by what the value becomes, not by how it was classified.

        `{"a": Booly(), "b": 1}` was `-\n  a:\n    !booly B\n  b: 1` where the node route wrote
        `- a: !booly B\n    b: 1`, because the compactness test asked a class question and the
        handler had not been consulted yet.
        """
        pyrs_yaml.register_type("!booly", _BoolyType())
        item = [{"a": _Booly(), "b": 1}]
        assert pyrs_yaml.safe_dump(item) == "- a: !booly B\n  b: 1\n"
        assert _node_route(item) == "holder:\n  - a: !booly B\n    b: 1\n"

    def test_a_handler_spelled_key_is_written_by_both_writers(self):
        pyrs_yaml.register_type("!booly", _BoolyType())
        assert pyrs_yaml.safe_dump({_Booly(): 1}) == "!booly B: 1\n"
        assert _node_route({_Booly(): 1}) == "holder:\n  !booly B: 1\n"
        # ...and it reads back through the handler, so the key that comes out is the class again
        # rather than a number or a string coerced into one.
        loaded = pyrs_yaml.safe_load(pyrs_yaml.safe_dump({_Booly(): 1}))
        assert [type(k) for k in loaded] == [_Booly]
        assert list(loaded.values()) == [1]

    def test_a_string_and_a_string_subclass_are_not_rewritten_by_a_handler(self):
        pyrs_yaml.register_type("!s", _WholeStrType())
        pyrs_yaml.register_type("!mystr", _MyStrType())
        assert pyrs_yaml.safe_dump({"m": "abc"}) == "m: abc\n"
        assert _node_route("abc") == "holder: abc\n"
        assert pyrs_yaml.safe_dump({"m": _MyStr("abc")}) == "m: abc\n"
        assert _node_route(_MyStr("abc")) == "holder: abc\n"
        assert pyrs_yaml.safe_dump([_MyStr("abc")]) == "- abc\n"
        # The text a `str` handler would have produced is not in any of those bytes.
        assert "<abc>" not in _node_route("abc") + pyrs_yaml.safe_dump({"m": "abc"})

    def test_a_container_is_still_written_as_a_container(self):
        pyrs_yaml.register_type("!s", _WholeStrType())
        assert pyrs_yaml.safe_dump({"m": {"a": 1}}) == "m:\n  a: 1\n"
        assert pyrs_yaml.safe_dump({"m": [1]}) == "m:\n  - 1\n"
