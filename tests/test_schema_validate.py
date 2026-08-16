"""Tests for schema structural validation (validate_against_schema)."""

import pytest

import pyrs_yaml

VALIDATE_SCHEMA = """\
name: app
extends: core
validate:
  - path: $.port
    type: int
    required: true
  - path: $.note
    required: true
  - path: $.tags[*]
    type: str
  - path: $.numbers
    sequence_of: int
  - path: $.config
    mapping_of: str
"""


class TestValidateAgainstSchema:
    def test_valid_document_passes(self):
        pyrs_yaml.validate_against_schema(
            "port: 80\nnote: hello\ntags: [a, b]\nnumbers: [1, 2]\nconfig: {k: v}\n",
            VALIDATE_SCHEMA,
        )

    def test_invalid_type_raises(self):
        with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
            pyrs_yaml.validate_against_schema("port: abc\nnote: hi\n", VALIDATE_SCHEMA)
        assert "expected int" in str(exc.value)
        assert "int" in str(exc.value)

    def test_required_missing_path_raises(self):
        with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
            pyrs_yaml.validate_against_schema("port: 80\n", VALIDATE_SCHEMA)
        assert "$.note" in str(exc.value)
        assert "required" in str(exc.value)

    def test_sequence_of_checks_elements(self):
        with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
            pyrs_yaml.validate_against_schema("port: 80\nnote: hi\nnumbers: [1, x]\n", VALIDATE_SCHEMA)
        assert "expected sequence element" in str(exc.value)

    def test_mapping_of_checks_values(self):
        with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
            pyrs_yaml.validate_against_schema("port: 80\nnote: hi\nconfig: {a: 5}\n", VALIDATE_SCHEMA)
        assert "expected mapping value" in str(exc.value)

    def test_wildcard_type_check(self):
        with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
            pyrs_yaml.validate_against_schema("port: 80\nnote: hi\ntags: [1, 2]\n", VALIDATE_SCHEMA)
        assert "expected str" in str(exc.value)

    def test_multiple_errors_reported(self):
        with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
            pyrs_yaml.validate_against_schema("port: abc\ntags: [1]\n", VALIDATE_SCHEMA)
        assert "required path is missing" in str(exc.value)  # missing required
        assert "expected int" in str(exc.value)  # bad type
        assert "expected str" in str(exc.value)  # bad element

    def test_no_validate_section_passes(self):
        schema = "name: plain\nextends: core\n"
        pyrs_yaml.validate_against_schema("any: thing\n", schema)

    def test_invalid_data_yaml_raises_parse_error(self):
        with pytest.raises((pyrs_yaml.YamlParseError, ValueError)):
            pyrs_yaml.validate_against_schema("not: valid: yaml: [[[\n", VALIDATE_SCHEMA)

    def test_invalid_schema_raises_parse_error(self):
        with pytest.raises((pyrs_yaml.YamlParseError, ValueError)):
            pyrs_yaml.validate_against_schema("a: 1\n", "rules: [invalid}")


class TestValidateNested:
    def test_nested_path_required(self):
        schema = """\
name: nested
extends: core
validate:
  - path: $.server.host
    type: str
    required: true
"""
        pyrs_yaml.validate_against_schema("server: {host: local}\n", schema)
        with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
            pyrs_yaml.validate_against_schema("server: {}\n", schema)
        assert "$.server.host" in str(exc.value)

    def test_nested_sequence_index_path(self):
        schema = """\
name: idx
extends: core
validate:
  - path: $.items[0]
    type: int
    required: true
"""
        pyrs_yaml.validate_against_schema("items: [1, 2]\n", schema)
        with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
            pyrs_yaml.validate_against_schema("items: [x]\n", schema)
        assert "expected int" in str(exc.value)
