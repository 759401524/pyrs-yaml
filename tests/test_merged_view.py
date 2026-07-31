"""Tests for MergedView and doc.version (v0.8.0 Phase 4 + Phase 5)."""

import pyrs_yaml
from pyrs_yaml import MergedView


class TestMergedView:
    """Test MergedView read-only anchor merge view."""

    def test_merged_view_basic(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["simple_mapping"])
        mv = doc.merged()
        assert isinstance(mv, MergedView)
        assert mv["key"] == "value"

    def test_merged_view_getitem_len(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["mixed"])
        mv = doc.merged()
        assert mv["name"] == "test"
        assert len(mv) >= 3

    def test_merged_view_iter(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["simple_mapping"])
        mv = doc.merged()
        keys = list(mv)
        assert "key" in keys

    def test_merged_view_nested(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["nested_mapping"])
        mv = doc.merged()
        parent = mv["parent"]
        assert isinstance(parent, MergedView._DictView)
        assert parent["child"] == "grandchild"

    def test_merged_view_list(self, yaml_strings):
        yaml = "data:\n  - a\n  - b\n  - c"
        doc = pyrs_yaml.parse(yaml)
        mv = doc.merged()
        data = mv["data"]
        assert isinstance(data, MergedView._ListView)
        assert len(data) == 3
        assert data[0] == "a"

    def test_merged_view_merge_keys(self):
        yaml = "base: &base\n  a: 1\nmerged:\n  <<: *base\n  b: 2"
        doc = pyrs_yaml.parse(yaml)
        mv = doc.merged()
        merged = mv["merged"]
        assert merged["a"] == 1
        assert merged["b"] == 2

    def test_merged_view_repr(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["simple_mapping"])
        mv = doc.merged()
        r = repr(mv)
        assert "MergedView" in r

    def test_merged_view_empty(self):
        doc = pyrs_yaml.parse("{}")
        mv = doc.merged()
        assert len(mv) == 0
        assert list(mv) == []

    def test_merged_view_sequence_root(self):
        doc = pyrs_yaml.parse("- a\n- b\n- c")
        mv = doc.merged()
        assert len(mv) == 3
        assert mv[0] == "a"
        assert mv[1] == "b"
        assert mv[2] == "c"


class TestDocVersion:
    """Test document metadata (v0.8.0 Phase 4)."""

    def test_doc_version(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["simple_mapping"])
        assert doc.version() == "1.2"

    def test_doc_version_nested(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["nested_mapping"])
        assert doc.version() == "1.2"

    def test_doc_version_sequence(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["sequence"])
        assert doc.version() == "1.2"
