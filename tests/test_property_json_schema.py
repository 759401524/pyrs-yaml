"""The JSON Schema validation surface, property-tested against the reference engine.

`Document.validate` delegates the rules to ``jsonschema`` and owns three things itself: the document
becoming the Python instance the engine sees (``to_dict``, including anchor and merge resolution),
schema acceptance (``dict`` or JSON text) plus the compiled-validator cache, and the
``YamlValidateError`` contract. ``tests/test_document_validate_suite.py`` judges all three against
the official draft2020-12 corpus, and two shapes are outside that corpus by construction:

* **the cache**. A corpus case validates one document against one schema object and throws both
  away, so every case is a cold miss. The cached path answers through
  ``best_match(validator.iter_errors(...))`` instead of ``jsonschema.validate``, and only a second
  call with the same schema object reaches it - what anyone validating in a loop does.
  ``test_validate.py`` pins that route one call site at a time; nothing generated reached it,
  because nothing generated was a sequence of documents against one schema.
* **the instance**. A corpus case enters through ``json.dumps``, so every mapping key it hands the
  engine is a string. A YAML document read directly keeps its key types - ``1: a`` becomes
  ``{1: 'a'}`` - and JSON Schema's data model has no member name that is not a string.

Both shapes are generated here, over a schema grammar and a document grammar no fixed case list
names. Drawing key spellings at random is what surfaced the two boundaries this module registers:
the dict door folds a key pair the document writes apart, and the two YAML readers answer opposite
ways on two adjacent pairs.
"""

from __future__ import annotations

import json
from typing import Any

import jsonschema
import pytest
import yaml as pyyaml
from hypothesis import HealthCheck, given, settings
from hypothesis import strategies as st
from jsonschema.validators import Draft202012Validator

import pyrs_yaml

pytestmark = pytest.mark.slow

#: The type names JSON Schema 2020-12 defines. Kept to the spec spellings so a generated schema is
#: always meta-valid: an invalid schema would judge `check_schema`, not the surface this tier owns.
_TYPE_NAMES = ["object", "array", "string", "number", "integer", "boolean", "null"]

#: Key spellings a generated YAML mapping may carry. `a`/`b`/`key` stay strings; the rest resolve to
#: an int, float, bool or null key under the Core schema, which is the shape the corpus cannot reach.
_KEY_SPELLINGS = [
    "a",
    "b",
    "key",
    "two words",
    "1",
    "2",
    "-3",
    "1.5",
    "true",
    "false",
    "null",
    "2020-01-02",
    "0o17",
]

_SCALAR_SPELLINGS = ["1", "2", "-3", "1.5", "true", "false", "null", "text", "'7'", '"x y"', "2020-01-02"]

#: Pattern keywords with a grammar Python's `re` can run. The corpus excludes the ECMAScript `\p{...}`
#: forms for the same reason (`EXCLUDED_CASES` in the suite file).
_PATTERNS = ["^a", "b$", "^[0-9]+$"]


def _resolved_key(spelling: str) -> Any:
    """What one key spelling becomes as a Python dict key."""
    return next(iter(pyrs_yaml.parse(f"{spelling}: 1\n").to_dict()))


#: Spellings grouped by the Python key they resolve to, so a generated mapping can be asked for keys
#: from distinct groups and hold no fold: `1` and `true` share a group because `1 == True`, and a
#: Python dict carries only one of them.
_KEY_GROUPS: list[list[str]] = []
for _spelling in _KEY_SPELLINGS:
    _key = _resolved_key(_spelling)
    for _group in _KEY_GROUPS:
        if _resolved_key(_group[0]) == _key:
            _group.append(_spelling)
            break
    else:
        _KEY_GROUPS.append([_spelling])

#: A pair the document writes as two keys and the dict carries as one - the fold this tier found.
_FOLD_PAIR = next(group for group in _KEY_GROUPS if len(group) > 1)

#: Spellings whose key is already a string, for the member names a generated schema asks about.
_STRING_KEY_SPELLINGS = [group[0] for group in _KEY_GROUPS if isinstance(_resolved_key(group[0]), str)]


def _json_schema(max_leaves: int = 6) -> st.SearchStrategy[Any]:
    """A meta-valid draft2020-12 schema, drawn from a grammar instead of a list.

    Boolean schemas are included because 2020-12 allows them and the acceptance path takes
    `str | dict`, so a `bool` reaching the engine is a form no corpus case hands it.
    """
    leaf: st.SearchStrategy[Any] = st.one_of(
        st.just(True),
        st.just(False),
        st.just({}),
        st.sampled_from(_TYPE_NAMES).map(lambda name: {"type": name}),
        st.integers(min_value=0, max_value=4).map(lambda n: {"minItems": n}),
        st.integers(min_value=0, max_value=4).map(lambda n: {"maxLength": n}),
        st.integers(min_value=-2, max_value=8).map(lambda n: {"minimum": n}),
        st.sampled_from(_PATTERNS).map(lambda pattern: {"pattern": pattern}),
        st.sampled_from(_STRING_KEY_SPELLINGS).map(lambda name: {"required": [name]}),
        st.just({"type": "object", "propertyNames": {"type": "string"}}),
    )
    return st.recursive(
        leaf,
        lambda children: st.one_of(
            st.dictionaries(st.sampled_from(_STRING_KEY_SPELLINGS), children, max_size=3).map(
                lambda properties: {"properties": properties}
            ),
            st.tuples(children, st.booleans()).map(
                lambda pair: {"properties": {"a": pair[0]}, "additionalProperties": pair[1]}
            ),
            children.map(lambda schema: {"items": schema}),
            children.map(lambda schema: {"not": schema}),
            st.lists(children, min_size=1, max_size=3).map(lambda schemas: {"anyOf": schemas}),
            st.lists(children, min_size=1, max_size=3).map(lambda schemas: {"prefixItems": schemas}),
            st.tuples(children, children).map(lambda pair: {"if": pair[0], "then": pair[1]}),
        ),
        max_leaves=max_leaves,
    )


def _mapping_body(draw: st.DataObject, max_keys: int) -> list[str]:
    """`k: v` pairs, one spelling from each of several distinct key groups, so no pair folds."""
    indices = draw(st.lists(st.sampled_from(range(len(_KEY_GROUPS))), min_size=1, max_size=max_keys, unique=True))
    return [f"{draw(st.sampled_from(_KEY_GROUPS[i]))}: {draw(_scalar_spelling())}" for i in indices]


@st.composite
def _scalar_spelling(draw: st.DataObject) -> str:
    """One value as YAML text: a scalar, a flow sequence, or a flow mapping keyed the same way."""
    kind = draw(st.integers(min_value=0, max_value=2))
    if kind == 0:
        return draw(st.sampled_from(_SCALAR_SPELLINGS))
    if kind == 1:
        return "[" + ", ".join(draw(st.lists(st.sampled_from(_SCALAR_SPELLINGS), max_size=3))) + "]"
    return "{" + ", ".join(_mapping_body(draw, 3)) + "}"


@st.composite
def _yaml_document(draw: st.DataObject) -> str:
    """YAML source for a block mapping whose keys stay distinct as Python keys.

    One spelling per key group, so the document reaches these doors with no fold in it - which keeps
    the door comparison below a claim about the conversion rather than about the fold.
    """
    return "\n".join(_mapping_body(draw, 4)) + "\n"


def _through_this_surface(document: pyrs_yaml.Document, schema: Any) -> tuple[str, str | None]:
    """`(verdict, message)` as `Document.validate` reports it.

    A rejection that arrives as anything but `YamlValidateError` is raised, not folded into the
    tuple: the contract is that this surface answers with the error type it documents.
    """
    try:
        document.validate(schema)
    except pyrs_yaml.YamlValidateError as exc:
        return ("invalid", str(exc))
    return ("valid", None)


def _through_the_reference(instance: Any, schema: Any) -> tuple[str, str | None]:
    """`(verdict, message)` as the engine this surface delegates to reports it."""
    try:
        jsonschema.validate(instance, schema)
    except jsonschema.exceptions.ValidationError as exc:
        return ("invalid", exc.message)
    except jsonschema.exceptions.SchemaError as exc:
        return ("schema-invalid", exc.message)
    return ("valid", None)


@settings(max_examples=300, deadline=5000, suppress_health_check=[HealthCheck.too_slow])
@given(schema=_json_schema(), sources=st.lists(_yaml_document(), min_size=2, max_size=4), as_text=st.booleans())
def test_validate_is_the_reference_on_both_cache_paths(schema: Any, sources: list[str], as_text: bool) -> None:
    """A warmed schema must not change what the engine answers about a later document.

    The cache keys on the schema, not the document, so a second call with one schema object is served
    by the validator the first call compiled - `test_the_repeat_call_is_the_cached_path` is the
    witness. Each call is also judged against the reference on that document's own instance, which
    puts the cold path, the cached path and the delegated answer in one comparison.
    """
    handed = json.dumps(schema) if as_text else schema
    for source in sources:
        document = pyrs_yaml.parse(source)
        expected = _through_the_reference(document.to_dict(), schema)
        for round_number in (1, 2):
            got = _through_this_surface(document, handed)
            assert got == expected, (
                f"round {round_number} of {source!r} against {handed!r}: this surface said {got}, "
                f"the engine says {expected}"
            )


def _mapping_entries(value: Any) -> int:
    """How many members the structure holds, at every depth."""
    if isinstance(value, dict):
        return len(value) + sum(_mapping_entries(item) for item in value.values())
    if isinstance(value, list):
        return sum(_mapping_entries(item) for item in value)
    return 0


def _root_keys_are_strings(instance: Any) -> bool:
    """Whether every member name the root object carries is a string.

    `propertyNames` asks exactly this of the members its schema reaches - a typed key one level down
    is not its question, measured: `{"a": {2: 1}}` passes `propertyNames: {type: string}`.
    """
    return isinstance(instance, dict) and all(isinstance(key, str) for key in instance)


@settings(max_examples=300, deadline=5000, suppress_health_check=[HealthCheck.too_slow])
@given(source=_yaml_document())
def test_the_two_doors_hold_the_same_document(source: str) -> None:
    """Neither door loses an entry, and the one thing they differ about is the key's type.

    `validate` reads the dict door, so whatever the two doors disagree about is a verdict this surface
    inherits. The generator keeps the pair-comparison clean - one spelling per key group, so no fold -
    and `TestTheDictDoorFoldsAKeyPairTheDocumentWritesApart` pins the fold itself.
    """
    document = pyrs_yaml.parse(source)
    instance = document.to_dict()
    projected = json.loads(document.to_json())
    assert _mapping_entries(instance) == _mapping_entries(projected), (
        f"{source!r}: to_dict holds {_mapping_entries(instance)} members, to_json holds "
        f"{_mapping_entries(projected)} - one door lost one"
    )
    schema = {"type": "object", "propertyNames": {"type": "string"}}
    verdict = _through_this_surface(document, schema)
    assert verdict == _through_the_reference(instance, schema), source
    assert verdict[0] == ("valid" if _root_keys_are_strings(instance) else "invalid"), (
        f"{source!r}: the dict door keys the root by {sorted(map(type, instance))}, verdict {verdict}"
    )
    assert _through_the_reference(projected, schema) == ("valid", None), projected


def test_the_repeat_call_is_the_cached_path(monkeypatch: pytest.MonkeyPatch) -> None:
    """The witness behind the property above: a second call with one schema object is a cache hit.

    The fresh path runs `cls.check_schema(schema)` inside `jsonschema.validate`; the cached path does
    not, so the count tells the two apart. Without it the property could compare two cold calls and
    report a tier that judged the cache while never reaching it.
    """
    unbound_check = Draft202012Validator.__dict__["check_schema"].__func__
    seen: list[int] = []

    def counting(cls, schema, *args, **kwargs):
        seen.append(1)
        return unbound_check(cls, schema, *args, **kwargs)

    monkeypatch.setattr(Draft202012Validator, "check_schema", classmethod(counting))
    schema = {"type": "object", "required": ["a"]}
    document = pyrs_yaml.parse("a: 1\n")

    del seen[:]
    document.validate(schema)
    assert len(seen) == 1, "the first call is the cold path, so it must meta-check"

    del seen[:]
    document.validate(schema)
    assert not seen, "the repeat call did not read the cached validator, so the cached path went unjudged"

    # A *different* document against the warmed schema is the case the property generates: the
    # rejection arrives from `best_match(iter_errors(...))`, not from `jsonschema.validate`.
    del seen[:]
    with pytest.raises(pyrs_yaml.YamlValidateError) as rejection:
        pyrs_yaml.parse("b: 2\n").validate(schema)
    assert not seen, "the warmed schema was expected to answer for the second document from the cache"
    assert str(rejection.value) == "'a' is a required property"


class TestTheDictDoorFoldsAKeyPairTheDocumentWritesApart:
    """Two keys the source writes apart, one entry at the dict door - and `validate` sees that.

    What the tier found by drawing key spellings at random: `1:` beside `true:` is two YAML keys, the
    reader accepts them, `to_yaml` and `to_json` both write both, and the Python dict carries one
    (`1 == True`, and setting an equal key replaces the value while keeping the first key). Registered
    as `api:dict-door-key-fold`: PyYAML's loader folds at this same point, measured 2026-10-31, so the
    fold is the dict door's own, not a defect this project introduced - what is this project's is that
    it also ships a door that does not fold.
    """

    @pytest.mark.known_residual("api:dict-door-key-fold")
    def test_the_pair_folds_at_the_dict_and_the_schema_answers_for_one_entry(self):
        first, second = _FOLD_PAIR
        source = f"{first}: one\n{second}: two\n"
        document = pyrs_yaml.parse(source)
        assert document.to_yaml() == source, source
        assert json.loads(document.to_json()) == {first: "one", second: "two"}, source
        folded = document.to_dict()
        assert len(folded) == 1, f"{source!r} was expected to fold to one entry, got {folded!r}"

        schema = {"type": "object", "minProperties": 2}
        assert _through_this_surface(document, schema)[0] == "invalid", (
            "the dict door handed the engine one entry, so a two-key document is refused"
        )
        assert _through_the_reference(json.loads(document.to_json()), schema)[0] == "valid", (
            "the JSON door keeps both, so the same schema accepts the same document"
        )

    def test_the_counterparty_folds_at_the_dict_the_same_way(self):
        first, second = _FOLD_PAIR
        source = f"{first}: one\n{second}: two\n"
        assert pyyaml.safe_load(source) == pyrs_yaml.parse(source).to_dict(), (
            "PyYAML no longer folds this pair, so the boundary is ours to reconsider, not to inherit"
        )


class TestTheTwoReadersAnswerOppositeWaysOnAdjacentKeyPairs:
    """`parse` and `safe_load` disagree about which of two keys is a duplicate - in both directions.

    The AST route tracks an untagged scalar key by its text, so style never reaches its decision and
    `1:` beside `'1':` is one key (`KeyIdentity::Text`, `crates/pyrs-ast/src/ast.rs`). The fast route
    tracks the resolved Python object, so `1` and `true` are one key and `1` and `'1':` are two. The
    pairs are adjacent and the answers cross over. Registered as `engine:key-identity-route-parity`.
    """

    @pytest.mark.known_residual("engine:key-identity-route-parity")
    def test_the_ast_route_refuses_what_the_fast_route_accepts(self):
        source = "1: one\n'1': two\n"
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError):
            pyrs_yaml.parse(source)
        assert pyrs_yaml.safe_load(source) == {1: "one", "1": "two"}
        assert pyyaml.safe_load(source) == {1: "one", "1": "two"}, (
            "the counterparties keep both, so the fast route is the one that matches them here"
        )

    @pytest.mark.known_residual("engine:key-identity-route-parity")
    def test_the_fast_route_refuses_what_the_ast_route_accepts(self):
        source = "1: one\ntrue: two\n"
        document = pyrs_yaml.parse(source)
        assert document.to_yaml() == source, "the AST route accepts the pair and holds both keys"
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError):
            pyrs_yaml.safe_load(source)
