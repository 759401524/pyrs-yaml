"""Path-transparency tests for the direct event->Python load fast path.

The alias cases double as the behavioral SPEC for anchored documents:
anchors currently take the AST pipeline (event-span replay was measured
slower than parse+to_dict and reverted), and these literals must agree with
PyYAML on every one of these shapes. Documents on the fast path must agree
with the AST path too. If a future change alters ANY line here, it is a
behavior break on one of the two paths.
"""

import pytest

import pyrs_yaml


class TestAliasParity:
    def test_scalar_anchor_referenced_twice(self):
        # issue #163: both references must expand. The pre-fix cycle guard
        # was a global accumulation set, so the second `*x` saw the anchor
        # address already marked and silently degraded to `None`.
        d = pyrs_yaml.safe_load("a: &x 1\nb: *x\nc: *x\n")
        assert d == {"a": 1, "b": 1, "c": 1}

    def test_map_anchor_referenced_twice_gets_independent_copies(self):
        d = pyrs_yaml.safe_load("base: &b {k: 1}\none: *b\ntwo: *b\n")
        assert d == {"base": {"k": 1}, "one": {"k": 1}, "two": {"k": 1}}
        # Each expansion is a fresh object, never a shared reference.
        assert d["base"] is not d["one"]
        assert d["one"] is not d["two"]

    def test_same_anchor_repeated_within_one_container(self):
        # Sibling references inside a single mapping must not poison
        # each other - the old guard leaked across sibling iterations.
        d = pyrs_yaml.safe_load("a: &x 1\nb: {p: *x, q: *x}\n")
        assert d == {"a": 1, "b": {"p": 1, "q": 1}}

    def test_anchor_referenced_many_times_in_a_sequence(self):
        d = pyrs_yaml.safe_load("a: &x [1, 2]\nb: [*x, *x, *x]\n")
        assert d == {"a": [1, 2], "b": [[1, 2], [1, 2], [1, 2]]}

    def test_nested_anchor_replay(self):
        d = pyrs_yaml.safe_load("x: &a {p: 1}\ny: &b {q: *a}\nz: *b\n")
        assert d == {
            "x": {"p": 1},
            "y": {"q": {"p": 1}},
            "z": {"q": {"p": 1}},
        }

    def test_self_referential_container_terminates(self):
        # A genuine cycle must still terminate. The guard is path-scoped, so
        # `*x` re-entering the anchor body while it is still being expanded
        # yields `None`; PyYAML yields a recursive `[...]` object, which
        # cannot be reproduced with an acyclic Python object graph.
        d = pyrs_yaml.safe_load("a: &x [1, *x]\n")
        assert d == {"a": [1, [1, None]]}

    def test_root_alias_stays_on_ast_path(self):
        # A root-level document that is just an alias has no anchored value
        # to replay; the AST pipeline owns the (error) diagnostics.
        with pytest.raises(ValueError):
            pyrs_yaml.safe_load("a: 1\nb: *missing\n")

    def test_anchor_with_merge_still_resolves_via_ast_fallback(self):
        # `<<` vetoes the fast path; behavior must match pre-direct exactly.
        d = pyrs_yaml.safe_load("base: &b {x: 1, y: 2}\nuse: {<<: *b, y: 3}\n")
        assert d["use"] == {"x": 1, "y": 3}

    def test_yaml_instance_same_semantics(self):
        y = pyrs_yaml.YAML()
        assert y.safe_load("a: &x 1\nb: *x\nc: *x\n") == {
            "a": 1,
            "b": 1,
            "c": 1,
        }
        assert y.safe_loads("a: &x 1\nb: *x\n") == [{"a": 1, "b": 1}]


class TestAliasParityPyYaml:
    """Bit-for-bit agreement with PyYAML on non-cyclic alias documents.

    Keys stay strings throughout: pyrs types a mapping key by its scalar
    text (``dict.set_item(value.as_ref())`` in ``convert.rs``) while PyYAML
    additionally schema-resolves keys (``1:`` -> ``1``), so an integer key
    would compare unequal for a reason unrelated to alias expansion.
    """

    @pytest.mark.parametrize(
        ("src", "expected"),
        [
            ("a: &x 1\nb: *x\nc: *x\n", {"a": 1, "b": 1, "c": 1}),
            ("a: &x [1, 2]\nb: *x\nc: *x\n", {"a": [1, 2], "b": [1, 2], "c": [1, 2]}),
            ("a: &x 1\nb: [*x, *x]\n", {"a": 1, "b": [1, 1]}),
            (
                "r: &x {p: 1}\nk1: *x\nk2: *x\nk3: *x\n",
                {"r": {"p": 1}, "k1": {"p": 1}, "k2": {"p": 1}, "k3": {"p": 1}},
            ),
            ("a: &x 1\nb: {p: *x, q: *x}\n", {"a": 1, "b": {"p": 1, "q": 1}}),
            ("x: &a {p: 1}\ny: &b {q: *a}\nz: *b\n", {"x": {"p": 1}, "y": {"q": {"p": 1}}, "z": {"q": {"p": 1}}}),
        ],
    )
    def test_matches_pyyaml(self, src, expected):
        yaml = pytest.importorskip("yaml")
        assert pyrs_yaml.safe_load(src) == expected
        assert pyrs_yaml.safe_load(src) == yaml.safe_load(src)
