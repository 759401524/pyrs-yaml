---
title: Streaming Parse
description: Parse and write YAML streams lazily with constant memory, suitable for large files and multi-document streams.
tags:
  - docs
status: new
---

## Streaming Parse

!!! note "Streaming vs full parse"
    Streaming parse uses lazy event iteration with O(anchors + 64KB chunk)
    memory, independent of input size. Use it for large files where loading
    the entire document into memory is impractical.

`YAML.load_stream(file_obj)` and `YAML.load_stream_file(path)` iterate YAML
events lazily — memory usage is O(anchors + 64KB chunk), independent of input
size. Suitable for 100MB+ files.

```python
from pyrs_yaml import YAML

for event in YAML().load_stream_file("huge.yaml"):
    print(event["type"], event["value"])
```

### Differences from parse_stream

| Behavior | load_stream | parse_stream |
| --- | --- | --- |
| Memory | O(anchors + chunk) | O(input) |
| Comments | Not emitted | Emitted |
| Anchor names | `anchor_{id}` | Original names |
| Errors | No source snippet | Source snippet |
| Empty input | `[stream_start, stream_end]` | `[]` |
| Tag handlers | Not applied | Applied (YAML.parse) |

### Resource management

Call `close()` when you stop early — it is the only guaranteed release point
(PyPy's delayed GC does not guarantee `Drop` timing). `close()` is idempotent
and does **not** close the file object you passed in.

### Streaming Write

`YAML().dump_stream(file_obj, iterable, ...)` and `YAML().dump_file(path, iterable, ...)`
serialize documents one at a time using constant memory (O(single-doc + 64KB chunk)),
independent of the total number of documents.

```python
from pyrs_yaml import YAML

buf = io.StringIO()
YAML().dump_stream(buf, [{"a": 1}, {"b": 2}])
# buf.getvalue() == "a: 1\n---\nb: 2\n"
```

#### Separator rules

- No `---` before the first document; every subsequent document gets a leading `---`.
- `explicit_start=True` adds `---` before the first document.
- `explicit_end=True` adds `...` after the last document.
- An empty iterable writes zero bytes.

#### Error semantics

A mid-stream failure (iterator exception, serialization error, write failure) leaves
already-written output in the target — there is no rollback for partial output.

#### Differences from `safe_dump`

| Aspect | dump_stream / dump_file | safe_dump |
|--------|------------------------|-----------|
| Output | Multi-document stream | Single document |
| Memory | O(single-doc + 64KB) | O(input) |
| Item type | `YamlDocument` (preserves comments/anchors) or plain Python objects | Single Python object |

#### Sort keys

Pass `sort_keys=True` to emit mapping keys in sorted order, matching
`safe_dump`'s `sort_keys` behavior.

### StreamIterator

The `StreamIterator` class is yielded by `parse_stream()` and `YAML().load_stream()` / `YAML().load_stream_file()`. It implements the iterator protocol and yields event dicts one at a time.

```python
from pyrs_yaml import parse_stream

iterator = parse_stream("key: value\n---\na: 1")
for event in iterator:
    print(event["type"], event["value"])
```

#### Iterator Protocol

`StreamIterator` implements `__iter__` (returns `self`) and `__next__`:

```python
def __iter__() -> StreamIterator: ...
def __next__() -> dict | None: ...
```

When the stream is exhausted, `__next__()` returns `None` (it does **not** raise `StopIteration`).

#### Event Dict Keys

Each event dict contains the following keys:

| Key | Type | Description |
| --- | --- | --- |
| `type` | `str` | Event type (see below) |
| `value` | `str` or `None` | Scalar value, alias name, or comment text |
| `style` | `str` or `None` | Scalar quote style: `"plain"`, `"single_quoted"`, `"double_quoted"`, `"literal"`, `"folded"`; for comments: `"standalone"` or `"inline"` |
| `anchor` | `str` or `None` | Anchor name (`&name`) |
| `tag` | `str` or `None` | Tag string (`!!str`, `!custom`) |
| `line` | `int` | Line number (0-indexed) |
| `column` | `int` | Column number (0-indexed) |

#### Event Types

| `type` | When Emitted |
| --- | --- |
| `stream_start` | Start of a YAML stream |
| `stream_end` | End of the stream |
| `document_start` | Start of a document |
| `document_end` | End of a document |
| `mapping_start` | Start of a mapping |
| `mapping_end` | End of a mapping |
| `sequence_start` | Start of a sequence |
| `sequence_end` | End of a sequence |
| `scalar` | A scalar value |
| `alias` | An alias reference (`*name`) |
| `comment` | A YAML comment |

#### Differences from `load_stream`

`parse_stream()` returns a `StreamIterator` that emits comments and preserves original anchor names. `YAML().load_stream()` / `YAML().load_stream_file()` return a `YamlStream` with different defaults (see the comparison table above).
