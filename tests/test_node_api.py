"""Tests for Python Node API (v0.8.0 Phase 2 + Phase 3 + Phase 6)."""

import contextlib

import pyrs_yaml
import pytest
from pyrs_yaml import Node


class TestNodeAPI:
    """Test Node class with find/filter/walk/parent/children."""

    def test_node_root(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["simple_mapping"])
        assert Node(doc).root_type == "mapping"

    def test_node_value(self, yaml_strings):
        node = Node(pyrs_yaml.parse("key: hello")).find("$.key")
        assert node.root_type == "scalar"
        assert node.value == "hello"

    @pytest.mark.parametrize(
        "yaml,path,expected",
        [
            ("key: value", "$.key", "value"),
            ("parent:\n  child: val", "$.parent.child", "val"),
            ("- a\n- b\n- c", "$[0]", "a"),
        ],
        ids=["simple", "nested", "index"],
    )
    def test_find_scalar(self, yaml, path, expected):
        assert Node(pyrs_yaml.parse(yaml)).find(path).value == expected

    @pytest.mark.parametrize(
        "yaml,path,check",
        [
            ("- a\n- b\n- c", "$[*]", lambda r: len(r) == 3 and r[0].value == "a"),
            ("parent:\n  child: val", "$..child", lambda r: len(r) == 1 and r[0].value == "val"),
            ("a:\n  b: 1\n  c: 2", "$..*", lambda r: len(r) >= 3),
            ("parent:\n  child: val", "$..x", lambda r: r == []),
            ("a:\n  val: 1\nb:\n  val: 2", "$..val", lambda r: len(r) == 2),
        ],
        ids=["wildcard", "deep", "deep-wildcard", "deep-no-match", "deep-multi"],
    )
    def test_find_list(self, yaml, path, check):
        assert check(Node(pyrs_yaml.parse(yaml)).find(path))

    def test_walk(self, yaml_strings):
        nodes = list(Node(pyrs_yaml.parse(yaml_strings["mixed"])).walk())
        assert len(nodes) >= 4

    def test_filter(self, yaml_strings):
        scalars = Node(pyrs_yaml.parse(yaml_strings["mixed"])).filter(lambda n: n.root_type == "scalar")
        assert len(scalars) >= 1

    def test_filter_no_match(self, yaml_strings):
        assert Node(pyrs_yaml.parse(yaml_strings["simple_mapping"])).filter(lambda n: n.root_type == "null") == []

    def test_parent_children(self, yaml_strings):
        root = Node(pyrs_yaml.parse(yaml_strings["nested_mapping"]))
        assert len(root.children) == 1
        child = root.children[0]
        assert child.children
        assert child.parent._path == root._path

    def test_to_yaml(self, yaml_strings):
        output = Node(pyrs_yaml.parse(yaml_strings["simple_mapping"])).to_yaml()
        assert isinstance(output, str) and len(output) > 0

    @pytest.mark.parametrize("yaml", ["{}", "[]"], ids=["empty-mapping", "empty-sequence"])
    def test_find_empty(self, yaml):
        assert Node(pyrs_yaml.parse(yaml)).children == []

    def test_walk_deeply_nested(self):
        nodes = list(Node(pyrs_yaml.parse("a:\n  b:\n    c:\n      d: 1")).walk())
        assert len(nodes) == 5

    def test_node_repr(self, yaml_strings):
        r = repr(Node(pyrs_yaml.parse(yaml_strings["simple_mapping"])))
        assert "Node" in r and "mapping" in r

    def test_node_eq(self, yaml_strings):
        doc = pyrs_yaml.parse(yaml_strings["simple_mapping"])
        assert Node(doc) == Node(doc)

    def test_find_index_out_of_range(self, yaml_strings):
        with pytest.raises(IndexError):
            _ = Node(pyrs_yaml.parse(yaml_strings["sequence"])).find("$[999]").value

    def test_children_scalar(self, yaml_strings):
        node = Node(pyrs_yaml.parse(yaml_strings["simple_mapping"])).find("$.key")
        assert node.children == []

    def test_to_yaml_scalar(self, yaml_strings):
        node = Node(pyrs_yaml.parse(yaml_strings["simple_mapping"])).find("$.key")
        assert node.to_yaml() == "value\n"

    def test_find_invalid_path(self, yaml_strings):
        with pytest.raises(ValueError, match="Path must start with"):
            Node(pyrs_yaml.parse(yaml_strings["simple_mapping"])).find("key")


class TestDocWalk:
    """Test doc.walk() and doc.scalars() (D4 Rust-backed traversal)."""

    def test_doc_walk_flat_mapping(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\nc: 3\n")
        paths = [n._path for n in doc.walk()]
        assert paths == [(), ("a",), ("b",), ("c",)]

    def test_doc_walk_nested(self):
        doc = pyrs_yaml.parse("a:\n  b: 1\n  c: 2\n")
        paths = [n._path for n in doc.walk()]
        assert paths == [(), ("a",), ("a", "b"), ("a", "c")]

    def test_doc_walk_sequence(self):
        doc = pyrs_yaml.parse("- a\n- b\n- c\n")
        paths = [n._path for n in doc.walk()]
        assert paths == [(), (0,), (1,), (2,)]

    def test_doc_scalars_flat(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\nc: 3\n")
        paths = [n._path for n in doc.scalars()]
        assert paths == [("a",), ("b",), ("c",)]

    def test_doc_scalars_nested(self):
        doc = pyrs_yaml.parse("a:\n  b: 1\n  c: 2\n")
        paths = [n._path for n in doc.scalars()]
        assert paths == [("a", "b"), ("a", "c")]

    def test_doc_walk_yields_nodes(self):
        doc = pyrs_yaml.parse("a: 1\n")
        for node in doc.walk():
            assert isinstance(node, Node)

    def test_doc_scalars_values(self):
        doc = pyrs_yaml.parse("a: hello\nb: world\n")
        values = {n._path: n.value for n in doc.scalars()}
        assert values == {("a",): "hello", ("b",): "world"}

    def test_doc_walk_empty_doc(self):
        doc = pyrs_yaml.parse("")
        paths = [n._path for n in doc.walk()]
        assert paths == [()]

    def test_doc_scalars_empty_doc(self):
        doc = pyrs_yaml.parse("")
        paths = [n._path for n in doc.scalars()]
        assert paths == [()]  # empty doc is a Null node, which is a scalar-like node

    def test_doc_walk_null_values(self):
        doc = pyrs_yaml.parse("a: null\nb: ~\n")
        paths = [n._path for n in doc.walk()]
        assert paths == [(), ("a",), ("b",)]

    def test_doc_scalars_null_values(self):
        doc = pyrs_yaml.parse("a: null\nb: ~\n")
        paths = [n._path for n in doc.scalars()]
        assert paths == [("a",), ("b",)]

    def test_doc_scalars_no_scalars(self):
        doc = pyrs_yaml.parse("a:\n  b:\n    c:\n")
        paths = [n._path for n in doc.scalars()]
        assert paths == [("a", "b", "c")]

    def test_doc_walk_deeply_nested(self):
        doc = pyrs_yaml.parse("a:\n  b:\n    c:\n      d: 1\n")
        paths = [n._path for n in doc.walk()]
        assert paths == [(), ("a",), ("a", "b"), ("a", "b", "c"), ("a", "b", "c", "d")]

    def test_doc_walk_flow_mapping(self):
        doc = pyrs_yaml.parse("a: {b: 1, c: 2}\n")
        paths = [n._path for n in doc.walk()]
        assert paths == [(), ("a",), ("a", "b"), ("a", "c")]

    def test_doc_walk_flow_sequence(self):
        doc = pyrs_yaml.parse("a: [1, 2, 3]\n")
        paths = [n._path for n in doc.walk()]
        assert paths == [(), ("a",), ("a", 0), ("a", 1), ("a", 2)]

    def test_doc_walk_mixed_types(self):
        doc = pyrs_yaml.parse("a: 1\nb: {c: 2}\nc: [3, 4]\n")
        paths = [n._path for n in doc.walk()]
        assert paths == [(), ("a",), ("b",), ("b", "c"), ("c",), ("c", 0), ("c", 1)]


class TestNodeRelease:
    """Test Node.release() lifecycle warnings (v0.8.0 Phase 6)."""

    def test_release_makes_stale(self, yaml_strings):
        node = Node(pyrs_yaml.parse(yaml_strings["simple_mapping"]))
        assert node.is_valid()
        node.release()
        assert not node.is_valid()

    def test_release_raises(self, yaml_strings):
        node = Node(pyrs_yaml.parse(yaml_strings["simple_mapping"]))
        node.release()
        with pytest.raises(pyrs_yaml.YamlDocumentError):
            _ = node.root_type

    def test_release_warning(self, yaml_strings):
        node = Node(pyrs_yaml.parse(yaml_strings["simple_mapping"]))
        node.release()
        with pytest.warns(RuntimeWarning), contextlib.suppress(pyrs_yaml.YamlDocumentError):
            _ = node.root_type
