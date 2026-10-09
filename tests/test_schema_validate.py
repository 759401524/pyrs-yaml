"""Tests for schema structural validation (validate_against_schema)."""

import re

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


class TestShapeAssertions:
    """A rule that names a node asserts what that node is.

    Measured against the previous build before the change: every document below passed, because each check sat
    inside an `if let` for the single node kind the rule could describe, and a node of any other kind was walked
    past. These are the tests for the ruling that closed the ROADMAP's open item.
    """

    @staticmethod
    def schema(rule_body: str) -> str:
        return "name: shape\nextends: core\nvalidate:\n" + rule_body

    def test_a_container_rule_asserts_the_container(self):
        schema = self.schema("  - path: $.config\n    mapping_of: str\n")
        pyrs_yaml.validate_against_schema("config: {a: x, b: y}\n", schema)
        for document in ("config: hello\n", "config: [a, b]\n", "config: 5\n"):
            with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
                pyrs_yaml.validate_against_schema(document, schema)
            assert "$.config: expected mapping of str but got" in str(exc.value)

        schema = self.schema("  - path: $.numbers\n    sequence_of: int\n")
        pyrs_yaml.validate_against_schema("numbers: [1, 2]\n", schema)
        with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
            pyrs_yaml.validate_against_schema("numbers: {a: 1}\n", schema)
        assert "$.numbers: expected sequence of int but got mapping" in str(exc.value)

    def test_a_type_rule_asserts_that_the_named_node_is_a_scalar(self):
        schema = self.schema("  - path: $.port\n    type: int\n")
        pyrs_yaml.validate_against_schema("port: 80\n", schema)
        with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
            pyrs_yaml.validate_against_schema("port: {a: 1}\n", schema)
        assert "$.port: expected int but got mapping" in str(exc.value)
        with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
            pyrs_yaml.validate_against_schema("port: [1]\n", schema)
        assert "$.port: expected int but got sequence" in str(exc.value)

    def test_map_and_seq_are_types_the_language_names(self):
        # Shape without a claim about the contents had no spelling at all before this.
        schema = self.schema("  - path: $.config\n    type: map\n")
        pyrs_yaml.validate_against_schema("config: {}\n", schema)
        pyrs_yaml.validate_against_schema("config: {a: [1, {deep: x}]}\n", schema)
        with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
            pyrs_yaml.validate_against_schema("config: [1]\n", schema)
        assert "$.config: expected map but got sequence" in str(exc.value)

        # The long names and the ordinary synonyms mean the same claim, because a schema
        # is typed by a person rather than emitted by a tool.
        for spelling in ("mapping", "object"):
            pyrs_yaml.validate_against_schema("x: {a: 1}\n", self.schema(f"  - path: $.x\n    type: {spelling}\n"))
        for spelling in ("sequence", "array", "list"):
            pyrs_yaml.validate_against_schema("x: [1]\n", self.schema(f"  - path: $.x\n    type: {spelling}\n"))

    def test_a_member_can_be_asked_for_its_shape(self):
        # "every value of outer must itself be a sequence" was unwriteable: `[*]` reaches
        # sequence indices, not mapping keys.
        schema = self.schema("  - path: $.outer\n    mapping_of: seq\n")
        pyrs_yaml.validate_against_schema("outer:\n  a: [1]\n  b: [2]\n", schema)
        with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
            pyrs_yaml.validate_against_schema("outer:\n  a: 1\n", schema)
        assert "$.outer.a: expected mapping value seq but got scalar" in str(exc.value)

    def test_a_nested_member_is_not_a_free_pass(self):
        # The same false negative one level down: a container among the members is not the
        # scalar type the rule named.
        with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
            pyrs_yaml.validate_against_schema(
                "n:\n  - 1\n  - [2, 3]\n", self.schema("  - path: $.n\n    sequence_of: int\n")
            )
        assert "$.n[1]: expected sequence element int but got sequence" in str(exc.value)
        with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
            pyrs_yaml.validate_against_schema(
                "c:\n  a: x\n  b:\n    deep: y\n", self.schema("  - path: $.c\n    mapping_of: str\n")
            )
        assert "$.c.b: expected mapping value str but got mapping" in str(exc.value)

    def test_a_wildcard_names_one_element_and_not_a_subtree(self):
        schema = self.schema("  - path: $.rows[*]\n    type: map\n")
        pyrs_yaml.validate_against_schema("rows:\n  - a: 1\n  - b: x\n", schema)
        with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
            pyrs_yaml.validate_against_schema("rows:\n  - [{a: 1}]\n", schema)
        assert "$.rows[0]: expected map but got sequence" in str(exc.value)
        # A suffix after the wildcard still reaches the member it names.
        nested = self.schema("  - path: $.rows[*].a\n    type: int\n")
        pyrs_yaml.validate_against_schema("rows:\n  - a: 1\n  - a: 2\n", nested)
        with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
            pyrs_yaml.validate_against_schema("rows:\n  - a: one\n", nested)
        assert '$.rows[0].a: expected int but got Str("one")' in str(exc.value)
        # A located error still names the path it matched, which is the half of the message
        # a schema author reads first.
        assert re.match(r"^\d+:\d+: \$\.rows\[0\]\.a: ", str(exc.value)), str(exc.value)

    def test_a_pathless_rule_selects_by_shape_instead_of_asserting_it(self):
        # Pathless `type:` has always meant "every scalar resolves to this", and a document
        # with nested structure has to be allowed to satisfy that.
        pyrs_yaml.validate_against_schema("top: scalar\nnested:\n  deep: also-scalar\n", self.schema("  - type: str\n"))
        # Pathless container rules still report members, which is a claim about members
        # rather than about the shape of a node they never named.
        with pytest.raises(pyrs_yaml.YamlValidateError) as exc:
            pyrs_yaml.validate_against_schema("plain: 5\nmapped: {k: v}\n", self.schema("  - mapping_of: str\n"))
        assert "$.mapped: expected mapping value str but got mapping" in str(exc.value)

    def test_a_pathless_container_type_is_a_schema_error(self):
        # A rule that can check nothing is the defect class this ruling removes, so it is
        # refused where it is written instead of accepted as a silent pass.
        for spelling in ("map", "seq", "object", "array"):
            with pytest.raises((pyrs_yaml.YamlParseError, ValueError)) as exc:
                pyrs_yaml.validate_against_schema("a: 1\n", self.schema(f"  - type: {spelling}\n"))
            assert "needs a path" in str(exc.value)

    def test_a_rule_cannot_carry_two_checks(self):
        # Every arm of the parser overwrote the previous one, so a rule with two checks used
        # to run only the last and say nothing about the first intent.
        for body in (
            "  - path: $.x\n    type: int\n    mapping_of: str\n",
            "  - path: $.x\n    sequence_of: int\n    type: seq\n",
        ):
            with pytest.raises((pyrs_yaml.YamlParseError, ValueError)) as exc:
                pyrs_yaml.validate_against_schema("x: 1\n", self.schema(body))
            assert "one check" in str(exc.value)
        # `required` combines with a check rather than competing with it.
        pyrs_yaml.validate_against_schema("x: 1\n", self.schema("  - path: $.x\n    type: int\n    required: true\n"))

    def test_an_alias_is_passed_rather_than_guessed(self):
        # An alias node does not carry the value it names and the validator holds no anchor
        # table, so the shape cannot be decided. Recorded as a boundary: a future
        # alias-aware validator has to break this test on purpose.
        schema = self.schema("  - path: $.b\n    type: int\n")
        pyrs_yaml.validate_against_schema("a: &x 5\nb: *x\n", schema)
        pyrs_yaml.validate_against_schema("a: &x hi\nb: *x\n", schema)
        pyrs_yaml.validate_against_schema("a: &x hi\nb: *x\n", self.schema("  - path: $.b\n    type: map\n"))
