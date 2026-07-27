# Design: Streaming YAML Parser (`parse_stream`)

## Overview

A low-memory, event-based streaming YAML parser that exposes saphyr-parser events to Python via both a generator and a callback API. Enables processing of multi-GB YAML files without building a full AST in memory.

## Context

- Project: pyyaml-rs v0.4.0
- Core parser: saphyr-parser (event-based, YAML 1.2 compliant)
- Current AST parser builds full `CustomNode` tree in memory — `parse()`, `safe_load()`, etc. all hold the entire document tree
- saphyr-parser `SpannedEventReceiver` is the foundation; `AstReceiver` is the existing implementation

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| API style | Generator + callback | Generator is idiomatic Python; callback allows early exit and lower overhead |
| Event format | Dicts (fast path) + Python classes (inspection) | Dicts avoid overhead in hot loops; classes give IDE/type support |
| Comments | Included as stream events | Round-trip tooling needs comment positions; saphyr tracks spans |
| Error handling | Raises `YamlParseError` | Consistent with `parse()` / `safe_load()` |
| GIL handling | Released during parsing | Heavy Rust work outside GIL; callbacks acquire GIL per event |

## Python API

### Generator mode (primary)

```python
for event in pyyaml_rs.parse_stream(yaml_text):
    if event["type"] == "scalar":
        print(event["value"])
    elif event["type"] == "mapping_start":
        print(f"mapping starts at line {event['line']}")
```

### Callback mode

```python
def on_event(event):
    if event["type"] == "scalar" and event["value"] == "STOP":
        return False  # stop parsing early
    return True       # continue

pyyaml_rs.parse_stream(yaml_text, on_event=on_event)
```

### Event objects

**Dict shape** (returned by generator, passed to callback):

```python
{
    "type": "scalar",          # str: scalar, mapping_start, mapping_end,
                                #   sequence_start, sequence_end,
                                #   document_start, document_end,
                                #   alias, comment
    "value": "hello",          # str or None (for mapping/sequence start/end)
    "style": "plain",          # str or None (plain, single_quoted, double_quoted, literal, folded)
    "anchor": None,            # str or None
    "tag": None,               # str or None (e.g., "!!int")
    "line": 0,                 # int, 0-based
    "column": 0,               # int, 0-based
}
```

**Event classes** (for inspection / construction):

```python
class StreamEvent:
    type: str
    line: int
    column: int

class ScalarEvent(StreamEvent):
    value: str
    style: str           # "plain" | "single_quoted" | "double_quoted" | "literal" | "folded"
    anchor: str | None
    tag: str | None

class MappingStartEvent(StreamEvent):
    anchor: str | None
    tag: str | None

class SequenceStartEvent(StreamEvent):
    anchor: str | None
    tag: str | None

class AliasEvent(StreamEvent):
    anchor: str

class CommentEvent(StreamEvent):
    text: str            # comment text without '#' prefix
    standalone: bool     # True = standalone line comment, False = inline
```

### Error handling

Raises `YamlParseError` with line/column info on parse failure, same as existing `parse()`.

```python
try:
    for event in pyyaml_rs.parse_stream(broken_yaml):
        pass
except pyyaml_rs.YamlParseError as e:
    print(f"Parse error at {e.line}:{e.column}: {e.message}")
```

## Rust Implementation

### New module: `src/parser/stream.rs`

- `StreamReceiver` struct implements `SpannedEventReceiver`
- Stores a reference to the Python callback (as `Option<Py<PyCallable>>`) — set when callback mode is used
- Uses `Py<PyAny>` channel or direct `Python::allow_threads` + callback invocation
- For generator mode: uses a `pyo3::PyResult<Option<HashMap<String, PyAny>>>` return per event

### Key function signatures

```rust
pub fn parse_stream(
    py: Python,
    yaml: &str,
    on_event: Option<&Bound<'_, PyCallable>>,
) -> PyResult<Py<PyAny>>
```

When `on_event` is `None`: returns a generator (Python iterator yielding dicts).
When `on_event` is `Some`: calls the callable per event; if it returns `False`, terminates early.

### GIL strategy

- `py.allow_threads(|| { parser.load(&mut receiver, false) })` — releases GIL during actual parsing
- Each event delivered to Python callback acquires GIL transiently
- Generator `__next__` acquires GIL to construct the Python dict and yield it

### Event emission

`StreamReceiver::on_event` maps each saphyr `Event` variant:

| saphyr event | Stream event type | dict fields |
|---|---|---|
| `StreamStart` | `stream_start` | `line`, `column` |
| `StreamEnd` | `stream_end` | `line`, `column` |
| `DocumentStart` | `document_start` | `line`, `column` |
| `DocumentEnd` | `document_end` | `line`, `column` |
| `Scalar(v, style, anchor_id, tag)` | `scalar` | `value`, `style`, `anchor`, `tag`, `line`, `column` |
| `MappingStart(anchor_id, tag)` | `mapping_start` | `anchor`, `tag`, `line`, `column` |
| `MappingEnd` | `mapping_end` | `line`, `column` |
| `SequenceStart(anchor_id, tag)` | `sequence_start` | `anchor`, `tag`, `line`, `column` |
| `SequenceEnd` | `sequence_end` | `line`, `column` |
| `Alias(anchor_id)` | `alias` | `anchor`, `line`, `column` |
| `Nothing` | (skipped) | — |

Comments are handled by the existing `extract_comments()` and `extract_anchors()` helpers from `comment.rs`. Stream events for comments emit after the preceding structural event with `standalone` flag.

## Testing

1. Unit tests for `StreamReceiver` in Rust (same pattern as `AstReceiver` tests)
2. Python tests in `tests/test_streaming.py`:
   - Generator iteration over simple and complex YAML
   - Callback termination (early exit)
   - Comment event emission
   - Error handling (malformed YAML raises `YamlParseError`)
   - Round-trip: `parse_stream` → reconstruct YAML matches input
   - Large document memory usage (100k+ documents)
3. Integration: `parse_stream` with `safe_load` equivalence for standard documents

## Files affected

- **New**: `src/parser/stream.rs` — `StreamReceiver` implementation
- **Modified**: `src/parser/mod.rs` — add `parse_stream` Rust function
- **Modified**: `src/lib.rs` — expose `parse_stream` as PyO3 function
- **New**: `tests/test_streaming.py` — Python tests
- **New**: `src/python/streaming.pyi` — type stubs
- **New**: `docs/superpowers/specs/2026-07-27-streaming-parser-design.md` — this document
