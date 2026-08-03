# Streaming Parse

`YAML.load_stream(file_obj)` and `YAML.load_stream_file(path)` iterate YAML
events lazily — memory usage is O(anchors + 64KB chunk), independent of input
size. Suitable for 100MB+ files.

```python
from pyrs_yaml import YAML

for event in YAML().load_stream_file("huge.yaml"):
    print(event["type"], event["value"])
```

## Differences from parse_stream

| Behavior | load_stream | parse_stream |
| --- | --- | --- |
| Memory | O(anchors + chunk) | O(input) |
| Comments | Not emitted | Emitted |
| Anchor names | `anchor_{id}` | Original names |
| Errors | No source snippet | Source snippet |
| Empty input | `[stream_start, stream_end]` | `[]` |
| Tag handlers | Not applied | Applied (YAML.parse) |

## Resource management

Call `close()` when you stop early — it is the only guaranteed release point
(PyPy's delayed GC does not guarantee `Drop` timing). `close()` is idempotent
and does **not** close the file object you passed in.

## Streaming Write

`YAML().dump_stream(file_obj, iterable, ...)` and `YAML().dump_file(path, iterable, ...)`
serialize documents one at a time using constant memory (O(single-doc + 64KB chunk)),
independent of the total number of documents.

```python
from pyrs_yaml import YAML

buf = io.StringIO()
YAML().dump_stream(buf, [{"a": 1}, {"b": 2}])
# buf.getvalue() == "a: 1\n---\nb: 2\n"
```

### Separator rules

- No `---` before the first document; every subsequent document gets a leading `---`.
- `explicit_start=True` adds `---` before the first document.
- `explicit_end=True` adds `...` after the last document.
- An empty iterable writes zero bytes.

### Error semantics

A mid-stream failure (iterator exception, serialization error, write failure) leaves
already-written output in the target — there is no rollback for partial output.

### Differences from `safe_dump`

| Aspect | dump_stream / dump_file | safe_dump |
|--------|------------------------|-----------|
| Output | Multi-document stream | Single document |
| Memory | O(single-doc + 64KB) | O(input) |
| Item type | `YamlDocument` (preserves comments/anchors) or plain Python objects | Single Python object |

### Sort keys

Pass `sort_keys=True` to emit mapping keys in sorted order, matching
`safe_dump`'s `sort_keys` behavior.
