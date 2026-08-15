"""Core YAML parsing tests — parsing only (no serialization)."""

import math

import pytest

import pyrs_yaml
from tests.data import yaml_samples as yaml


class TestBasicParsing:
    """Test basic YAML parsing functionality"""

    def test_parses_plain_string_as_scalar(self):
        assert pyrs_yaml.parse(yaml.PLAIN_STRING).root_type() == "scalar"

    def test_parses_mapping_from_string(self):
        assert pyrs_yaml.parse(yaml.SIMPLE_MAPPING).root_type() == "mapping"

    def test_parses_sequence_from_string(self):
        assert pyrs_yaml.parse(yaml.SEQUENCE).root_type() == "sequence"

    def test_parses_nested_mapping(self):
        assert pyrs_yaml.parse(yaml.NESTED_MAPPING).root_type() == "mapping"

    def test_parses_empty_value_as_none(self):
        assert pyrs_yaml.parse(yaml.EMPTY_VALUE).get("key") is None

    @pytest.mark.parametrize("variant", ["null", "Null", "NULL", "~"], ids=["null", "Null", "NULL", "tilde"])
    def test_parses_null_variant_as_none(self, variant):
        assert pyrs_yaml.parse(f"key: {variant}").get("key") is None

    @pytest.mark.parametrize("variant", ["true", "True", "TRUE"], ids=["true", "True", "TRUE"])
    def test_parses_boolean_true_variant(self, variant):
        assert pyrs_yaml.parse(f"key: {variant}").get("key") is True

    @pytest.mark.parametrize("variant", ["false", "False", "FALSE"], ids=["false", "False", "FALSE"])
    def test_parses_boolean_false_variant(self, variant):
        assert pyrs_yaml.parse(f"key: {variant}").get("key") is False

    @pytest.mark.parametrize(
        "yaml_str,key,expected",
        [
            ("key: 42", "key", 42),
            ("key: -17", "key", -17),
        ],
        ids=["positive", "negative"],
    )
    def test_parses_integer(self, yaml_str, key, expected):
        assert pyrs_yaml.parse(yaml_str).get(key) == expected

    @pytest.mark.parametrize(
        "yaml_str,key,expected",
        [
            ("key: 3.14", "key", 3.14),
            ("key: -0.5", "key", -0.5),
        ],
        ids=["positive", "negative"],
    )
    def test_parses_float(self, yaml_str, key, expected):
        assert abs(pyrs_yaml.parse(yaml_str).get(key) - expected) < 1e-10


class TestQuoteStyles:
    """Test different quote style parsing"""

    @pytest.mark.parametrize(
        "yaml_str,expected",
        [
            ("key: value", "value"),
            ("key: 'value'", "value"),
            ('key: "value"', "value"),
        ],
        ids=["plain", "single-quoted", "double-quoted"],
    )
    def test_parses_quoted_value(self, yaml_str, expected):
        assert pyrs_yaml.parse(yaml_str).get("key") == expected

    def test_parses_special_chars_in_quoted_value(self):
        assert pyrs_yaml.parse(yaml.SPECIAL_CHARS).get("key") == "value:with:colons"


class TestYaml12Types:
    """Test YAML 1.2 type resolution"""

    @pytest.mark.parametrize(
        "yaml_str,key,expected",
        [
            ("key: 0o14", "key", 12),
            ("key: 0x0C", "key", 12),
            ("key: 6.022e23", "key", 6.022e23),
        ],
        ids=["octal", "hex", "scientific"],
    )
    def test_parses_numeric_type(self, yaml_str, key, expected):
        value = pyrs_yaml.parse(yaml_str).get(key)
        if isinstance(expected, float):
            assert abs(value - expected) < 1e10
        else:
            assert value == expected

    def test_parses_infinity(self):
        assert math.isinf(pyrs_yaml.parse(yaml.INFINITY).get("key"))

    def test_parses_nan(self):
        assert math.isnan(pyrs_yaml.parse(yaml.NAN).get("key"))


class TestTags:
    """Test YAML tag parsing"""

    @pytest.mark.parametrize(
        "yaml_str,tag",
        [
            (yaml.TAG_STR, "!!str"),
            (yaml.TAG_CUSTOM, "!custom"),
            (yaml.TAG_INT, "!!int"),
            (yaml.TAG_VERBATIM, "!<tag:yaml.org,2002:str>"),
        ],
        ids=["primary", "local", "int", "verbatim"],
    )
    def test_preserves_tag_in_output(self, yaml_str, tag):
        assert tag in pyrs_yaml.parse(yaml_str).to_yaml()


class TestBlockScalars:
    """Test block scalar parsing"""

    @pytest.mark.parametrize(
        "yaml_str,indicator",
        [
            (yaml.LITERAL_BLOCK, "|"),
            (yaml.FOLDED_BLOCK, ">"),
            (yaml.STRIP_BLOCK, "|-"),
            (yaml.KEEP_BLOCK, "|+"),
            (yaml.FOLDED_STRIP, ">-"),
        ],
        ids=["literal", "folded", "literal-strip", "literal-keep", "folded-strip"],
    )
    def test_preserves_block_indicator(self, yaml_str, indicator):
        assert indicator in pyrs_yaml.parse(yaml_str).to_yaml()


class TestComplexKeys:
    """Test complex key parsing"""

    @pytest.mark.parametrize(
        "yaml_str", [yaml.EXPLICIT_KEY_SEQ, yaml.EXPLICIT_KEY_MAP], ids=["sequence-key", "mapping-key"]
    )
    def test_parses_explicit_key(self, yaml_str):
        assert "?" in pyrs_yaml.parse(yaml_str).to_yaml()


class TestAnchorsAliases:
    """Test anchor and alias parsing"""

    def test_parses_anchor_definition(self):
        assert "&defaults" in pyrs_yaml.parse(yaml.ANCHOR_SIMPLE).to_yaml()

    def test_resolves_alias_reference(self):
        doc = pyrs_yaml.parse(yaml.ANCHOR_MERGE)
        assert doc.get("prod")["v"] == 1

    def test_resolves_alias_with_override(self):
        prod = pyrs_yaml.parse(yaml.ANCHOR_MERGE_OVERRIDE).get("prod")
        assert prod["timeout"] == 30
        assert prod["host"] == "prod.com"


class TestComments:
    """Test comment parsing"""

    @pytest.mark.parametrize(
        "yaml_str,expected",
        [
            (yaml.COMMENT_TOP, "# This is a comment"),
            (yaml.COMMENT_INLINE, "# inline comment"),
            (yaml.COMMENT_BOTH, "# Comment"),
        ],
        ids=["standalone", "inline", "both"],
    )
    def test_preserves_comment_in_output(self, yaml_str, expected):
        assert expected in pyrs_yaml.parse(yaml_str).to_yaml()


class TestParseAllDocs:
    """Test parse_all_docs function"""

    @pytest.mark.parametrize(
        "yaml_str,expected_count,expected_values",
        [
            ("key: value", 1, {"key": "value"}),
            (yaml.MULTI_DOC_TWO, 2, {"a": 1, "b": 2}),
            ("", 0, None),
        ],
        ids=["single", "multiple", "empty"],
    )
    def test_parses_multiple_documents(self, yaml_str, expected_count, expected_values):
        docs = pyrs_yaml.parse_all_docs(yaml_str)
        assert len(docs) == expected_count
        if expected_values:
            for key, value in expected_values.items():
                assert docs[0 if key in ("a", "key") else 1].get(key) == value
