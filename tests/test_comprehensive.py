"""
Comprehensive tests for pyamlium_custom
Covers all features, APIs, and edge cases
"""

import pytest
import pyamlium_custom
import tempfile
import os


# ============================================================================
# 1. Basic Parsing Tests
# ============================================================================

class TestBasicParsing:
    """Test basic YAML parsing functionality"""

    def test_parse_scalar_string(self):
        doc = pyamlium_custom.parse("hello")
        assert doc.root_type() == "scalar"
        assert doc.to_yaml() == "hello\n"

    def test_parse_scalar_integer(self):
        doc = pyamlium_custom.parse("42")
        assert doc.get("42") is None  # Root is scalar, not mapping

    def test_parse_mapping(self):
        doc = pyamlium_custom.parse("key: value")
        assert doc.root_type() == "mapping"
        assert doc.get("key") == "value"

    def test_parse_sequence(self):
        doc = pyamlium_custom.parse("- item1\n- item2")
        assert doc.root_type() == "sequence"

    def test_parse_nested_mapping(self):
        yaml_str = "outer:\n  inner: value"
        doc = pyamlium_custom.parse(yaml_str)
        assert doc.root_type() == "mapping"

    def test_parse_empty_value(self):
        doc = pyamlium_custom.parse("key:")
        assert doc.get("key") is None

    def test_parse_null_values(self):
        for null_str in ["null", "Null", "NULL", "~"]:
            doc = pyamlium_custom.parse(f"key: {null_str}")
            assert doc.get("key") is None

    def test_parse_boolean_values(self):
        doc = pyamlium_custom.parse("t: true\nf: false")
        assert doc.get("t") is True
        assert doc.get("f") is False

    def test_parse_integer_values(self):
        doc = pyamlium_custom.parse("pos: 42\nneg: -17")
        assert doc.get("pos") == 42
        assert doc.get("neg") == -17

    def test_parse_float_values(self):
        doc = pyamlium_custom.parse("pi: 3.14\nneg: -0.5")
        assert abs(doc.get("pi") - 3.14) < 1e-10
        assert abs(doc.get("neg") - (-0.5)) < 1e-10


# ============================================================================
# 2. Quote Style Tests
# ============================================================================

class TestQuoteStyles:
    """Test different quote styles preservation"""

    def test_plain_scalar(self):
        yaml_str = "key: value"
        doc = pyamlium_custom.parse(yaml_str)
        assert doc.to_yaml() == "key: value\n"

    def test_single_quoted(self):
        yaml_str = "key: 'value'"
        doc = pyamlium_custom.parse(yaml_str)
        assert "value" in doc.to_yaml()

    def test_double_quoted(self):
        yaml_str = 'key: "value"'
        doc = pyamlium_custom.parse(yaml_str)
        assert "value" in doc.to_yaml()

    def test_special_chars_need_quotes(self):
        yaml_str = 'key: "value:with:colons"'
        doc = pyamlium_custom.parse(yaml_str)
        assert doc.get("key") == "value:with:colons"


# ============================================================================
# 3. Comment Tests
# ============================================================================

class TestComments:
    """Test comment preservation"""

    def test_standalone_comment(self):
        yaml_str = "# This is a comment\nkey: value"
        doc = pyamlium_custom.parse(yaml_str)
        output = doc.to_yaml()
        assert "# This is a comment" in output

    def test_inline_comment(self):
        yaml_str = "key: value  # inline comment"
        doc = pyamlium_custom.parse(yaml_str)
        output = doc.to_yaml()
        assert "# inline comment" in output

    def test_comment_roundtrip(self):
        yaml_str = "# Comment\nkey: value  # inline"
        doc = pyamlium_custom.parse(yaml_str)
        output = doc.to_yaml()
        assert "# Comment" in output
        assert "# inline" in output


# ============================================================================
# 4. Anchor and Alias Tests
# ============================================================================

class TestAnchorsAliases:
    """Test anchor and alias support"""

    def test_anchor_definition(self):
        yaml_str = "defaults: &defaults\n  timeout: 30"
        doc = pyamlium_custom.parse(yaml_str)
        assert "&defaults" in doc.to_yaml()

    def test_alias_reference(self):
        # Test that alias is resolved correctly (merge key resolution)
        yaml_str = "defaults: &d\n  v: 1\nprod:\n  <<: *d"
        doc = pyamlium_custom.parse(yaml_str)
        # After merge resolution, the alias is resolved to actual values
        assert doc.get("prod")["v"] == 1

    def test_alias_resolution(self):
        yaml_str = "defaults: &d\n  timeout: 30\nprod:\n  <<: *d\n  host: prod.com"
        doc = pyamlium_custom.parse(yaml_str)
        prod = doc.get("prod")
        assert prod["timeout"] == 30
        assert prod["host"] == "prod.com"


# ============================================================================
# 5. Tag Tests
# ============================================================================

class TestTags:
    """Test YAML tag support"""

    def test_primary_tag(self):
        yaml_str = "name: !!str John"
        doc = pyamlium_custom.parse(yaml_str)
        assert "!!str" in doc.to_yaml()

    def test_local_tag(self):
        yaml_str = "name: !custom value"
        doc = pyamlium_custom.parse(yaml_str)
        assert "!custom" in doc.to_yaml()

    def test_int_tag(self):
        yaml_str = "age: !!int 30"
        doc = pyamlium_custom.parse(yaml_str)
        assert "!!int" in doc.to_yaml()


# ============================================================================
# 6. Block Scalar Tests
# ============================================================================

class TestBlockScalars:
    """Test block scalar support"""

    def test_literal_block(self):
        yaml_str = "key: |\n  line1\n  line2"
        doc = pyamlium_custom.parse(yaml_str)
        assert "|" in doc.to_yaml()

    def test_folded_block(self):
        yaml_str = "key: >\n  this is\n  folded"
        doc = pyamlium_custom.parse(yaml_str)
        assert ">" in doc.to_yaml()

    def test_literal_strip(self):
        yaml_str = "key: |-\n  line1\n  line2"
        doc = pyamlium_custom.parse(yaml_str)
        assert "|-" in doc.to_yaml()

    def test_literal_keep(self):
        yaml_str = "key: |+\n  line1\n  line2\n"
        doc = pyamlium_custom.parse(yaml_str)
        assert "|+" in doc.to_yaml()

    def test_folded_strip(self):
        yaml_str = "key: >-\n  this is folded"
        doc = pyamlium_custom.parse(yaml_str)
        assert ">-" in doc.to_yaml()


# ============================================================================
# 7. Complex Key Tests
# ============================================================================

class TestComplexKeys:
    """Test complex key support"""

    def test_sequence_key(self):
        yaml_str = "? [key1, key2]\n: value"
        doc = pyamlium_custom.parse(yaml_str)
        assert "?" in doc.to_yaml()

    def test_mapping_key(self):
        yaml_str = "? {a: 1}\n: value"
        doc = pyamlium_custom.parse(yaml_str)
        assert "?" in doc.to_yaml()


# ============================================================================
# 8. Escape Sequence Tests
# ============================================================================

class TestEscapeSequences:
    """Test escape sequence handling"""

    def test_newline_escape(self):
        yaml_str = 'text: "hello\\nworld"'
        doc = pyamlium_custom.parse(yaml_str)
        assert doc.get("text") == "hello\nworld"

    def test_tab_escape(self):
        yaml_str = 'text: "hello\\tworld"'
        doc = pyamlium_custom.parse(yaml_str)
        assert doc.get("text") == "hello\tworld"

    def test_backslash_escape(self):
        yaml_str = 'text: "back\\\\slash"'
        doc = pyamlium_custom.parse(yaml_str)
        assert doc.get("text") == "back\\slash"

    def test_quote_escape(self):
        yaml_str = 'text: "say \\"hello\\""'
        doc = pyamlium_custom.parse(yaml_str)
        assert doc.get("text") == 'say "hello"'

    def test_unicode_escape(self):
        yaml_str = 'text: "\\u0041\\u0042"'
        doc = pyamlium_custom.parse(yaml_str)
        assert doc.get("text") == "AB"

    def test_null_escape(self):
        yaml_str = 'text: "a\\0b"'
        doc = pyamlium_custom.parse(yaml_str)
        assert doc.get("text") == "a\x00b"


# ============================================================================
# 9. YAML 1.2 Type Tests
# ============================================================================

class TestYaml12Types:
    """Test YAML 1.2 type resolution"""

    def test_booleans(self):
        for b in ["true", "True", "TRUE"]:
            doc = pyamlium_custom.parse(f"key: {b}")
            assert doc.get("key") is True
        for b in ["false", "False", "FALSE"]:
            doc = pyamlium_custom.parse(f"key: {b}")
            assert doc.get("key") is False

    def test_null_variants(self):
        for n in ["null", "Null", "NULL", "~"]:
            doc = pyamlium_custom.parse(f"key: {n}")
            assert doc.get("key") is None

    def test_octal_integer(self):
        doc = pyamlium_custom.parse("key: 0o14")
        assert doc.get("key") == 12

    def test_hex_integer(self):
        doc = pyamlium_custom.parse("key: 0x0C")
        assert doc.get("key") == 12

    def test_scientific_notation(self):
        doc = pyamlium_custom.parse("key: 6.022e23")
        assert abs(doc.get("key") - 6.022e23) < 1e10

    def test_infinity(self):
        doc = pyamlium_custom.parse("key: .inf")
        import math
        assert math.isinf(doc.get("key"))

    def test_nan(self):
        doc = pyamlium_custom.parse("key: .nan")
        import math
        assert math.isnan(doc.get("key"))


# ============================================================================
# 10. Serialization Tests
# ============================================================================

class TestSerialization:
    """Test YAML serialization"""

    def test_serialize_scalar(self):
        doc = pyamlium_custom.parse("key: value")
        output = doc.to_yaml()
        assert "key: value" in output

    def test_serialize_mapping(self):
        doc = pyamlium_custom.parse("a: 1\nb: 2")
        output = doc.to_yaml()
        assert "a: 1" in output
        assert "b: 2" in output

    def test_serialize_sequence(self):
        doc = pyamlium_custom.parse("- a\n- b")
        output = doc.to_yaml()
        assert "- a" in output
        assert "- b" in output


# ============================================================================
# 11. PyYAML Compatible API Tests
# ============================================================================

class TestPyyamlCompatible:
    """Test pyyaml-compatible API"""

    def test_safe_load(self):
        data = pyamlium_custom.safe_load("key: value")
        assert data == {"key": "value"}

    def test_safe_load_types(self):
        data = pyamlium_custom.safe_load("n: 42\nf: 3.14\nb: true\ns: hello")
        assert data["n"] == 42
        assert abs(data["f"] - 3.14) < 1e-10
        assert data["b"] is True
        assert data["s"] == "hello"

    def test_safe_loads(self):
        docs = pyamlium_custom.safe_loads("a: 1\n---\nb: 2")
        assert len(docs) == 2

    def test_safe_dump(self):
        data = {"key": "value", "num": 42}
        output = pyamlium_custom.safe_dump(data)
        assert "key: value" in output
        assert "42" in output

    def test_safe_dumps(self):
        data = {"key": "value"}
        output = pyamlium_custom.safe_dumps(data)
        assert "key: value" in output


# ============================================================================
# 12. From Dict/JSON Tests
# ============================================================================

class TestFromDictJson:
    """Test from_dict and from_json functions"""

    def test_from_dict_simple(self):
        data = {"name": "John", "age": 30}
        yaml_str = pyamlium_custom.from_dict(data)
        assert "name: John" in yaml_str
        assert "30" in yaml_str

    def test_from_dict_nested(self):
        data = {"app": {"name": "myapp", "version": "1.0"}}
        yaml_str = pyamlium_custom.from_dict(data)
        assert "app:" in yaml_str
        assert "name: myapp" in yaml_str

    def test_from_dict_list(self):
        data = {"items": [1, 2, 3]}
        yaml_str = pyamlium_custom.from_dict(data)
        assert "- 1" in yaml_str
        assert "- 2" in yaml_str

    def test_from_json_simple(self):
        json_str = '{"name": "Alice", "active": true}'
        yaml_str = pyamlium_custom.from_json(json_str)
        assert "name: Alice" in yaml_str
        assert "active: true" in yaml_str

    def test_from_json_nested(self):
        json_str = '{"db": {"host": "localhost", "port": 5432}}'
        yaml_str = pyamlium_custom.from_json(json_str)
        assert "db:" in yaml_str
        assert "host: localhost" in yaml_str

    def test_from_json_array(self):
        json_str = '{"items": [1, 2, 3]}'
        yaml_str = pyamlium_custom.from_json(json_str)
        assert "- 1" in yaml_str


# ============================================================================
# 13. Read Markdown Tests
# ============================================================================

class TestReadMarkdown:
    """Test read_markdown and read_markdown_str functions"""

    def test_read_markdown_with_frontmatter(self):
        md = "---\ntitle: My Post\ntags: [python]\n---\n# Content"
        frontmatter, content = pyamlium_custom.read_markdown_str(md)
        assert frontmatter is not None
        assert frontmatter["title"] == "My Post"
        assert "# Content" in content

    def test_read_markdown_no_frontmatter(self):
        md = "# Just content\nNo frontmatter here."
        frontmatter, content = pyamlium_custom.read_markdown_str(md)
        assert frontmatter is None
        assert content == md

    def test_read_markdown_empty_frontmatter(self):
        md = "---\n---\n# Content"
        frontmatter, content = pyamlium_custom.read_markdown_str(md)
        assert frontmatter is None

    def test_read_markdown_file(self):
        # Use a fixed path in temp directory
        test_file = os.path.join(tempfile.gettempdir(), "test_readme.md")
        with open(test_file, 'w') as f:
            f.write("---\ntitle: Test\n---\nContent")
        try:
            frontmatter, content = pyamlium_custom.read_markdown(test_file)
            assert frontmatter is not None
            assert frontmatter["title"] == "Test"
        finally:
            if os.path.exists(test_file):
                os.remove(test_file)


# ============================================================================
# 14. File I/O Tests
# ============================================================================

class TestFileIO:
    """Test file reading functionality"""

    def test_parse_file(self):
        test_file = os.path.join(tempfile.gettempdir(), "test_parse.yaml")
        with open(test_file, 'w') as f:
            f.write("key: value\nlist:\n  - a\n  - b")
        try:
            doc = pyamlium_custom.parse_file(test_file)
            assert doc.get("key") == "value"
        finally:
            if os.path.exists(test_file):
                os.remove(test_file)

    def test_parse_file_nonexistent(self):
        with pytest.raises(Exception):
            pyamlium_custom.parse_file("/nonexistent/file.yaml")


# ============================================================================
# 15. Edge Cases
# ============================================================================

class TestEdgeCases:
    """Test edge cases and special scenarios"""

    def test_empty_yaml(self):
        doc = pyamlium_custom.parse("")
        assert doc.root_type() == "null"

    def test_only_comment(self):
        doc = pyamlium_custom.parse("# just a comment")
        # Should not crash

    def test_special_chars_in_key(self):
        yaml_str = '"key:with:colons": value'
        doc = pyamlium_custom.parse(yaml_str)
        # Should not crash

    def test_multiline_string(self):
        yaml_str = "key: |\n  line1\n  line2\n  line3"
        doc = pyamlium_custom.parse(yaml_str)
        assert doc.root_type() == "mapping"

    def test_deeply_nested(self):
        yaml_str = "a:\n  b:\n    c:\n      d: value"
        doc = pyamlium_custom.parse(yaml_str)
        assert doc.root_type() == "mapping"

    def test_multiple_documents(self):
        yaml_str = "a: 1\n---\nb: 2"
        docs = pyamlium_custom.safe_loads(yaml_str)
        assert len(docs) == 2


# ============================================================================
# 16. Performance Sanity Tests
# ============================================================================

class TestPerformance:
    """Basic performance sanity checks"""

    def test_parse_speed(self):
        import time
        yaml_str = "key: value\nlist:\n  - a\n  - b\n  - c"
        start = time.perf_counter()
        for _ in range(1000):
            pyamlium_custom.parse(yaml_str)
        elapsed = time.perf_counter() - start
        assert elapsed < 1.0  # Should parse 1000 times in under 1 second

    def test_serialize_speed(self):
        import time
        doc = pyamlium_custom.parse("key: value\nlist:\n  - a\n  - b")
        start = time.perf_counter()
        for _ in range(1000):
            doc.to_yaml()
        elapsed = time.perf_counter() - start
        assert elapsed < 1.0


# ============================================================================
# 17. Round-Trip Tests
# ============================================================================

class TestRoundTrip:
    """Test perfect round-trip preservation"""

    def test_roundtrip_simple(self):
        original = "key: value\n"
        doc = pyamlium_custom.parse(original)
        assert doc.to_yaml() == original

    def test_roundtrip_with_comment(self):
        original = "# Comment\nkey: value\n"
        doc = pyamlium_custom.parse(original)
        assert doc.to_yaml() == original

    def test_roundtrip_inline_comment(self):
        original = "key: value  # comment\n"
        doc = pyamlium_custom.parse(original)
        assert doc.to_yaml() == original

    def test_roundtrip_anchor(self):
        original = "defaults: &defaults\n  timeout: 30\n"
        doc = pyamlium_custom.parse(original)
        assert "&defaults" in doc.to_yaml()

    def test_roundtrip_tag(self):
        original = "name: !!str John\n"
        doc = pyamlium_custom.parse(original)
        assert "!!str" in doc.to_yaml()

    def test_roundtrip_chomping(self):
        original = "key: |-\n  line1\n  line2\n"
        doc = pyamlium_custom.parse(original)
        assert "|-" in doc.to_yaml()
