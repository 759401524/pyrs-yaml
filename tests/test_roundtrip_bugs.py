"""Regression tests for the four round-trip bugs surfaced by the property
tests (previously documented as known limitations in test_property_roundtrip).

- Bug 1: lone quote characters as keys broke round-trip.
- Bug 2: empty collections dumped to an empty document (re-parsed as None).
- Bug 3: get() heuristics made dotted/bracket keys unreachable as literal keys.
- Bug 4: quoted scalars were schema-resolved instead of staying strings
  (YAML 1.2: only plain scalars are resolved).
"""

import pytest

import pyrs_yaml


class TestLoneQuoteKeys:
    def test_lone_single_quote_key_roundtrips(self):
        data = {"'": None}
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump(data)) == data

    def test_lone_double_quote_key_roundtrips(self):
        data = {'"': 1}
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump(data)) == data

    def test_document_roundtrip(self):
        doc = pyrs_yaml.parse("'': 1\n")
        assert doc.to_dict() == {"": 1}


class TestEmptyCollections:
    def test_empty_list_roundtrips(self):
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump([])) == []

    def test_empty_dict_roundtrips(self):
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump({})) == {}

    def test_nested_empty_collections(self):
        data = {"a": [], "b": {}}
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump(data)) == data

    def test_document_empty_collections(self):
        doc = pyrs_yaml.parse("a:\n  b: []\n")
        assert doc.to_dict() == {"a": {"b": []}}

    def test_from_dict_empty(self):
        assert pyrs_yaml.safe_load(pyrs_yaml.from_dict({})) == {}
        assert pyrs_yaml.safe_load(pyrs_yaml.from_dict([])) == []


class TestGetLiteralKey:
    def test_setitem_then_get_literal_key(self):
        doc = pyrs_yaml.parse("c[0]: 99\n")
        doc["c[0]"] = 100
        assert doc.get("c[0]") == 100
        assert doc.to_dict() == {"c[0]": 100}

    def test_dotted_key_reachable(self):
        doc = pyrs_yaml.parse("a.b: 1\n")
        assert doc.get("a.b") == 1

    def test_bracket_key_reachable(self):
        doc = pyrs_yaml.parse("c[0]: 2\n")
        assert doc.get("c[0]") == 2

    def test_path_access_via_find(self):
        doc = pyrs_yaml.parse("arr:\n  - 1\n  - 2\n")
        assert doc.find("$.arr[1]").value == 2
        with pytest.raises(IndexError):
            _ = doc.find("$.arr[9]").value


class TestQuotedScalarIsString:
    def test_safe_dump_true_roundtrips_as_string(self):
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump("true")) == "true"

    def test_quoted_number_stays_string(self):
        assert pyrs_yaml.safe_load('"42"') == "42"
        assert pyrs_yaml.safe_load("'42'") == "42"

    def test_quoted_bool_and_null_stay_strings(self):
        assert pyrs_yaml.safe_load('"true"') == "true"
        assert pyrs_yaml.safe_load('"null"') == "null"

    def test_plain_negative_roundtrips_as_int(self):
        doc = pyrs_yaml.parse("x: -1\n")
        assert doc.to_dict() == {"x": -1}
        assert pyrs_yaml.parse(doc.to_yaml()).to_dict() == {"x": -1}

    def test_plain_bool_roundtrips(self):
        doc = pyrs_yaml.parse("x: true\n")
        assert doc.to_dict() == {"x": True}
        assert pyrs_yaml.parse(doc.to_yaml()).to_dict() == {"x": True}
