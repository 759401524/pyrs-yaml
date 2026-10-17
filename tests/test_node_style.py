"""Tests for Node style/format setters/getters (scalar_style, flow_style, chomping)."""

import pytest

import pyrs_yaml
from pyrs_yaml import Node


class TestNodeScalarStyle:
    def test_get_scalar_style_none(self):
        node = Node(pyrs_yaml.parse("key: value")).find("$.key")
        assert node.scalar_style == "plain"

    def test_get_scalar_style_on_non_scalar(self):
        node = Node(pyrs_yaml.parse("key: value"))
        assert node.scalar_style is None

    def test_set_scalar_style_single_quoted(self):
        doc = pyrs_yaml.parse("key: value")
        Node(doc).find("$.key").set_scalar_style("single_quoted")
        assert doc.to_yaml() == "key: 'value'\n"

    def test_set_scalar_style_double_quoted(self):
        doc = pyrs_yaml.parse("key: value")
        Node(doc).find("$.key").set_scalar_style("double_quoted")
        assert doc.to_yaml() == 'key: "value"\n'

    def test_set_scalar_style_read_back(self):
        doc = pyrs_yaml.parse("key: value")
        Node(doc).find("$.key").set_scalar_style("double_quoted")
        assert Node(doc).find("$.key").scalar_style == "double_quoted"

    def test_set_scalar_style_replace(self):
        doc = pyrs_yaml.parse("key: 'value'")
        Node(doc).find("$.key").set_scalar_style("plain")
        assert doc.to_yaml() == "key: value\n"

    def test_set_scalar_style_on_non_scalar_noop(self):
        doc = pyrs_yaml.parse("key: value")
        Node(doc).find("$").set_scalar_style("single_quoted")
        assert doc.to_yaml() == "key: value\n"

    def test_set_scalar_style_invalid_raises(self):
        doc = pyrs_yaml.parse("key: value")
        with pytest.raises(pyrs_yaml.YamlEditError):
            Node(doc).find("$.key").set_scalar_style("bogus")

    def test_scalar_style_nested(self):
        doc = pyrs_yaml.parse("parent:\n  child: val")
        Node(doc).find("$.parent.child").set_scalar_style("single_quoted")
        assert doc.to_yaml() == "parent:\n  child: 'val'\n"


class TestNodeFlowStyle:
    def test_get_flow_style_block(self):
        node = Node(pyrs_yaml.parse("items:\n  - a\n  - b")).find("$.items")
        assert node.flow_style is False

    def test_get_flow_style_flow(self):
        node = Node(pyrs_yaml.parse("items: [a, b]")).find("$.items")
        assert node.flow_style is True

    def test_get_flow_style_on_scalar(self):
        node = Node(pyrs_yaml.parse("key: value")).find("$.key")
        assert node.flow_style is None

    def test_set_flow_style_true_sequence(self):
        doc = pyrs_yaml.parse("items:\n  - a\n  - b")
        Node(doc).find("$.items").set_flow_style(True)
        assert doc.to_yaml() == "items: [a, b]\n"

    def test_set_flow_style_false_sequence(self):
        doc = pyrs_yaml.parse("items: [a, b]")
        Node(doc).find("$.items").set_flow_style(False)
        assert doc.to_yaml() == "items:\n  - a\n  - b\n"

    def test_set_flow_style_true_mapping(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2")
        Node(doc).find("$").set_flow_style(True)
        assert doc.to_yaml() == "{a: 1, b: 2}\n"

    def test_set_flow_style_on_scalar_noop(self):
        doc = pyrs_yaml.parse("key: value")
        Node(doc).find("$.key").set_flow_style(True)
        assert doc.to_yaml() == "key: value\n"


class TestNodeChomping:
    def test_get_chomping_default(self):
        node = Node(pyrs_yaml.parse("key: |\n  line1\n  line2")).find("$.key")
        assert node.chomping == "clip"

    def test_get_chomping_strip(self):
        node = Node(pyrs_yaml.parse("key: |-\n  line1\n")).find("$.key")
        assert node.chomping == "strip"

    def test_get_chomping_on_non_scalar(self):
        node = Node(pyrs_yaml.parse("key: value")).find("$.key")
        # plain scalar — chomping is still Some(Clip) on Scalar variants
        assert node.chomping == "clip"

    @pytest.mark.parametrize(
        ("source", "expected"),
        [
            ("key: |-\n\n", "strip"),
            ("key: >-\n\n", "strip"),
            ("key: |+\n\n", "keep"),
            ("key: >+\n\n", "keep"),
        ],
        ids=["literal-strip", "folded-strip", "literal-keep", "folded-keep"],
    )
    def test_get_chomping_on_a_block_with_no_content_line(self, source, expected):
        # granit spans a content-less block scalar on the header's own sigil, and the scan that
        # recovers the indicators backed up past the `: ` into the key — so each spelling here read
        # as "clip", and the AST did not keep what the source wrote.
        assert Node(pyrs_yaml.parse(source)).find("$.key").chomping == expected

    def test_a_content_less_block_is_still_written_with_the_safe_header(self):
        """Recording the indicator is not the same as emitting it. `|-` and `|+` for an empty body
        both re-read as the default, so writing either one drifts on the next round (libFuzzer
        crash-89d81d99, crash-b5dcc38f). The three spellings denote the same empty value; the AST
        keeps the claim the author made, the writer keeps the fixed point."""
        doc = pyrs_yaml.parse("key: |-\n\n")
        assert doc.to_yaml() == "key: |\n\n"
        assert pyrs_yaml.parse(doc.to_yaml()).to_yaml() == "key: |\n\n"

    def test_set_chomping_strip(self):
        doc = pyrs_yaml.parse("key: |\n  line1\n  line2\n")
        Node(doc).find("$.key").set_chomping("strip")
        assert doc.to_yaml() == "key: |-\n  line1\n  line2\n"

    def test_set_chomping_keep(self):
        doc = pyrs_yaml.parse("key: |\n  line1\n")
        Node(doc).find("$.key").set_chomping("keep")
        assert doc.to_yaml() == "key: |+\n  line1\n"

    def test_set_chomping_invalid_raises(self):
        doc = pyrs_yaml.parse("key: value")
        with pytest.raises(pyrs_yaml.YamlEditError):
            Node(doc).find("$.key").set_chomping("bogus")

    def test_a_clipped_block_whose_text_ends_without_a_break_is_stripped(self):
        """Asking for clip on a body that has no trailing break does not give it one.

        `|` *denotes* one trailing line break, so writing it for `line1\nline2` re-reads as
        `line1\nline2\n` - a byte the document invents. The writer keeps the header the body can actually
        carry (`|-`), so the value survives the round trip. The instruction is expressible the other way
        (`set_chomping("strip")` above drops a break on purpose), which is why only this direction is
        corrected.
        """
        doc = pyrs_yaml.parse("key: |-\n  line1\n  line2\n")
        Node(doc).find("$.key").set_chomping("clip")
        assert doc.to_yaml() == "key: |-\n  line1\n  line2\n"
        assert pyrs_yaml.safe_load(doc.to_yaml())["key"] == "line1\nline2"


class TestNodeStyleErrors:
    def test_edit_alias_error(self):
        doc = pyrs_yaml.parse("a: &x 1\nb: *x")
        alias_node = Node(doc).find("$.b")
        with pytest.raises(pyrs_yaml.YamlEditError):
            alias_node.set_scalar_style("single_quoted")

    def test_edit_missing_path(self):
        node = Node(pyrs_yaml.parse("key: value")).find("$.nope")
        with pytest.raises(pyrs_yaml.YamlEditError):
            node.set_scalar_style("single_quoted")

    def test_stale_node_after_edit(self):
        doc = pyrs_yaml.parse("key: value")
        node = Node(doc).find("$.key")
        node.set_scalar_style("single_quoted")
        with pytest.raises(pyrs_yaml.YamlDocumentError):
            _ = node.scalar_style
