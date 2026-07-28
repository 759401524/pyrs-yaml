"""Integration tests for pyrs_yaml.parse_stream()."""

import pyrs_yaml
import pytest

# ============================================================================
# Generator Mode
# ============================================================================


class TestStreamYieldsEvents:
    """Test generator mode yields event dicts correctly."""

    def test_stream_yields_events(self):
        """parse_stream yields event dicts; first is stream_start."""
        events = list(pyrs_yaml.parse_stream("key: value"))
        assert len(events) > 0
        assert events[0]["type"] == "stream_start"

    def test_stream_scalar_event(self):
        """Scalar events have correct value, style fields."""
        events = list(pyrs_yaml.parse_stream('message: "hello world"'))
        scalar_events = [e for e in events if e["type"] == "scalar" and e["value"] == "hello world"]
        assert len(scalar_events) >= 1
        assert scalar_events[0]["style"] == "double_quoted"

    def test_stream_sequence_events(self):
        """Sequence produces sequence_start/sequence_end."""
        events = list(pyrs_yaml.parse_stream("- a\n- b\n- c"))
        types = [e["type"] for e in events]
        assert "sequence_start" in types
        assert "sequence_end" in types

    def test_stream_mapping_events(self):
        """Mapping produces mapping_start/mapping_end."""
        events = list(pyrs_yaml.parse_stream("key: value"))
        types = [e["type"] for e in events]
        assert "mapping_start" in types
        assert "mapping_end" in types

    def test_stream_document_boundaries(self):
        """Multi-doc YAML has document_start events."""
        events = list(pyrs_yaml.parse_stream("---\nkey1: val1\n---\nkey2: val2"))
        types = [e["type"] for e in events]
        assert types.count("document_start") == 2

    def test_stream_anchor_alias(self):
        """Anchor/alias produce scalar/alias events."""
        events = list(pyrs_yaml.parse_stream("defaults: &defaults\n  timeout: 30\nref: *defaults"))
        types = [e["type"] for e in events]
        assert "alias" in types
        alias_events = [e for e in events if e["type"] == "alias"]
        assert len(alias_events) == 1
        assert alias_events[0]["value"] == "defaults"

    def test_stream_includes_line_column(self):
        """Every event has int line/column."""
        events = list(pyrs_yaml.parse_stream("key: value"))
        for event in events:
            assert isinstance(event["line"], int)
            assert isinstance(event["column"], int)


# ============================================================================
# Callback Mode
# ============================================================================


class TestStreamCallbackMode:
    """Test callback mode of parse_stream."""

    def test_stream_callback_mode(self):
        """parse_stream with on_event calls function per event, returns None."""
        collected = []

        def on_event(event):
            collected.append(event)
            return True

        result = pyrs_yaml.parse_stream("key: value", on_event=on_event)
        assert result is None
        assert len(collected) > 0
        assert collected[0]["type"] == "stream_start"

    def test_stream_callback_early_exit(self):
        """Returning False from callback stops parsing early."""
        collected = []

        def on_event(event):
            collected.append(event)
            return event["type"] != "scalar"

        pyrs_yaml.parse_stream("key: value", on_event=on_event)
        types = [e["type"] for e in collected]
        assert "scalar" in types
        assert "mapping_end" not in types
        assert "document_end" not in types
        assert "stream_end" not in types

    def test_stream_callback_preserves_order(self):
        """Events are in document order."""
        collected = []

        def on_event(event):
            collected.append(event)
            return True

        pyrs_yaml.parse_stream("key: value", on_event=on_event)
        types = [e["type"] for e in collected]
        stream_start_idx = types.index("stream_start")
        doc_start_idx = types.index("document_start")
        mapping_start_idx = types.index("mapping_start")
        mapping_end_idx = types.index("mapping_end")
        doc_end_idx = types.index("document_end")
        stream_end_idx = types.index("stream_end")
        assert stream_start_idx < doc_start_idx < mapping_start_idx < mapping_end_idx < doc_end_idx < stream_end_idx


# ============================================================================
# Error Handling
# ============================================================================


class TestStreamErrorHandling:
    """Test error handling in parse_stream."""

    def test_stream_invalid_yaml_raises(self):
        """Malformed YAML raises YamlParseError."""
        with pytest.raises(pyrs_yaml.YamlParseError):
            list(pyrs_yaml.parse_stream("key: {unclosed"))

    def test_stream_invalid_bytes_raises(self):
        """Bad UTF-8 bytes raises YamlParseError."""
        with pytest.raises(pyrs_yaml.YamlParseError):
            list(pyrs_yaml.parse_stream(b"\xff\xfe"))


# ============================================================================
# Comment Events
# ============================================================================


class TestStreamCommentEvents:
    """Test comment event emission in parse_stream."""

    def test_stream_standalone_comment(self):
        """Standalone comment produces comment event with standalone=True."""
        events = list(pyrs_yaml.parse_stream("# comment\nkey: value"))
        comment_events = [e for e in events if e["type"] == "comment"]
        assert len(comment_events) == 1
        assert comment_events[0]["value"] == "comment"
        assert comment_events[0]["style"] == "standalone"

    def test_stream_inline_comment(self):
        """Inline comment produces comment event with standalone=False."""
        events = list(pyrs_yaml.parse_stream("key: value  # inline"))
        comment_events = [e for e in events if e["type"] == "comment"]
        assert len(comment_events) >= 1
        assert comment_events[0]["value"] == "inline"
        assert comment_events[0]["style"] == "inline"


# ============================================================================
# Full Event Types
# ============================================================================


class TestStreamFullEventTypes:
    """Test that all StreamEventType variants are present."""

    def test_stream_has_all_event_types(self):
        """Parse a comprehensive YAML document covering all event types."""
        yaml_str = "# doc comment\nkey1: value1\nseq:\n  - a\n  - b\nnested:\n  child: val\n"
        events = list(pyrs_yaml.parse_stream(yaml_str))
        types = set(e["type"] for e in events)
        assert "stream_start" in types
        assert "document_start" in types
        assert "mapping_start" in types
        assert "mapping_end" in types
        assert "sequence_start" in types
        assert "sequence_end" in types
        assert "scalar" in types
        assert "document_end" in types
        assert "stream_end" in types

    def test_stream_events_have_required_keys(self):
        """Every event dict has the required keys."""
        events = list(pyrs_yaml.parse_stream("key: value"))
        required_keys = {"type", "value", "style", "anchor", "tag", "line", "column"}
        for event in events:
            assert required_keys.issubset(event.keys())
            assert isinstance(event["type"], str)
            assert event["line"] is None or isinstance(event["line"], int)
            assert event["column"] is None or isinstance(event["column"], int)
