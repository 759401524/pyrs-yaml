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
