"""Tests for YamlDocument.source() and reparse()."""

import pyrs_yaml


class TestSource:
    def test_source_from_parse(self):
        yaml_str = "a: 1\nb: 2"
        doc = pyrs_yaml.parse(yaml_str)
        assert doc.source() == yaml_str

    def test_source_from_parse_file(self):
        import tempfile
        from pathlib import Path

        with tempfile.TemporaryDirectory() as td:
            path = Path(td) / "test.yaml"
            path.write_bytes(b"x: 10\ny: true")
            doc = pyrs_yaml.parse_file(str(path))
            assert "x: 10" in doc.source()
            assert "y: true" in doc.source()

    def test_source_from_parse_all_docs(self):
        yaml_str = "---\na: 1\n---\nb: 2"
        docs = pyrs_yaml.parse_all_docs(yaml_str)
        assert len(docs) == 2
        assert docs[0].source() == yaml_str
        assert docs[1].source() == yaml_str


class TestReparse:
    def test_reparse_basic(self):
        doc = pyrs_yaml.parse("a: 1\nb: 2")
        assert doc.get("a") == 1
        # Modify and re-parse (simulated via source modification through
        # the document — we test that reparse succeeds and returns None)
        doc.reparse()
        assert doc.get("a") == 1

    def test_reparse_no_source(self):
        # Parse from bytes — no string source stored
        doc = pyrs_yaml.parse(b"a: 1")
        # bytes still get converted to a string source in parse(), so
        # test by constructing a doc with source=None is not possible from
        # public API. Instead, test that reparse works fine with bytes input.
        assert doc.source() == "a: 1"
        doc.reparse()
        assert doc.get("a") == 1

    def test_reparse_schema_change(self):
        # Re-parse with a different schema should change behavior
        doc = pyrs_yaml.parse("x: on")
        assert doc.get("x") == "on"  # core schema — "on" is a string
        doc.reparse(schema="yaml1.1")
        assert doc.get("x") is True  # yaml1.1 schema — "on" is a bool

    def test_reparse_resolve_merges(self):
        yaml_str = "base: &base\n  x: 1\nderived:\n  <<: *base\n  y: 2"
        doc = pyrs_yaml.parse(yaml_str, resolve_merges=True)
        derived = doc.get("derived")
        assert derived["x"] == 1
        assert derived["y"] == 2
        doc.reparse(resolve_merges=False)
        derived2 = doc.get("derived")
        assert derived2.get("x") is None
        assert derived2["y"] == 2
        assert "<<" in str(derived2)
