"""The inline dict schema route must carry the whole definition, not the parts the emitter knew about.

`safe_load(schema={...})`, `YAML(schema={...})` and friends serialize the dict to YAML and register it, which is
how a schema written as Python data reaches the engine. Two failures came from the hand-rolled serializer that did
that, both measured here before being fixed: a `validate` section was dropped without a word (so the registered
schema validated nothing, and the caller had no way to notice), and a pattern containing both quote characters was
emitted as a double-quoted scalar with an unescaped `"` inside it, which the parser then refused.
"""

import pytest

import pyrs_yaml

HEX_RULE = {"pattern": r"^0x[0-9a-fA-F]+$", "type": "int"}


class TestWholeDefinitionSurvives:
    def test_an_inline_dict_keeps_its_validate_section(self):
        """The rules a dict carries are the rules the registered name enforces.

        This is the assertion that catches the silent loss: a dropped `validate` section makes both raising calls
        return cleanly, because a schema with no rules accepts every document.
        """
        schema = {
            "extends": "core",
            "rules": [HEX_RULE],
            "validate": [{"path": "$.port", "type": "int", "required": True}],
        }
        text = pyrs_yaml._schema_to_yaml(schema)
        pyrs_yaml.register_schema("inline_validate_probe", text)

        pyrs_yaml.validate_against_schema("port: 80\n", "inline_validate_probe")
        with pytest.raises(pyrs_yaml.YamlValidateError) as wrong_type:
            pyrs_yaml.validate_against_schema("port: eighty\n", "inline_validate_probe")
        assert "$.port: expected int" in str(wrong_type.value)
        with pytest.raises(pyrs_yaml.YamlValidateError) as missing:
            pyrs_yaml.validate_against_schema("other: 1\n", "inline_validate_probe")
        assert "required path is missing" in str(missing.value)

    def test_every_key_of_the_definition_is_written(self):
        text = pyrs_yaml._schema_to_yaml(
            {
                "name": "probe",
                "version": 1,
                "extends": "failsafe",
                "rules": [HEX_RULE],
                "validate": [{"path": "$.port", "type": "int"}],
            }
        )
        for key in ("name:", "version:", "extends:", "rules:", "validate:"):
            assert key in text, key
        assert "extends: failsafe" in text, text

    def test_a_dict_resolves_the_same_as_the_equivalent_yaml_text(self):
        schema = {"rules": [HEX_RULE]}
        through_dict = pyrs_yaml.safe_load("addr: 0xFF\nplain: 12", schema=schema)
        name = pyrs_yaml._coerce_schema(schema)
        through_text = pyrs_yaml.safe_load("addr: 0xFF\nplain: 12", schema=name)
        assert through_dict == through_text == {"addr": 255, "plain": 12}

    def test_extends_still_defaults_to_core(self):
        # The line emitter wrote `extends: core` when the dict omitted it; the serializer must not lose that.
        text = pyrs_yaml._schema_to_yaml({"rules": [HEX_RULE]})
        assert "extends: core" in text, text
        assert pyrs_yaml.safe_load("addr: 0x10", schema={"rules": [HEX_RULE]}) == {"addr": 16}


class TestQuotingIsFaithful:
    def test_a_pattern_with_both_quote_characters_survives(self):
        """The old rule wrapped a value in double quotes whenever it held an apostrophe - and left the `"` inside
        it unescaped, which the schema parser then rejected."""
        pattern = """^["']?[0-9]+["']?$"""
        schema = {"rules": [{"pattern": pattern, "type": "int"}]}

        # the rule matches, so 12 resolves to an int and the definition parses at all
        assert pyrs_yaml.safe_load("n: 12", schema=schema) == {"n": 12}
        # a quoted scalar is a string under every schema, which is the other half of the rule working
        assert pyrs_yaml.safe_load("n: '12'", schema=schema) == {"n": "12"}
        assert pattern in pyrs_yaml._schema_to_yaml(schema)

    def test_the_old_quoting_produced_text_the_parser_rejects(self):
        """Reconstructing the previous output keeps the contrast honest: this is not a hypothetical."""
        old_form = "extends: core\nrules:\n  - pattern: \"^[\"']?[0-9]+[\"']?$\"\n    type: 'int'\n"
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.register_schema("old_form_probe", old_form)
