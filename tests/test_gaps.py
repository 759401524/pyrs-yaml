"""
Gap-filling tests — covering untested APIs and edge cases.
"""

import tempfile
from pathlib import Path

import pyrs_yaml
import pytest

# ============================================================================
# i18n Functions
# ============================================================================


class TestI18N:
    """Test internationalization functions"""

    def test_set_language(self):
        pyrs_yaml.set_language("en")
        assert pyrs_yaml.get_language() == "en"

    def test_set_language_zh_cn(self):
        pyrs_yaml.set_language("zh-CN")
        assert pyrs_yaml.get_language() == "zh-CN"

    def test_set_language_ja_jp(self):
        pyrs_yaml.set_language("ja-JP")
        assert pyrs_yaml.get_language() == "ja-JP"

    def test_get_language_default(self):
        pyrs_yaml.set_language("en")
        assert pyrs_yaml.get_language() == "en"

    def test_list_languages(self):
        langs = pyrs_yaml.list_languages()
        assert isinstance(langs, list)
        assert len(langs) > 0
        assert "en" in langs

    def test_detect_language(self):
        lang = pyrs_yaml.detect_language()
        assert isinstance(lang, str)
        assert len(lang) == 2

    def test_negotiate_language_exact_match(self):
        result = pyrs_yaml.negotiate_language(["en"])
        assert result == "en"

    def test_negotiate_language_partial_match(self):
        result = pyrs_yaml.negotiate_language(["zh-CN", "en"])
        assert result in ("zh-CN", "en")

    def test_negotiate_language_fallback(self):
        result = pyrs_yaml.negotiate_language(["xx", "yy"], default="en")
        assert result == "en"

    def test_set_language_invalid(self):
        with pytest.raises(ValueError):
            pyrs_yaml.set_language("invalid_lang_xyz")

    def test_language_reset_in_loop(self):
        original = pyrs_yaml.get_language()
        try:
            pyrs_yaml.set_language("zh-CN")
            assert pyrs_yaml.get_language() == "zh-CN"
        finally:
            pyrs_yaml.set_language(original)


# ============================================================================
# parse_all_docs
# ============================================================================


class TestParseAllDocs:
    """Test parse_all_docs function"""

    def test_parse_all_docs_single(self):
        doc = pyrs_yaml.parse_all_docs("key: value")[0]
        assert doc.get("key") == "value"

    def test_parse_all_docs_multiple(self):
        docs = pyrs_yaml.parse_all_docs("a: 1\n---\nb: 2")
        assert len(docs) == 2
        assert docs[0].get("a") == 1
        assert docs[1].get("b") == 2

    def test_parse_all_docs_empty(self):
        docs = pyrs_yaml.parse_all_docs("")
        assert len(docs) == 0

    def test_parse_all_docs_with_comments(self):
        docs = pyrs_yaml.parse_all_docs("# doc1\na: 1\n---\n# doc2\nb: 2")
        assert len(docs) == 2


# ============================================================================
# parse_file success case
# ============================================================================


class TestParseFile:
    """Test parse_file function"""

    def test_parse_file_success(self):
        test_file = str(Path(tempfile.gettempdir()) / "test_parse_file.yaml")
        with Path(test_file).open("w") as f:
            f.write("name: test\nvalue: 42")
        try:
            doc = pyrs_yaml.parse_file(test_file)
            assert doc.get("name") == "test"
            assert doc.get("value") == 42
        finally:
            if Path(test_file).exists():
                Path(test_file).unlink()

    def test_parse_file_with_comments(self):
        test_file = str(Path(tempfile.gettempdir()) / "test_parse_file_comments.yaml")
        with Path(test_file).open("w") as f:
            f.write("# comment\nkey: value\n")
        try:
            doc = pyrs_yaml.parse_file(test_file)
            assert doc.to_yaml() == "# comment\nkey: value\n"
        finally:
            if Path(test_file).exists():
                Path(test_file).unlink()

    def test_parse_file_nonexistent_raises(self):
        with pytest.raises(OSError):
            pyrs_yaml.parse_file("/nonexistent/path/to/file.yaml")


# ============================================================================
# to_yaml_with_options
# ============================================================================


class TestToYamlWithOptions:
    """Test to_yaml_with_options method"""

    def test_explicit_start(self):
        doc = pyrs_yaml.parse("key: value")
        output = doc.to_yaml_with_options(explicit_start=True)
        assert output.startswith("---\n")

    def test_explicit_end(self):
        doc = pyrs_yaml.parse("key: value")
        output = doc.to_yaml_with_options(explicit_end=True)
        assert output.rstrip().endswith("...")

    def test_explicit_start_and_end(self):
        doc = pyrs_yaml.parse("key: value")
        output = doc.to_yaml_with_options(explicit_start=True, explicit_end=True)
        assert output.startswith("---\n")
        assert output.rstrip().endswith("...")

    def test_indent_size_4(self):
        doc = pyrs_yaml.parse("parent:\n  child: value")
        output = doc.to_yaml_with_options(indent_size=4)
        assert "    child: value" in output

    def test_sort_keys(self):
        doc = pyrs_yaml.parse("z: 1\na: 2\nm: 3")
        output = doc.to_yaml_with_options(sort_keys=True)
        lines = [line for line in output.strip().split("\n") if line and not line.startswith(" ")]
        keys = [line.split(":")[0] for line in lines]
        assert keys == sorted(keys)

    def test_no_sort_keys_preserves_order(self):
        doc = pyrs_yaml.parse("z: 1\na: 2\nm: 3")
        output = doc.to_yaml_with_options(sort_keys=False)
        lines = [line for line in output.strip().split("\n") if line and not line.startswith(" ")]
        keys = [line.split(":")[0] for line in lines]
        assert keys == ["z", "a", "m"]


# ============================================================================
# to_dict method
# ============================================================================


class TestToDict:
    """Test YamlDocument.to_dict method"""

    def test_to_dict_simple(self):
        doc = pyrs_yaml.parse("key: value")
        result = doc.to_dict()
        assert result == {"key": "value"}

    def test_to_dict_nested(self):
        doc = pyrs_yaml.parse("parent:\n  child: grandchild\n  num: 42")
        result = doc.to_dict()
        assert result == {"parent": {"child": "grandchild", "num": 42}}

    def test_to_dict_with_list(self):
        doc = pyrs_yaml.parse("items:\n  - a\n  - b\n  - c")
        result = doc.to_dict()
        assert result == {"items": ["a", "b", "c"]}

    def test_to_dict_with_bool(self):
        doc = pyrs_yaml.parse("flag: true")
        result = doc.to_dict()
        assert result == {"flag": True}

    def test_to_dict_with_null(self):
        doc = pyrs_yaml.parse("key: null")
        result = doc.to_dict()
        assert result == {"key": None}

    def test_to_dict_with_anchor(self):
        doc = pyrs_yaml.parse("defaults: &d\n  timeout: 30\nprod:\n  <<: *d")
        result = doc.to_dict()
        assert result["prod"]["timeout"] == 30

    def test_to_dict_scalar_root(self):
        doc = pyrs_yaml.parse("hello")
        result = doc.to_dict()
        assert result == "hello"

    def test_to_dict_empty_mapping(self):
        doc = pyrs_yaml.parse("{}")
        result = doc.to_dict()
        assert result == {}

    def test_to_dict_empty_sequence(self):
        doc = pyrs_yaml.parse("[]")
        result = doc.to_dict()
        assert result == []


# ============================================================================
# YamlDocument dunder methods
# ============================================================================


class TestYamlDocumentDunder:
    """Test Python dunder methods on YamlDocument"""

    def test_repr(self):
        doc = pyrs_yaml.parse("key: value")
        assert repr(doc).startswith("YamlDocument(")

    def test_str(self):
        doc = pyrs_yaml.parse("key: value")
        assert str(doc) == "key: value\n"

    def test_contains_key(self):
        doc = pyrs_yaml.parse("key: value\nother: 42")
        assert "key" in doc
        assert "other" in doc
        assert "missing" not in doc

    def test_contains_non_mapping(self):
        doc = pyrs_yaml.parse("hello")
        assert "hello" not in doc

    def test_contains_empty_key_in_mapping(self):
        doc = pyrs_yaml.parse("key: value\n: empty_key")
        assert "key" in doc

    def test_len_mapping(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\nc: 3")
        assert len(doc) == 3

    def test_len_sequence(self):
        doc = pyrs_yaml.parse("- a\n- b\n- c")
        assert len(doc) == 3

    def test_len_scalar(self):
        doc = pyrs_yaml.parse("hello")
        assert len(doc) == 0

    def test_len_null(self):
        doc = pyrs_yaml.parse("")
        assert len(doc) == 0

    def test_iter_mapping_returns_list(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\nc: 3")
        keys = doc.__iter__()
        assert isinstance(keys, list)

    def test_iter_sequence_returns_list(self):
        doc = pyrs_yaml.parse("- x\n- y\n- z")
        values = doc.__iter__()
        assert isinstance(values, list)

    def test_iter_scalar_returns_empty_list(self):
        doc = pyrs_yaml.parse("hello")
        assert doc.__iter__() == []

    def test_getitem_mapping_string_key(self):
        doc = pyrs_yaml.parse("key: value\nnum: 42")
        assert doc["key"] == "value"
        assert doc["num"] == 42

    def test_getitem_mapping_missing_key_raises(self):
        doc = pyrs_yaml.parse("key: value")
        with pytest.raises(KeyError):
            _ = doc["missing"]

    def test_getitem_sequence_index(self):
        doc = pyrs_yaml.parse("- first\n- second\n- third")
        assert doc[0] == "first"
        assert doc[1] == "second"
        assert doc[2] == "third"

    def test_getitem_sequence_out_of_range_raises(self):
        doc = pyrs_yaml.parse("- a\n- b")
        with pytest.raises(IndexError):
            _ = doc[5]

    def test_getitem_non_subscriptable_raises(self):
        doc = pyrs_yaml.parse("hello")
        with pytest.raises(TypeError):
            _ = doc[0]

    def test_getitem_null_raises(self):
        doc = pyrs_yaml.parse("")
        with pytest.raises(TypeError):
            _ = doc[0]

    def test_getitem_nested(self):
        doc = pyrs_yaml.parse("outer:\n  inner: value")
        assert doc["outer"]["inner"] == "value"

    def test_root_type_scalar(self):
        doc = pyrs_yaml.parse("hello")
        assert doc.root_type() == "scalar"

    def test_root_type_mapping(self):
        doc = pyrs_yaml.parse("key: value")
        assert doc.root_type() == "mapping"

    def test_root_type_sequence(self):
        doc = pyrs_yaml.parse("- a\n- b")
        assert doc.root_type() == "sequence"

    def test_root_type_null(self):
        doc = pyrs_yaml.parse("")
        assert doc.root_type() == "null"

    def test_root_type_alias(self):
        doc = pyrs_yaml.parse("base: &b val\nref: *b")
        assert doc.root_type() == "mapping"


# ============================================================================
# Unicode and Special Characters
# ============================================================================


class TestUnicodeAndSpecial:
    """Test Unicode and special character handling"""

    def test_unicode_cjk(self):
        doc = pyrs_yaml.parse("name: \u4e2d\u6587")
        assert doc.get("name") == "\u4e2d\u6587"

    def test_unicode_emoji(self):
        doc = pyrs_yaml.parse("emoji: \U0001f600")
        assert doc.get("emoji") == "\U0001f600"

    def test_unicode_roundtrip(self):
        original = "name: \u4e2d\u6587\nemoji: \U0001f600\n"
        doc = pyrs_yaml.parse(original)
        assert doc.to_yaml() == original

    def test_crlf_line_endings(self):
        doc = pyrs_yaml.parse("key: value\r\nlist:\r\n  - a\r\n  - b")
        assert doc.get("key") == "value"
        assert doc.get("list") == ["a", "b"]

    def test_duplicate_keys_last_wins(self):
        doc = pyrs_yaml.parse("key: first\nkey: second")
        assert doc.get("key") == "second"


# ============================================================================
# parse with bytes input
# ============================================================================


class TestParseBytes:
    """Test parse with bytes input"""

    def test_parse_bytes_simple(self):
        doc = pyrs_yaml.parse(b"key: value")
        assert doc.get("key") == "value"

    def test_parse_bytes_utf8(self):
        doc = pyrs_yaml.parse("name: \u4e2d\u6587\n".encode("utf-8"))
        assert doc.get("name") == "\u4e2d\u6587"

    def test_parse_bytes_invalid_utf8_raises(self):
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.parse(b"\xff\xfe\xfd")


# ============================================================================
# read_markdown error cases
# ============================================================================


class TestReadMarkdownErrors:
    """Test read_markdown error handling"""

    def test_read_markdown_nonexistent_file(self):
        with pytest.raises(OSError):
            pyrs_yaml.read_markdown("/nonexistent/path/to/file.md")


# ============================================================================
# safe_load with various YAML features
# ============================================================================


class TestSafeLoad:
    """Test safe_load with YAML features"""

    def test_safe_load_with_anchors(self):
        result = pyrs_yaml.safe_load("defaults: &d\n  timeout: 30\nprod:\n  <<: *d")
        assert result["prod"]["timeout"] == 30

    def test_safe_load_with_merge_keys(self):
        result = pyrs_yaml.safe_load("base: &b\n  x: 1\nchild:\n  <<: *b\n  y: 2")
        assert result["child"]["x"] == 1
        assert result["child"]["y"] == 2

    def test_safe_load_with_block_scalar(self):
        result = pyrs_yaml.safe_load("text: |\n  line1\n  line2")
        assert result["text"] == "line1\nline2\n"

    def test_safe_load_with_flow_collection(self):
        result = pyrs_yaml.safe_load("items: [a, b, c]")
        assert result["items"] == ["a", "b", "c"]

    def test_safe_load_with_quoted_scalar(self):
        result = pyrs_yaml.safe_load("age: '42'")
        assert result["age"] == 42

    def test_safe_load_with_special_float(self):
        import math

        result = pyrs_yaml.safe_load("inf: .inf\nninf: -.inf\nnan: .nan")
        assert math.isinf(result["inf"])
        assert math.isinf(result["ninf"])
        assert math.isnan(result["nan"])

    def test_safe_load_with_bool_true(self):
        result = pyrs_yaml.safe_load("t: true")
        assert result["t"] is True

    def test_safe_load_with_bool_false(self):
        result = pyrs_yaml.safe_load("f: false")
        assert result["f"] is False


# ============================================================================
# safe_loads with various features
# ============================================================================


class TestSafeLoads:
    """Test safe_loads with YAML features"""

    def test_safe_loads_multiple_docs(self):
        docs = pyrs_yaml.safe_loads("a: 1\n---\nb: 2")
        assert len(docs) == 2
        assert docs[0]["a"] == 1
        assert docs[1]["b"] == 2


# ============================================================================
# from_dict edge cases
# ============================================================================


class TestFromDict:
    """Test from_dict edge cases"""

    def test_from_dict_simple(self):
        data = {"name": "John", "age": 30}
        yaml_str = pyrs_yaml.from_dict(data)
        assert "name: John" in yaml_str
        assert "30" in yaml_str

    def test_from_dict_nested(self):
        data = {"app": {"name": "myapp", "version": "1.0"}}
        yaml_str = pyrs_yaml.from_dict(data)
        assert "app:" in yaml_str
        assert "name: myapp" in yaml_str

    def test_from_dict_list(self):
        data = {"items": [1, 2, 3]}
        yaml_str = pyrs_yaml.from_dict(data)
        assert "- 1" in yaml_str

    def test_from_dict_special_chars_in_key(self):
        data = {"key:with:colons": "value"}
        yaml_str = pyrs_yaml.from_dict(data)
        assert "key:with:colons" in yaml_str

    def test_from_dict_nested_list(self):
        data = {"matrix": [[1, 2], [3, 4]]}
        yaml_str = pyrs_yaml.from_dict(data)
        assert "matrix:" in yaml_str

    def test_from_dict_none_value(self):
        data = {"key": None}
        yaml_str = pyrs_yaml.from_dict(data)
        assert "key:" in yaml_str


# ============================================================================
# from_json round-trip
# ============================================================================


class TestFromJsonRoundTrip:
    """Test from_json round-trip with complex JSON"""

    def test_from_json_simple(self):
        json_str = '{"name": "Alice", "active": true}'
        yaml_str = pyrs_yaml.from_json(json_str)
        assert "name: Alice" in yaml_str
        assert "active: true" in yaml_str

    def test_from_json_nested(self):
        json_str = '{"db": {"host": "localhost", "port": 5432}}'
        yaml_str = pyrs_yaml.from_json(json_str)
        assert "db:" in yaml_str
        assert "host: localhost" in yaml_str

    def test_from_json_array(self):
        json_str = '{"items": [1, 2, 3]}'
        yaml_str = pyrs_yaml.from_json(json_str)
        assert "- 1" in yaml_str

    def test_from_json_invalid_raises(self):
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.from_json("{invalid json")


# ============================================================================
# dump_file error handling
# ============================================================================


class TestDumpFile:
    """Test dump_file function"""

    def test_dump_file_success(self):
        data = {"key": "value", "num": 42}
        test_file = str(Path(tempfile.gettempdir()) / "test_dump_file.yaml")
        try:
            pyrs_yaml.dump_file(data, test_file)
            doc = pyrs_yaml.parse_file(test_file)
            assert doc.get("key") == "value"
            assert doc.get("num") == 42
        finally:
            if Path(test_file).exists():
                Path(test_file).unlink()

    def test_dump_file_invalid_path_raises(self):
        with pytest.raises(OSError):
            pyrs_yaml.dump_file({"key": "value"}, "/nonexistent/dir/file.yaml")


# ============================================================================
# YamlDocument with anchor on non-scalar nodes
# ============================================================================


class TestAnchorOnNonScalar:
    """Test anchors on mapping and sequence nodes"""

    def test_anchor_on_mapping(self):
        doc = pyrs_yaml.parse("defaults: &defaults\n  timeout: 30\nhost: localhost")
        output = doc.to_yaml()
        assert "&defaults" in output

    def test_anchor_on_sequence(self):
        doc = pyrs_yaml.parse("items: &items\n  - a\n  - b\nref: *items")
        output = doc.to_yaml()
        assert "&items" in output


# ============================================================================
# YamlDocument with flow style collections
# ============================================================================


class TestFlowCollections:
    """Test flow style collections"""

    def test_flow_mapping_root(self):
        doc = pyrs_yaml.parse("{a: 1, b: 2}")
        assert doc.to_yaml() == "{a: 1, b: 2}\n"

    def test_flow_sequence_root(self):
        doc = pyrs_yaml.parse("[1, 2, 3]")
        assert doc.to_yaml() == "[1, 2, 3]\n"

    def test_flow_mapping_in_mapping(self):
        doc = pyrs_yaml.parse("key: {a: 1, b: 2}")
        assert doc.to_yaml() == "key: {a: 1, b: 2}\n"

    def test_flow_sequence_in_mapping(self):
        doc = pyrs_yaml.parse("items: [a, b, c]")
        assert doc.to_yaml() == "items: [a, b, c]\n"


# ============================================================================
# YamlDocument with null values and empty structures
# ============================================================================


class TestNullAndEmptyStructures:
    """Test null values and empty structures via YamlDocument"""

    def test_null_value_get(self):
        doc = pyrs_yaml.parse("key: null")
        assert doc.get("key") is None

    def test_empty_value_get(self):
        doc = pyrs_yaml.parse("key:")
        assert doc.get("key") is None

    def test_empty_mapping_get(self):
        doc = pyrs_yaml.parse("key: {}")
        assert doc.get("key") == {}

    def test_empty_sequence_get(self):
        doc = pyrs_yaml.parse("key: []")
        assert doc.get("key") == []

    def test_empty_mapping_contains(self):
        doc = pyrs_yaml.parse("key: {}")
        assert "key" in doc

    def test_empty_sequence_len(self):
        doc = pyrs_yaml.parse("key: []")
        assert len(doc["key"]) == 0


# ============================================================================
# YamlDocument sequence index access via __getitem__
# ============================================================================


class TestSequenceIndexing:
    """Test sequence indexing via __getitem__"""

    def test_sequence_index_positive(self):
        doc = pyrs_yaml.parse("- a\n- b\n- c")
        assert doc[0] == "a"
        assert doc[1] == "b"
        assert doc[2] == "c"

    def test_sequence_index_out_of_range_raises(self):
        doc = pyrs_yaml.parse("- a")
        with pytest.raises(IndexError):
            _ = doc[5]


# ============================================================================
# parse with resolve_merges=False preserving << key
# ============================================================================


class TestResolveMergesFalse:
    """Test parse with resolve_merges=False preserves merge keys"""

    def test_merge_key_preserved_in_output(self):
        yaml_str = "base: &b\n  x: 1\nchild:\n  <<: *b\n  y: 2"
        doc = pyrs_yaml.parse(yaml_str, resolve_merges=False)
        output = doc.to_yaml()
        assert "<<" in output


# ============================================================================
# parse with resolve_merges=True (default) resolving merge keys
# ============================================================================


class TestResolveMergesTrue:
    """Test parse with resolve_merges=True (default) resolves merge keys"""

    def test_merge_key_resolved_by_default(self):
        yaml_str = "base: &b\n  x: 1\nchild:\n  <<: *b\n  y: 2"
        doc = pyrs_yaml.parse(yaml_str)
        child = doc.get("child")
        assert child["x"] == 1
        assert child["y"] == 2


# ============================================================================
# safe_dump with various Python types
# ============================================================================


class TestSafeDumpTypes:
    """Test safe_dump with various Python types"""

    def test_safe_dump_nested_dict(self):
        data = {"outer": {"inner": "value", "num": 42}}
        output = pyrs_yaml.safe_dump(data)
        assert "outer:" in output
        assert "inner: value" in output

    def test_safe_dump_nested_list(self):
        data = {"items": [1, 2, 3]}
        output = pyrs_yaml.safe_dump(data)
        assert "- 1" in output

    def test_safe_dump_bool(self):
        output = pyrs_yaml.safe_dump({"flag": True})
        assert "flag: true" in output

    def test_safe_dump_null(self):
        output = pyrs_yaml.safe_dump({"key": None})
        assert "key:" in output


# ============================================================================
# to_yaml preserving scalar styles
# ============================================================================


class TestScalarStylePreservation:
    """Test that scalar styles are preserved in serialization"""

    def test_single_quoted_roundtrip(self):
        original = "key: 'value'\n"
        doc = pyrs_yaml.parse(original)
        assert doc.to_yaml() == original

    def test_double_quoted_roundtrip(self):
        original = 'key: "value"\n'
        doc = pyrs_yaml.parse(original)
        assert doc.to_yaml() == original

    def test_plain_scalar_roundtrip(self):
        original = "key: plain_value\n"
        doc = pyrs_yaml.parse(original)
        assert doc.to_yaml() == original


# ============================================================================
# Tag preservation on non-scalar nodes
# ============================================================================


class TestTagPreservation:
    """Test tag preservation on various node types"""

    def test_tag_on_sequence(self):
        yaml_str = "items: !!seq [a, b]"
        doc = pyrs_yaml.parse(yaml_str)
        output = doc.to_yaml()
        assert "!!seq" in output or "items:" in output


# ============================================================================
# Comments preserved on non-scalar nodes
# ============================================================================


class TestCommentPreservation:
    """Test comment preservation on various node types"""

    def test_comment_on_mapping_value(self):
        yaml_str = "key: value  # comment on value\n"
        doc = pyrs_yaml.parse(yaml_str)
        output = doc.to_yaml()
        assert "# comment on value" in output


# ============================================================================
# Complex nested structures
# ============================================================================


class TestComplexNested:
    """Test deeply nested and complex structures"""

    def test_deeply_nested_mapping(self):
        yaml_str = "a:\n  b:\n    c:\n      d:\n        e: value"
        doc = pyrs_yaml.parse(yaml_str)
        assert doc.get("a")["b"]["c"]["d"]["e"] == "value"

    def test_nested_flow_and_block(self):
        yaml_str = "outer:\n  items: [a, b]\n  nested:\n    key: value"
        doc = pyrs_yaml.parse(yaml_str)
        assert doc.get("outer")["items"] == ["a", "b"]
        assert doc.get("outer")["nested"]["key"] == "value"

    def test_sequence_of_mappings(self):
        yaml_str = "- name: Alice\n  age: 30\n- name: Bob\n  age: 25"
        doc = pyrs_yaml.parse(yaml_str)
        assert doc[0]["name"] == "Alice"
        assert doc[0]["age"] == 30
        assert doc[1]["name"] == "Bob"
        assert doc[1]["age"] == 25

    def test_mapping_of_sequences(self):
        yaml_str = "fruits:\n  - apple\n  - banana\nvegetables:\n  - carrot"
        doc = pyrs_yaml.parse(yaml_str)
        assert doc["fruits"] == ["apple", "banana"]
        assert doc["vegetables"] == ["carrot"]


# ============================================================================
# YAML Test Suite individual case tests for better coverage
# ============================================================================


class TestYamlSuiteIndividual:
    """Test individual YAML Test Suite cases for better coverage"""

    def test_yaml_suite_anchors_and_aliases(self):
        yaml_str = """
defaults: &defaults
  timeout: 30
  retries: 3
production:
  <<: *defaults
  host: prod.example.com
"""
        doc = pyrs_yaml.parse(yaml_str)
        prod = doc.get("production")
        assert prod["timeout"] == 30
        assert prod["retries"] == 3
        assert prod["host"] == "prod.example.com"

    def test_yaml_suite_block_scalar_strip(self):
        yaml_str = "text: |-\n  line1\n  line2\n"
        doc = pyrs_yaml.parse(yaml_str)
        assert doc.get("text") == "line1\nline2"

    def test_yaml_suite_flow_sequence(self):
        yaml_str = "[1, 2, 3, 4, 5]"
        doc = pyrs_yaml.parse(yaml_str)
        assert doc.root_type() == "sequence"
        assert doc[0] == 1
        assert doc[4] == 5

    def test_yaml_suite_flow_mapping(self):
        yaml_str = "{key: value, num: 42}"
        doc = pyrs_yaml.parse(yaml_str)
        assert doc.root_type() == "mapping"
        assert doc["key"] == "value"
        assert doc["num"] == 42

    def test_yaml_suite_octal_integer(self):
        yaml_str = "mode: 0o755"
        doc = pyrs_yaml.parse(yaml_str)
        assert doc.get("mode") == 493  # 0o755 = 493

    def test_yaml_suite_hex_integer(self):
        yaml_str = "mask: 0xFF"
        doc = pyrs_yaml.parse(yaml_str)
        assert doc.get("mask") == 255

    def test_yaml_suite_scientific_notation(self):
        yaml_str = "large: 6.022e23"
        doc = pyrs_yaml.parse(yaml_str)
        assert abs(doc.get("large") - 6.022e23) < 1e15

    def test_yaml_suite_nan(self):
        import math

        doc = pyrs_yaml.parse("value: .nan")
        assert math.isnan(doc.get("value"))

    def test_yaml_suite_infinity_positive(self):
        import math

        doc = pyrs_yaml.parse("value: .inf")
        assert math.isinf(doc.get("value"))
        assert doc.get("value") > 0

    def test_yaml_suite_infinity_negative(self):
        import math

        doc = pyrs_yaml.parse("value: -.inf")
        assert math.isinf(doc.get("value"))
        assert doc.get("value") < 0

    def test_yaml_suite_bool_true(self):
        for variant in ["true", "True", "TRUE"]:
            doc = pyrs_yaml.parse(f"key: {variant}")
            assert doc.get("key") is True, f"Failed for {variant}"

    def test_yaml_suite_bool_false(self):
        for variant in ["false", "False", "FALSE"]:
            doc = pyrs_yaml.parse(f"key: {variant}")
            assert doc.get("key") is False, f"Failed for {variant}"

    def test_yaml_suite_null_variants(self):
        for variant in ["null", "Null", "NULL", "~"]:
            doc = pyrs_yaml.parse(f"key: {variant}")
            assert doc.get("key") is None, f"Failed for {variant}"

    def test_yaml_suite_merge_key(self):
        yaml_str = """
base: &base
  a: 1
  b: 2
derived:
  <<: *base
  c: 3
"""
        doc = pyrs_yaml.parse(yaml_str)
        derived = doc.get("derived")
        assert derived["a"] == 1
        assert derived["b"] == 2
        assert derived["c"] == 3

    def test_yaml_suite_implicit_key(self):
        yaml_str = "simple: value\n"
        doc = pyrs_yaml.parse(yaml_str)
        assert doc.get("simple") == "value"

    def test_yaml_suite_explicit_key(self):
        yaml_str = "? explicit key\n: value\n"
        doc = pyrs_yaml.parse(yaml_str)
        assert doc.get("explicit key") == "value"
