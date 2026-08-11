"""Error handling tests — YamlParseError, YamlTypeError, IO errors, edge cases."""

import tempfile
from pathlib import Path

import pytest

import pyrs_yaml


class TestFileIO:
    """Test file reading functionality"""

    def test_parse_file(self):
        test_file = str(Path(tempfile.gettempdir()) / "test_parse.yaml")
        with Path(test_file).open("w") as f:
            f.write("key: value\nlist:\n  - a\n  - b")
        try:
            doc = pyrs_yaml.parse_file(test_file)
            assert doc.get("key") == "value"
        finally:
            if Path(test_file).exists():
                Path(test_file).unlink()

    def test_parse_file_nonexistent(self):
        with pytest.raises(OSError):
            pyrs_yaml.parse_file("/nonexistent/file.yaml")


class TestEdgeCases:
    """Test edge cases and special scenarios"""

    def test_empty_yaml(self):
        doc = pyrs_yaml.parse("")
        assert doc.root_type() == "null"

    def test_only_comment(self):
        doc = pyrs_yaml.parse("# just a comment")
        assert doc.root_type() == "null"

    def test_special_chars_in_key(self):
        yaml_str = '"key:with:colons": value'
        doc = pyrs_yaml.parse(yaml_str)
        assert doc.root_type() == "mapping"

    def test_multiline_string(self):
        yaml_str = "key: |\n  line1\n  line2\n  line3"
        doc = pyrs_yaml.parse(yaml_str)
        assert doc.root_type() == "mapping"

    def test_deeply_nested(self):
        yaml_str = "a:\n  b:\n    c:\n      d: value"
        doc = pyrs_yaml.parse(yaml_str)
        assert doc.root_type() == "mapping"

    def test_multiple_documents(self):
        yaml_str = "a: 1\n---\nb: 2"
        docs = pyrs_yaml.safe_loads(yaml_str)
        assert len(docs) == 2


class TestCustomExceptions:
    """Test custom exception types for precise error handling"""

    @pytest.mark.parametrize(
        "name",
        [
            "YamlParseError",
            "YamlSerializeError",
            "YamlTypeError",
        ],
        ids=["parse", "serialize", "type"],
    )
    def test_exception_exists(self, name):
        assert hasattr(pyrs_yaml, name)

    @pytest.mark.parametrize(
        "cls_name,base",
        [
            ("YamlParseError", ValueError),
            ("YamlSerializeError", ValueError),
            ("YamlTypeError", TypeError),
        ],
        ids=["parse-is-value-error", "serialize-is-value-error", "type-is-type-error"],
    )
    def test_exception_inheritance(self, cls_name, base):
        assert issubclass(getattr(pyrs_yaml, cls_name), base)

    def test_parse_error_caught_by_value_error(self):
        with pytest.raises(ValueError):
            pyrs_yaml.parse("{{invalid yaml")

    def test_parse_error_is_custom_type(self):
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.parse("{{invalid yaml")


class TestErrorContext:
    """Test that error messages include useful context."""

    def test_parse_error_has_line_info(self):
        invalid_yaml = "key: value: extra_colon"
        try:
            pyrs_yaml.parse(invalid_yaml)
            raise AssertionError("should have raised")
        except pyrs_yaml.YamlParseError as e:
            msg = str(e)
            assert "line" in msg.lower() or "col" in msg.lower() or "|" in msg, (
                f"Error should contain line/col/context info: {msg}"
            )

    def test_parse_error_different_line(self):
        multiline_yaml = """a: 1
b: 2
c: value: extra
d: 4
"""
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.parse(multiline_yaml)

    def test_parse_error_utf8(self):
        invalid_yaml = "key: \x00value"
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.parse(invalid_yaml)
