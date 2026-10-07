"""Route parity: the two YAML writers must emit the same bytes for the same data.

YAML text is produced by two implementations that mirror each other by design instead of
sharing code: `Serializer::write_*` over parsed nodes, and `py/direct_dump.rs` over Python
objects. That duplication is why #287 had to fix the same empty-container spelling at both
sites, and why a reviewer had to *notice* the second site — nothing compared them. The
defence-matrix measurement registered that as `route-parity:node-writer-vs-direct-dump`;
this file closes it.

Every row below is a measurement, not an expectation: the pinned text is what the node
writer emits, and the assertion is that the direct writer emits the same bytes for the
equivalent Python object. Rows whose text was written from a guess about "the obvious
spelling" failed on the first run (`- a:\n    b: 1` is not how a non-compact mapping under
a dash is spelled; `a: '1'` is not the quote style either writer chooses) and were
corrected from output rather than relaxed.
"""

from __future__ import annotations

import pytest

import pyrs_yaml

# (label, canonical text both writers must emit, the object the direct route is given)
PARITY_TABLE = [
    ("scalar value", "a: 1\n", {"a": 1}),
    ("string value", "a: text\n", {"a": "text"}),
    ("int-like string is quoted", 'a: "1"\n', {"a": "1"}),
    ("string with a colon", 'a: "x: y"\n', {"a": "x: y"}),
    ("empty string", 'a: ""\n', {"a": ""}),
    ("null value", "a: null\n", {"a": None}),
    ("bool value", "a: true\n", {"a": True}),
    ("float value", "a: 1.5\n", {"a": 1.5}),
    ("nested mapping", "a:\n  b: 1\n", {"a": {"b": 1}}),
    ("three levels", "a:\n  b:\n    c: 1\n", {"a": {"b": {"c": 1}}}),
    ("sequence of scalars", "a:\n  - 1\n  - 2\n", {"a": [1, 2]}),
    ("empty mapping value", "a: {}\n", {"a": {}}),
    ("empty sequence value", "a: []\n", {"a": []}),
    ("empty containers nested", "a:\n  b: {}\n  c: []\n", {"a": {"b": {}, "c": []}}),
    ("three levels of empty", "a:\n  b:\n    c: {}\n", {"a": {"b": {"c": {}}}}),
    ("empty mapping under a dash", "- {}\n", [{}]),
    ("empty sequence under a dash", "- []\n", [[]]),
    ("empty mapping then scalar", "- {}\n- 1\n", [{}, 1]),
    ("compact mapping item", "- a: 1\n  b: 2\n", [{"a": 1, "b": 2}]),
    ("compact mapping with empty value", "- a: {}\n  b: 1\n", [{"a": {}, "b": 1}]),
    ("compact mapping with empty second", "- b: 1\n  a: {}\n", [{"b": 1, "a": {}}]),
    ("non-compact mapping under a dash", "- \n  a:\n    b: 1\n", [{"a": {"b": 1}}]),
    ("sequence under a dash", "- \n  - 1\n  - 2\n", [[1, 2]]),
    ("nested sequences under keys", "a:\n  - \n    - 1\n  - \n    - 2\n    - 3\n", {"a": [[1], [2, 3]]}),
    ("mapping of a sequence", "- \n  a:\n    - 1\n    - 2\n", [{"a": [1, 2]}]),
    ("empty at top", "{}\n", {}),
    ("empty list at top", "[]\n", []),
    ("multiple keys", "a: 1\nb: 2\nc: 3\n", {"a": 1, "b": 2, "c": 3}),
    ("unicode value", "a: 中文 \U0001f600\n", {"a": "中文 \U0001f600"}),
    ("quoted key", '"1": a\n', {"1": "a"}),
    ("deep mix", "a:\n  b:\n    c:\n      - 1\n      - d: {}\n", {"a": {"b": {"c": [1, {"d": {}}]}}}),
]

IDS = [label for label, _text, _obj in PARITY_TABLE]
CASES = [(text, obj) for _label, text, obj in PARITY_TABLE]

# Differences that are intended: the node route preserves what the source spelled, the
# direct route has no source. Pinned at their actual values with the reason, so a future
# "let's make them agree" that throws away quoting, flow style, block scalars, anchors or
# tags has to be an argument rather than an accident.
STYLE_PRESERVATION = [
    ("single quotes survive", "a: 'text'\n", {"a": "text"}),
    ("flow style survives", "a: [1, 2]\n", {"a": [1, 2]}),
    ("literal block survives", "a: |\n  l1\n  l2\n", {"a": "l1\nl2\n"}),
    # The writer re-wraps a folded block: `a: >` + `  f1` + `  f2` reads as the text
    # "f1 f2\n" and is emitted on one folded line. The pinned form is what the writer
    # settles on, not what the author typed, because this table's first run caught the
    # guess and the data is identical either way.
    ("folded block survives", "a: >\n  f1 f2\n", {"a": "f1 f2\n"}),
    ("anchor and alias survive", "- &x 1\n- *x\n", [1, 1]),
    ("tag survives", "a: !custom 1\n", {"a": 1}),
    ("tilde null keeps its spelling", "a: ~\n", {"a": None}),
]


@pytest.mark.parametrize("text,obj", CASES, ids=IDS)
def test_the_node_writer_settles_on_the_pinned_text(text, obj):
    """Reference half: the node route emits the pinned text and stops moving."""
    assert pyrs_yaml.parse(text).to_yaml() == text


@pytest.mark.parametrize("text,obj", CASES, ids=IDS)
def test_the_direct_writer_emits_the_same_bytes(text, obj):
    """The mirrored half — the comparison that did not exist until this file did."""
    assert pyrs_yaml.safe_dump(obj) == text


@pytest.mark.parametrize("text,obj", CASES, ids=IDS)
def test_the_two_routes_agree_with_each_other(text, obj):
    """Stated separately, because a failure here names the class precisely.

    `#287` reached a fuzzer instead of a test: `safe_dump([{}])` produced `"- \\n  {}\\n"`,
    which re-reads as a *flow* node, so the next dump inlined it. The data was never wrong;
    the text kept moving, and the two writers disagreed about where an empty container
    belongs. The same class was still open at the mapping-value site when this table was
    first run — `safe_dump({"a": {}})` produced `"a:\\n  {}\\n"` — and is fixed in
    `direct_dump.rs` alongside `is_compact_mapping`, which had been forfeiting the compact
    dash form for any item holding an empty container.
    """
    assert pyrs_yaml.safe_dump(obj) == pyrs_yaml.parse(text).to_yaml()


@pytest.mark.parametrize("text,obj", CASES, ids=IDS)
def test_dumping_through_the_other_route_is_a_fixed_point(text, obj):
    """Object → text → object → text must not move, whichever writer produced step one."""
    once = pyrs_yaml.safe_dump(obj)
    assert pyrs_yaml.safe_dump(pyrs_yaml.safe_load(once)) == once
    node_once = pyrs_yaml.parse(once).to_yaml()
    assert pyrs_yaml.safe_dump(pyrs_yaml.safe_load(node_once)) == node_once


@pytest.mark.parametrize("text,obj", [case[1:] for case in STYLE_PRESERVATION], ids=[c[0] for c in STYLE_PRESERVATION])
def test_style_preservation_diverges_on_purpose(text, obj):
    """The node route keeps the author's spelling; the direct route has no author to keep.

    Asserted as a divergence in both directions: the pinned text must settle under the node
    writer, and the direct writer must *not* reproduce it (it emits its own canonical form),
    while both must carry the same data.
    """
    assert pyrs_yaml.safe_load(text) == obj
    assert pyrs_yaml.parse(text).to_yaml() == text
    assert pyrs_yaml.safe_dump(obj) != text
    assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump(obj)) == obj


def test_non_string_keys_do_not_survive_the_object_view_yet():
    """A pin for a known defect, not an endorsement of it.

    The object view resolves a scalar *value* with the schema rules but leaves keys as text,
    so `1: a` dumps to `1: a` and reads back as `{"1": "a"}` — and the second dump then
    quotes it. PR #292 closes exactly that. The rows stay here, pinned at their actual
    values with the mechanism named, because a parity table that quietly *omitted* them
    would report green while the class went unmeasured; when that PR lands these asserts
    break and the shapes move into `PARITY_TABLE`.
    """
    assert pyrs_yaml.safe_dump({1: "a"}) == "1: a\n"
    assert pyrs_yaml.safe_load("1: a\n") == {"1": "a"}
    assert pyrs_yaml.safe_dump(pyrs_yaml.safe_load("1: a\n")) == '"1": a\n'
    assert pyrs_yaml.safe_dump({True: "a"}) == "true: a\n"
    assert pyrs_yaml.safe_load("true: a\n") == {"true": "a"}
