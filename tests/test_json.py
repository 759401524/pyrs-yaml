"""JSON/YAML conversion tests — from_dict, from_json."""

import pytest

import pyrs_yaml


class TestFromDict:
    """Test from_dict function"""

    @pytest.mark.parametrize(
        "data,checks",
        [
            ({"name": "John", "age": 30}, ["name: John", "30"]),
            ({"app": {"name": "myapp", "version": "1.0"}}, ["app:", "name: myapp"]),
            ({"items": [1, 2, 3]}, ["- 1", "- 2"]),
        ],
        ids=["simple", "nested", "list"],
    )
    def test_converts_dict_to_yaml(self, data, checks):
        yaml_str = pyrs_yaml.from_dict(data)
        for check in checks:
            assert check in yaml_str


class TestFromJson:
    """Test from_json function"""

    @pytest.mark.parametrize(
        "json_str,checks",
        [
            ('{"name": "Alice", "active": true}', ["name: Alice", "active: true"]),
            ('{"db": {"host": "localhost", "port": 5432}}', ["db:", "host: localhost"]),
            ('{"items": [1, 2, 3]}', ["- 1"]),
        ],
        ids=["simple", "nested", "array"],
    )
    def test_converts_json_to_yaml(self, json_str, checks):
        yaml_str = pyrs_yaml.from_json(json_str)
        for check in checks:
            assert check in yaml_str

    def test_rejects_invalid_json(self):
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.from_json("{invalid json}")
