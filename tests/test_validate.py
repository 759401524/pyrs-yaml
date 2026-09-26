"""Tests for YamlDocument.validate() and to_json()."""

import json

import pytest

import pyrs_yaml


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


class TestValidateCacheSemantics:
    """The compiled-validator cache must never change observable behavior."""

    def test_repeated_dict_schema_still_validates(self):
        schema = {"type": "object", "required": ["a"]}
        doc = pyrs_yaml.parse("a: 1")
        doc.validate(schema)  # miss: full path, warms cache
        doc.validate(schema)  # hit: cached validator
        with pytest.raises(pyrs_yaml.YamlValidateError):
            pyrs_yaml.parse("b: 2").validate(schema)  # failure via hit path

    def test_mutated_dict_schema_invalidates_cache(self):
        schema = {"type": "object", "properties": {"n": {"type": "string"}}}
        pyrs_yaml.parse("n: ok").validate(schema)  # cache under old content
        schema["properties"]["n"]["type"] = "integer"  # in-place mutation
        with pytest.raises(pyrs_yaml.YamlValidateError):
            pyrs_yaml.parse("n: text").validate(schema)

    def test_str_schema_hit_reports_same_message_as_jsonschema(self):
        schema = json.dumps({"type": "object", "properties": {"n": {"type": "integer"}}, "required": ["n"]})
        pyrs_yaml.parse("n: 5").validate(schema)  # warm cache
        with pytest.raises(pyrs_yaml.YamlValidateError) as ours:
            pyrs_yaml.parse("n: x").validate(schema)  # cached path
        assert "is not of type 'integer'" in str(ours.value)

    def test_distinct_equal_dicts_are_cached_independently(self):
        # Two equal-but-distinct dict objects validate the same instance:
        # each gets its own identity-keyed entry, and a later failure on
        # either still raises through its cached validator.
        s1 = {"type": "object", "properties": {"a": {"type": "integer"}}}
        s2 = {"type": "object", "properties": {"a": {"type": "integer"}}}
        doc = pyrs_yaml.parse("a: 1")
        doc.validate(s1)  # caches under id(s1)
        doc.validate(s2)  # equal but distinct object: caches under id(s2)
        with pytest.raises(pyrs_yaml.YamlValidateError):
            pyrs_yaml.parse("a: text").validate(s1)
        with pytest.raises(pyrs_yaml.YamlValidateError):
            pyrs_yaml.parse("a: text").validate(s2)
