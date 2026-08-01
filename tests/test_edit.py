"""Tests for in-place editing (set/insert/append/delete/rename) of pyrs-yaml.

Round-trip (comments/anchors/tags/scalar-style/flow-style/mapping-order) is the
primary test pattern.
"""

import pyrs_yaml
import pytest


class TestSetPath:
    def test_set_scalar_preserves_comment(self):
        doc = pyrs_yaml.parse("a: 1  # keep me\n")
        doc._set_path(["a"], 2)
        out = doc.to_yaml()
        assert out == "a: 2  # keep me\n"

    def test_set_creates_missing_key(self):
        doc = pyrs_yaml.parse("a: 1\n")
        doc._set_path(["b"], 2)
        assert doc.to_yaml() == "a: 1\nb: 2\n"

    def test_set_missing_intermediate_raises(self):
        doc = pyrs_yaml.parse("a: 1\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc._set_path(["x", "y"], 3)

    def test_set_negative_index_raises(self):
        doc = pyrs_yaml.parse("arr: [1, 2, 3]\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc._set_path(["arr", -1], 9)

    def test_set_tuple_value_raises(self):
        doc = pyrs_yaml.parse("a: 1\n")
        with pytest.raises(pyrs_yaml.YamlTypeError):
            doc._set_path(["a"], (1, 2))

    def test_set_nested_mapping(self):
        doc = pyrs_yaml.parse("a:\n  b: 1\n  c: 2\n")
        doc._set_path(["a", "b"], 99)
        assert doc.to_yaml() == "a:\n  b: 99\n  c: 2\n"

    def test_set_sequence_index(self):
        doc = pyrs_yaml.parse("arr:\n  - 1\n  - 2\n")
        doc._set_path(["arr", 0], 10)
        assert doc.to_yaml() == "arr:\n  - 10\n  - 2\n"

    def test_set_bumps_revision_and_dirty(self):
        doc = pyrs_yaml.parse("a: 1\n")
        rev0 = doc._revision()
        doc._set_path(["a"], 5)
        assert doc._revision() == rev0 + 1
        assert doc.to_yaml() == "a: 5\n"


class TestEditErrors:
    def test_edit_error_hierarchy(self):
        assert issubclass(pyrs_yaml.YamlEditError, ValueError)
        assert issubclass(pyrs_yaml.YamlPathError, ValueError)
        assert pyrs_yaml.YamlEditError.__module__ == "pyrs_yaml"
        assert pyrs_yaml.YamlPathError.__module__ == "pyrs_yaml"


class TestLazySourceSync:
    def test_source_stable_when_unchanged(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\n")
        assert doc.source() == "a: 1\nb: 2\n"
        assert doc.to_yaml() == "a: 1\nb: 2\n"

    def test_reparse_stable_when_unchanged(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\n")
        doc.reparse()
        assert doc.to_yaml() == "a: 1\nb: 2\n"


class TestInsertPath:
    def test_append_to_sequence(self):
        doc = pyrs_yaml.parse("arr: [1, 2]\n")
        doc._append_path(["arr"], 3)
        assert doc.to_yaml() == "arr: [1, 2, 3]\n"

    def test_insert_mid(self):
        doc = pyrs_yaml.parse("arr: [1, 2, 3]\n")
        doc._insert_path(["arr"], 1, 99)
        assert doc.to_yaml() == "arr: [1, 99, 2, 3]\n"

    def test_insert_at_end(self):
        doc = pyrs_yaml.parse("arr: [1]\n")
        doc._insert_path(["arr"], 1, 99)
        assert doc.to_yaml() == "arr: [1, 99]\n"

    def test_insert_out_of_bounds_raises(self):
        doc = pyrs_yaml.parse("arr: [1]\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc._insert_path(["arr"], 5, 99)

    def test_insert_into_scalar_raises(self):
        doc = pyrs_yaml.parse("a: 1\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc._insert_path(["a"], 0, 99)

    def test_insert_into_mapping_raises(self):
        doc = pyrs_yaml.parse("a:\n  b: 1\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc._insert_path(["a"], 0, 99)

    def test_insert_preserves_item_comment(self):
        doc = pyrs_yaml.parse("arr:\n  - 1\n  - 2  # keep me\n")
        doc._insert_path(["arr"], 0, 99)
        assert doc.to_yaml() == "arr:\n  - 99\n  - 1\n  - 2  # keep me\n"

    def test_insert_bumps_revision(self):
        doc = pyrs_yaml.parse("arr: [1]\n")
        rev0 = doc._revision()
        doc._insert_path(["arr"], 0, 99)
        assert doc._revision() == rev0 + 1


class TestDeletePath:
    def test_delete_mapping_pair(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\n")
        doc._delete_path(["a"])
        assert doc.to_yaml() == "b: 2\n"

    def test_delete_mapping_pair_with_comment_on_key(self):
        doc = pyrs_yaml.parse("a: 1  # keep\nb: 2\n")
        doc._delete_path(["a"])
        assert doc.to_yaml() == "b: 2\n"

    def test_delete_sequence_element(self):
        doc = pyrs_yaml.parse("arr: [1, 2, 3]\n")
        doc._delete_path(["arr", 1])
        assert doc.to_yaml() == "arr: [1, 3]\n"

    def test_delete_preserves_order(self):
        doc = pyrs_yaml.parse("b: 2\na: 1\nc: 3\n")
        doc._delete_path(["a"])
        assert doc.to_yaml() == "b: 2\nc: 3\n"

    def test_delete_root_raises(self):
        doc = pyrs_yaml.parse("a: 1\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc._delete_path([])

    def test_delete_missing_key_raises(self):
        doc = pyrs_yaml.parse("a: 1\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc._delete_path(["nope"])

    def test_delete_out_of_bounds_raises(self):
        doc = pyrs_yaml.parse("arr: [1]\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc._delete_path(["arr", 5])

    def test_delete_into_scalar_raises(self):
        doc = pyrs_yaml.parse("a: 1\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc._delete_path(["a", "b"])

    def test_delete_bumps_revision(self):
        doc = pyrs_yaml.parse("a: 1\n")
        rev0 = doc._revision()
        doc._delete_path(["a"])
        assert doc._revision() == rev0 + 1
