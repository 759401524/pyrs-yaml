"""Integration tests for YAML schema profiles (core, json, failsafe, yaml1.1)."""

import pytest

import pyyaml_rs


class TestCoreSchema:
    """YAML 1.2 Core — default behavior."""

    def test_bool(self):
        doc = pyyaml_rs.parse("x: true\ny: false", schema="core")
        assert doc.get("x") is True
        assert doc.get("y") is False

    def test_null_variants(self):
        doc = pyyaml_rs.parse("a:\nb: ~\nc: null\nd: NULL\ne: Null", schema="core")
        assert doc.get("a") is None
        assert doc.get("b") is None
        assert doc.get("c") is None
        assert doc.get("d") is None
        assert doc.get("e") is None

    def test_integer(self):
        doc = pyyaml_rs.parse("a: 42\nb: -10\nc: 0", schema="core")
        assert doc.get("a") == 42
        assert doc.get("b") == -10
        assert doc.get("c") == 0

    def test_octal(self):
        doc = pyyaml_rs.parse("x: 0o10\ny: 0O77", schema="core")
        assert doc.get("x") == 8
        assert doc.get("y") == 63

    def test_hex(self):
        doc = pyyaml_rs.parse("x: 0xFF\ny: 0X0A", schema="core")
        assert doc.get("x") == 255
        assert doc.get("y") == 10

    def test_float(self):
        doc = pyyaml_rs.parse("a: 3.14\nb: 1e10\nc: -1.5E-3", schema="core")
        assert abs(doc.get("a") - 3.14) < 1e-10
        assert abs(doc.get("b") - 1e10) < 1.0
        assert abs(doc.get("c") - (-1.5e-3)) < 1e-10

    def test_infinity_nan(self):
        doc = pyyaml_rs.parse("a: .inf\nb: -.inf\nc: .nan\nd: nan", schema="core")
        assert doc.get("a") == float("inf")
        assert doc.get("b") == float("-inf")
        assert doc.get("c") is None or str(doc.get("c")).lower() == "nan"
        assert doc.get("d") is None or str(doc.get("d")).lower() == "nan"

    def test_string(self):
        doc = pyyaml_rs.parse("a: hello\nb: 12abc", schema="core")
        assert doc.get("a") == "hello"
        assert doc.get("b") == "12abc"


class TestJsonSchema:
    """JSON-compatible YAML — no inf, nan, octal, hex."""

    def test_bool(self):
        doc = pyyaml_rs.parse("x: true\ny: false", schema="json")
        assert doc.get("x") is True
        assert doc.get("y") is False

    def test_null_variants(self):
        doc = pyyaml_rs.parse("a: ~\nb: null\nc: NULL", schema="json")
        assert doc.get("a") is None
        assert doc.get("b") is None
        assert doc.get("c") is None

    def test_integer(self):
        doc = pyyaml_rs.parse("a: 42\nb: -10", schema="json")
        assert doc.get("a") == 42
        assert doc.get("b") == -10

    def test_octal_becomes_string(self):
        doc = pyyaml_rs.parse("x: 0o10\ny: 0O77", schema="json")
        assert doc.get("x") == "0o10"
        assert doc.get("y") == "0O77"

    def test_hex_becomes_string(self):
        doc = pyyaml_rs.parse("x: 0xFF\ny: 0X0A", schema="json")
        assert doc.get("x") == "0xFF"
        assert doc.get("y") == "0X0A"

    def test_inf_becomes_string(self):
        doc = pyyaml_rs.parse("a: .inf\nb: -.inf\nc: inf\nd: -inf", schema="json")
        assert doc.get("a") == ".inf"
        assert doc.get("b") == "-.inf"
        assert doc.get("c") == "inf"
        assert doc.get("d") == "-inf"

    def test_nan_becomes_string(self):
        doc = pyyaml_rs.parse("a: .nan\nb: nan", schema="json")
        assert doc.get("a") == ".nan"
        assert doc.get("b") == "nan"


class TestFailsafeSchema:
    """Failsafe — every plain scalar is a string."""

    def test_all_strings(self):
        doc = pyyaml_rs.parse(
            "a: true\nb: false\nc: null\nd: 42\ne: 3.14\nf: .inf\ng: hello",
            schema="failsafe",
        )
        assert doc.get("a") == "true"
        assert doc.get("b") == "false"
        assert doc.get("c") == "null"
        assert doc.get("d") == "42"
        assert doc.get("e") == "3.14"
        assert doc.get("f") == ".inf"
        assert doc.get("g") == "hello"

    def test_block_scalar_is_string(self):
        doc = pyyaml_rs.parse("desc: |\n  multiline\n  text", schema="failsafe")
        assert doc.get("desc") == "multiline\ntext\n"


class TestYaml11Schema:
    """YAML 1.1 — adds legacy boolean lexemes."""

    def test_core_bools(self):
        doc = pyyaml_rs.parse("a: true\nb: false", schema="yaml1.1")
        assert doc.get("a") is True
        assert doc.get("b") is False

    def test_legacy_yes_no(self):
        doc = pyyaml_rs.parse("a: yes\nb: no\nc: Yes\nd: No\ne: YES\nf: NO", schema="yaml1.1")
        assert doc.get("a") is True
        assert doc.get("b") is False
        assert doc.get("c") is True
        assert doc.get("d") is False
        assert doc.get("e") is True
        assert doc.get("f") is False

    def test_legacy_y_n(self):
        doc = pyyaml_rs.parse("a: y\nb: n\nc: Y\nd: N", schema="yaml1.1")
        assert doc.get("a") is True
        assert doc.get("b") is False
        assert doc.get("c") is True
        assert doc.get("d") is False

    def test_legacy_on_off(self):
        doc = pyyaml_rs.parse("a: on\nb: off\nc: On\nd: Off\ne: ON\nf: OFF", schema="yaml1.1")
        assert doc.get("a") is True
        assert doc.get("b") is False
        assert doc.get("c") is True
        assert doc.get("d") is False
        assert doc.get("e") is True
        assert doc.get("f") is False

    def test_octal_hex_work(self):
        doc = pyyaml_rs.parse("a: 0o10\nb: 0xFF", schema="yaml1.1")
        assert doc.get("a") == 8
        assert doc.get("b") == 255

    def test_inf_nan_work(self):
        doc = pyyaml_rs.parse("a: .inf\nb: -.inf\nc: .nan", schema="yaml1.1")
        assert doc.get("a") == float("inf")
        assert doc.get("b") == float("-inf")


class TestSchemaAliases:
    """Test YAML tag alias names for schemas."""

    def test_yaml_org_2002(self):
        doc = pyyaml_rs.parse("x: 42", schema="yaml.org,2002")
        assert doc.get("x") == 42

    def test_json_tag_alias(self):
        doc = pyyaml_rs.parse("x: 0o10", schema="yaml.org,2002:json")
        assert doc.get("x") == "0o10"

    def test_failsafe_tag_alias(self):
        doc = pyyaml_rs.parse("x: true", schema="yaml.org,2002:failsafe")
        assert doc.get("x") == "true"

    def test_yaml11_tag_alias(self):
        doc = pyyaml_rs.parse("x: on", schema="yaml.org,2002:yaml1.1")
        assert doc.get("x") is True

    def test_invalid_schema_raises(self):
        with pytest.raises(pyyaml_rs.YamlTypeError):
            pyyaml_rs.parse("x: true", schema="invalid_schema")

    def test_1_1_alias(self):
        doc = pyyaml_rs.parse("x: yes", schema="1.1")
        assert doc.get("x") is True


class TestSchemaWithParseAllDocs:
    """Schema with multi-document parsing."""

    def test_multiple_docs_json(self):
        docs = pyyaml_rs.parse_all_docs("---\na: .inf\n---\nb: 0xFF", schema="json")
        assert docs[0].get("a") == ".inf"
        assert docs[1].get("b") == "0xFF"

    def test_multiple_docs_core(self):
        docs = pyyaml_rs.parse_all_docs("---\na: .inf\n---\nb: 0xFF", schema="core")
        assert docs[0].get("a") == float("inf")
        assert docs[1].get("b") == 255


class TestSchemaWithSafeLoad:
    """Schema with safe_load / safe_loads."""

    def test_safe_load_json(self):
        result = pyyaml_rs.safe_load("a: .inf\nb: 0xFF", schema="json")
        assert result["a"] == ".inf"
        assert result["b"] == "0xFF"

    def test_safe_load_yaml11(self):
        result = pyyaml_rs.safe_load("a: yes\nb: on", schema="yaml1.1")
        assert result["a"] is True
        assert result["b"] is True

    def test_safe_load_failsafe(self):
        result = pyyaml_rs.safe_load("a: true\nb: 42", schema="failsafe")
        assert result["a"] == "true"
        assert result["b"] == "42"

    def test_safe_loads(self):
        result = pyyaml_rs.safe_loads("---\na: .inf\n---\nb: 0xFF", schema="json")
        assert result[0]["a"] == ".inf"
        assert result[1]["b"] == "0xFF"


class TestDefaultSchema:
    """Verify default schema='core' when no schema arg given."""

    def test_default_core(self):
        doc = pyyaml_rs.parse("x: true\ny: 42")
        assert doc.get("x") is True
        assert doc.get("y") == 42

    def test_safe_load_default(self):
        result = pyyaml_rs.safe_load("a: .inf")
        assert result["a"] == float("inf")

    def test_parse_all_docs_default(self):
        docs = pyyaml_rs.parse_all_docs("---\na: 0xFF")
        assert docs[0].get("a") == 255


class TestSchemaRounding:
    """Schema affects round-trip through safe_load/to_dict."""

    def test_json_preserves_literals_as_strings(self):
        doc = pyyaml_rs.parse("x: 0xFF\ny: .inf", schema="json")
        d = doc.to_dict()
        assert d["x"] == "0xFF"
        assert d["y"] == ".inf"

    def test_yaml11_preserves_legacy_bools(self):
        doc = pyyaml_rs.parse("x: yes\ny: on", schema="yaml1.1")
        d = doc.to_dict()
        assert d["x"] is True
        assert d["y"] is True
