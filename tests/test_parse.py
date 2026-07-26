"""
Core YAML parsing tests — parsing only (no serialization).
"""

import pyyaml_rs

# ============================================================================
# Basic Parsing
# ============================================================================

class TestBasicParsing:
    """Test basic YAML parsing functionality"""

    def test_parse_scalar_string(self):
        doc = pyyaml_rs.parse("hello")
        assert doc.root_type() == "scalar"
        assert doc.to_yaml() == "hello\n"

    def test_parse_scalar_integer(self):
        doc = pyyaml_rs.parse("42")
        assert doc.get("42") is None  # Root is scalar, not mapping

    def test_parse_mapping(self):
        doc = pyyaml_rs.parse("key: value")
        assert doc.root_type() == "mapping"
        assert doc.get("key") == "value"

    def test_parse_sequence(self):
        doc = pyyaml_rs.parse("- item1\n- item2")
        assert doc.root_type() == "sequence"

    def test_parse_nested_mapping(self):
        yaml_str = "outer:\n  inner: value"
        doc = pyyaml_rs.parse(yaml_str)
        assert doc.root_type() == "mapping"

    def test_parse_empty_value(self):
        doc = pyyaml_rs.parse("key:")
        assert doc.get("key") is None

    def test_parse_null_values(self):
        for null_str in ["null", "Null", "NULL", "~"]:
            doc = pyyaml_rs.parse(f"key: {null_str}")
            assert doc.get("key") is None

    def test_parse_boolean_values(self):
        doc = pyyaml_rs.parse("t: true\nf: false")
        assert doc.get("t") is True
        assert doc.get("f") is False

    def test_parse_integer_values(self):
        doc = pyyaml_rs.parse("pos: 42\nneg: -17")
        assert doc.get("pos") == 42
        assert doc.get("neg") == -17

    def test_parse_float_values(self):
        doc = pyyaml_rs.parse("pi: 3.14\nneg: -0.5")
        assert abs(doc.get("pi") - 3.14) < 1e-10
        assert abs(doc.get("neg") - (-0.5)) < 1e-10


# ============================================================================
# Quote Styles
# ============================================================================

class TestQuoteStyles:
    """Test different quote styles preservation"""

    def test_plain_scalar(self):
        yaml_str = "key: value"
        doc = pyyaml_rs.parse(yaml_str)
        assert doc.to_yaml() == "key: value\n"

    def test_single_quoted(self):
        yaml_str = "key: 'value'"
        doc = pyyaml_rs.parse(yaml_str)
        assert "value" in doc.to_yaml()

    def test_double_quoted(self):
        yaml_str = 'key: "value"'
        doc = pyyaml_rs.parse(yaml_str)
        assert "value" in doc.to_yaml()

    def test_special_chars_need_quotes(self):
        yaml_str = 'key: "value:with:colons"'
        doc = pyyaml_rs.parse(yaml_str)
        assert doc.get("key") == "value:with:colons"


# ============================================================================
# YAML 1.2 Type Resolution
# ============================================================================

class TestYaml12Types:
    """Test YAML 1.2 type resolution"""

    def test_booleans(self):
        for b in ["true", "True", "TRUE"]:
            doc = pyyaml_rs.parse(f"key: {b}")
            assert doc.get("key") is True
        for b in ["false", "False", "FALSE"]:
            doc = pyyaml_rs.parse(f"key: {b}")
            assert doc.get("key") is False

    def test_null_variants(self):
        for n in ["null", "Null", "NULL", "~"]:
            doc = pyyaml_rs.parse(f"key: {n}")
            assert doc.get("key") is None

    def test_octal_integer(self):
        doc = pyyaml_rs.parse("key: 0o14")
        assert doc.get("key") == 12

    def test_hex_integer(self):
        doc = pyyaml_rs.parse("key: 0x0C")
        assert doc.get("key") == 12

    def test_scientific_notation(self):
        doc = pyyaml_rs.parse("key: 6.022e23")
        assert abs(doc.get("key") - 6.022e23) < 1e10

    def test_infinity(self):
        doc = pyyaml_rs.parse("key: .inf")
        import math
        assert math.isinf(doc.get("key"))

    def test_nan(self):
        doc = pyyaml_rs.parse("key: .nan")
        import math
        assert math.isnan(doc.get("key"))


# ============================================================================
# Tags
# ============================================================================

class TestTags:
    """Test YAML tag support"""

    def test_primary_tag(self):
        yaml_str = "name: !!str John"
        doc = pyyaml_rs.parse(yaml_str)
        assert "!!str" in doc.to_yaml()

    def test_local_tag(self):
        yaml_str = "name: !custom value"
        doc = pyyaml_rs.parse(yaml_str)
        assert "!custom" in doc.to_yaml()

    def test_int_tag(self):
        yaml_str = "age: !!int 30"
        doc = pyyaml_rs.parse(yaml_str)
        assert "!!int" in doc.to_yaml()


# ============================================================================
# Block Scalars
# ============================================================================

class TestBlockScalars:
    """Test block scalar support"""

    def test_literal_block(self):
        yaml_str = "key: |\n  line1\n  line2"
        doc = pyyaml_rs.parse(yaml_str)
        assert "|" in doc.to_yaml()

    def test_folded_block(self):
        yaml_str = "key: >\n  this is\n  folded"
        doc = pyyaml_rs.parse(yaml_str)
        assert ">" in doc.to_yaml()

    def test_literal_strip(self):
        yaml_str = "key: |-\n  line1\n  line2"
        doc = pyyaml_rs.parse(yaml_str)
        assert "|-" in doc.to_yaml()

    def test_literal_keep(self):
        yaml_str = "key: |+\n  line1\n  line2\n"
        doc = pyyaml_rs.parse(yaml_str)
        assert "|+" in doc.to_yaml()

    def test_folded_strip(self):
        yaml_str = "key: >-\n  this is folded"
        doc = pyyaml_rs.parse(yaml_str)
        assert ">-" in doc.to_yaml()


# ============================================================================
# Complex Keys
# ============================================================================

class TestComplexKeys:
    """Test complex key support"""

    def test_sequence_key(self):
        yaml_str = "? [key1, key2]\n: value"
        doc = pyyaml_rs.parse(yaml_str)
        assert "?" in doc.to_yaml()

    def test_mapping_key(self):
        yaml_str = "? {a: 1}\n: value"
        doc = pyyaml_rs.parse(yaml_str)
        assert "?" in doc.to_yaml()


# ============================================================================
# Anchors and Aliases (parsing + resolution)
# ============================================================================

class TestAnchorsAliases:
    """Test anchor and alias support"""

    def test_anchor_definition(self):
        yaml_str = "defaults: &defaults\n  timeout: 30"
        doc = pyyaml_rs.parse(yaml_str)
        assert "&defaults" in doc.to_yaml()

    def test_alias_reference(self):
        yaml_str = "defaults: &d\n  v: 1\nprod:\n  <<: *d"
        doc = pyyaml_rs.parse(yaml_str)
        assert doc.get("prod")["v"] == 1

    def test_alias_resolution(self):
        yaml_str = "defaults: &d\n  timeout: 30\nprod:\n  <<: *d\n  host: prod.com"
        doc = pyyaml_rs.parse(yaml_str)
        prod = doc.get("prod")
        assert prod["timeout"] == 30
        assert prod["host"] == "prod.com"


# ============================================================================
# Comments (parsing only — round-trip in test_roundtrip.py)
# ============================================================================

class TestComments:
    """Test comment parsing"""

    def test_standalone_comment(self):
        yaml_str = "# This is a comment\nkey: value"
        doc = pyyaml_rs.parse(yaml_str)
        output = doc.to_yaml()
        assert "# This is a comment" in output

    def test_inline_comment(self):
        yaml_str = "key: value  # inline comment"
        doc = pyyaml_rs.parse(yaml_str)
        output = doc.to_yaml()
        assert "# inline comment" in output

    def test_comment_roundtrip(self):
        yaml_str = "# Comment\nkey: value  # inline"
        doc = pyyaml_rs.parse(yaml_str)
        output = doc.to_yaml()
        assert "# Comment" in output
        assert "# inline" in output
