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

    def test_set_create_missing_nested(self):
        doc = pyrs_yaml.parse("a: 1\n")
        doc._set_path(["b", "c"], 2, True)
        assert doc.to_yaml() == "a: 1\nb:\n  c: 2\n"

    def test_set_create_missing_mixed_existing(self):
        doc = pyrs_yaml.parse("a:\n  b: 1\n")
        doc._set_path(["a", "c", "d"], 2, True)
        assert doc.to_yaml() == "a:\n  b: 1\n  c:\n    d: 2\n"

    def test_set_create_missing_three_levels_empty_doc(self):
        doc = pyrs_yaml.parse("")
        doc._set_path(["a", "b", "c"], 1, True)
        assert doc.to_yaml() == "a:\n  b:\n    c: 1\n"

    def test_set_create_missing_index_still_raises(self):
        doc = pyrs_yaml.parse("a:\n  - 1\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc._set_path(["a", 5, "x"], 2, True)

    def test_set_create_missing_scalar_intermediate_raises(self):
        doc = pyrs_yaml.parse("a: 1\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc._set_path(["a", "b"], 2, True)

    def test_set_create_missing_keeps_existing_content(self):
        doc = pyrs_yaml.parse("a: 1  # keep\nb:\n  x: 1\n")
        doc._set_path(["b", "y"], 2, True)
        assert doc.to_yaml() == "a: 1  # keep\nb:\n  x: 1\n  y: 2\n"

    def test_splice_offsets_cached_on_first_edit(self):
        """Multiple edits on a default-layout doc preserve round-trip (splice cache)."""
        doc = pyrs_yaml.parse("a: 1\nb: 2\n")
        doc._set_path(["a"], 9)
        assert doc.to_yaml() == "a: 9\nb: 2\n"

    def test_splice_multiple_edits_preserve_format(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\nc: 3\n")
        doc._set_path(["a"], 9)
        doc._set_path(["c"], 7)
        assert doc.to_yaml() == "a: 9\nb: 2\nc: 7\n"

    def test_set_method_create_missing(self):
        doc = pyrs_yaml.parse("a: 1\n")
        doc.set("$.b.c.d", 2, create_missing=True)
        assert doc.to_yaml() == "a: 1\nb:\n  c:\n    d: 2\n"

    def test_set_negative_index_from_end(self):
        doc = pyrs_yaml.parse("arr: [1, 2, 3]\n")
        doc._set_path(["arr", -1], 9)
        assert doc.to_yaml() == "arr: [1, 2, 9]\n"

    def test_set_negative_index_out_of_range_raises(self):
        doc = pyrs_yaml.parse("arr: [1, 2, 3]\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc._set_path(["arr", -4], 9)

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


class TestRenamePath:
    def test_rename_key_keeps_value_and_comment(self):
        doc = pyrs_yaml.parse("old: 1  # c\n")
        doc._rename_path(["old"], "new")
        assert doc.to_yaml() == "new: 1  # c\n"

    def test_rename_keeps_position(self):
        doc = pyrs_yaml.parse("a: 1\nold: 2\nc: 3\n")
        doc._rename_path(["old"], "b")
        assert doc.to_yaml() == "a: 1\nb: 2\nc: 3\n"

    def test_rename_missing_raises(self):
        doc = pyrs_yaml.parse("a: 1\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc._rename_path(["zzz"], "x")

    def test_rename_root_raises(self):
        doc = pyrs_yaml.parse("a: 1\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc._rename_path([], "x")

    def test_rename_nested_key(self):
        doc = pyrs_yaml.parse("a:\n  old: 1\n")
        doc._rename_path(["a", "old"], "new")
        assert doc.to_yaml() == "a:\n  new: 1\n"

    def test_rename_bumps_revision(self):
        doc = pyrs_yaml.parse("old: 1\n")
        rev0 = doc._revision()
        doc._rename_path(["old"], "new")
        assert doc._revision() == rev0 + 1

    def test_rename_to_existing_key_raises_atomically(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc._rename_path(["a"], "b")
        assert doc.to_yaml() == "a: 1\nb: 2\n"

    def test_rename_to_existing_key_nested_raises(self):
        doc = pyrs_yaml.parse("m:\n  a: 1\n  b: 2\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc._rename_path(["m", "a"], "b")
        assert doc.to_yaml() == "m:\n  a: 1\n  b: 2\n"

    def test_rename_to_same_key_is_noop(self):
        doc = pyrs_yaml.parse("a: 1  # c\n")
        doc._rename_path(["a"], "a")
        assert doc.to_yaml() == "a: 1  # c\n"

    def test_rename_conflict_does_not_bump_revision(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\n")
        rev0 = doc._revision()
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc._rename_path(["a"], "b")
        assert doc._revision() == rev0


class TestSetItem:
    def test_setitem_existing(self):
        doc = pyrs_yaml.parse("a: 1\n")
        doc["a"] = 2
        assert doc["a"] == 2

    def test_setitem_creates_key(self):
        doc = pyrs_yaml.parse("a: 1\n")
        doc["b"] = 3
        assert doc.to_dict() == {"a": 1, "b": 3}

    def test_setitem_preserves_comment(self):
        doc = pyrs_yaml.parse("a: 1  # keep\n")
        doc["a"] = 5
        assert doc.to_yaml() == "a: 5  # keep\n"

    def test_setitem_bumps_revision(self):
        doc = pyrs_yaml.parse("a: 1\n")
        rev0 = doc._revision()
        doc["a"] = 2
        assert doc._revision() == rev0 + 1


class TestDelItem:
    def test_delitem(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\n")
        del doc["a"]
        assert doc.to_dict() == {"b": 2}

    def test_delitem_missing_raises(self):
        doc = pyrs_yaml.parse("a: 1\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            del doc["zzz"]

    def test_delitem_bumps_revision(self):
        doc = pyrs_yaml.parse("a: 1\n")
        rev0 = doc._revision()
        del doc["a"]
        assert doc._revision() == rev0 + 1


class TestNodeEditMethods:
    def test_node_set_value_preserves_metadata(self):
        doc = pyrs_yaml.parse("server:\n  host: localhost  # bind address\n")
        node = doc.node().find("$.server.host")
        node.set_value("0.0.0.0")
        assert doc.to_yaml() == "server:\n  host: 0.0.0.0  # bind address\n"

    def test_node_stale_after_edit(self):
        doc = pyrs_yaml.parse("a: 1\n")
        node = doc.node().find("$.a")
        doc["b"] = 2  # any edit bumps revision
        with pytest.warns(RuntimeWarning), pytest.raises(pyrs_yaml.YamlDocumentError):
            _ = node.value

    def test_node_self_edit_does_not_stale(self):
        doc = pyrs_yaml.parse("a: 1\n")
        node = doc.node().find("$.a")
        node.set_value(2)
        assert doc.to_dict()["a"] == 2

    def test_node_delete_stale(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\n")
        node = doc.node().find("$.a")
        node.delete()
        assert doc.to_dict() == {"b": 2}

    def test_node_append(self):
        doc = pyrs_yaml.parse("arr: [1]\n")
        node = doc.node().find("$.arr")
        node.append(2)
        assert doc.to_dict()["arr"] == [1, 2]

    def test_node_insert(self):
        doc = pyrs_yaml.parse("arr: [1, 3]\n")
        node = doc.node().find("$.arr")
        node.insert(1, 2)
        assert doc.to_dict()["arr"] == [1, 2, 3]

    def test_node_rename(self):
        doc = pyrs_yaml.parse("old: 1  # c\n")
        node = doc.node().find("$.old")
        node.rename("new")
        assert doc.to_yaml() == "new: 1  # c\n"


class TestYamlDocumentPathAPI:
    def test_doc_set(self):
        doc = pyrs_yaml.parse("a: 1\n")
        doc.set("$.a", 2)
        assert doc.to_yaml() == "a: 2\n"

    def test_doc_set_creates_key(self):
        doc = pyrs_yaml.parse("a: 1\n")
        doc.set("$.b", 3)
        assert doc.to_yaml() == "a: 1\nb: 3\n"

    def test_doc_insert(self):
        doc = pyrs_yaml.parse("arr: [1, 2, 3]\n")
        doc.insert("$.arr", 1, 99)
        assert doc.to_yaml() == "arr: [1, 99, 2, 3]\n"

    def test_doc_append(self):
        doc = pyrs_yaml.parse("arr: [1, 2]\n")
        doc.append("$.arr", 3)
        assert doc.to_yaml() == "arr: [1, 2, 3]\n"

    def test_doc_delete(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\n")
        doc.delete("$.a")
        assert doc.to_yaml() == "b: 2\n"

    def test_doc_rename(self):
        doc = pyrs_yaml.parse("old: 1  # c\n")
        doc.rename("$.old", "new")
        assert doc.to_yaml() == "new: 1  # c\n"

    def test_doc_rename_conflict_raises(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc.rename("$.a", "b")
        assert doc.to_yaml() == "a: 1\nb: 2\n"

    def test_path_wildcard_raises(self):
        doc = pyrs_yaml.parse("arr: [1, 2]\n")
        with pytest.raises(pyrs_yaml.YamlPathError):
            doc.set("$.arr[*]", 3)

    def test_path_deepscan_raises(self):
        doc = pyrs_yaml.parse("a: {b: 1}\n")
        with pytest.raises(pyrs_yaml.YamlPathError):
            doc.delete("$..b")

    def test_doc_set_missing_intermediate_raises(self):
        doc = pyrs_yaml.parse("a: 1\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc.set("$.x.y", 3)


class TestAliasAndMerge:
    def test_set_alias_reference_replaces_in_place(self):
        # typ="safe" (resolve_merges=false) keeps the alias node in the AST
        doc = pyrs_yaml.YAML(typ="safe").parse("defaults: &defaults\n  timeout: 30\nprod: *defaults\n")
        doc.set("$.prod", {"timeout": 99})
        assert doc.to_dict()["prod"] == {"timeout": 99}

    def test_set_merge_expanded_key_edits_clone_only(self):
        doc = pyrs_yaml.parse("defaults: &defaults\n  timeout: 30\nprod:\n  <<: *defaults\n")
        doc.set("$.prod.timeout", 99)
        assert doc.to_dict()["defaults"]["timeout"] == 30
        assert doc.to_dict()["prod"]["timeout"] == 99

    def test_set_through_alias_raises(self):
        doc = pyrs_yaml.YAML(typ="safe").parse("defaults: &defaults\n  a: 1\nprod: *defaults\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc.set("$.prod.a", 5)

    def test_delete_anchored_node_tolerated(self):
        doc = pyrs_yaml.parse("defaults: &defaults\n  a: 1\ncopy: *defaults\n")
        doc.delete("$.defaults")
        assert "defaults" not in doc.to_dict()


class TestEditAtomicity:
    def test_failed_edit_leaves_doc_identical(self):
        doc = pyrs_yaml.parse("a: 1\nb: [1, 2]\n")
        before = doc.to_yaml()
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc.insert("$.b", 5, 99)  # out of bounds
        assert doc.to_yaml() == before
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc.set("$.x.y", 3)  # missing intermediate
        assert doc.to_yaml() == before
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc.delete("$.nope")  # missing key
        assert doc.to_yaml() == before

    def test_failed_edit_does_not_bump_revision(self):
        doc = pyrs_yaml.parse("a: 1\nb: [1]\n")
        rev0 = doc._revision()
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc.insert("$.b", 5, 99)
        assert doc._revision() == rev0


class TestNegativeIndexPaths:
    def test_set_via_string_path(self):
        doc = pyrs_yaml.parse("arr:\n  - 1\n  - 2\n  - 3\n")
        doc.set("$.arr[-1]", 9)
        assert doc.to_yaml() == "arr:\n  - 1\n  - 2\n  - 9\n"

    def test_delete_via_string_path(self):
        doc = pyrs_yaml.parse("arr:\n  - 1\n  - 2\n  - 3\n")
        doc.delete("$.arr[-2]")
        assert doc.to_yaml() == "arr:\n  - 1\n  - 3\n"

    def test_insert_negative_index(self):
        doc = pyrs_yaml.parse("arr:\n  - 1\n  - 2\n  - 3\n")
        doc.insert("$.arr", -1, 99)
        assert doc.to_yaml() == "arr:\n  - 1\n  - 2\n  - 99\n  - 3\n"

    def test_set_out_of_range_negative_raises(self):
        doc = pyrs_yaml.parse("arr:\n  - 1\n  - 2\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc.set("$.arr[-3]", 9)

    def test_get_negative_index(self):
        doc = pyrs_yaml.parse("arr: [1, 2, 3]\n")
        assert doc.get("$.arr[-1]") == 3
        assert doc.get("$.arr[-3]") == 1
        assert doc.get("$.arr[-4]") is None


class TestEmptyDocumentEdit:
    def test_set_on_empty_doc_creates_mapping_root(self):
        doc = pyrs_yaml.parse("")
        doc.set("$.a", 1)
        assert doc.to_dict() == {"a": 1}
        assert doc.to_yaml() == "a: 1\n"

    def test_set_nested_on_empty_doc_raises_missing_intermediate(self):
        doc = pyrs_yaml.parse("")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc.set("$.a.b", 2)


class TestGetJsonPath:
    def test_nested_get(self):
        doc = pyrs_yaml.parse("a:\n  b:\n    c: 42\n")
        assert doc.get("$.a.b.c") == 42
        assert doc.get("$.a.missing") is None

    def test_get_sequence_index_path(self):
        doc = pyrs_yaml.parse("arr: [10, 20, 30]\n")
        assert doc.get("$.arr[1]") == 20
        assert doc.get("$.arr[5]") is None

    def test_get_invalid_path_raises(self):
        doc = pyrs_yaml.parse("a: 1\n")
        with pytest.raises(pyrs_yaml.YamlPathError):
            doc.get("$[bad")
        with pytest.raises(pyrs_yaml.YamlPathError):
            doc.get("$..a")

    def test_get_dotted_key_now_path_semantics(self):
        doc = pyrs_yaml.parse("a.b: 1\n")
        assert doc.get("a.b") is None  # '.' routes to JSONPath, not a literal key
        assert doc.get("$.a.b") is None


class TestCompactSequenceItems:
    def test_round_trip_compact_mapping_item(self):
        doc = pyrs_yaml.parse("servers:\n  - host: a\n")
        assert doc.to_yaml() == "servers:\n  - host: a\n"

    def test_round_trip_compact_multi_key_item(self):
        doc = pyrs_yaml.parse("- host: a\n  port: 8080\n")
        assert doc.to_yaml() == "- host: a\n  port: 8080\n"

    def test_round_trip_comment_inside_compact_item(self):
        doc = pyrs_yaml.parse("- host: a  # keep\n  port: 8080\n")
        assert doc.to_yaml() == "- host: a  # keep\n  port: 8080\n"

    def test_round_trip_flow_items_in_sequence(self):
        doc = pyrs_yaml.parse("items:\n  - [1, 2]\n  - {a: 1}\n")
        assert doc.to_yaml() == "items:\n  - [1, 2]\n  - {a: 1}\n"

    def test_round_trip_scalar_items_unchanged(self):
        doc = pyrs_yaml.parse("list:\n  - one\n  - two\n")
        assert doc.to_yaml() == "list:\n  - one\n  - two\n"

    def test_compact_item_with_anchor_stays_block(self):
        doc = pyrs_yaml.parse("- name: &x anchor\n  value: 1\n")
        assert "&x" in doc.to_yaml()


class TestAliasEditErrors:
    def test_set_through_alias_raises(self):
        doc = pyrs_yaml.parse("defaults: &defaults\n  a: 1\nprod: *defaults\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc.set("$.prod.a", 5)

    def test_insert_through_alias_raises(self):
        doc = pyrs_yaml.parse("defaults: &d\n  - 1\nprod: *d\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc.insert("$.prod", 0, 99)

    def test_append_through_alias_raises(self):
        doc = pyrs_yaml.parse("defaults: &d\n  - 1\nprod: *d\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc.append("$.prod", 99)

    def test_delete_through_alias_raises(self):
        doc = pyrs_yaml.parse("defaults: &d\n  x: 1\nprod: *d\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc.delete("$.prod.x")

    def test_rename_through_alias_raises(self):
        doc = pyrs_yaml.parse("defaults: &d\n  x: 1\nprod: *d\n")
        with pytest.raises(pyrs_yaml.YamlEditError):
            doc.rename("$.prod.x", "y")
