"""Cross-library parity for merge-key resolution (`<<`).

Why this gate exists as its own file: two root causes of *data loss* in merge
resolution (a nested `<<` in an inline source, and one inside an anchored source)
surfaced only through a hand-measured comparison table, because every shape involved
emits stable text. The round-trip oracle - `to_yaml(parse(to_yaml(x))) == to_yaml(x)` -
passes for a document that is stable and merely under-resolved, so it cannot be the
only defence for merge semantics. This file pins the resolved value against the two
reference libraries the project already tests with.

Parity here means *key sets and values*, not order: `dict` equality ignores insertion
order, which keeps the engine's documented document-order posture out of the assertion
while still catching a missing or spurious pair.

The deliberate divergences are listed in ``DELIBERATE``, not hidden: a `<<` that
resolves to nothing stays an ordinary key here, because deleting user text is the
worse failure for a round-trip library. PyYAML and ruamel, whose target is object
construction, either empty the mapping or refuse to load it.
"""

from __future__ import annotations

import pytest
import yaml

import pyrs_yaml

# PyYAML is imported unconditionally on purpose: it is a declared test dependency
# (`.ci/requirements-test.txt`), and a gate that skips silently when a dependency is
# missing is exactly the failure mode this file exists to prevent. ruamel is treated
# as optional below, matching the `HAS_RUAMEL` posture the benchmark suite already uses.

# --- Every spelling of a merge that all three engines must agree on. ----------

PARITY_SHAPES = [
    # A plain alias merge, and a merge with siblings that override it.
    "base: &b {x: 1}\nuse:\n  <<: *b\n",
    "base: &b {x: 1}\nuse:\n  <<: *b\n  x: 9\n",
    "base1: &b1 {a: 1}\nbase2: &b2 {b: 2}\ncur:\n  <<: [*b1, *b2]\n  c: 3\n",
    # A merge source that itself merges: inline, block, and three markers deep.
    "<<: {<<: {x: 1}}",
    "<<:\n <<:\n   x: 1\n",
    "<<:\n <<:\n   <<:\n     x: 1\n",
    # Precedence is per level: the source's own key beats what it inherits.
    "<<: {<<: {x: 1}, x: 9}",
    "<<: {<<: {x: 1, y: 1}, y: 2}",
    # A `<<` deeper inside a value travels with it.
    "<<: {a: {<<: {x: 1}}}",
    "<<: {<<: {a: {<<: {x: 1}}}}",
    # Template chains through anchors - the everyday config shape.
    "base: &b {x: 1}\nmid: &m {<<: *b, y: 2}\nuse:\n  <<: *m\n  z: 3\n",
    "a: &A {p: 1}\nb: &B {<<: *A, q: 2}\nc: &C {<<: *B, r: 3}\n<<: *C\n",
    "a: &A {x: 1}\nb: &B {<<: *A, y: 2}\n<<: [*A, *B]\n",
    "b: &B {<<: {x: 1}, y: 2}\n<<: *B\nx: 9\n",
    # A sequence source whose items are inline mappings.
    "<<:\n - {x: 1}\n - {y: 2}\n",
    # Lookalikes are not merges: style and tag decide identity.
    '"<<": {x: 1}\n',
    '<<: {"<<": {x: 1}}\n',
]

# --- Where staying a round-trip library means disagreeing with a loader. ------

DELIBERATE = [
    # Nothing to merge, so `<<` stays an ordinary key instead of vanishing.
    # PyYAML and ruamel both answer with an empty mapping.
    ("<<: []", {"<<": []}),
    ("<<: {}", {"<<": {}}),
    # A scalar or null merge value is a *typed error* for the loaders; here it is
    # data the user wrote, and it is kept and re-emitted instead of raising.
    ("<<: 5", {"<<": 5}),
    ("<<:", {"<<": None}),
]


@pytest.mark.parametrize("src", PARITY_SHAPES)
def test_merge_resolution_matches_pyyaml(src):
    assert pyrs_yaml.safe_load(src) == yaml.safe_load(src)


@pytest.mark.parametrize("src", PARITY_SHAPES)
def test_merge_resolution_matches_ruamel(src):
    ruamel_yaml = pytest.importorskip("ruamel.yaml")
    reader = ruamel_yaml.YAML(typ="safe")
    assert pyrs_yaml.safe_load(src) == dict(reader.load(src))


@pytest.mark.parametrize("src", PARITY_SHAPES + [src for src, _ in DELIBERATE])
def test_every_merge_shape_settles_in_one_round(src):
    """The invariant the fuzz tier owns, asserted over the parity set as well.

    A shape that resolves correctly and still moves between rounds is a bug in the
    writer; one that is stable and resolves wrongly is a bug in the reader. This set
    exists to catch the second, so the first has to hold over the same inputs.
    """
    once = pyrs_yaml.parse(src).to_yaml()
    twice = pyrs_yaml.parse(once).to_yaml()
    assert twice == once, f"{src!r} drifted: {once!r} -> {twice!r}"


@pytest.mark.parametrize(("src", "want"), DELIBERATE)
def test_documented_divergence_keeps_the_users_text(src, want):
    assert pyrs_yaml.safe_load(src) == want
