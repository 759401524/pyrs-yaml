"""YAML serialization and PyYAML-compatible API tests."""

import pyrs_yaml
import pytest


class TestSerialization:
    """Test YAML serialization"""

    @pytest.mark.parametrize(
        "yaml_str,expected",
        [
            ("key: value", "key: value"),
            ("a: 1\nb: 2", "a: 1"),
            ("- a\n- b", "- a"),
        ],
        ids=["scalar", "mapping", "sequence"],
    )
    def test_serializes_to_yaml(self, yaml_str, expected):
        assert expected in pyrs_yaml.parse(yaml_str).to_yaml()


class TestPyyamlCompatible:
    """Test pyyaml-compatible API"""

    def test_safe_loads_mapping(self):
        assert pyrs_yaml.safe_load("key: value") == {"key": "value"}

    @pytest.mark.parametrize(
        "yaml_str,key,expected",
        [
            ("n: 42", "n", 42),
            ("f: 3.14", "f", 3.14),
            ("b: true", "b", True),
            ("s: hello", "s", "hello"),
        ],
        ids=["int", "float", "bool", "string"],
    )
    def test_safe_load_type(self, yaml_str, key, expected):
        data = pyrs_yaml.safe_load(yaml_str)
        assert data[key] == expected

    def test_safe_loads_multiple_docs(self):
        assert len(pyrs_yaml.safe_loads("a: 1\n---\nb: 2")) == 2

    @pytest.mark.parametrize(
        "data,expected",
        [
            ({"key": "value", "num": 42}, ["key: value", "42"]),
            ({"key": "value"}, ["key: value"]),
        ],
        ids=["multi", "single"],
    )
    def test_safe_dump_includes_data(self, data, expected):
        output = pyrs_yaml.safe_dump(data)
        for e in expected:
            assert e in output
