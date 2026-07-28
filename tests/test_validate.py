"""Tests for YamlDocument.validate() and to_json()."""

import json

import pyrs_yaml


class TestToJson:
    def test_to_json_simple(self):
        doc = pyrs_yaml.parse("a: 1\nb: hello")
        result = doc.to_json()
        data = json.loads(result)
        assert data == {"a": 1, "b": "hello"}

    def test_to_json_indent(self):
        doc = pyrs_yaml.parse("x: y")
        s = doc.to_json(indent=4)
        assert "    " in s


class TestValidateValid:
    def test_validate_simple_dict(self):
        doc = pyrs_yaml.parse("name: Alice\nage: 30")
        schema = {"type": "object", "properties": {"name": {"type": "string"}, "age": {"type": "integer"}}}
        doc.validate(schema)  # should not raise

    def test_validate_array(self):
        doc = pyrs_yaml.parse("- 1\n- 2\n- 3")
        schema = {"type": "array", "items": {"type": "integer"}}
        doc.validate(schema)

    def test_validate_from_json_string(self):
        doc = pyrs_yaml.parse("value: hello")
        schema_json = '{"type": "object", "properties": {"value": {"type": "string"}}}'
        doc.validate(schema_json)

    def test_validate_required(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2")
        schema = {"type": "object", "required": ["a", "b"]}
        doc.validate(schema)

    def test_validate_string_scalar(self):
        doc = pyrs_yaml.parse("hello world")
        schema = {"type": "string"}
        doc.validate(schema)

    def test_validate_integer(self):
        doc = pyrs_yaml.parse("42")
        schema = {"type": "integer"}
        doc.validate(schema)

    def test_validate_nested(self):
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
    def test_validate_type_mismatch(self):
        doc = pyrs_yaml.parse("name: 123")
        schema = {"type": "object", "properties": {"name": {"type": "string"}}}
        try:
            doc.validate(schema)
            raise AssertionError("should have raised")
        except pyrs_yaml.YamlValidateError as e:
            assert "string" in str(e).lower()

    def test_validate_missing_required(self):
        doc = pyrs_yaml.parse("a: 1")
        schema = {"type": "object", "required": ["a", "b"]}
        try:
            doc.validate(schema)
            raise AssertionError("should have raised")
        except pyrs_yaml.YamlValidateError:
            pass

    def test_validate_array_type_mismatch(self):
        doc = pyrs_yaml.parse("- hello\n- world")
        schema = {"type": "array", "items": {"type": "integer"}}
        try:
            doc.validate(schema)
            raise AssertionError("should have raised")
        except pyrs_yaml.YamlValidateError:
            pass

    def test_validate_min_properties(self):
        doc = pyrs_yaml.parse("a: 1")
        schema = {"type": "object", "minProperties": 2}
        try:
            doc.validate(schema)
            raise AssertionError("should have raised")
        except pyrs_yaml.YamlValidateError:
            pass


class TestYamlValidateError:
    def test_is_value_error(self):
        assert issubclass(pyrs_yaml.YamlValidateError, ValueError)
