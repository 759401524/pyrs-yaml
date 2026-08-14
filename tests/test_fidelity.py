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
