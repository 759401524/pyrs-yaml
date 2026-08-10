# YAML Class

The `YAML` class is a configured parser instance that controls parsing behavior through `typ`, `schema`, `max_depth`, and `allow_duplicate_keys` settings. It supports round-trip (`rt`), safe, and full YAML parsing modes.

## Overview

```python
class YAML:
    """Configured YAML parser instance (rt / safe / full)."""
```

## Constructor

### `__init__()`

Create a configured YAML parser instance.

```python
__init__(
    typ: str = "rt",
    schema: str = "core",
    max_depth: int = 1000,
    allow_duplicate_keys: bool = False,
) -> None
```

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `typ` | `str` | `"rt"` | Parser type. One of `"rt"` (round-trip), `"safe"`, `"full"`. |
| `schema` | `str` | `"core"` | YAML schema. One of `"core"`, `"yaml1.1"`, `"failsafe"`, `"json"`. |
| `max_depth` | `int` | `1000` | Maximum nesting depth for parsing. |
| `allow_duplicate_keys` | `bool` | `False` | Whether to allow duplicate mapping keys. |

**Raises:** `YamlTypeError` if `typ` or `schema` is invalid.

**Example:**

```python
from pyrs_yaml import YAML

# Round-trip parser (default)
yaml = YAML()

# Safe parser (no merge resolution)
yaml_safe = YAML(typ="safe")

# Full parser with YAML 1.1 schema
yaml_full = YAML(typ="full", schema="yaml1.1")
```

## Methods

### `parse()`

Parse a YAML string and return a `YamlDocument` with full metadata preservation.

```python
parse(yaml: str | bytes) -> YamlDocument
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `yaml` | `str \| bytes` | The YAML content to parse. |

**Returns:** A `YamlDocument` with round-trip editing support, comment preservation, and source tracking.

**Notes:**

- Merge resolution (`<<`) is enabled when `typ` is `"rt"` or `"full"`.
- The returned document preserves comments, anchors, and formatting.

**Example:**

```python
yaml = YAML(typ="rt")
doc = yaml.parse("name: Alice\nage: 30\n")
print(doc.root_type())  # mapping
print(doc["name"])      # Alice
```

### `safe_load()`

Parse YAML into a plain Python `dict` or `list`, resolving anchors and merges.

```python
safe_load(yaml: str) -> dict[str, Any] | list[Any]
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `yaml` | `str` | The YAML content to parse. |

**Returns:** A plain Python `dict` or `list` with all YAML anchors resolved.

**Notes:**

- This method does not preserve comments, formatting, or source tracking.
- All anchor references are resolved — the result is a plain Python object.
- Throws `YamlTypeError` on parse errors.

**Example:**

```python
yaml = YAML(typ="safe")
data = yaml.safe_load("""
person: &ref
  name: Alice
alias: *ref
""")
# data == {"person": {"name": "Alice"}, "alias": {"name": "Alice"}}
```

### `safe_loads()`

Parse a multi-document YAML string into a list of `dict`/`list` objects.

```python
safe_loads(yaml: str) -> list[dict[str, Any] | list[Any]]
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `yaml` | `str` | The multi-document YAML content. |

**Returns:** A list of plain Python `dict` or `list` objects, one per document.

**Notes:**

- Documents are separated by `---` markers.
- Anchors and merges are resolved within each document.
- Comments and formatting are not preserved.

**Example:**

```python
yaml = YAML(typ="safe")
docs = yaml.safe_loads("""
---
a: 1
---
b: 2
""")
# docs == [{"a": 1}, {"b": 2}]
```

### `parse_file()`

Parse a YAML file and return a `YamlDocument` with full metadata preservation.

```python
parse_file(path: str) -> YamlDocument
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `path` | `str` | The file path to read and parse. |

**Returns:** A `YamlDocument` with round-trip editing support.

**Raises:** `IOError` if the file cannot be read.

**Notes:**

- The file is read from disk using Rust's `std::fs::read_to_string` — no GIL blocking.
- The source is stored in the document for round-trip fidelity.

**Example:**

```python
yaml = YAML(typ="rt")
doc = yaml.parse_file("config.yaml")
print(doc["database"]["host"])
```

### `parse_all_docs()`

Parse a multi-document YAML string and return a list of `YamlDocument` objects.

```python
parse_all_docs(yaml: str) -> list[YamlDocument]
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `yaml` | `str` | The multi-document YAML content. |

**Returns:** A list of `YamlDocument` objects, one per document.

**Notes:**

- Documents are separated by `---` markers.
- Each document retains full round-trip support (comments, anchors, formatting).
- Merge resolution is enabled when `typ` is `"rt"` or `"full"`.

**Example:**

```python
yaml = YAML(typ="rt")
docs = yaml.parse_all_docs("""
---
a: 1
---
b: 2
""")
for doc in docs:
    print(doc.root_type())
```

### `dump_stream()`

Streaming writer: serialize Python objects to a file-like object, using constant memory.

```python
dump_stream(
    file_obj: Any,
    iterable: Any,
    explicit_start: bool = False,
    explicit_end: bool = False,
    sort_keys: bool = False,
) -> None
```

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `file_obj` | `Any` | — | A writable file-like object with a `write(str)` method. |
| `iterable` | `Any` | — | An iterable of Python objects to serialize. |
| `explicit_start` | `bool` | `False` | Whether to emit `---` at the start of each document. |
| `explicit_end` | `bool` | `False` | Whether to emit `...` at the end of each document. |
| `sort_keys` | `bool` | `False` | Whether to sort mapping keys alphabetically. |

**Raises:** `YamlTypeError` if `file_obj` does not have a `write` method.

**Notes:**

- Uses constant memory — no need to hold the entire output in memory.
- The GIL is released during the Rust serialization phase.
- Each item in the iterable becomes a separate YAML document.

**Example:**

```python
import io
from pyrs_yaml import YAML

yaml = YAML()
buf = io.StringIO()
yaml.dump_stream(buf, [{"a": 1}, {"b": 2}], explicit_start=True)
print(buf.getvalue())
# ---
# a: 1
# ---
# b: 2
```

### `dump_file()`

Streaming writer: serialize Python objects directly to a file on disk.

```python
dump_file(
    path: str,
    iterable: Any,
    explicit_start: bool = False,
    explicit_end: bool = False,
    sort_keys: bool = False,
) -> None
```

**Parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `path` | `str` | — | The file path to write to. |
| `iterable` | `Any` | — | An iterable of Python objects to serialize. |
| `explicit_start` | `bool` | `False` | Whether to emit `---` at the start of each document. |
| `explicit_end` | `bool` | `False` | Whether to emit `...` at the end of each document. |
| `sort_keys` | `bool` | `False` | Whether to sort mapping keys alphabetically. |

**Raises:** `IOError` if the file cannot be created or written.

**Notes:**

- Uses Rust's `std::fs::File` directly — no GIL blocking during I/O.
- Each item in the iterable becomes a separate YAML document.
- Uses constant memory, suitable for large outputs.

**Example:**

```python
from pyrs_yaml import YAML

yaml = YAML()
yaml.dump_file("output.yaml", [{"x": 2}, {"x": 3}], sort_keys=True)
```

### `load_stream()`

Lazy event iterator: incrementally read from a file-like object.

```python
load_stream(file_obj: Any) -> YamlStream
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `file_obj` | `Any` | A readable file-like object with a `read()` method returning `str` or `bytes`. |

**Returns:** A `YamlStream` iterator that yields parsed event dicts lazily.

**Raises:** `YamlTypeError` if `file_obj` does not have a `read` method.

**Notes:**

- The stream is parsed incrementally — no need to load the entire file into memory.
- Each yielded event is a `dict` with keys like `"type"`, `"key"`, `"value"`, `"start_mark"`, `"end_mark"`.
- The stream ends when `__next__` returns `None`.

**Example:**

```python
import io
from pyrs_yaml import YAML

yaml = YAML()
buf = io.StringIO("key: value\n")
stream = yaml.load_stream(buf)
for event in stream:
    if event is None:
        break
    print(event["type"])
```

### `load_stream_file()`

Lazy event iterator: incrementally read from a file path.

```python
load_stream_file(path: str) -> YamlStream
```

**Parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `path` | `str` | The file path to read incrementally. |

**Returns:** A `YamlStream` iterator that yields parsed event dicts lazily.

**Raises:** `IOError` if the file cannot be opened.

**Notes:**

- Uses Rust's `std::fs::File` with buffered I/O — no GIL blocking during reads.
- Parses the file incrementally, ideal for large YAML files.

**Example:**

```python
from pyrs_yaml import YAML

yaml = YAML()
stream = yaml.load_stream_file("large.yaml")
for event in stream:
    if event is None:
        break
    print(event)
```

## Usage Examples

### Round-trip editing with a configured instance

```python
from pyrs_yaml import YAML

yaml = YAML(typ="rt", schema="core")
doc = yaml.parse("""
# User configuration
user:
  name: Alice
  age: 30
  tags: [admin, user]
""")

# Edit the document
doc["user"]["age"] = 31
doc["user"]["tags"].append("staff")

# Serialize back — comments and formatting are preserved
print(doc.to_yaml())
```

### Safe parsing with JSON schema

```python
from pyrs_yaml import YAML

yaml = YAML(typ="safe", schema="json")
data = yaml.safe_load("{name: Bob, age: 25}")
print(data["name"])  # Bob
```

### Multi-document stream handling

```python
from pyrs_yaml import YAML

yaml = YAML(typ="rt")
docs = yaml.parse_all_docs("""
---
doc: first
---
doc: second
""")
for doc in docs:
    print(doc["doc"])

# Or dump multiple documents
yaml.dump_file("multi.yaml", [{"id": 1}, {"id": 2}], explicit_start=True)
```

## See Also

- [`YamlDocument`](yaml-document.md) — the round-trip editable document object
- [`YamlStream`](reference.md#yamlstream) — the lazy event stream iterator
- [`parse()`](reference.md#parse) — module-level convenience function
- [`safe_load()`](reference.md#safe_load) — module-level convenience function
