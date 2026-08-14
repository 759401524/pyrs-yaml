"""Tests for YAML() instance API (v0.8.0 Phase 1)."""

import pytest

import pyrs_yaml


class TestYAMLInstance:
    """Test YAML() instance API with configurable type/schema/max_depth."""

    @pytest.mark.parametrize(
        "typ,schema",
        [
            ("rt", "core"),
            ("safe", "core"),
            ("rt", "json"),
        ],
        ids=["default", "safe-type", "json-schema"],
    )
    def test_parse_variants(self, yaml_strings, typ, schema):
        doc = pyrs_yaml.YAML(typ=typ, schema=schema).parse(yaml_strings["simple_mapping"])
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

    @pytest.mark.parametrize(
        "param,value",
        [
            ("typ", "invalid"),
            ("schema", "invalid"),
        ],
        ids=["invalid-type", "invalid-schema"],
    )
    def test_invalid_param(self, param, value):
        kwargs = {param: value}
        with pytest.raises(pyrs_yaml.YamlTypeError):
            pyrs_yaml.YAML(**kwargs)

    def test_parse_max_depth(self, yaml_strings):
        doc = pyrs_yaml.YAML(max_depth=1).parse(yaml_strings["simple_mapping"])
        assert doc.get("key") == "value"

    def test_parse_bytes(self):
        doc = pyrs_yaml.YAML().parse(b"key: value")
        assert doc.get("key") == "value"

    def test_resolve_merges_by_yaml_type(self):
        # yaml_type rt/full resolve << merge keys; safe preserves them literally.
        merge_yaml = "base: &b\n  x: 1\nchild:\n  <<: *b\n  y: 2\n"
        for typ, expected in [
            ("rt", {"x": 1, "y": 2}),
            ("full", {"x": 1, "y": 2}),
            ("safe", {"<<": {"x": 1}, "y": 2}),
        ]:
            child = pyrs_yaml.YAML(typ=typ).parse(merge_yaml).to_dict()["child"]
            assert child == expected, f"YAML({typ}) child={child}"

    @pytest.mark.parametrize(
        "schema,value",
        [
            ("yaml1.1", True),
            ("failsafe", "yes"),
        ],
        ids=["yaml11-schema", "failsafe-schema"],
    )
    def test_parse_schema_variants(self, schema, value):
        doc = pyrs_yaml.YAML(schema=schema).parse("bool: yes")
        assert doc.get("bool") == value
