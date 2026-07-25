"""
Comprehensive tests for pyyaml_rs
Covers all features, APIs, and edge cases
"""

import pytest
import pyyaml_rs
import tempfile
import os


# ============================================================================
# 1. Basic Parsing Tests
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
# 2. Quote Style Tests
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
# 3. Comment Tests
# ============================================================================

class TestComments:
    """Test comment preservation"""

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


# ============================================================================
# 4. Anchor and Alias Tests
# ============================================================================

class TestAnchorsAliases:
    """Test anchor and alias support"""

    def test_anchor_definition(self):
        yaml_str = "defaults: &defaults\n  timeout: 30"
        doc = pyyaml_rs.parse(yaml_str)
        assert "&defaults" in doc.to_yaml()

    def test_alias_reference(self):
        # Test that alias is resolved correctly (merge key resolution)
        yaml_str = "defaults: &d\n  v: 1\nprod:\n  <<: *d"
        doc = pyyaml_rs.parse(yaml_str)
        # After merge resolution, the alias is resolved to actual values
        assert doc.get("prod")["v"] == 1

    def test_alias_resolution(self):
        yaml_str = "defaults: &d\n  timeout: 30\nprod:\n  <<: *d\n  host: prod.com"
        doc = pyyaml_rs.parse(yaml_str)
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
# 6. Block Scalar Tests
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
# 7. Complex Key Tests
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
# 8. Escape Sequence Tests
# ============================================================================

class TestEscapeSequences:
    """Test escape sequence handling"""

    def test_newline_escape(self):
        yaml_str = 'text: "hello\\nworld"'
        doc = pyyaml_rs.parse(yaml_str)
        assert doc.get("text") == "hello\nworld"

    def test_tab_escape(self):
        yaml_str = 'text: "hello\\tworld"'
        doc = pyyaml_rs.parse(yaml_str)
        assert doc.get("text") == "hello\tworld"

    def test_backslash_escape(self):
        yaml_str = 'text: "back\\\\slash"'
        doc = pyyaml_rs.parse(yaml_str)
        assert doc.get("text") == "back\\slash"

    def test_quote_escape(self):
        yaml_str = 'text: "say \\"hello\\""'
        doc = pyyaml_rs.parse(yaml_str)
        assert doc.get("text") == 'say "hello"'

    def test_unicode_escape(self):
        yaml_str = 'text: "\\u0041\\u0042"'
        doc = pyyaml_rs.parse(yaml_str)
        assert doc.get("text") == "AB"

    def test_null_escape(self):
        yaml_str = 'text: "a\\0b"'
        doc = pyyaml_rs.parse(yaml_str)
        assert doc.get("text") == "a\x00b"


# ============================================================================
# 9. YAML 1.2 Type Tests
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
# 10. Serialization Tests
# ============================================================================

class TestSerialization:
    """Test YAML serialization"""

    def test_serialize_scalar(self):
        doc = pyyaml_rs.parse("key: value")
        output = doc.to_yaml()
        assert "key: value" in output

    def test_serialize_mapping(self):
        doc = pyyaml_rs.parse("a: 1\nb: 2")
        output = doc.to_yaml()
        assert "a: 1" in output
        assert "b: 2" in output

    def test_serialize_sequence(self):
        doc = pyyaml_rs.parse("- a\n- b")
        output = doc.to_yaml()
        assert "- a" in output
        assert "- b" in output


# ============================================================================
# 11. PyYAML Compatible API Tests
# ============================================================================

class TestPyyamlCompatible:
    """Test pyyaml-compatible API"""

    def test_safe_load(self):
        data = pyyaml_rs.safe_load("key: value")
        assert data == {"key": "value"}

    def test_safe_load_types(self):
        data = pyyaml_rs.safe_load("n: 42\nf: 3.14\nb: true\ns: hello")
        assert data["n"] == 42
        assert abs(data["f"] - 3.14) < 1e-10
        assert data["b"] is True
        assert data["s"] == "hello"

    def test_safe_loads(self):
        docs = pyyaml_rs.safe_loads("a: 1\n---\nb: 2")
        assert len(docs) == 2

    def test_safe_dump(self):
        data = {"key": "value", "num": 42}
        output = pyyaml_rs.safe_dump(data)
        assert "key: value" in output
        assert "42" in output

    def test_safe_dumps(self):
        data = {"key": "value"}
        output = pyyaml_rs.safe_dumps(data)
        assert "key: value" in output


# ============================================================================
# 12. From Dict/JSON Tests
# ============================================================================

class TestFromDictJson:
    """Test from_dict and from_json functions"""

    def test_from_dict_simple(self):
        data = {"name": "John", "age": 30}
        yaml_str = pyyaml_rs.from_dict(data)
        assert "name: John" in yaml_str
        assert "30" in yaml_str

    def test_from_dict_nested(self):
        data = {"app": {"name": "myapp", "version": "1.0"}}
        yaml_str = pyyaml_rs.from_dict(data)
        assert "app:" in yaml_str
        assert "name: myapp" in yaml_str

    def test_from_dict_list(self):
        data = {"items": [1, 2, 3]}
        yaml_str = pyyaml_rs.from_dict(data)
        assert "- 1" in yaml_str
        assert "- 2" in yaml_str

    def test_from_json_simple(self):
        json_str = '{"name": "Alice", "active": true}'
        yaml_str = pyyaml_rs.from_json(json_str)
        assert "name: Alice" in yaml_str
        assert "active: true" in yaml_str

    def test_from_json_nested(self):
        json_str = '{"db": {"host": "localhost", "port": 5432}}'
        yaml_str = pyyaml_rs.from_json(json_str)
        assert "db:" in yaml_str
        assert "host: localhost" in yaml_str

    def test_from_json_array(self):
        json_str = '{"items": [1, 2, 3]}'
        yaml_str = pyyaml_rs.from_json(json_str)
        assert "- 1" in yaml_str


# ============================================================================
# 13. Read Markdown Tests
# ============================================================================

class TestReadMarkdown:
    """Test read_markdown and read_markdown_str functions"""

    def test_read_markdown_with_frontmatter(self):
        md = "---\ntitle: My Post\ntags: [python]\n---\n# Content"
        frontmatter, content = pyyaml_rs.read_markdown_str(md)
        assert frontmatter is not None
        assert frontmatter["title"] == "My Post"
        assert "# Content" in content

    def test_read_markdown_no_frontmatter(self):
        md = "# Just content\nNo frontmatter here."
        frontmatter, content = pyyaml_rs.read_markdown_str(md)
        assert frontmatter is None
        assert content == md

    def test_read_markdown_empty_frontmatter(self):
        md = "---\n---\n# Content"
        frontmatter, content = pyyaml_rs.read_markdown_str(md)
        assert frontmatter is None

    def test_read_markdown_file(self):
        # Use a fixed path in temp directory
        test_file = os.path.join(tempfile.gettempdir(), "test_readme.md")
        with open(test_file, 'w') as f:
            f.write("---\ntitle: Test\n---\nContent")
        try:
            frontmatter, content = pyyaml_rs.read_markdown(test_file)
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
            doc = pyyaml_rs.parse_file(test_file)
            assert doc.get("key") == "value"
        finally:
            if os.path.exists(test_file):
                os.remove(test_file)

    def test_parse_file_nonexistent(self):
        with pytest.raises(Exception):
            pyyaml_rs.parse_file("/nonexistent/file.yaml")


# ============================================================================
# 15. Edge Cases
# ============================================================================

class TestEdgeCases:
    """Test edge cases and special scenarios"""

    def test_empty_yaml(self):
        doc = pyyaml_rs.parse("")
        assert doc.root_type() == "null"

    def test_only_comment(self):
        doc = pyyaml_rs.parse("# just a comment")
        # Should not crash

    def test_special_chars_in_key(self):
        yaml_str = '"key:with:colons": value'
        doc = pyyaml_rs.parse(yaml_str)
        # Should not crash

    def test_multiline_string(self):
        yaml_str = "key: |\n  line1\n  line2\n  line3"
        doc = pyyaml_rs.parse(yaml_str)
        assert doc.root_type() == "mapping"

    def test_deeply_nested(self):
        yaml_str = "a:\n  b:\n    c:\n      d: value"
        doc = pyyaml_rs.parse(yaml_str)
        assert doc.root_type() == "mapping"

    def test_multiple_documents(self):
        yaml_str = "a: 1\n---\nb: 2"
        docs = pyyaml_rs.safe_loads(yaml_str)
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
            pyyaml_rs.parse(yaml_str)
        elapsed = time.perf_counter() - start
        assert elapsed < 1.0  # Should parse 1000 times in under 1 second

    def test_serialize_speed(self):
        import time
        doc = pyyaml_rs.parse("key: value\nlist:\n  - a\n  - b")
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
        doc = pyyaml_rs.parse(original)
        assert doc.to_yaml() == original

    def test_roundtrip_with_comment(self):
        original = "# Comment\nkey: value\n"
        doc = pyyaml_rs.parse(original)
        assert doc.to_yaml() == original

    def test_roundtrip_inline_comment(self):
        original = "key: value  # comment\n"
        doc = pyyaml_rs.parse(original)
        assert doc.to_yaml() == original

    def test_roundtrip_anchor(self):
        original = "defaults: &defaults\n  timeout: 30\n"
        doc = pyyaml_rs.parse(original)
        assert "&defaults" in doc.to_yaml()

    def test_roundtrip_tag(self):
        original = "name: !!str John\n"
        doc = pyyaml_rs.parse(original)
        assert "!!str" in doc.to_yaml()

    def test_roundtrip_chomping(self):
        original = "key: |-\n  line1\n  line2\n"
        doc = pyyaml_rs.parse(original)
        assert "|-" in doc.to_yaml()

    def test_roundtrip_nested_mapping(self):
        original = "parent:\n  child1: value1\n  child2: value2\n"
        doc = pyyaml_rs.parse(original)
        assert doc.to_yaml() == original

    def test_roundtrip_sequence(self):
        original = "- item1\n- item2\n- item3\n"
        doc = pyyaml_rs.parse(original)
        assert doc.to_yaml() == original

    def test_roundtrip_mixed(self):
        original = "list:\n  - a\n  - b\nmapping:\n  key: value\n"
        doc = pyyaml_rs.parse(original)
        assert doc.to_yaml() == original

    def test_roundtrip_chomping_variants(self):
        for indicator in ["|-", "|", "|+", ">-", ">", ">+"]:
            if indicator.startswith("|"):
                original = f"key: {indicator}\n  line1\n  line2\n"
            else:
                original = f"key: {indicator}\n  line1\n  line2\n"
            doc = pyyaml_rs.parse(original)
            assert indicator in doc.to_yaml(), f"Chomping indicator {indicator} lost in round-trip"

    def test_roundtrip_multiple_anchors(self):
        original = "a: &anchor1 val1\nb: &anchor2 val2\n"
        doc = pyyaml_rs.parse(original)
        output = doc.to_yaml()
        assert "&anchor1" in output
        assert "&anchor2" in output

    def test_roundtrip_empty_values(self):
        original = "key1:\nkey2: value\n"
        doc = pyyaml_rs.parse(original)
        output = doc.to_yaml()
        assert "key1:" in output
        assert "key2: value" in output

    def test_roundtrip_alias_reference(self):
        original = "defaults: &defaults\n  timeout: 30\nproduction:\n  <<: *defaults\n  host: x\n"
        doc = pyyaml_rs.parse(original, resolve_merges=False)
        output = doc.to_yaml()
        assert "*defaults" in output

    def test_roundtrip_scalar_styles(self):
        for style_yaml, expected_marker in [
            ("key: plain_value\n", "plain_value"),
            ("key: 'single quoted'\n", "'single quoted'"),
            ('key: "double quoted"\n', '"double quoted"'),
        ]:
            doc = pyyaml_rs.parse(style_yaml)
            assert expected_marker in doc.to_yaml(), f"Scalar style marker {expected_marker} lost"

    def test_roundtrip_local_tag(self):
        original = "key: !custom value\n"
        doc = pyyaml_rs.parse(original)
        assert "!custom" in doc.to_yaml()

    def test_roundtrip_complex_key(self):
        original = "? [key1, key2]\n: value\n"
        doc = pyyaml_rs.parse(original)
        output = doc.to_yaml()
        assert "?" in output or "key1" in output

    @pytest.mark.xfail(reason="Flow collections {} and [] not supported in AST yet")
    def test_roundtrip_empty_mapping(self):
        original = "{}\n"
        doc = pyyaml_rs.parse(original)
        assert doc.to_yaml() == original

    @pytest.mark.xfail(reason="Flow collections {} and [] not supported in AST yet")
    def test_roundtrip_empty_sequence(self):
        original = "[]\n"
        doc = pyyaml_rs.parse(original)
        assert doc.to_yaml() == original
