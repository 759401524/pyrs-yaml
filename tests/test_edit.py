"""Tests for in-place editing (set/insert/append/delete/rename) of pyrs-yaml.

Round-trip (comments/anchors/tags/scalar-style/flow-style/mapping-order) is the
primary test pattern.
"""

import pyrs_yaml


class TestEditErrors:
    def test_edit_error_hierarchy(self):
        assert issubclass(pyrs_yaml.YamlEditError, ValueError)
        assert issubclass(pyrs_yaml.YamlPathError, ValueError)
        assert pyrs_yaml.YamlEditError.__module__ == "pyrs_yaml"
        assert pyrs_yaml.YamlPathError.__module__ == "pyrs_yaml"


class TestLazySourceSync:
    def test_source_stable_when_unchanged(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\n")
        assert doc.source() == "a: 1\nb: 2\n"
        assert doc.to_yaml() == "a: 1\nb: 2\n"

    def test_reparse_stable_when_unchanged(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2\n")
        doc.reparse()
        assert doc.to_yaml() == "a: 1\nb: 2\n"
