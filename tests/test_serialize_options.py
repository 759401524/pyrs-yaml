"""Tests for SerializeOptions expansion (Phase 2 of v0.9.0)."""

import pyrs_yaml
from tests.data import yaml_samples as yaml


class TestSerializeOptions:
    """Test SerializeOptions expansion: width, indent_mapping, indent_sequence, indent_offset."""

    def test_width_zero_no_wrapping(self):
        doc = pyrs_yaml.parse(yaml.LONG_VALUE)
        output = doc.to_yaml_with_options(width=0)
        assert "x" * 100 in output

    def test_width_10_no_fold_for_unbroken_token(self):
        # A long token with no whitespace cannot be folded losslessly (a plain
        # scalar line break re-parses to an inserted space), so it stays on a
        # single (long) line to preserve round-trip fidelity.
        doc = pyrs_yaml.parse(yaml.LONG_VALUE)
        output = doc.to_yaml_with_options(width=10)
        lines = output.strip().split("\n")
        assert len(lines) == 1
        assert "x" * 100 in output

    def test_width_10_wraps_text_with_spaces(self):
        # Text with whitespace wraps at word boundaries and folds back exactly.
        doc = pyrs_yaml.parse("s: " + "lorem ipsum dolor sit amet " * 10)
        output = doc.to_yaml_with_options(width=30)
        lines = output.strip().split("\n")
        assert len(lines) > 1

    def test_indent_size_4(self):
        doc = pyrs_yaml.YAML(typ="rt").parse("parent:\n  child: value")
        output = doc.to_yaml_with_options(indent_size=4)
        assert "    child: value" in output

    def test_indent_mapping_4(self):
        doc = pyrs_yaml.YAML(typ="rt").parse("parent:\n  child: value")
        output = doc.to_yaml_with_options(indent_mapping=4)
        assert output == "parent:\n    child: value\n"

    def test_indent_sequence_4(self):
        doc = pyrs_yaml.YAML(typ="rt").parse("- - 1\n  - 2")
        output = doc.to_yaml_with_options(indent_sequence=4)
        assert output == "- \n    - 1\n    - 2\n"

    def test_indent_offset_2(self):
        doc = pyrs_yaml.YAML(typ="rt").parse("a: 1")
        output = doc.to_yaml_with_options(indent_offset=2)
        assert output == "  a: 1\n"

    def test_indent_compose(self):
        doc = pyrs_yaml.YAML(typ="rt").parse("a:\n  b:\n    - 1")
        output = doc.to_yaml_with_options(indent_offset=2, indent_mapping=4, indent_sequence=2)
        assert output == "  a:\n      b:\n          - 1\n"

    def test_width_1_does_not_hang(self):
        doc = pyrs_yaml.parse(yaml.LONG_VALUE)
        output = doc.to_yaml_with_options(width=1)
        assert output.count("x") == 100
        assert "\n" in output

    def test_width_larger_than_line_no_wrap(self):
        doc = pyrs_yaml.parse(yaml.LONG_VALUE)
        output = doc.to_yaml_with_options(width=1000)
        assert "x" * 100 in output
        assert output.strip().count("\n") == 0
