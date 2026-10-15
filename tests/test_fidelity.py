"""Fidelity tests: round-trip preservation of untouched bytes after edits."""

from hypothesis import HealthCheck, given, settings

import pyrs_yaml
from tests.strategies import roundtrip_safe_json


class TestFidelity:
    """Verify that edit operations preserve untouched bytes (splice fidelity)."""

    def test_untouched_bytes_survive_simple_edit(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\nc: 3\n")
        doc._set_path(["b"], 9)
        assert doc.to_yaml() == "a: 1\nb: 9\nc: 3\n"

    def test_untouched_bytes_survive_comment_preservation(self):
        doc = pyrs_yaml.parse("a: 1  # keep\nb: 2\n")
        doc._set_path(["a"], 9)
        assert doc.to_yaml() == "a: 9  # keep\nb: 2\n"

    def test_both_notes_of_a_pair_survive_the_rewrite(self):
        """A key note and a value note each reach the document, once, and the text settles.

        `key: value  # one` has a single trailing slot. When both sides claimed it the writer kept
        the value's claim and the key's note vanished from the caller's document; the key note now
        takes a line of its own above the pair. Which node ends up owning that line is an engine
        detail the bindings cannot name (no path addresses a key node), so what a Python caller can
        hold the writer to is exactly this: both texts are present, neither duplicated, and re-writing
        the re-read document changes nothing.
        """
        for src in ["? a # kn\n: b # vv\n", "x: 1\n? a # kn\n: b # vv\n"]:
            emitted = pyrs_yaml.parse(src).to_yaml()
            assert emitted.count("# kn") == 1, (src, emitted)
            assert emitted.count("# vv") == 1, (src, emitted)
            assert pyrs_yaml.parse(emitted).to_yaml() == emitted, (src, emitted)

    def test_untouched_bytes_survive_nested_edit(self):
        doc = pyrs_yaml.parse("a:\n  b: 1\n  c: 2\n")
        doc._set_path(["a", "b"], 9)
        assert doc.to_yaml() == "a:\n  b: 9\n  c: 2\n"

    def test_untouched_bytes_survive_sequence_edit(self):
        doc = pyrs_yaml.parse("- a\n- b\n- c\n")
        doc._set_path([1], "x")
        assert doc.to_yaml() == "- a\n- x\n- c\n"


class TestFidelityProperty:
    """Property-based fidelity checks on random structures."""

    @settings(max_examples=100, deadline=3000, suppress_health_check=[HealthCheck.too_slow])
    @given(roundtrip_safe_json)
    def test_parse_roundtrip_is_stable(self, value):
        doc = pyrs_yaml.parse(pyrs_yaml.safe_dump(value))
        assert doc.to_dict() == value

    @settings(max_examples=100, deadline=3000, suppress_health_check=[HealthCheck.too_slow])
    @given(roundtrip_safe_json)
    def test_yaml_roundtrip_does_not_panic(self, value):
        doc = pyrs_yaml.parse(pyrs_yaml.safe_dump(value))
        out = doc.to_yaml()
        assert isinstance(out, str)
