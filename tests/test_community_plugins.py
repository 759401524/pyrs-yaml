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
