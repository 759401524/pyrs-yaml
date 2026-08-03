"""Integration tests for YAML.load_stream / load_stream_file."""

import io
import itertools

import pyrs_yaml
import pytest


class TestLoadStream:
    def test_load_stream_stringio(self):
        """load_stream yields event dicts from a StringIO file object."""
        events = list(pyrs_yaml.YAML().load_stream(io.StringIO("key: value")))
        assert events[0]["type"] == "stream_start"
        types = [e["type"] for e in events]
        assert "scalar" in types
        assert types[-1] == "stream_end"

    def test_load_stream_read_raises_surfaces_as_parse_error(self):
        """read() throwing exception surfaces as YamlParseError."""

        class BadFile:
            def read(self, n=-1):
                raise RuntimeError("disk failure")

        with pytest.raises(pyrs_yaml.YamlParseError, match="disk failure"):
            list(pyrs_yaml.YAML().load_stream(BadFile()))

    def test_load_stream_binary_file(self):
        """load_stream accepts a binary file object (read returns bytes)."""
        events = list(pyrs_yaml.YAML().load_stream(io.BytesIO(b"key: value")))
        assert any(e["type"] == "scalar" and e["value"] == "value" for e in events)

    def test_load_stream_no_read_raises_type_error(self):
        """Object without read() raises YamlTypeError with translated message."""
        with pytest.raises(pyrs_yaml.YamlTypeError, match="read"):
            pyrs_yaml.YAML().load_stream(object())  # type: ignore[arg-type]

    def test_load_stream_invalid_utf8_raises(self):
        """Invalid UTF-8 bytes raise YamlParseError."""
        with pytest.raises(pyrs_yaml.YamlParseError):
            list(pyrs_yaml.YAML().load_stream(io.BytesIO(b"\xff\xfe")))

    def test_load_stream_invalid_yaml_raises_with_line_col(self):
        """Malformed YAML raises YamlParseError (position via ScanError marker)."""
        with pytest.raises(pyrs_yaml.YamlParseError) as excinfo:
            list(pyrs_yaml.YAML().load_stream(io.StringIO("key: {unclosed")))
        assert "line" in str(excinfo.value).lower() or "col" in str(excinfo.value).lower()

    def test_load_stream_close_is_idempotent(self):
        """close() can be called multiple times and stops reading."""
        s = pyrs_yaml.YAML().load_stream(io.StringIO("key: value"))
        s.close()
        s.close()
        assert list(s) == []

    def test_load_stream_error_then_stopiteration(self):
        """Error surfaces exactly once; subsequent __next__ returns StopIteration (no repeat raise)."""
        it = pyrs_yaml.YAML().load_stream(io.StringIO("key: {unclosed"))
        with pytest.raises(pyrs_yaml.YamlParseError):
            while True:
                next(it)
        with pytest.raises(StopIteration):
            next(it)


class TestLoadStreamFile:
    def test_load_stream_file_roundtrip(self, tmp_path):
        """load_stream_file parses a file on disk."""
        p = tmp_path / "doc.yaml"
        p.write_text("key: value\n", encoding="utf-8")
        events = list(pyrs_yaml.YAML().load_stream_file(str(p)))
        assert events[0]["type"] == "stream_start"
        assert events[-1]["type"] == "stream_end"

    def test_load_stream_file_missing_raises_ioerror(self, tmp_path):
        """Missing file raises OSError."""
        with pytest.raises(OSError):
            pyrs_yaml.YAML().load_stream_file(str(tmp_path / "nope.yaml"))

    def test_load_stream_file_no_schema_param(self):
        """load_stream_file takes only path (no schema/max_depth params)."""
        import inspect

        sig = inspect.signature(pyrs_yaml.YAML.load_stream_file)
        assert set(sig.parameters) <= {"self", "path"}
        sig2 = inspect.signature(pyrs_yaml.YAML.load_stream)
        assert set(sig2.parameters) <= {"self", "file_obj"}


# ============================================================================
# Parity with parse_stream (§7 tests 1, 1a, 1b, 1c)
# ============================================================================

INPUTS = [
    "key: value",
    "a:\n  b: 1\n  c: [1, 2, 3]",
    "# comment\nkey: value  # inline",
    "defaults: &defaults\n  timeout: 30\nref: *defaults",
    "---\nkey1: val1\n---\nkey2: val2",
    "- a\r\n- b\r\n- c",
    "nested:\n  child: |\n    literal text\n",
    'msg: "hello\\nworld"',
]


def _normalize_stream_events(events, streaming):
    out = []
    for e in events:
        if e["type"] == "comment":
            continue
        e = dict(e)
        if streaming and e["type"] == "alias":
            assert e["value"] is not None and len(e["value"]) > 0
            e["value"] = "<alias>"
        elif streaming and e["anchor"] is not None:
            assert e["anchor"].startswith("anchor_")
            e["anchor"] = "<anchor>"
        elif not streaming and e["type"] == "alias":
            e["value"] = "<alias>"
        elif not streaming and e["anchor"] is not None:
            e["anchor"] = "<anchor>"
        out.append(e)
    return out


@pytest.mark.parametrize("yaml_str", INPUTS)
def test_parity_structure_events(yaml_str):
    streamed = list(pyrs_yaml.YAML().load_stream(io.StringIO(yaml_str)))
    parsed = list(pyrs_yaml.parse_stream(yaml_str))
    assert _normalize_stream_events(streamed, True) == _normalize_stream_events(parsed, False)


@pytest.mark.parametrize("yaml_str", ["---\na: 1\n---\nb: 2\n", "a: 1\n---\nb: 2\n"])
def test_parity_multidoc_sequence(yaml_str):
    streamed = [e["type"] for e in pyrs_yaml.YAML().load_stream(io.StringIO(yaml_str))]
    parsed = [e["type"] for e in pyrs_yaml.parse_stream(yaml_str) if e["type"] != "comment"]
    assert streamed == parsed
    assert streamed.count("document_start") == 2
    assert streamed[-1] == "stream_end"


def test_parity_cross_document_alias_load_stream_succeeds():
    """load_stream does not validate cross-doc aliases (saphyr streaming parser)."""
    events = list(pyrs_yaml.YAML().load_stream(io.StringIO("---\na: &x 1\n---\nb: *x\n")))
    assert any(e["type"] == "alias" for e in events)


def test_parity_cross_document_alias_parse_stream_raises():
    """parse_stream (full parser) validates cross-doc aliases."""
    with pytest.raises(pyrs_yaml.YamlParseError):
        list(pyrs_yaml.parse_stream("---\na: &x 1\n---\nb: *x\n"))


@pytest.mark.parametrize("empty", ["", "  \n \n", "\n\n"])
def test_parity_empty_input(empty):
    streamed = [e["type"] for e in pyrs_yaml.YAML().load_stream(io.StringIO(empty))]
    parsed = [e["type"] for e in pyrs_yaml.parse_stream(empty)]
    assert parsed == []
    assert streamed == ["stream_start", "stream_end"]


def test_parity_line_column_equal():
    yaml_str = "a: 1\nb:\n  - 2\n  - 3\n"
    streamed = pyrs_yaml.YAML().load_stream(io.StringIO(yaml_str))
    parsed = [e for e in pyrs_yaml.parse_stream(yaml_str) if e["type"] != "comment"]
    for se, pe in zip(streamed, parsed):
        assert (se["line"], se["column"]) == (pe["line"], pe["column"])


def test_parity_all_seven_keys_present():
    keys = {"type", "value", "style", "anchor", "tag", "line", "column"}
    for e in pyrs_yaml.YAML().load_stream(io.StringIO("key: value")):
        assert keys.issubset(e.keys())


# ============================================================================
# §7 tests 2, 3, 7 — memory-bound, early termination, free-threaded
# ============================================================================


class TrackingFile(io.StringIO):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.read_calls = 0

    def read(self, n=-1):  # type: ignore[override]
        self.read_calls += 1
        return super().read(n)


def test_early_termination_stops_reading():
    big = "".join(f"key{i}: value{i}\n" for i in range(10000))
    f = TrackingFile(big)
    s = pyrs_yaml.YAML().load_stream(f)
    for _ in itertools.islice(s, 20):
        pass
    calls_after_20 = f.read_calls
    s.close()
    for _ in itertools.islice(s, 20):
        pass
    assert f.read_calls == calls_after_20


def test_early_termination_break_stops_reading():
    big = "".join(f"key{i}: value{i}\n" for i in range(10000))
    f = TrackingFile(big)
    for e in pyrs_yaml.YAML().load_stream(f):
        if e["type"] == "scalar":
            break
    import sys

    if sys.implementation.name == "cpython":
        assert f.read_calls < 100


def test_memory_bound_load_stream_file(tmp_path):
    pytest.importorskip("psutil", reason="psutil provides RSS measurement")
    path = tmp_path / "big.yaml"
    with path.open("w", encoding="utf-8") as fh:
        for i in range(1_000_000):
            fh.write(f"k{i}: v{i}\n")
    import psutil

    proc = psutil.Process()
    before = proc.memory_info().rss
    count = 0
    for e in pyrs_yaml.YAML().load_stream_file(str(path)):
        if e["type"] == "scalar":
            count += 1
    after = proc.memory_info().rss
    assert count > 0
    assert after - before < 128 * 1024 * 1024
