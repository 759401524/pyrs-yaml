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

    def test_rule_path_with_a_multibyte_key_fires_on_the_right_node(self):
        """A non-ASCII rule path used to abort the process, and must still name the node it matched.

        `rule_path_to_segments` advanced its cursor one byte at a time, so a path like `$.café` stopped inside
        the multi-byte key and the next slice panicked with "start byte index is not a char boundary" - through
        the public API, on a valid schema. The passing case proves no panic; the raising case proves the path
        actually navigated, because a walk that silently matched nothing would also "pass".
        """
        schema = """name: multibyte
extends: core
validate:
  - path: $.café
    type: str
    required: true
"""
        pyrs_yaml.validate_against_schema("café: crème\n", schema)
        with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
            pyrs_yaml.validate_against_schema("café: 5\n", schema)
        assert "expected str" in str(exc.value)
        # A rule that matched nothing would also "pass" the case above, so prove the path selects the key and
        # not just any node: the same document with the wrong type under an ASCII key must not fire the rule.
        pyrs_yaml.validate_against_schema("café: crème\nother: 5\n", schema)

        emoji = """name: emoji
extends: core
validate:
  - path: $.emoji\U0001f600key
    type: int
"""
        pyrs_yaml.validate_against_schema("emoji\U0001f600key: 7\n", emoji)
        with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
            pyrs_yaml.validate_against_schema("emoji\U0001f600key: seven\n", emoji)
        assert "expected int" in str(exc.value)

    def test_the_root_document_can_be_a_rule_target(self):
        """`path: $` addresses the document itself.

        The parser had a branch for exactly that path, but a separator was required before the emptiness was
        tested, so a root rule could never be reached - and `$x` must stay unparseable rather than be read as
        a key. The element paths it produces (`$.a`) are what show the root was walked, not skipped.
        """
        schema = """name: root
extends: core
validate:
  - path: $
    mapping_of: int
"""
        pyrs_yaml.validate_against_schema("a: 1\nb: 2\n", schema)
        with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
            pyrs_yaml.validate_against_schema("a: 1\nb: two\n", schema)
        # The wording names the mapping-value check, which only runs once the rule has been applied to the
        # document itself - a root rule that never resolved would leave this call silently passing.
        assert "expected mapping value int" in str(exc.value)

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
