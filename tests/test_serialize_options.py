"""Tests for SerializeOptions expansion (Phase 2 of v0.9.0)."""

import pyrs_yaml

from tests.data import yaml_samples as yaml


class TestSerializeOptions:
    """Test SerializeOptions expansion: width, indent_mapping, indent_sequence, indent_offset."""

    def test_width_zero_no_wrapping(self):
        doc = pyrs_yaml.parse(yaml.LONG_VALUE)
        output = doc.to_yaml_with_options(width=0)
        assert "x" * 100 in output

    def test_width_10_wraps_long_value(self):
        doc = pyrs_yaml.parse(yaml.LONG_VALUE)
        output = doc.to_yaml_with_options(width=10)
        lines = output.strip().split("\n")
        assert len(lines) > 1

    def test_indent_size_4(self):
        doc = pyrs_yaml.YAML(typ="rt").parse("parent:\n  child: value")
        output = doc.to_yaml_with_options(indent_size=4)
        assert "    child: value" in output
