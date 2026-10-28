"""Streaming write tests: YAML.dump_stream / YAML.dump_file."""

import io
import subprocess
import sys

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


# The guard above answers at depth 1000, which means the writer needs a thousand of its own
# stack frames in place before it can refuse. Whether it has them is a frame-size question, and
# a frame-size question can only be asked against a stack of known size — so the child sets one.
_SMALL_STACK_DUMP_CHILD = """
import io, threading
import pyrs_yaml

threading.stack_size(256 * 1024)
doc = {}
cursor = doc
for _ in range(1100):
    cursor["x"] = {}
    cursor = cursor["x"]
outcome = []

def run():
    try:
        pyrs_yaml.YAML().dump_stream(io.StringIO(), [doc])
        outcome.append("WROTE")
    except pyrs_yaml.YamlMaxDepthError:
        outcome.append("MAXDEPTH")
    except BaseException as exc:
        outcome.append("OTHER " + type(exc).__name__)

thread = threading.Thread(target=run)
thread.start()
thread.join()
print(outcome[0] if outcome else "NO ANSWER", flush=True)
"""


def test_the_depth_guard_answers_on_a_small_thread_stack():
    """A deep document is refused by the guard, not by killing the interpreter.

    Measured on Windows 2026-10-28: a per-pair classification that pulls the scalar extractor and
    the complex-key delegation into the mapping writer's frame put the 1,100-level dump of this
    test's own fixture at 891 levels instead of a refusal, and it needed a 4 MB thread stack
    before the guard answered at all — where the writer before it answered inside 64 KB. A stack
    overflow is a fatal interpreter error, so the regression cannot be asserted in-process: it
    aborts the whole pytest session (exit 127) and the rest of the suite never runs. The child
    carries the small stack, and its death is an ordinary red.
    """
    proc = subprocess.run(
        [sys.executable, "-c", _SMALL_STACK_DUMP_CHILD],
        capture_output=True,
        text=True,
        timeout=180,
    )
    assert proc.returncode == 0, (
        "the depth guard never answered: the writer overflowed the 256 KB thread stack "
        f"(exit {proc.returncode})\n{proc.stderr[-400:]}"
    )
    assert proc.stdout.strip() == "MAXDEPTH", proc.stdout.strip()


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
