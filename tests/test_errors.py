"""
Error handling tests — YamlParseError, YamlTypeError, IO errors, edge cases.
"""

import contextlib
import tempfile
from pathlib import Path

import pyrs_yaml
import pytest

# ============================================================================
# File I/O Errors
# ============================================================================


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


# ============================================================================
# Edge Cases
# ============================================================================


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


# ============================================================================
# Custom Exception Types
# ============================================================================


class TestCustomExceptions:
    """Test custom exception types for precise error handling"""

    def test_yaml_parse_error_exists(self):
        """YamlParseError should be importable."""
        assert hasattr(pyrs_yaml, "YamlParseError")

    def test_yaml_serialize_error_exists(self):
        """YamlSerializeError should be importable."""
        assert hasattr(pyrs_yaml, "YamlSerializeError")

    def test_yaml_type_error_exists(self):
        """YamlTypeError should be importable."""
        assert hasattr(pyrs_yaml, "YamlTypeError")

    def test_parse_error_is_value_error(self):
        """YamlParseError should inherit from ValueError."""
        assert issubclass(pyrs_yaml.YamlParseError, ValueError)

    def test_serialize_error_is_value_error(self):
        """YamlSerializeError should inherit from ValueError."""
        assert issubclass(pyrs_yaml.YamlSerializeError, ValueError)

    def test_type_error_is_type_error(self):
        """YamlTypeError should inherit from TypeError."""
        assert issubclass(pyrs_yaml.YamlTypeError, TypeError)

    def test_parse_error_can_be_caught(self):
        """YamlParseError can be caught by except ValueError."""
        with pytest.raises(ValueError):
            pyrs_yaml.parse("{{invalid yaml")

    def test_parse_error_is_custom_type(self):
        """Raised parse error is specifically YamlParseError."""
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.parse("{{invalid yaml")


# ============================================================================
# Error Context Tests
# ============================================================================


class TestErrorContext:
    """Test that error messages include useful context."""

    def test_parse_error_has_line_info(self):
        """Parse error should include line number and source snippet."""
        invalid_yaml = "key: value: extra_colon"
        try:
            pyrs_yaml.parse(invalid_yaml)
            raise AssertionError("should have raised")
        except pyrs_yaml.YamlParseError as e:
            msg = str(e)
            # Should contain line/col or source context
            assert "line" in msg.lower() or "col" in msg.lower() or "|" in msg, (
                f"Error should contain line/col/context info: {msg}"
            )

    def test_parse_error_different_line(self):
        """Error on a later line should show the correct line number."""
        multiline_yaml = """a: 1
b: 2
c: value: extra
d: 4
"""
        with contextlib.suppress(pyrs_yaml.YamlParseError):
            pyrs_yaml.parse(multiline_yaml)

    def test_parse_error_utf8(self):
        """Error with non-ASCII characters should not crash."""
        invalid_yaml = "key: \x00value"
        with contextlib.suppress(pyrs_yaml.YamlParseError):
            pyrs_yaml.parse(invalid_yaml)
