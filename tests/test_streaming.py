"""Streaming parse tests (migrated from Rust py::streaming tests)."""

import io

import pyrs_yaml


def _docs_from_events(events):
    """Extract document content from raw event stream."""
    docs = []
    current = []
    in_doc = False
    for ev in events:
        t = ev["type"]
        if t == "document_start":
            in_doc = True
            current = []
        elif t == "document_end":
            if in_doc:
                docs.append(current)
                in_doc = False
        elif t == "stream_end":
            if current and in_doc:
                docs.append(current)
        elif in_doc:
            current.append(ev)
    return docs


class TestStreaming:
    """Streaming YAML parse behavior."""

    def test_simple_scalar(self):
        yaml = "hello\n"
        buf = io.StringIO(yaml)
        events = list(pyrs_yaml.YAML().load_stream(buf))
        docs = _docs_from_events(events)
        assert len(docs) == 1
        assert any(e["value"] == "hello" for e in docs[0])

    def test_mapping(self):
        yaml = "key: value\n"
        buf = io.StringIO(yaml)
        events = list(pyrs_yaml.YAML().load_stream(buf))
        docs = _docs_from_events(events)
        assert len(docs) == 1
        values = {e["value"] for e in docs[0] if e["type"] == "scalar"}
        assert "key" in values
        assert "value" in values

    def test_multiple_documents(self):
        yaml = "---\na: 1\n---\nb: 2\n"
        buf = io.StringIO(yaml)
        events = list(pyrs_yaml.YAML().load_stream(buf))
        docs = _docs_from_events(events)
        assert len(docs) == 2

    def test_empty_input(self):
        buf = io.StringIO("")
        events = list(pyrs_yaml.YAML().load_stream(buf))
        assert len(events) >= 2  # stream_start + stream_end

    def test_sequence(self):
        yaml = "- one\n- two\n- three\n"
        buf = io.StringIO(yaml)
        events = list(pyrs_yaml.YAML().load_stream(buf))
        docs = _docs_from_events(events)
        assert len(docs) == 1
        scalars = [e["value"] for e in docs[0] if e["type"] == "scalar"]
        assert scalars == ["one", "two", "three"]

    def test_nested_mapping(self):
        yaml = "a:\n  b: 1\n"
        buf = io.StringIO(yaml)
        events = list(pyrs_yaml.YAML().load_stream(buf))
        docs = _docs_from_events(events)
        assert len(docs) == 1
        scalars = [e["value"] for e in docs[0] if e["type"] == "scalar"]
        assert "a" in scalars
        assert "b" in scalars
        assert "1" in scalars


class TestStreamEventFields:
    """Binding-layer event dict field stability (stream_events.rs)."""

    def test_scalar_event_fields(self):
        yaml = "hello\n"
        events = list(pyrs_yaml.YAML().load_stream(io.StringIO(yaml)))
        scalars = [e for e in events if e["type"] == "scalar"]
        assert len(scalars) == 1
        ev = scalars[0]
        assert ev["value"] == "hello"
        assert ev["style"] in ("plain", "single-quoted", "double-quoted")
        assert isinstance(ev["line"], int)
        assert isinstance(ev["column"], int)
        assert ev["anchor"] is None
        assert ev["tag"] is None

    def test_mapping_event_fields(self):
        yaml = "k: v\n"
        events = list(pyrs_yaml.YAML().load_stream(io.StringIO(yaml)))
        maps = [e for e in events if e["type"] == "mapping_start"]
        assert len(maps) == 1
        ev = maps[0]
        assert ev["value"] is None
        assert ev["style"] is None
        assert isinstance(ev["line"], int)
        assert isinstance(ev["column"], int)

    def test_sequence_event_fields(self):
        yaml = "- 1\n- 2\n"
        events = list(pyrs_yaml.YAML().load_stream(io.StringIO(yaml)))
        seqs = [e for e in events if e["type"] == "sequence_start"]
        assert len(seqs) == 1
        ev = seqs[0]
        assert ev["value"] is None
        assert ev["anchor"] is None
        assert ev["tag"] is None

    def test_anchor_tag_propagates(self):
        yaml = "x: &myanchor !tag hello\n"
        events = list(pyrs_yaml.YAML().load_stream(io.StringIO(yaml)))
        scalars = [e for e in events if e["type"] == "scalar"]
        assert len(scalars) == 2  # key= x, value= hello
        tagged = [e for e in scalars if e["tag"] is not None]
        assert len(tagged) == 1
        ev = tagged[0]
        assert ev["value"] == "hello"
        # Anchor names are auto-numbered by the parser; just verify propagation.
        assert ev["anchor"] is not None
        assert ev["tag"] == "!tag"

    def test_multiple_document_event_structure(self):
        yaml = "---\na: 1\n---\nb: 2\n"
        events = list(pyrs_yaml.YAML().load_stream(io.StringIO(yaml)))
        types = [e["type"] for e in events]
        # stream_start → document_start → scalar/mapping → document_end → ...
        doc_starts = [i for i, t in enumerate(types) if t == "document_start"]
        doc_ends = [i for i, t in enumerate(types) if t == "document_end"]
        assert len(doc_starts) == 2
        assert len(doc_ends) == 2
        assert types[-1] == "stream_end"


class TestBlockScalarStyle:
    """stream_events style field for block scalars (audit §15)."""

    def test_literal_style_field(self):
        events = list(pyrs_yaml.YAML().load_stream(io.StringIO("k: |\n  hello\n")))
        scalars = [e for e in events if e["type"] == "scalar" and e["value"] != "k"]
        assert len(scalars) == 1
        assert scalars[0]["value"] == "hello\n"
        assert scalars[0]["style"] == "literal"

    def test_folded_style_field(self):
        events = list(pyrs_yaml.YAML().load_stream(io.StringIO("k: >\n  hello\n")))
        scalars = [e for e in events if e["type"] == "scalar" and e["value"] != "k"]
        assert len(scalars) == 1
        assert scalars[0]["value"] == "hello\n"
        assert scalars[0]["style"] == "folded"
