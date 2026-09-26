"""Path-transparency tests for the direct event->Python load fast path.

The alias cases double as the behavioral SPEC for anchored documents:
anchors currently take the AST pipeline (event-span replay was measured
slower than parse+to_dict and reverted), and these literals were pinned
from AST-path measurements. Documents on the fast path must agree with
the AST path on every one of these shapes. If a future change alters ANY
line here, it is a behavior break on one of the two paths.
"""

import pytest

import pyrs_yaml


class TestAliasParity:
    def test_scalar_anchor_twice_second_is_none(self):
        d = pyrs_yaml.safe_load("a: &x 1\nb: *x\nc: *x\n")
        assert d == {"a": 1, "b": 1, "c": None}

    def test_map_anchor_twice_second_is_none_not_shared(self):
        d = pyrs_yaml.safe_load("base: &b {k: 1}\none: *b\ntwo: *b\n")
        assert d == {"base": {"k": 1}, "one": {"k": 1}, "two": None}
        assert d["base"] is not d["one"]

    def test_nested_anchor_replay(self):
        d = pyrs_yaml.safe_load("x: &a {p: 1}\ny: &b {q: *a}\nz: *b\n")
        assert d == {"x": {"p": 1}, "y": {"q": {"p": 1}}, "z": {"q": None}}

    def test_self_referential_container_expands_once(self):
        # The anchor body is built in place (not via visited); its inner
        # alias is the FIRST expansion (one level), and the next level hits
        # the visited set -> None.
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
            "c": None,
        }
        assert y.safe_loads("a: &x 1\nb: *x\n") == [{"a": 1, "b": 1}]
