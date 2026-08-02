"""Integration tests for YAML.load_stream / load_stream_file."""

import io

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
        assert list(s) == []  # close 后 StopIteration

    def test_load_stream_error_then_stopiteration(self):
        """Error surfaces exactly once; subsequent __next__ returns StopIteration (no repeat raise)."""
        it = pyrs_yaml.YAML().load_stream(io.StringIO("key: {unclosed"))
        with pytest.raises(pyrs_yaml.YamlParseError):
            # 惰性迭代: 错误在解析到达非法 token 时抛出(可能先产出若干事件).
            while True:
                next(it)
        # 第二次 next → StopIteration(不重复抛错)
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
