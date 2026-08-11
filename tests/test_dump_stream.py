"""Streaming write tests: YAML.dump_stream / YAML.dump_file."""

import io

import pytest

import pyrs_yaml


def test_dump_stream_single_matches_safe_dump():
    buf = io.StringIO()
    pyrs_yaml.YAML().dump_stream(buf, [{"a": 1}])
    assert buf.getvalue() == pyrs_yaml.safe_dump({"a": 1})


def test_dump_stream_multi_doc_separators():
    buf = io.StringIO()
    pyrs_yaml.YAML().dump_stream(buf, [{"a": 1}, {"b": 2}, {"c": 3}])
    out = buf.getvalue()
    assert out.count("---") == 2
    assert list(pyrs_yaml.safe_loads(out)) == [{"a": 1}, {"b": 2}, {"c": 3}]


def test_dump_stream_empty_iterable_writes_nothing():
    buf = io.StringIO()
    pyrs_yaml.YAML().dump_stream(buf, [])
    assert buf.getvalue() == ""


def test_dump_stream_explicit_start_end():
    buf = io.StringIO()
    pyrs_yaml.YAML().dump_stream(buf, [{"a": 1}], explicit_start=True, explicit_end=True)
    assert buf.getvalue() == "---\na: 1\n...\n"


def test_dump_stream_yaml_document_preserves_comments():
    doc = pyrs_yaml.parse("a: 1  # keep me\n")
    buf = io.StringIO()
    pyrs_yaml.YAML().dump_stream(buf, [doc])
    assert "# keep me" in buf.getvalue()


def test_dump_stream_mixed_item_types():
    doc = pyrs_yaml.parse("k: v\n")
    buf = io.StringIO()
    pyrs_yaml.YAML().dump_stream(buf, [doc, {"x": 1}])
    out = buf.getvalue()
    assert out.count("---") == 1
    assert len(list(pyrs_yaml.safe_loads(out))) == 2


def test_dump_stream_sort_keys():
    buf = io.StringIO()
    pyrs_yaml.YAML().dump_stream(buf, [{"b": 1, "a": 2}], sort_keys=True)
    assert buf.getvalue() == "a: 2\nb: 1\n"


def test_dump_stream_string_keyword_quoted():
    buf = io.StringIO()
    pyrs_yaml.YAML().dump_stream(buf, [{"a": "true"}])
    assert buf.getvalue() == 'a: "true"\n'


def test_dump_stream_compact_mapping_not_sorted():
    buf = io.StringIO()
    pyrs_yaml.YAML().dump_stream(buf, [[{"b": 1, "a": 2}]], sort_keys=True)
    assert buf.getvalue() == "- b: 1\n  a: 2\n"


def test_dump_stream_max_depth_error():
    d = {}
    cur = d
    for _ in range(1100):
        cur["x"] = {}
        cur = cur["x"]
    buf = io.StringIO()
    with pytest.raises(pyrs_yaml.YamlMaxDepthError):
        pyrs_yaml.YAML().dump_stream(buf, [d])


def test_dump_stream_object_without_write_raises_type_error():
    with pytest.raises(pyrs_yaml.YamlTypeError):
        pyrs_yaml.YAML().dump_stream(object(), [{"a": 1}])


def test_dump_stream_write_error_wrapped_as_ioerror():
    class Broken:
        def write(self, _s):
            raise OSError("boom")

    with pytest.raises(OSError):
        pyrs_yaml.YAML().dump_stream(Broken(), [{"a": 1}])


def test_dump_stream_iterable_error_propagates_and_partial_output():
    def gen():
        yield {"a": 1}
        raise ValueError("mid-stream")

    buf = io.StringIO()
    with pytest.raises(ValueError):
        pyrs_yaml.YAML().dump_stream(buf, gen())
    assert buf.getvalue() == "a: 1\n"


def test_dump_stream_write_returns_none_accepted():
    class NoneWriter:
        def __init__(self):
            self.parts = []

        def write(self, s):
            self.parts.append(s)

    w = NoneWriter()
    pyrs_yaml.YAML().dump_stream(w, [{"a": 1}])
    assert "".join(w.parts) == "a: 1\n"


def test_dump_file_roundtrip(tmp_path):
    p = tmp_path / "out.yaml"
    pyrs_yaml.YAML().dump_file(str(p), [{"a": 1}, {"b": 2}])
    assert list(pyrs_yaml.safe_loads(p.read_text())) == [{"a": 1}, {"b": 2}]


def test_dump_file_explicit_flags(tmp_path):
    p = tmp_path / "out.yaml"
    pyrs_yaml.YAML().dump_file(str(p), [{"a": 1}], explicit_start=True, explicit_end=True)
    assert p.read_text() == "---\na: 1\n...\n"


def test_dump_stream_large_stream_stays_correct():
    docs = [{"doc": i, "payload": "x" * 100} for i in range(500)]
    buf = io.StringIO()
    pyrs_yaml.YAML().dump_stream(buf, docs)
    reloaded = list(pyrs_yaml.safe_loads(buf.getvalue()))
    assert len(reloaded) == 500
    assert reloaded[0]["doc"] == 0
    assert reloaded[-1]["doc"] == 499


def test_dump_file_chunked_write(tmp_path):
    """Writing a large doc triggers chunk-level flush."""
    p = tmp_path / "out.yaml"
    pyrs_yaml.YAML().dump_file(str(p), [{"a": 1, "b": 2, "c": 3, "d": 4}])
    assert p.read_text().strip() == "a: 1\nb: 2\nc: 3\nd: 4"


def test_dump_file_multi_doc(tmp_path):
    """Multiple docs written to file."""
    p = tmp_path / "out.yaml"
    pyrs_yaml.YAML().dump_file(str(p), [{"a": 1}, {"b": 2}])
    assert list(pyrs_yaml.safe_loads(p.read_text())) == [{"a": 1}, {"b": 2}]


def test_dump_file_flush_on_finish(tmp_path):
    """Writer flushes remaining data on finish."""
    p = tmp_path / "out.yaml"
    pyrs_yaml.YAML().dump_file(str(p), [{"a": 1}])
    text = p.read_text()
    assert text.strip() == "a: 1"
