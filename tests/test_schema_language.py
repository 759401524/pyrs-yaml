"""Tests for the YAML Schema Language (Spiral 2 of v0.14.0)."""

import pytest

import pyrs_yaml

HEX_SCHEMA = """\
name: hex
extends: core
rules:
  - pattern: ^0x[0-9a-fA-F]+$
    type: int
"""

DATE_SCHEMA = """\
name: dates
extends: core
rules:
  - pattern: ^\\d{4}-\\d{2}-\\d{2}$
    type: str
"""

BOOL_SCHEMA = """\
name: bools
extends: failsafe
rules:
  - pattern: ^(yes|no|Yes|No|YES|NO)$
    type: bool
"""


class TestSchemaLanguage:
    """Test YAML Schema Language (register_schema + custom schema usage)."""

    def test_register_and_use_hex_schema(self):
        pyrs_yaml.register_schema("test_hex", HEX_SCHEMA)
        doc = pyrs_yaml.YAML(schema="test_hex").parse("val: 0x1F\n")
        assert doc.get("val") == 31

    def test_safe_load_with_custom_schema(self):
        pyrs_yaml.register_schema("test_safe", HEX_SCHEMA)
        d = pyrs_yaml.safe_load("h: 0xFF\nn: 42\ns: hello\n", schema="test_safe")
        assert d["h"] == 255
        assert d["n"] == 42
        assert d["s"] == "hello"

    def test_date_overrides_int(self):
        """Core schema would parse 2026-08-11 as int (starts with digit).
        Custom schema keeps it as string."""
        pyrs_yaml.register_schema("test_dates", DATE_SCHEMA)
        d = pyrs_yaml.safe_load("d: 2026-08-11\n", schema="test_dates")
        assert isinstance(d["d"], str)
        assert d["d"] == "2026-08-11"

    def test_extends_failsafe(self):
        """With extends: failsafe, non-matching values stay strings."""
        pyrs_yaml.register_schema("test_bools", BOOL_SCHEMA)
        d = pyrs_yaml.safe_load("a: yes\nb: 42\nc: hello\n", schema="test_bools")
        assert d["a"] is True
        assert d["b"] == "42"  # failsafe -> string
        assert d["c"] == "hello"

    def test_invalid_schema_name_raises(self):
        with pytest.raises(pyrs_yaml.YamlTypeError):
            pyrs_yaml.YAML(schema="nonexistent_schema")

    def test_register_schema_from_inline_yaml(self):
        schema = "name: binary\nextends: core\nrules:\n  - pattern: ^0b[01]+$\n    type: int\n"
        pyrs_yaml.register_schema("test_binary", schema)
        doc = pyrs_yaml.YAML(schema="test_binary").parse("b: 0b1010\n")
        assert doc.get("b") == 10

    def test_core_schema_unchanged(self):
        """Built-in core schema still works as before."""
        d = pyrs_yaml.safe_load("h: 0x1F\n", schema="core")
        assert d["h"] == 31

    def test_failsafe_schema_unchanged(self):
        d = pyrs_yaml.safe_load("n: 42\n", schema="failsafe")
        assert d["n"] == "42"

    def test_round_trip_with_custom_schema(self):
        pyrs_yaml.register_schema("test_rt", HEX_SCHEMA)
        doc = pyrs_yaml.parse("val: 0xFF\n", schema="test_rt")
        assert doc.get("val") == 255
        out = doc.to_yaml()
        assert "val: 255" in out or "val: 0xFF" in out

    def test_inline_dict_missing_rules_raises(self):
        """Inline schema dict without rules raises a clear error."""
        with pytest.raises(ValueError, match="rules"):
            pyrs_yaml.YAML(schema={"extends": "core"})

    def test_inline_dict_empty_rules_raises(self):
        with pytest.raises(ValueError, match="rules"):
            pyrs_yaml.YAML(schema={"rules": []})

    def test_inline_dict_missing_pattern_raises(self):
        with pytest.raises(ValueError, match="pattern"):
            pyrs_yaml.YAML(schema={"rules": [{"type": "int"}]})

    def test_inline_dict_default_extends_core(self):
        """Inline dict without 'extends' defaults to core."""
        y = pyrs_yaml.YAML(schema={"rules": [{"pattern": "^0x[0-9a-fA-F]+$", "type": "int"}]})
        d = y.safe_load("h: 0xFF\nn: 42\n")
        assert d["h"] == 255
        assert d["n"] == 42  # core fallback resolves int

    def test_coerce_schema_invalid_type(self):
        """_coerce_schema raises TypeError for non-str, non-dict input."""
        match = r"schema must be str or dict"
        with pytest.raises(TypeError, match=match):
            pyrs_yaml.parse("a: 1", schema=42)
        with pytest.raises(TypeError, match=match):
            pyrs_yaml.safe_load("a: 1", schema=[1, 2, 3])
        with pytest.raises(TypeError, match=match):
            pyrs_yaml.parse_file("dummy.yaml", schema=None)

    def test_parse_schema_yaml_invalid_yaml(self):
        """Invalid YAML string raises YamlParseError, not silent suppression."""
        with pytest.raises((pyrs_yaml.YamlParseError, ValueError)):
            pyrs_yaml.register_schema("bad", "not: valid: yaml: [[[")
        with pytest.raises((pyrs_yaml.YamlParseError, ValueError)):
            pyrs_yaml.register_schema("bad2", "rules: [invalid}")


class TestSchemaRegression:
    """Regression tests for the schema system."""

    def test_re_register_same_schema(self):
        """Re-registering the same schema by name is idempotent."""
        pyrs_yaml.register_schema("hex2", HEX_SCHEMA)
        pyrs_yaml.register_schema("hex2", HEX_SCHEMA)  # no error
        y = pyrs_yaml.YAML(schema="hex2")
        d = y.parse("a: 0xFF")
        assert d.get("a") == 255


class TestSchemaFileIO:
    """Tests for load_schema() and list_schemas()."""

    def test_list_schemas_builtin(self):
        """list_schemas() returns the four built-in schemas."""
        names = pyrs_yaml.list_schemas()
        for builtin in ("failsafe", "json", "core", "yaml1.1"):
            assert builtin in names

    def test_list_schemas_after_register(self):
        """list_schemas() includes custom schemas after registration."""
        pyrs_yaml.register_schema("test_list_io", HEX_SCHEMA)
        assert "test_list_io" in pyrs_yaml.list_schemas()

    def test_load_schema_from_file(self, tmp_path):
        """load_schema() reads a schema definition from a file and registers it."""
        schema_file = tmp_path / "hex.yaml"
        schema_file.write_text(HEX_SCHEMA, encoding="utf-8")
        pyrs_yaml.load_schema("test_load_hex", str(schema_file))
        assert "test_load_hex" in pyrs_yaml.list_schemas()
        doc = pyrs_yaml.parse("a: 0xFF", schema="test_load_hex")
        assert doc.get("a") == 255

    def test_load_schema_nonexistent_file(self):
        """load_schema() raises on a missing file."""
        with pytest.raises((pyrs_yaml.YamlParseError, OSError, FileNotFoundError)):
            pyrs_yaml.load_schema("nope", "/nonexistent/path/schema.yaml")

    def test_load_schema_invalid_content(self, tmp_path):
        """load_schema() raises on invalid schema YAML content."""
        bad = tmp_path / "bad.yaml"
        bad.write_text("not: valid: yaml: [[[\n", encoding="utf-8")
        with pytest.raises((pyrs_yaml.YamlParseError, ValueError)):
            pyrs_yaml.load_schema("bad_load", str(bad))

    def test_load_schema_date_schema(self, tmp_path):
        """load_schema() works with different schema definitions (date)."""
        schema_file = tmp_path / "dates.yaml"
        schema_file.write_text(DATE_SCHEMA, encoding="utf-8")
        pyrs_yaml.load_schema("test_load_dates", str(schema_file))
        doc = pyrs_yaml.parse("d: 2026-08-16", schema="test_load_dates")
        assert doc.get("d") == "2026-08-16"
