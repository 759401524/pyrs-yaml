"""
JSON/YAML conversion tests — from_dict, from_json.
"""

import pytest

import pyyaml_rs

# ============================================================================
# From Dict
# ============================================================================

class TestFromDict:
    """Test from_dict function"""

    def test_from_dict_simple(self):
        data = {"name": "John", "age": 30}
        yaml_str = pyyaml_rs.from_dict(data)
        assert "name: John" in yaml_str
        assert "30" in yaml_str

    def test_from_dict_nested(self):
        data = {"app": {"name": "myapp", "version": "1.0"}}
        yaml_str = pyyaml_rs.from_dict(data)
        assert "app:" in yaml_str
        assert "name: myapp" in yaml_str

    def test_from_dict_list(self):
        data = {"items": [1, 2, 3]}
        yaml_str = pyyaml_rs.from_dict(data)
        assert "- 1" in yaml_str
        assert "- 2" in yaml_str


# ============================================================================
# From JSON
# ============================================================================

class TestFromJson:
    """Test from_json function"""

    def test_from_json_simple(self):
        json_str = '{"name": "Alice", "active": true}'
        yaml_str = pyyaml_rs.from_json(json_str)
        assert "name: Alice" in yaml_str
        assert "active: true" in yaml_str

    def test_from_json_nested(self):
        json_str = '{"db": {"host": "localhost", "port": 5432}}'
        yaml_str = pyyaml_rs.from_json(json_str)
        assert "db:" in yaml_str
        assert "host: localhost" in yaml_str

    def test_from_json_array(self):
        json_str = '{"items": [1, 2, 3]}'
        yaml_str = pyyaml_rs.from_json(json_str)
        assert "- 1" in yaml_str

    def test_from_json_invalid(self):
        """Invalid JSON raises YamlParseError."""
        with pytest.raises(pyyaml_rs.YamlParseError):
            pyyaml_rs.from_json("{invalid json}")
