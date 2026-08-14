"""Smoke tests for safe_dump and from_dict (identical implementations)."""

import pyrs_yaml


class TestSafeDump:
    def test_dump_simple_dict(self):
        result = pyrs_yaml.safe_dump({"key": "value"})
        assert "key: value" in result

    def test_dump_nested_dict(self):
        result = pyrs_yaml.safe_dump({"a": {"b": 1}})
        assert "a:" in result
        assert "b: 1" in result

    def test_dump_list(self):
        result = pyrs_yaml.safe_dump([1, 2, 3])
        assert "- 1" in result
        assert "- 2" in result
        assert "- 3" in result

    def test_dump_mixed(self):
        result = pyrs_yaml.safe_dump({"items": [1, "two", True]})
        assert "items:" in result
        assert "- 1" in result
        assert "- two" in result
        assert "- true" in result

    def test_dump_empty_dict(self):
        result = pyrs_yaml.safe_dump({})
        # Empty dict emits the explicit flow form so it re-parses as {} (not null).
        assert result.strip() == "{}"

    def test_dump_empty_list(self):
        result = pyrs_yaml.safe_dump([])
        # Empty list emits the explicit flow form so it re-parses as [] (not null).
        assert result.strip() == "[]"

    def test_dump_string_with_special_chars(self):
        result = pyrs_yaml.safe_dump({"key": "hello: world"})
        assert "key:" in result

    def test_dump_none_value(self):
        result = pyrs_yaml.safe_dump({"key": None})
        assert "key: null" in result

    def test_dump_bool_values(self):
        result = pyrs_yaml.safe_dump({"a": True, "b": False})
        assert "a: true" in result
        assert "b: false" in result

    def test_dump_numeric_values(self):
        result = pyrs_yaml.safe_dump({"i": 42, "f": 3.14})
        assert "i: 42" in result
        assert "f: 3.14" in result


class TestFromDict:
    def test_from_dict_same_as_safe_dump(self):
        data = {"key": "value", "num": 42}
        assert pyrs_yaml.from_dict(data) == pyrs_yaml.safe_dump(data)

    def test_from_dict_nested(self):
        data = {"a": {"b": [1, 2]}}
        result = pyrs_yaml.from_dict(data)
        assert "a:" in result
        assert "b:" in result
        assert "- 1" in result
        assert "- 2" in result

    def test_from_dict_empty(self):
        assert pyrs_yaml.from_dict({}) == pyrs_yaml.safe_dump({})
        assert pyrs_yaml.from_dict([]) == pyrs_yaml.safe_dump([])


class TestRoundTrip:
    def test_dump_then_load(self):
        data = {"name": "test", "values": [1, 2, 3]}
        yaml_str = pyrs_yaml.safe_dump(data)
        loaded = pyrs_yaml.safe_load(yaml_str)
        assert loaded == data

    def test_from_dict_then_load(self):
        data = {"a": 1, "b": 2}
        yaml_str = pyrs_yaml.from_dict(data)
        loaded = pyrs_yaml.safe_load(yaml_str)
        assert loaded == data


class TestSequenceModes:
    """direct_dump write_sequence branch coverage (audit §15)."""

    def test_compact_mapping_in_sequence(self):
        # write_sequence is_compact_mapping=true (direct_dump path via safe_dump).
        out = pyrs_yaml.safe_dump([{"a": 1}])
        assert out.strip() == "- a: 1"
        assert pyrs_yaml.safe_load(out) == [{"a": 1}]

    def test_block_mapping_in_sequence(self):
        out = pyrs_yaml.safe_dump([{"a": 1, "b": 2}])
        assert pyrs_yaml.safe_load(out) == [{"a": 1, "b": 2}]

    def test_nested_list_in_sequence(self):
        # write_sequence nested-list branch.
        out = pyrs_yaml.safe_dump([[1, 2], [3, 4]])
        assert pyrs_yaml.safe_load(out) == [[1, 2], [3, 4]]


class TestFloatSpecialValues:
    """safe_dump of float('inf') / float('nan') (non-numpy path)."""

    def test_inf(self):
        out = pyrs_yaml.safe_dump(float("inf"))
        assert pyrs_yaml.safe_load(out) == float("inf")

    def test_neg_inf(self):
        out = pyrs_yaml.safe_dump(float("-inf"))
        assert pyrs_yaml.safe_load(out) == float("-inf")

    def test_nan(self):
        out = pyrs_yaml.safe_dump(float("nan"))
        # NaN differs from itself; just confirm it round-trips as a float.
        assert isinstance(pyrs_yaml.safe_load(out), float)
