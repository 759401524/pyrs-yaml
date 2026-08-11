"""Integration tests for YAML schema profiles (core, json, failsafe, yaml1.1)."""

import pytest

import pyrs_yaml


class TestCoreSchema:
    """YAML 1.2 Core — default behavior."""

    @pytest.mark.parametrize(
        "yaml_str,key,expected",
        [
            ("x: true", "x", True),
            ("y: false", "y", False),
        ],
        ids=["true", "false"],
    )
    def test_parses_bool(self, yaml_str, key, expected):
        assert pyrs_yaml.parse(yaml_str, schema="core").get(key) is expected

    @pytest.mark.parametrize(
        "yaml_str,key",
        [
            ("a:", "a"),
            ("b: ~", "b"),
            ("c: null", "c"),
            ("d: NULL", "d"),
            ("e: Null", "e"),
        ],
        ids=["empty", "tilde", "null", "NULL", "Null"],
    )
    def test_parses_null_variant(self, yaml_str, key):
        assert pyrs_yaml.parse(yaml_str, schema="core").get(key) is None

    @pytest.mark.parametrize(
        "yaml_str,key,expected",
        [
            ("a: 42", "a", 42),
            ("b: -10", "b", -10),
            ("c: 0", "c", 0),
        ],
        ids=["positive", "negative", "zero"],
    )
    def test_parses_integer(self, yaml_str, key, expected):
        assert pyrs_yaml.parse(yaml_str, schema="core").get(key) == expected

    @pytest.mark.parametrize(
        "yaml_str,key,expected",
        [
            ("x: 0o10", "x", 8),
            ("y: 0O77", "y", 63),
        ],
        ids=["lowercase", "uppercase"],
    )
    def test_parses_octal(self, yaml_str, key, expected):
        assert pyrs_yaml.parse(yaml_str, schema="core").get(key) == expected

    @pytest.mark.parametrize(
        "yaml_str,key,expected",
        [
            ("x: 0xFF", "x", 255),
            ("y: 0X0A", "y", 10),
        ],
        ids=["lowercase", "uppercase"],
    )
    def test_parses_hex(self, yaml_str, key, expected):
        assert pyrs_yaml.parse(yaml_str, schema="core").get(key) == expected

    @pytest.mark.parametrize(
        "yaml_str,key,expected",
        [
            ("a: 3.14", "a", 3.14),
            ("b: 1e10", "b", 1e10),
            ("c: -1.5E-3", "c", -1.5e-3),
        ],
        ids=["decimal", "exponential", "negative-exponential"],
    )
    def test_parses_float(self, yaml_str, key, expected):
        assert abs(pyrs_yaml.parse(yaml_str, schema="core").get(key) - expected) < 1e-10

    @pytest.mark.parametrize(
        "yaml_str,key,expected",
        [
            ("a: .inf", "a", float("inf")),
            ("b: -.inf", "b", float("-inf")),
        ],
        ids=["positive", "negative"],
    )
    def test_parses_infinity(self, yaml_str, key, expected):
        assert pyrs_yaml.parse(yaml_str, schema="core").get(key) == expected

    @pytest.mark.parametrize(
        "yaml_str,key",
        [
            ("c: .nan", "c"),
            ("d: nan", "d"),
        ],
        ids=["dot-nan", "bare-nan"],
    )
    def test_parses_nan(self, yaml_str, key):
        value = pyrs_yaml.parse(yaml_str, schema="core").get(key)
        assert value is None or str(value).lower() == "nan"

    @pytest.mark.parametrize(
        "yaml_str,key,expected",
        [
            ("a: hello", "a", "hello"),
            ("b: 12abc", "b", "12abc"),
        ],
        ids=["plain", "numeric-prefix"],
    )
    def test_parses_string(self, yaml_str, key, expected):
        assert pyrs_yaml.parse(yaml_str, schema="core").get(key) == expected


class TestJsonSchema:
    """JSON-compatible YAML — no inf, nan, octal, hex."""

    @pytest.mark.parametrize(
        "yaml_str,key,expected",
        [
            ("x: true", "x", True),
            ("y: false", "y", False),
        ],
        ids=["true", "false"],
    )
    def test_parses_bool(self, yaml_str, key, expected):
        assert pyrs_yaml.parse(yaml_str, schema="json").get(key) is expected

    @pytest.mark.parametrize(
        "yaml_str,key",
        [
            ("a: ~", "a"),
            ("b: null", "b"),
            ("c: NULL", "c"),
        ],
        ids=["tilde", "null", "NULL"],
    )
    def test_parses_null_variant(self, yaml_str, key):
        assert pyrs_yaml.parse(yaml_str, schema="json").get(key) is None

    @pytest.mark.parametrize(
        "yaml_str,key,expected",
        [
            ("a: 42", "a", 42),
            ("b: -10", "b", -10),
        ],
        ids=["positive", "negative"],
    )
    def test_parses_integer(self, yaml_str, key, expected):
        assert pyrs_yaml.parse(yaml_str, schema="json").get(key) == expected

    @pytest.mark.parametrize(
        "yaml_str,key",
        [
            ("x: 0o10", "x"),
            ("y: 0O77", "y"),
        ],
        ids=["lowercase", "uppercase"],
    )
    def test_parses_octal_as_string(self, yaml_str, key):
        assert isinstance(pyrs_yaml.parse(yaml_str, schema="json").get(key), str)

    @pytest.mark.parametrize(
        "yaml_str,key",
        [
            ("x: 0xFF", "x"),
            ("y: 0X0A", "y"),
        ],
        ids=["lowercase", "uppercase"],
    )
    def test_parses_hex_as_string(self, yaml_str, key):
        assert isinstance(pyrs_yaml.parse(yaml_str, schema="json").get(key), str)

    @pytest.mark.parametrize(
        "yaml_str,key,expected",
        [
            ("a: .inf", "a", ".inf"),
            ("b: -.inf", "b", "-.inf"),
            ("c: inf", "c", "inf"),
            ("d: -inf", "d", "-inf"),
        ],
        ids=["dot-inf", "dot-neg-inf", "bare-inf", "bare-neg-inf"],
    )
    def test_parses_inf_as_string(self, yaml_str, key, expected):
        assert pyrs_yaml.parse(yaml_str, schema="json").get(key) == expected

    @pytest.mark.parametrize(
        "yaml_str,key,expected",
        [
            ("a: .nan", "a", ".nan"),
            ("b: nan", "b", "nan"),
        ],
        ids=["dot-nan", "bare-nan"],
    )
    def test_parses_nan_as_string(self, yaml_str, key, expected):
        assert pyrs_yaml.parse(yaml_str, schema="json").get(key) == expected


class TestFailsafeSchema:
    """Failsafe — every plain scalar is a string."""

    @pytest.mark.parametrize(
        "yaml_str,key,expected",
        [
            ("a: true", "a", "true"),
            ("b: false", "b", "false"),
            ("c: null", "c", "null"),
            ("d: 42", "d", "42"),
            ("e: 3.14", "e", "3.14"),
            ("f: .inf", "f", ".inf"),
            ("g: hello", "g", "hello"),
        ],
        ids=["bool-true", "bool-false", "null", "int", "float", "inf", "plain"],
    )
    def test_parses_scalar_as_string(self, yaml_str, key, expected):
        assert pyrs_yaml.parse(yaml_str, schema="failsafe").get(key) == expected

    def test_parses_block_scalar_as_string(self):
        assert pyrs_yaml.parse("desc: |\n  multiline\n  text", schema="failsafe").get("desc") == "multiline\ntext\n"


class TestYaml11Schema:
    """YAML 1.1 — adds legacy boolean lexemes."""

    @pytest.mark.parametrize(
        "yaml_str,key",
        [
            ("a: yes", "a"),
            ("b: no", "b"),
            ("c: Yes", "c"),
            ("d: No", "d"),
            ("e: YES", "e"),
            ("f: NO", "f"),
        ],
        ids=["yes", "no", "Yes", "No", "YES", "NO"],
    )
    def test_parses_legacy_yes_no(self, yaml_str, key):
        doc = pyrs_yaml.parse(yaml_str, schema="yaml1.1")
        expected = "yes" in yaml_str.split(":")[1].strip().lower()
        assert doc.get(key) is expected

    @pytest.mark.parametrize(
        "yaml_str,key",
        [
            ("a: y", "a"),
            ("b: n", "b"),
            ("c: Y", "c"),
            ("d: N", "d"),
        ],
        ids=["y", "n", "Y", "N"],
    )
    def test_parses_legacy_y_n(self, yaml_str, key):
        doc = pyrs_yaml.parse(yaml_str, schema="yaml1.1")
        expected = yaml_str.split(":")[1].strip().lower() == "y"
        assert doc.get(key) is expected

    @pytest.mark.parametrize(
        "yaml_str,key",
        [
            ("a: on", "a"),
            ("b: off", "b"),
            ("c: On", "c"),
            ("d: Off", "d"),
            ("e: ON", "e"),
            ("f: OFF", "f"),
        ],
        ids=["on", "off", "On", "Off", "ON", "OFF"],
    )
    def test_parses_legacy_on_off(self, yaml_str, key):
        doc = pyrs_yaml.parse(yaml_str, schema="yaml1.1")
        expected = "on" in yaml_str.split(":")[1].strip().lower()
        assert doc.get(key) is expected

    @pytest.mark.parametrize(
        "yaml_str,key,expected",
        [
            ("a: 0o10", "a", 8),
            ("b: 0xFF", "b", 255),
        ],
        ids=["octal", "hex"],
    )
    def test_parses_octal_hex(self, yaml_str, key, expected):
        assert pyrs_yaml.parse(yaml_str, schema="yaml1.1").get(key) == expected

    @pytest.mark.parametrize(
        "yaml_str,key,expected",
        [
            ("a: .inf", "a", float("inf")),
            ("b: -.inf", "b", float("-inf")),
        ],
        ids=["positive", "negative"],
    )
    def test_parses_infinity(self, yaml_str, key, expected):
        assert pyrs_yaml.parse(yaml_str, schema="yaml1.1").get(key) == expected


class TestSchemaAliases:
    """Test YAML tag alias names for schemas."""

    @pytest.mark.parametrize(
        "schema_name,yaml_str,key,expected",
        [
            ("yaml.org,2002", "x: 42", "x", 42),
            ("yaml.org,2002:json", "x: 0o10", "x", "0o10"),
            ("yaml.org,2002:failsafe", "x: true", "x", "true"),
            ("yaml.org,2002:yaml1.1", "x: on", "x", True),
            ("1.1", "x: yes", "x", True),
        ],
        ids=["core", "json", "failsafe", "yaml11", "1.1"],
    )
    def test_parses_with_schema_alias(self, schema_name, yaml_str, key, expected):
        assert pyrs_yaml.parse(yaml_str, schema=schema_name).get(key) == expected

    def test_rejects_invalid_schema(self):
        with pytest.raises(pyrs_yaml.YamlTypeError):
            pyrs_yaml.parse("x: true", schema="invalid_schema")


class TestSchemaWithParseAllDocs:
    """Schema with multi-document parsing."""

    @pytest.mark.parametrize(
        "schema,key,expected",
        [
            ("json", "a", ".inf"),
            ("core", "a", float("inf")),
        ],
        ids=["json", "core"],
    )
    def test_parses_multi_doc_with_schema(self, schema, key, expected):
        docs = pyrs_yaml.parse_all_docs("---\na: .inf\n---\nb: 0xFF", schema=schema)
        value = docs[0].get(key)
        if isinstance(expected, float):
            assert value == expected
        else:
            assert value == expected


class TestSchemaWithSafeLoad:
    """Schema with safe_load / safe_loads."""

    @pytest.mark.parametrize(
        "schema,yaml_str,key,expected",
        [
            ("json", "a: .inf\nb: 0xFF", "a", ".inf"),
            ("yaml1.1", "a: yes\nb: on", "a", True),
            ("failsafe", "a: true\nb: 42", "a", "true"),
        ],
        ids=["json", "yaml1.1", "failsafe"],
    )
    def test_safe_load_with_schema(self, schema, yaml_str, key, expected):
        assert pyrs_yaml.safe_load(yaml_str, schema=schema)[key] == expected

    def test_safe_loads_with_json_schema(self):
        result = pyrs_yaml.safe_loads("---\na: .inf\n---\nb: 0xFF", schema="json")
        assert result[0]["a"] == ".inf"
        assert result[1]["b"] == "0xFF"


class TestDefaultSchema:
    """Verify default schema='core' when no schema arg given."""

    def test_parses_with_default_core(self):
        doc = pyrs_yaml.parse("x: true\ny: 42")
        assert doc.get("x") is True
        assert doc.get("y") == 42

    def test_safe_load_default_core(self):
        assert pyrs_yaml.safe_load("a: .inf")["a"] == float("inf")

    def test_parse_all_docs_default_core(self):
        assert pyrs_yaml.parse_all_docs("---\na: 0xFF")[0].get("a") == 255


class TestSchemaRounding:
    """Schema affects round-trip through safe_load/to_dict."""

    def test_json_preserves_literals_as_strings(self):
        d = pyrs_yaml.parse("x: 0xFF\ny: .inf", schema="json").to_dict()
        assert d["x"] == "0xFF"
        assert d["y"] == ".inf"

    def test_yaml11_preserves_legacy_bools(self):
        d = pyrs_yaml.parse("x: yes\ny: on", schema="yaml1.1").to_dict()
        assert d["x"] is True
        assert d["y"] is True
