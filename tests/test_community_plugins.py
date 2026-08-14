"""Test Community Plugins (Spiral 3)."""

from datetime import datetime, timezone

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
