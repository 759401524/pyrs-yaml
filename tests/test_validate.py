"""Tests for YamlDocument.validate() and to_json()."""

import json

import pyrs_yaml
import pytest


class TestToJson:
    def test_converts_document_to_json(self):
        data = json.loads(pyrs_yaml.parse("a: 1\nb: hello").to_json())
        assert data == {"a": 1, "b": "hello"}

    def test_to_json_respects_indent(self):
        assert "    " in pyrs_yaml.parse("x: y").to_json(indent=4)


class TestValidateValid:
    @pytest.mark.parametrize(
        "yaml_str,schema",
        [
            (
                "name: Alice\nage: 30",
                {"type": "object", "properties": {"name": {"type": "string"}, "age": {"type": "integer"}}},
            ),
            ("- 1\n- 2\n- 3", {"type": "array", "items": {"type": "integer"}}),
            ("value: hello", '{"type": "object", "properties": {"value": {"type": "string"}}}'),
            ("a: 1\nb: 2", {"type": "object", "required": ["a", "b"]}),
            ("hello world", {"type": "string"}),
            ("42", {"type": "integer"}),
        ],
        ids=["object", "array", "json-string", "required", "string-scalar", "integer-scalar"],
    )
    def test_validates_matching_schema(self, yaml_str, schema):
        pyrs_yaml.parse(yaml_str).validate(schema)

    def test_validates_nested_schema(self):
        doc = pyrs_yaml.parse("user:\n  name: Bob\n  roles:\n    - admin\n    - user")
        schema = {
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "roles": {"type": "array", "items": {"type": "string"}},
                    },
                }
            },
        }
        doc.validate(schema)


class TestValidateInvalid:
    def test_rejects_type_mismatch(self):
        with pytest.raises(pyrs_yaml.YamlValidateError, match="string"):
            pyrs_yaml.parse("name: 123").validate({"type": "object", "properties": {"name": {"type": "string"}}})

    def test_rejects_missing_required(self):
        with pytest.raises(pyrs_yaml.YamlValidateError):
            pyrs_yaml.parse("a: 1").validate({"type": "object", "required": ["a", "b"]})

    def test_rejects_array_type_mismatch(self):
        with pytest.raises(pyrs_yaml.YamlValidateError):
            pyrs_yaml.parse("- hello\n- world").validate({"type": "array", "items": {"type": "integer"}})

    def test_rejects_min_properties_violation(self):
        with pytest.raises(pyrs_yaml.YamlValidateError):
            pyrs_yaml.parse("a: 1").validate({"type": "object", "minProperties": 2})


class TestYamlValidateError:
    def test_is_subclass_of_value_error(self):
        assert issubclass(pyrs_yaml.YamlValidateError, ValueError)
