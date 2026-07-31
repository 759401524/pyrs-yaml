"""Tests for YAML() instance API (v0.8.0 Phase 1)."""

import pyrs_yaml
import pytest


class TestYAMLInstance:
    """Test YAML() instance API with configurable type/schema/max_depth."""

    def test_parse_default(self, yaml_strings):
        doc = pyrs_yaml.YAML().parse(yaml_strings["simple_mapping"])
        assert isinstance(doc, pyrs_yaml.YamlDocument)
        assert doc.get("key") == "value"

    def test_parse_with_type(self, yaml_strings):
        doc = pyrs_yaml.YAML(typ="safe").parse(yaml_strings["nested_mapping"])
        assert doc.get("parent").get("child") == "grandchild"

    def test_parse_with_schema(self, yaml_strings):
        doc = pyrs_yaml.YAML(schema="json").parse(yaml_strings["simple_mapping"])
        assert doc.get("key") == "value"

    def test_safe_load(self, yaml_strings):
        result = pyrs_yaml.YAML().safe_load(yaml_strings["mixed"])
        assert isinstance(result, dict)
        assert result["name"] == "test"

    def test_safe_loads(self, yaml_strings):
        result = pyrs_yaml.YAML().safe_loads(yaml_strings["sequence"])
        assert isinstance(result, list)
        assert len(result) == 1
        assert result[0] == ["a", "b", "c"]

    def test_parse_file(self, temp_yaml_file):
        doc = pyrs_yaml.YAML().parse_file(temp_yaml_file)
        assert doc.get("key") == "value"

    def test_parse_all_docs(self):
        docs = pyrs_yaml.YAML().parse_all_docs("---\na: 1\n---\nb: 2")
        assert len(docs) == 2
        assert docs[0].get("a") == 1
        assert docs[1].get("b") == 2

    def test_invalid_type(self):
        with pytest.raises(pyrs_yaml.YamlTypeError):
            pyrs_yaml.YAML(typ="invalid")

    def test_invalid_schema(self):
        with pytest.raises(pyrs_yaml.YamlTypeError):
            pyrs_yaml.YAML(schema="invalid")

    def test_parse_max_depth(self, yaml_strings):
        doc = pyrs_yaml.YAML(max_depth=1).parse(yaml_strings["simple_mapping"])
        assert doc.get("key") == "value"

    def test_parse_bytes(self):
        doc = pyrs_yaml.YAML().parse(b"key: value")
        assert doc.get("key") == "value"

    def test_parse_schema_yaml11(self):
        doc = pyrs_yaml.YAML(schema="yaml1.1").parse("bool: yes")
        assert doc.get("bool") is True

    def test_parse_schema_failsafe(self):
        doc = pyrs_yaml.YAML(schema="failsafe").parse("bool: yes")
        assert doc.get("bool") == "yes"
