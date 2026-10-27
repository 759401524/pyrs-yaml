"""Integration tests for YAML.load_stream / load_stream_file."""

import gc
import io
import itertools
import tracemalloc

import pytest

import pyrs_yaml


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


def test_parity_cross_document_alias_load_stream_raises():
    """load_stream validates cross-doc aliases (granit-parser stricter validation)."""
    with pytest.raises(pyrs_yaml.YamlParseError):
        list(pyrs_yaml.YAML().load_stream(io.StringIO("---\na: &x 1\n---\nb: *x\n")))


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
    # `YamlStream` is lazy and has no len(), so the comparison below needs both sides materialised.
    streamed = list(pyrs_yaml.YAML().load_stream(io.StringIO(yaml_str)))
    parsed = [e for e in pyrs_yaml.parse_stream(yaml_str) if e["type"] != "comment"]
    # `zip()` truncates to the shorter side: one producer emitting 13 events and the other 3 would
    # compare three pairs and report green. Measured 2026-10-25: 13 events on both sides.
    assert len(streamed) == len(parsed) >= 13, f"streamed {len(streamed)} events vs parsed {len(parsed)}"
    for se, pe in zip(streamed, parsed):
        assert (se["line"], se["column"]) == (pe["line"], pe["column"])


def test_parity_all_seven_keys_present():
    keys = {"type", "value", "style", "anchor", "tag", "line", "column"}
    events = list(pyrs_yaml.YAML().load_stream(io.StringIO("key: value")))
    # Measured 2026-10-25: 8 events for one pair. A stream that emitted none would inspect no event.
    assert len(events) >= 8, f"load_stream emitted {len(events)} events, so no event was inspected"
    for e in events:
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


_SMALL_LINES = 2_000
# The ratio is the assertion. A single-size ceiling cannot tell a bounded reader from one that
# accumulates slowly; 100x the document can.
_BIG_LINES = 100 * _SMALL_LINES
# The bound is a fraction of the buffering route's own growth, measured in the same run on the same
# interpreter, because an absolute byte allowance is not portable. Measured 2026-10-27 on this host:
# the 1x and 100x streaming readings are 3,168 bytes and 3,168 bytes under CPython 3.12 and 3,200 /
# 3,200 under 3.14t - zero apart on both - while holding every event costs 1,995,530 / 201,849,970
# and 2,423,458 / 245,041,146. The `test-freethreaded (3.14t)` leg on ubuntu-latest read **292,544**
# for the 1x document, which a 262,144-byte allowance written from a Windows reading rejected for a
# reason that has nothing to do with the claim. Growth is the claim, so growth is what gets compared:
# a hundredfold document may add at most 1/100 of what a buffering reader adds to itself.
_GROWTH_SHARE = 100
# What that bound works out to in the units the defect would occupy. Over the 400,006 events the 100x
# document emits it is 4.996 bytes per event under 3.12 and 6.065 under 3.14t, so retaining a hundred
# bytes per event - a leaked reference per node, say - fails. This ceiling is what stops the fraction
# from going slack on some future interpreter where the buffering reference happens to be large.
_PER_EVENT_CEILING = 100


def _write(path, lines):
    with path.open("w", encoding="utf-8") as fh:
        for i in range(lines):
            fh.write(f"k{i}: v{i}\n")
    return str(path)


def _streamed_bytes(path):
    """Events consumed, and the traced Python bytes still held once they are all dropped.

    `tracemalloc` counts the interpreter's own domain, so this measures the events the route hands
    out, not the engine's read buffer (a fixed chunk, never accumulated here). That is the half the
    old RSS assertion was credited with, and unlike it, this runs on every interpreter.
    """
    tracemalloc.start()
    base = tracemalloc.get_traced_memory()[0]
    stream = pyrs_yaml.YAML().load_stream_file(path)
    events = 0
    for event in stream:
        events += 1
        del event
    stream.close()
    gc.collect()
    held = tracemalloc.get_traced_memory()[0] - base
    tracemalloc.stop()
    return events, max(held, 0)


def _held_bytes(path):
    """The same walk keeping every event: what a buffering reader looks like from the Python side."""
    tracemalloc.start()
    base = tracemalloc.get_traced_memory()[0]
    kept = [event for event in pyrs_yaml.YAML().load_stream_file(path)]
    gc.collect()
    held = tracemalloc.get_traced_memory()[0] - base
    del kept
    tracemalloc.stop()
    return max(held, 0)


def test_streaming_a_hundred_times_more_document_costs_no_more_memory(tmp_path):
    small = _write(tmp_path / "small.yaml", _SMALL_LINES)
    big = _write(tmp_path / "big.yaml", _BIG_LINES)

    small_events, small_bytes = _streamed_bytes(small)
    big_events, big_bytes = _streamed_bytes(big)
    buffered_growth = _held_bytes(big) - _held_bytes(small)

    assert big_events > 40 * small_events, "the 100x fixture is not 100x the events, so the pair proves nothing"
    assert buffered_growth > 0, (
        f"holding every event of the 100x document grows by {buffered_growth} bytes over the 1x one, "
        "so the reference this bound is scaled by has collapsed and the ratio means nothing"
    )
    assert abs(big_bytes - small_bytes) <= buffered_growth // _GROWTH_SHARE, (
        f"the 100x document holds {big_bytes} traced bytes against the 1x document's {small_bytes}, "
        f"and a bounded reader may add at most {buffered_growth // _GROWTH_SHARE} - 1/{_GROWTH_SHARE} "
        "of what an accumulating one adds over the same document"
    )
    assert buffered_growth // _GROWTH_SHARE < _PER_EVENT_CEILING * big_events, (
        f"the bound allows {buffered_growth // _GROWTH_SHARE} bytes across {big_events} events, i.e. "
        f"over {_PER_EVENT_CEILING} per event, so it would not notice a reader keeping one leaked "
        "reference per node; the ratio has gone slack and needs restating"
    )


def test_the_streaming_memory_guard_can_be_reddened(tmp_path):
    """The guard's own predicate, applied to a reader that does accumulate.

    Measured 2026-10-27 on the two interpreters of this host: the buffering walk costs 1,995,530
    bytes at 1x and 201,849,970 at 100x under 3.12, and 2,423,458 and 245,041,146 under 3.14t. The
    streaming walk's own growth is 0 on both. So the shape that accumulates violates the bound the
    shape that does not satisfies with its whole budget unused - which is what makes the first test a
    guard rather than a number.
    """
    small = _write(tmp_path / "small.yaml", _SMALL_LINES)
    big = _write(tmp_path / "big.yaml", _BIG_LINES)

    small_held = _held_bytes(small)
    big_held = _held_bytes(big)
    growth = big_held - small_held

    assert growth > growth // _GROWTH_SHARE, (
        f"holding every event of both documents grows by {growth} bytes, which fits inside its own "
        f"1/{_GROWTH_SHARE} - the bound the guard applies is then measuring nothing"
    )


class _ByteReader:
    """File-like object that yields exactly one byte per read() call.

    Forces ChunkCharIter::fill_pyobj / push_bytes to handle multi-byte UTF-8
    split across chunk boundaries, and a trailing incomplete sequence at EOF.
    """

    def __init__(self, data: bytes):
        self._data = data
        self._pos = 0

    def read(self, n: int = -1):
        if self._pos >= len(self._data):
            return b""
        b = self._data[self._pos]
        self._pos += 1
        return bytes([b])


class TestChunkedUtf8:
    """Cross-chunk boundary handling for incremental readers (audit §15)."""

    def test_multibyte_char_split_across_chunks(self):
        yaml = "a: \u4e2d\n".encode("utf-8")
        events = list(pyrs_yaml.YAML().load_stream(_ByteReader(yaml)))
        types = [e["type"] for e in events]
        # mapping with two scalars parses despite byte-at-a-time delivery.
        assert types.count("scalar") == 2

    def test_trailing_incomplete_utf8_at_eof(self):
        # 0xef is the lead byte of a 3-byte sequence that never completes;
        # the incremental reader reports an invalid-UTF-8 parse error instead
        # of panicking.
        yaml = b"a: b\xef\n"
        with pytest.raises(pyrs_yaml.YamlParseError):
            list(pyrs_yaml.YAML().load_stream(_ByteReader(yaml)))

    def test_invalid_utf8_byte_in_value(self):
        yaml = b"a: \xff\n"
        with pytest.raises(pyrs_yaml.YamlParseError):
            list(pyrs_yaml.YAML().load_stream(_ByteReader(yaml)))


class TestParseStreamCallback:
    """parse_stream(on_event=...) callback contract (audit §15)."""

    def test_early_termination_on_false(self):
        seen = []

        def cb(ev):
            seen.append(ev["type"])
            return False

        pyrs_yaml.parse_stream("a: 1\n---\nb: 2\n", on_event=cb)
        # Returning False stops the stream immediately after the first event.
        assert seen == ["stream_start"]

    def test_continue_consumes_all(self):
        seen = []

        def cb(ev):
            seen.append(ev["type"])
            return True

        pyrs_yaml.parse_stream("a: 1\n---\nb: 2\n", on_event=cb)
        assert seen.count("document_start") == 2
