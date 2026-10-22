"""The `rules:` schema language's scalar-type resolution, judged by `jsonschema`.

The `rules:` grammar was invented in this repository, so the unit, property and fuzz tiers all
ask this engine what it believes: `type: int` resolves `0x1F` to `31` because the resolver in
`crates/pyrs-yaml-core/src/parser/yaml/schema_language.rs` says so. No outside reader could
contradict that — until now. This file asks each `type:` name the grammar accepts a question it
does not own: *is the value the engine produced really an instance of the JSON Schema type that
name denotes?* `jsonschema` owns that membership, and it can say no.

The membership judgment is where the two grammars must agree and can diverge. JSON Schema keeps
`boolean` and `integer` disjoint even though Python's `bool` is a subclass of `int` — so a rule
that tagged `yes` as `integer`, or `0x1F` as `boolean`, would still resolve to a Python value here
while `jsonschema` rejects it under that type. The decoy in every row is a standard type the same
value must NOT satisfy, so the authority is not a rubber stamp: it actually refuses.

Alias spellings (`integer`, `boolean`, `double`, `string`) are exercised too, because `from_name`
maps them onto the same kinds, and the standard type the value lands in must not depend on which
spelling the author wrote. The companion test below checks the other half of "type parsing": a name
that is in neither vocabulary is refused by both this grammar and `jsonschema`.
"""

from __future__ import annotations

import jsonschema
import pytest

import pyrs_yaml

#: (the `type:` name as written in `rules:`, its pattern, a scalar the pattern matches, the JSON
#: Schema type the resolved value MUST be an instance of, one it MUST NOT be).
TYPE_ROWS = [
    pytest.param("int", r"^0x[0-9a-fA-F]+$", "0x1F", "integer", "string", id="int-hex"),
    pytest.param("integer", r"^\d+$", "80", "integer", "boolean", id="integer-alias"),
    pytest.param("bool", r"^(yes|no)$", "yes", "boolean", "integer", id="bool-true"),
    pytest.param("boolean", r"^(yes|no)$", "no", "boolean", "number", id="boolean-alias"),
    pytest.param("float", r"^\d+\.\d+$", "2.5", "number", "integer", id="float-decimal"),
    pytest.param("double", r"^\d+\.\d+$", "2.5", "number", "string", id="double-alias"),
    pytest.param("str", r"^\d+$", "42", "string", "integer", id="str-digits"),
    pytest.param("string", r"^[a-z]+$", "hello", "string", "null", id="string-alias"),
    pytest.param("null", r"^~$", "~", "null", "boolean", id="null-tilde"),
]

#: Type names that are neither in this grammar nor in the JSON Schema vocabulary.
NOT_A_TYPE = ["datetime", "map", "sequence"]


def _resolve(rule_type: str, pattern: str, scalar: str) -> object:
    """The value the `rules:` grammar produces for ``scalar`` under a single ``pattern``/``type`` rule."""
    schema = f"rules:\n  - pattern: '{pattern}'\n    type: {rule_type}\n"
    pyrs_yaml.register_schema("authority_probe", schema)
    return pyrs_yaml.safe_load(f"v: {scalar}\n", schema="authority_probe")["v"]


@pytest.mark.parametrize("rule_type,pattern,scalar,member,decoy", TYPE_ROWS)
def test_a_rule_type_names_the_json_schema_type_it_claims(
    rule_type: str, pattern: str, scalar: str, member: str, decoy: str
) -> None:
    """The value a `type:` name resolves to must really be of that name's standard type, judged by `jsonschema`."""
    value = _resolve(rule_type, pattern, scalar)
    # The authority confirms the type the grammar claims ...
    jsonschema.validate(value, {"type": member})
    # ... and refuses a type it does not, so the confirmation is not vacuous.
    with pytest.raises(jsonschema.ValidationError):
        jsonschema.validate(value, {"type": decoy})


@pytest.mark.parametrize("name", NOT_A_TYPE)
def test_a_name_outside_both_vocabularies_is_refused_by_both(name: str) -> None:
    """Neither grammar treats the name as a type: this engine rejects the rule, `jsonschema` rejects the schema."""
    schema = f"rules:\n  - pattern: '^x$'\n    type: {name}\n"
    with pytest.raises(pyrs_yaml.YamlParseError):
        pyrs_yaml.register_schema("authority_probe_bad", schema)
    with pytest.raises(jsonschema.exceptions.SchemaError):
        jsonschema.validate(1, {"type": name})
