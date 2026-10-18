"""The schema language's verdict, asked as a property over generated documents and rule sets.

This is the tier the defence matrix measured as missing: the schema row had 39 unit tests, two Rust
properties and a fuzz target, and no property tier on the Python side - so every one of those unit
cases names a shape somebody thought of. A validator's real risk is the shape nobody names.

The oracle here is not another engine. It is a reference reading of the rule text written from the
grammar this crate documents (`path` selects nodes, `type` / `sequence_of` / `mapping_of` state what
the selected node must be, `required` that it resolve to a non-null value), evaluated on the Python
tree the same document loads to. Two independent implementations of the same rules - one walking an
AST, one walking `dict`/`list` - disagree only when one of them is wrong.

The second and third properties hold the engine to itself rather than to the reference: the verdict
may not depend on how the document is spelled, and the two ways of handing over one rule set -
inline and registered under a name - may not answer differently for one document.
"""

from __future__ import annotations

from hypothesis import given
from hypothesis import strategies as st

import pyrs_yaml

KEYS = ["port", "note", "tags", "config", "sub"]
SCALARS = st.one_of(
    st.none(),
    st.booleans(),
    st.integers(min_value=-3, max_value=3),
    st.just(1.5),
    st.sampled_from(["abc", "x y"]),
)
VALUES = st.recursive(
    SCALARS,
    lambda inner: st.one_of(
        st.lists(inner, max_size=3),
        st.dictionaries(st.sampled_from(KEYS), inner, max_size=3),
    ),
    max_leaves=12,
)
DOCS = st.dictionaries(st.sampled_from(KEYS), VALUES, max_size=4)

TYPE_NAMES = {
    "null": lambda v: v is None,
    "bool": lambda v: v is True or v is False,
    "int": lambda v: isinstance(v, int) and not isinstance(v, bool) and not isinstance(v, float),
    "float": lambda v: isinstance(v, float),
    "str": lambda v: isinstance(v, str),
    "map": lambda v: isinstance(v, dict),
    "seq": lambda v: isinstance(v, list),
}


def _kind(value) -> str:
    for name, holds in TYPE_NAMES.items():
        if holds(value):
            return name
    raise AssertionError(f"generated a value this oracle cannot name: {value!r}")


def _segments(path: str) -> list:
    """`$.tags[0].name` -> [("key","tags"), ("index",0), ("key","name")]."""
    out: list = []
    i = 1 if path.startswith("$") else 0
    n = len(path)
    while i < n:
        if path[i] == ".":
            i += 1
            start = i
            while i < n and path[i] not in ".[":
                i += 1
            out.append(("key", path[start:i]))
        elif path.startswith("[*]", i):
            out.append(("star", None))
            i += 3
        elif path[i] == "[":
            end = path.index("]", i)
            out.append(("index", int(path[i + 1 : end])))
            i = end + 1
        else:  # pragma: no cover - the generator below only emits the shapes above
            raise AssertionError(f"cannot parse {path!r} past {path[i:]!r}")
    return out


def _select(node, segments) -> list:
    """Every value the path names, flattened over `[*]`."""
    if not segments:
        return [node]
    (kind, arg), *rest = segments
    if kind == "key":
        if not isinstance(node, dict) or arg not in node:
            return []
        return _select(node[arg], rest)
    if not isinstance(node, list):
        return []
    if kind == "index":
        if arg >= len(node):
            return []
        return _select(node[arg], rest)
    return [found for item in node for found in _select(item, rest)]


def _violates(doc: dict, rule: dict) -> bool:
    """What the reference says about one rule: True when the document breaks it."""
    found = _select(doc, _segments(rule["path"]))
    if rule.get("required") and (not found or any(value is None for value in found)):
        return True
    check = rule.get("type")
    if check is not None and any(not TYPE_NAMES[check](value) for value in found):
        return True
    for key, check in (("sequence_of", "seq"), ("mapping_of", "map")):
        wanted = rule.get(key)
        if wanted is None:
            continue
        for value in found:
            if not TYPE_NAMES[check](value):
                return True
            children = value if check == "seq" else list(value.values())
            if any(not TYPE_NAMES[wanted](child) for child in children):
                return True
    return False


def _verdict(text: str, schema: str) -> bool:
    """True when the engine refuses, read from the exception type rather than the message."""
    return _refuses(lambda: pyrs_yaml.validate_against_schema(text, schema))


_registered: list[int] = []


def _verdict_registered(text: str, schema: str) -> bool:
    """The same rules through the other door: registered once under a fresh name, then asked by name."""
    _registered.append(1)
    name = f"prop_schema_{len(_registered)}"
    pyrs_yaml.register_schema(name, schema)
    return _refuses(lambda: pyrs_yaml.validate_against_registered_schema(text, name))


def _refuses(call) -> bool:
    try:
        call()
    except pyrs_yaml.YamlValidateError:
        return True
    return False


def _rules(doc: dict, wrong: bool) -> list:
    """One rule per top-level key, typed as the document says - or as something else when `wrong`."""
    out = []
    for key, value in sorted(doc.items()):
        name = _kind(value)
        if wrong and name in TYPE_NAMES:
            name = "str" if name != "str" else "int"
        rule = {"path": f"$.{key}", "type": name}
        out.append(rule)
        if isinstance(value, list) and value:
            element = _kind(value[-1]) if not wrong else ("str" if name != "str" else "int")
            out.append({"path": f"$.{key}[*]", "type": element})
        if isinstance(value, dict) and value:
            sub = sorted(value)[0]
            element = _kind(value[sub]) if not wrong else ("str" if name != "str" else "int")
            out.append({"path": f"$.{key}.{sub}", "type": element})
    if wrong:
        out.append({"path": "$.absent", "required": True})
    return out


def _schema_text(rules: list) -> str:
    return pyrs_yaml.safe_dump({"validate": rules})


@given(DOCS)
def test_the_validator_and_the_reference_agree_on_both_readings_of_the_rules(doc):
    text = pyrs_yaml.safe_dump(doc)
    for wrong in (False, True):
        rules = _rules(doc, wrong)
        schema = _schema_text(rules)
        want = any(_violates(doc, rule) for rule in rules)
        got = _verdict(text, schema)
        assert got == want, f"{want=} {got=}\ndoc={doc!r}\nschema={schema!r}"


@given(DOCS)
def test_the_verdict_does_not_depend_on_how_the_document_is_spelled(doc):
    # Re-emitting from the loaded tree changes spacing, quoting and indentation - it may not change
    # whether the document satisfies the rules. A verdict that moved here would mean the validator
    # read something about the spelling rather than the value.
    text = pyrs_yaml.safe_dump(doc)
    rules = _rules(doc, wrong=False)
    schema = _schema_text(rules)
    respelled = pyrs_yaml.safe_dump(pyrs_yaml.safe_load(text))
    assert _verdict(respelled, schema) == _verdict(text, schema), f"{text!r} and {respelled!r} answer differently"


@given(DOCS)
def test_the_two_ways_of_handing_over_the_same_rules_answer_the_same_question(doc):
    # The rule text can be passed inline or registered under a name. Same rules, same document, one
    # verdict - and the route that registers is the one `load_schema` and the CLI use.
    text = pyrs_yaml.safe_dump(doc)
    schema = _schema_text(_rules(doc, wrong=True))
    assert _verdict_registered(text, schema) == _verdict(text, schema), f"{text!r}\n{schema!r}"
