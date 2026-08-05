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
