"""Tests for Node metadata setters/getters (comment, anchor, tag)."""

import pytest

import pyrs_yaml
from pyrs_yaml import Node


class TestNodeComment:
    def test_get_comment_none(self):
        node = Node(pyrs_yaml.parse("key: value")).find("$.key")
        assert node.comment is None

    def test_set_comment_standalone(self):
        doc = pyrs_yaml.parse("key: value")
        Node(doc).find("$.key").set_comment("a note")
        assert doc.to_yaml() == "key:\n  # a note\n  value\n"

    def test_set_comment_inline(self):
        doc = pyrs_yaml.parse("key: value")
        Node(doc).find("$.key").set_comment("inline note", standalone=False)
        assert doc.to_yaml() == "key: value  # inline note\n"

    def test_set_comment_replace(self):
        doc = pyrs_yaml.parse("key: value")
        Node(doc).find("$.key").set_comment("new")
        assert doc.to_yaml() == "key:\n  # new\n  value\n"

    def test_remove_comment(self):
        doc = pyrs_yaml.parse("key:\n  # note\n  value")
        Node(doc).find("$.key").remove_comment()
        assert doc.to_yaml() == "key: value\n"

    def test_comment_on_empty_flow_mapping_valid_yaml(self):
        doc = pyrs_yaml.parse("key: {}\n")
        Node(doc).find("$.key").set_comment("note")
        out = doc.to_yaml()
        assert "key: {}  # note" in out
        # Empty flow container + standalone comment must stay valid YAML.
        reparsed = pyrs_yaml.parse(out)
        assert reparsed.to_dict() == {"key": {}}

    def test_comment_on_empty_flow_sequence_valid_yaml(self):
        doc = pyrs_yaml.parse("key: []\n")
        Node(doc).find("$.key").set_comment("note")
        out = doc.to_yaml()
        assert "key: []  # note" in out
        reparsed = pyrs_yaml.parse(out)
        assert reparsed.to_dict() == {"key": []}

    def test_remove_comment_inline(self):
        doc = pyrs_yaml.parse("key: value  # note")
        Node(doc).find("$.key").remove_comment()
        assert doc.to_yaml() == "key: value\n"

    def test_comment_sequence_item(self):
        doc = pyrs_yaml.parse("- a\n- b")
        Node(doc).find("$[1]").set_comment("second")
        assert doc.to_yaml() == "- a\n- \n  # second\n  b\n"

    def test_comment_nested(self):
        doc = pyrs_yaml.parse("parent:\n  child: val")
        Node(doc).find("$.parent.child").set_comment("deep")
        assert doc.to_yaml() == "parent:\n  child:\n    # deep\n    val\n"

    def test_comment_roundtrip(self):
        doc = pyrs_yaml.parse("key: value")
        Node(doc).find("$.key").set_comment("updated")
        out = doc.to_yaml()
        reparsed = pyrs_yaml.parse(out)
        assert "updated" in reparsed.to_yaml()

    def test_comment_read_after_edit(self):
        doc = pyrs_yaml.parse("key: value")
        Node(doc).find("$.key").set_comment("updated")
        assert Node(doc).find("$.key").comment == "updated"


class TestNodeAnchor:
    def test_get_anchor_none(self):
        node = Node(pyrs_yaml.parse("key: value")).find("$.key")
        assert node.anchor is None

    def test_set_anchor(self):
        doc = pyrs_yaml.parse("key: value")
        Node(doc).find("$.key").set_anchor("k1")
        assert doc.to_yaml() == "key: &k1 value\n"

    def test_set_anchor_read(self):
        doc = pyrs_yaml.parse("key: value")
        Node(doc).find("$.key").set_anchor("k1")
        assert Node(doc).find("$.key").anchor == "k1"

    def test_set_anchor_replace(self):
        doc = pyrs_yaml.parse("key: &old value")
        Node(doc).find("$.key").set_anchor("new")
        assert doc.to_yaml() == "key: &new value\n"

    def test_remove_anchor(self):
        doc = pyrs_yaml.parse("key: &k1 value")
        Node(doc).find("$.key").remove_anchor()
        assert doc.to_yaml() == "key: value\n"

    def test_anchor_mapping(self):
        doc = pyrs_yaml.parse("key: value")
        Node(doc).find("$.key").set_anchor("k1")
        assert doc.to_yaml() == "key: &k1 value\n"


class TestNodeTag:
    def test_get_tag_none(self):
        node = Node(pyrs_yaml.parse("key: value")).find("$.key")
        assert node.tag is None

    def test_set_tag_local(self):
        doc = pyrs_yaml.parse("key: value")
        Node(doc).find("$.key").set_tag("!custom")
        assert doc.to_yaml() == "key: !custom value\n"

    def test_set_tag_local_read(self):
        doc = pyrs_yaml.parse("key: value")
        Node(doc).find("$.key").set_tag("!custom")
        assert Node(doc).find("$.key").tag == "!custom"

    def test_set_tag_primary(self):
        doc = pyrs_yaml.parse("key: 123")
        Node(doc).find("$.key").set_tag("!!int")
        assert doc.to_yaml() == "key: !!int 123\n"

    def test_set_tag_verbatim(self):
        doc = pyrs_yaml.parse("key: value")
        Node(doc).find("$.key").set_tag("!<tag:yaml.org,2002:str>")
        assert doc.to_yaml() == "key: !<tag:yaml.org,2002:str> value\n"

    def test_set_tag_verbatim_read(self):
        doc = pyrs_yaml.parse("key: value")
        Node(doc).find("$.key").set_tag("!<tag:yaml.org,2002:str>")
        assert Node(doc).find("$.key").tag == "!<tag:yaml.org,2002:str>"

    def test_remove_tag(self):
        doc = pyrs_yaml.parse("key: !!str value")
        Node(doc).find("$.key").remove_tag()
        assert doc.to_yaml() == "key: value\n"

    def test_tag_nested(self):
        doc = pyrs_yaml.parse("parent:\n  child: val")
        Node(doc).find("$.parent.child").set_tag("!custom")
        assert doc.to_yaml() == "parent:\n  child: !custom val\n"


class TestNodeMetadataErrors:
    def test_edit_alias_error(self):
        doc = pyrs_yaml.parse("a: &x 1\nb: *x")
        alias_node = Node(doc).find("$.b")
        with pytest.raises(pyrs_yaml.YamlEditError):
            alias_node.set_comment("nope")

    def test_edit_missing_path(self):
        node = Node(pyrs_yaml.parse("key: value")).find("$.nope")
        with pytest.raises(pyrs_yaml.YamlEditError):
            node.set_comment("nope")

    def test_stale_node_after_edit(self):
        doc = pyrs_yaml.parse("key: value")
        node = Node(doc).find("$.key")
        node.set_anchor("k1")
        with pytest.raises(pyrs_yaml.YamlDocumentError):
            _ = node.anchor
