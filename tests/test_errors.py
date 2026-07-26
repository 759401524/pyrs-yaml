"""
Error handling tests — YamlParseError, YamlTypeError, IO errors, edge cases.
"""

import os
import tempfile

import pytest
import pyyaml_rs


# ============================================================================
# File I/O Errors
# ============================================================================

class TestFileIO:
    """Test file reading functionality"""

    def test_parse_file(self):
        test_file = os.path.join(tempfile.gettempdir(), "test_parse.yaml")
        with open(test_file, "w") as f:
            f.write("key: value\nlist:\n  - a\n  - b")
        try:
            doc = pyyaml_rs.parse_file(test_file)
            assert doc.get("key") == "value"
        finally:
            if os.path.exists(test_file):
                os.remove(test_file)

    def test_parse_file_nonexistent(self):
        with pytest.raises(OSError):
            pyyaml_rs.parse_file("/nonexistent/file.yaml")


# ============================================================================
# Edge Cases
# ============================================================================

class TestEdgeCases:
    """Test edge cases and special scenarios"""

    def test_empty_yaml(self):
        doc = pyyaml_rs.parse("")
        assert doc.root_type() == "null"

    def test_only_comment(self):
        doc = pyyaml_rs.parse("# just a comment")
        assert doc.root_type() == "null"

    def test_special_chars_in_key(self):
        yaml_str = '"key:with:colons": value'
        doc = pyyaml_rs.parse(yaml_str)
        assert doc.root_type() == "mapping"

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
# Custom Exception Types
# ============================================================================

class TestCustomExceptions:
    """Test custom exception types for precise error handling"""

    def test_yaml_parse_error_exists(self):
        """YamlParseError should be importable."""
        assert hasattr(pyyaml_rs, "YamlParseError")

    def test_yaml_serialize_error_exists(self):
        """YamlSerializeError should be importable."""
        assert hasattr(pyyaml_rs, "YamlSerializeError")

    def test_yaml_type_error_exists(self):
        """YamlTypeError should be importable."""
        assert hasattr(pyyaml_rs, "YamlTypeError")

    def test_parse_error_is_value_error(self):
        """YamlParseError should inherit from ValueError."""
        assert issubclass(pyyaml_rs.YamlParseError, ValueError)

    def test_serialize_error_is_value_error(self):
        """YamlSerializeError should inherit from ValueError."""
        assert issubclass(pyyaml_rs.YamlSerializeError, ValueError)

    def test_type_error_is_type_error(self):
        """YamlTypeError should inherit from TypeError."""
        assert issubclass(pyyaml_rs.YamlTypeError, TypeError)

    def test_parse_error_can_be_caught(self):
        """YamlParseError can be caught by except ValueError."""
        with pytest.raises(ValueError):
            pyyaml_rs.parse("{{invalid yaml")

    def test_parse_error_is_custom_type(self):
        """Raised parse error is specifically YamlParseError."""
        with pytest.raises(pyyaml_rs.YamlParseError):
            pyyaml_rs.parse("{{invalid yaml")
