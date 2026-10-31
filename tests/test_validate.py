"""Tests for YamlDocument.validate() and to_json()."""

import json
import sys

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


class TestTheDelegatedEvaluatorContract:
    """What `validate()` answers when the schema text is YAML, or when no evaluator is installed.

    `Document.validate` applies rules this repository does not own: `document.rs` imports `jsonschema`
    and forwards, and nothing this project publishes installs that package - `[project.dependencies]`
    is `typing-extensions` alone, the three extras are `cli`, `pydantic` and `settings`, and
    `jsonschema` sits in the test group. Both refusals below are therefore what an ordinary install
    reaches, and neither answer comes from the package's own exception family. They are registered
    rather than fixed here because each fix is a decision the tree has not taken: an extra is
    packaging, widening or wrapping the schema-text door is an error-contract change, and
    `docs/dev/json-schema-engine.md` designs the evaluator that would remove both.
    """

    def test_the_dict_and_json_text_doors_answer_in_the_documented_family(self):
        """The refusals below are about the spelling and the install, not about the rules."""
        schema = {"type": "object", "required": ["port"]}
        for spelling in (schema, json.dumps(schema)):
            pyrs_yaml.parse("port: 8080\n").validate(spelling)
            with pytest.raises(pyrs_yaml.YamlValidateError):
                pyrs_yaml.parse("other: 1\n").validate(spelling)

    @pytest.mark.known_residual("api:validate-schema-text-is-json-only")
    def test_a_yaml_spelled_schema_refuses_with_a_stdlib_json_error(self):
        doc = pyrs_yaml.parse("port: 8080\n")
        doc.validate({"type": "object", "required": ["port"]})
        # The same rules, spelled the way every other schema this package reads is spelled, reach
        # `json.loads` first and come back as a stdlib parse error - not a `YamlValidateError`.
        with pytest.raises(json.JSONDecodeError):
            doc.validate("type: object\nrequired: [port]\n")

    @pytest.mark.known_residual("api:validate-evaluator-is-not-installed-by-anything-we-ship")
    def test_a_base_install_answers_with_a_bare_module_not_found_error(self):
        # `sys.modules[...] = None` halts the import exactly as an absent distribution does, so this
        # runs on every leg whether or not the evaluator is installed here. The schema names nothing
        # the rest of the suite validates against, so the compiled-validator cache stays cold and the
        # call really reaches the import.
        held = sys.modules.get("jsonschema")
        sys.modules["jsonschema"] = None
        try:
            with pytest.raises(ModuleNotFoundError):
                pyrs_yaml.parse("port: 8080\n").validate({"title": "residual-validate-evaluator"})
        finally:
            if held is None:
                del sys.modules["jsonschema"]
            else:
                sys.modules["jsonschema"] = held
