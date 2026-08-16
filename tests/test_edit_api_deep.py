"""Tests for deep editing API enhancements (path, find_first, sort_keys, move, value_eq, set_many)."""

import pyrs_yaml


class TestNodePath:
    def test_path_property(self):
        doc = pyrs_yaml.parse("a:\n  b: 1\n")
        node = doc.node().find("$.a.b")
        assert node.path == ("a", "b")

    def test_path_root(self):
        doc = pyrs_yaml.parse("a: 1\n")
        assert doc.node().path == ()

    def test_path_sequence_index(self):
        doc = pyrs_yaml.parse("items: [10, 20, 30]\n")
        assert doc.node().find("$.items[2]").path == ("items", 2)


class TestNodeFindFirst:
    def test_find_first_single(self):
        doc = pyrs_yaml.parse("a: 1\n")
        node = doc.node().find_first("$.a")
        assert node is not None
        assert node.value == 1

    def test_find_first_wildcard(self):
        doc = pyrs_yaml.parse("items: [10, 20, 30]\n")
        node = doc.node().find_first("$.items[*]")
        assert node is not None
        assert node.value == 10

    def test_find_first_missing(self):
        doc = pyrs_yaml.parse("a: 1\n")
        assert doc.node().find_first("$.b") is None


class TestNodeValueEq:
    def test_value_eq_scalar(self):
        doc = pyrs_yaml.parse("a: 1\n")
        assert doc.node().find("$.a").value_eq(1)

    def test_value_eq_mapping(self):
        doc1 = pyrs_yaml.parse("a:\n  x: 1\n")
        doc2 = pyrs_yaml.parse("a:\n  x: 1\n")
        assert doc1.node().find("$.a").value_eq(doc2.node().find("$.a"))

    def test_value_eq_different(self):
        doc = pyrs_yaml.parse("a: 1\n")
        assert not doc.node().find("$.a").value_eq(2)

    def test_value_eq_with_node(self):
        doc = pyrs_yaml.parse("a: [1, 2]\n")
        n1 = doc.node().find("$.a")
        assert n1.value_eq(n1)


class TestDocSortKeys:
    def test_sort_keys_root(self):
        doc = pyrs_yaml.parse("z: 1\na: 2\nm: 3\n")
        doc.sort_keys()
        assert doc.to_yaml() == "a: 2\nm: 3\nz: 1\n"

    def test_sort_keys_path(self):
        doc = pyrs_yaml.parse("outer:\n  z: 1\n  a: 2\nkeep: first\n")
        doc.sort_keys("$.outer")
        assert doc.to_yaml() == "outer:\n  a: 2\n  z: 1\nkeep: first\n"

    def test_sort_keys_round_trip_preserves_meta(self):
        doc = pyrs_yaml.parse("z: 1  # z-note\na: 2\n")
        doc.sort_keys()
        assert doc.to_yaml() == "a: 2\nz: 1  # z-note\n"


class TestNodeMove:
    def test_move_to_empty_mapping(self):
        doc = pyrs_yaml.parse("src:\n  x: 1\n  y: 2\ndst: {}\n")
        doc.node().find("$.src").move("$.dst")
        assert doc.to_yaml() == "dst:\n  x: 1\n  y: 2\n"

    def test_move_preserves_subtree(self):
        doc = pyrs_yaml.parse("src:\n  nested:\n    deep: [1, 2]\ndst: {}\n")
        doc.node().find("$.src").move("$.dst")
        # dst was declared flow-style ({}); the pasted value keeps flow formatting
        assert doc.to_yaml() == "dst:\n  nested:\n    deep: [1, 2]\n"

    def test_move_set_missing(self):
        doc = pyrs_yaml.parse("src: 5\ndst: 0\n")
        doc.node().find("$.src").move("$.dst")
        assert doc.node().find("$.dst").value == 5


class TestDocSetMany:
    def test_set_many_simple(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\nc: 3\n")
        doc.set_many({"$.a": 100, "$.c": 300})
        assert doc.to_yaml() == "a: 100\nb: 2\nc: 300\n"

    def test_set_many_wildcard(self):
        doc = pyrs_yaml.parse("items:\n  - name: a\n    active: true\n  - name: b\n    active: true\n")
        doc.set_many({"$.items[*].active": False})
        assert doc.to_yaml() == "items:\n  - name: a\n    active: false\n  - name: b\n    active: false\n"

    def test_set_many_mixed(self):
        doc = pyrs_yaml.parse("items: [{n: 1, v: x}, {n: 2, v: y}]\n")
        doc.set_many({"$.items[0].n": 10, "$.items[1].v": "z"})
        assert doc.node().find("$.items[0].n").value == 10
        assert doc.node().find("$.items[1].v").value == "z"

    def test_set_many_deep_scan(self):
        doc = pyrs_yaml.parse("a:\n  x: 1\nb:\n  x: 2\n")
        doc.set_many({"$..x": 9})
        assert doc.node().find("$.a.x").value == 9
        assert doc.node().find("$.b.x").value == 9
