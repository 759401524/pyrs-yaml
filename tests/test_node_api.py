"""Tests for Python Node API (v0.8.0 Phase 2 + Phase 3 + Phase 6)."""

import contextlib

import pyrs_yaml
import pytest
from pyrs_yaml import Node


class TestNodeAPI:
    """Test Node class with find/filter/walk/parent/children."""

    def test_node_root(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["simple_mapping"])
        node = Node(doc)
        assert node.root_type == "mapping"

    def test_node_value(self, yaml_strings):
        doc = pyrs_yaml.parse("key: hello")
        node = Node(doc).find("$.key")
        assert node.root_type == "scalar"
        assert node.value == "hello"

    def test_find_simple(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["simple_mapping"])
        node = Node(doc).find("$.key")
        assert node.value == "value"

    def test_find_nested(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["nested_mapping"])
        node = Node(doc).find("$.parent.child")
        assert node.value == "grandchild"

    def test_find_index(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["sequence"])
        node = Node(doc).find("$[0]")
        assert node.value == "a"

    def test_find_wildcard(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["sequence"])
        nodes = Node(doc).find("$[*]")
        assert isinstance(nodes, list)
        assert len(nodes) == 3
        assert nodes[0].value == "a"

    def test_find_deep(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["nested_mapping"])
        nodes = Node(doc).find("$..child")
        assert isinstance(nodes, list)
        assert len(nodes) == 1
        assert nodes[0].value == "grandchild"

    def test_find_deep_wildcard(self, yaml_strings):
        yaml = "a:\n  b: 1\n  c: 2"
        doc = pyrs_yaml.parse(yaml)
        nodes = Node(doc).find("$..*")
        assert isinstance(nodes, list)
        assert len(nodes) >= 3

    def test_walk(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["mixed"])
        nodes = list(Node(doc).walk())
        assert len(nodes) >= 4

    def test_filter(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["mixed"])
        scalars = Node(doc).filter(lambda n: n.root_type == "scalar")
        assert len(scalars) >= 1

    def test_parent_children(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["nested_mapping"])
        root = Node(doc)
        children = root.children
        assert len(children) == 1
        child = children[0]
        assert child.children
        assert child.parent is not None
        assert child.parent._path == root._path

    def test_to_yaml(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["simple_mapping"])
        node = Node(doc)
        output = node.to_yaml()
        assert isinstance(output, str)
        assert len(output) > 0

    def test_find_empty_mapping(self):
        doc = pyrs_yaml.parse("{}")
        node = Node(doc)
        assert node.children == []

    def test_find_empty_sequence(self):
        doc = pyrs_yaml.parse("[]")
        node = Node(doc)
        assert node.children == []

    def test_walk_deeply_nested(self):
        yaml = "a:\n  b:\n    c:\n      d: 1"
        doc = pyrs_yaml.parse(yaml)
        nodes = list(Node(doc).walk())
        assert len(nodes) == 5

    def test_filter_no_match(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["simple_mapping"])
        nodes = Node(doc).filter(lambda n: n.root_type == "null")
        assert nodes == []

    def test_node_repr(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["simple_mapping"])
        node = Node(doc)
        r = repr(node)
        assert "Node" in r
        assert "mapping" in r

    def test_node_eq(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["simple_mapping"])
        a = Node(doc)
        b = Node(doc)
        assert a == b

    def test_find_index_out_of_range(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["sequence"])
        node = Node(doc).find("$[999]")
        with pytest.raises(IndexError):
            _ = node.value

    def test_find_deep_no_match(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["nested_mapping"])
        nodes = Node(doc).find("$..nonexistent")
        assert nodes == []

    def test_find_deep_multiple_matches(self):
        yaml = "a:\n  val: 1\nb:\n  val: 2"
        doc = pyrs_yaml.parse(yaml)
        nodes = Node(doc).find("$..val")
        assert len(nodes) == 2

    def test_children_scalar(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["simple_mapping"])
        node = Node(doc).find("$.key")
        assert node.children == []

    def test_to_yaml_scalar(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["simple_mapping"])
        node = Node(doc).find("$.key")
        output = node.to_yaml()
        assert output == "value\n"

    def test_find_invalid_path(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["simple_mapping"])
        with pytest.raises(ValueError, match="Path must start with"):
            Node(doc).find("key")


class TestNodeRelease:
    """Test Node.release() lifecycle warnings (v0.8.0 Phase 6)."""

    def test_release_makes_stale(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["simple_mapping"])
        node = Node(doc)
        assert node.is_valid()
        node.release()
        assert not node.is_valid()

    def test_release_raises(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["simple_mapping"])
        node = Node(doc)
        node.release()
        with pytest.raises(pyrs_yaml.YamlDocumentError):
            _ = node.root_type

    def test_release_warning(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["simple_mapping"])
        node = Node(doc)
        node.release()
        with pytest.warns(RuntimeWarning), contextlib.suppress(pyrs_yaml.YamlDocumentError):
            _ = node.root_type
