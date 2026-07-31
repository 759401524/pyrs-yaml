"""Tests for MergedView and doc.version (v0.8.0 Phase 4 + Phase 5)."""

import pyrs_yaml
import pytest
from pyrs_yaml import MergedView


class TestMergedView:
    """Test MergedView read-only anchor merge view."""

    def test_merged_view_basic(self, yaml_strings):
        mv = pyrs_yaml.parse(yaml_strings["simple_mapping"]).merged()
        assert isinstance(mv, MergedView)
        assert mv["key"] == "value"

    def test_merged_view_getitem_len(self, yaml_strings):
        mv = pyrs_yaml.parse(yaml_strings["mixed"]).merged()
        assert mv["name"] == "test"
        assert len(mv) >= 3

    def test_merged_view_iter(self, yaml_strings):
        mv = pyrs_yaml.parse(yaml_strings["simple_mapping"]).merged()
        assert "key" in list(mv)

    def test_merged_view_nested(self, yaml_strings):
        mv = pyrs_yaml.parse(yaml_strings["nested_mapping"]).merged()
        parent = mv["parent"]
        assert isinstance(parent, MergedView._DictView)
        assert parent["child"] == "grandchild"

    def test_merged_view_list(self):
        mv = pyrs_yaml.parse("data:\n  - a\n  - b\n  - c").merged()
        data = mv["data"]
        assert isinstance(data, MergedView._ListView)
        assert len(data) == 3
        assert data[0] == "a"

    def test_merged_view_merge_keys(self):
        mv = pyrs_yaml.parse("base: &base\n  a: 1\nmerged:\n  <<: *base\n  b: 2").merged()
        merged = mv["merged"]
        assert merged["a"] == 1
        assert merged["b"] == 2

    def test_merged_view_repr(self, yaml_strings):
        assert "MergedView" in repr(pyrs_yaml.parse(yaml_strings["simple_mapping"]).merged())

    def test_merged_view_empty(self):
        mv = pyrs_yaml.parse("{}").merged()
        assert len(mv) == 0
        assert list(mv) == []

    def test_merged_view_sequence_root(self):
        mv = pyrs_yaml.parse("- a\n- b\n- c").merged()
        assert len(mv) == 3
        assert mv[0] == "a"
        assert mv[1] == "b"
        assert mv[2] == "c"


class TestDocVersion:
    """Test document metadata (v0.8.0 Phase 4)."""

    @pytest.mark.parametrize(
        "key", ["simple_mapping", "nested_mapping", "sequence"], ids=["mapping", "nested", "sequence"]
    )
    def test_doc_version(self, yaml_strings, key):
        assert pyrs_yaml.parse(yaml_strings[key]).version() == "1.2"
