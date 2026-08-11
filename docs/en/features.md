---
title: Features
description: Explore pyrs-yaml's feature set, including YAML 1.2 compliance, round-trip, in-place editing, and NumPy support.
tags:
  - docs
status: new
---

## Features

pyrs-yaml is designed to be a **drop-in replacement** for PyYAML while adding powerful features that PyYAML lacks.

### YAML 1.2 Compliance

Powered by **saphyr-parser**, pyrs-yaml achieves **98.1% pass rate** on the YAML Test Suite.

### Perfect Round-Trip

Unlike PyYAML, pyrs-yaml **preserves all formatting and metadata**:

- **Comments** — standalone and inline
- **Anchors** (`&name`) and **aliases** (`*name`)
- **Tags** (`!!str`, `!!int`, etc.)
- **Chomping indicators** (`\|-`, `\|+`, `>-`, `>+`)
- **Scalar styles** (plain, single-quoted, double-quoted, literal, folded)
- **Flow/block formatting** — `[]`/`{}` vs block style preserved

### Performance

!!! note "Benchmark environment"
    All benchmarks are measured on the author's machine (Windows 11, Python
    3.12). Relative speedups (×N) are consistent across hardware but absolute
    times may vary.

Rust backend delivers **25–40× speedup** over PyYAML:

| Operation | pyrs-yaml | PyYAML |
|-----------|-----------|--------|
| Parse (large) | 0.07 ms | 1.83 ms |
| Serialize (large) | 0.07 ms | 2.92 ms |
| Round-trip | 0.07 ms | 2.90 ms |

### Custom AST

The **CustomNode** AST gives you full control over YAML structure:

- Inspect and modify nodes programmatically
- Add custom metadata (comments, anchors, tags)
- Build YAML from scratch with full formatting control
- Advanced use cases: template engines, config generators, code formatters

### PyYAML Compatibility

Drop-in replacement with familiar API:

```python
import pyrs_yaml as yaml  # Use as 'yaml' for easy migration

yaml.safe_load(yaml_text)
yaml.safe_dump(data)
yaml.safe_loads(yaml_text)
yaml.safe_dumps(data)
```

### Async I/O

Non-blocking serialization and parsing via `asyncio`:

```python
import asyncio
import pyrs_yaml


async def main():
    yaml = await pyrs_yaml.safe_dump_async({"a": 1})
    data = await pyrs_yaml.safe_loads_async(yaml)
    print(data)  # {'a': 1}


asyncio.run(main())
```

Available functions: `safe_dump_async`, `safe_load_async`, `safe_loads_async`.

### JSON Schema Validation

Validate parsed YAML documents against JSON Schema:

```python
doc = pyrs_yaml.parse("name: Alice\nage: 30")
doc.validate({"type": "object", "properties": {"name": {"type": "string"}}})

# Schema as JSON string
doc.validate('{"type": "object", "required": ["name"]}')
```

Raises `YamlValidateError` on validation failure.

### YAML Schema Language

Define custom schemas that control how plain scalars resolve to Python types:

```python
import pyrs_yaml

# Register a custom schema from a YAML string
pyrs_yaml.register_schema("hex", """
name: hex
extends: core
rules:
  - pattern: ^0x[0-9a-fA-F]+$
    type: int
""")

# Use with YAML instance or module-level functions
y = pyrs_yaml.YAML(schema="hex")
doc = y.parse("addr: 0xFF")
assert doc.get("addr") == 255

d = pyrs_yaml.safe_load("addr: 0x1F", schema="hex")
assert d["addr"] == 31
```

Or pass a dict inline instead of registering:

```python
d = pyrs_yaml.safe_load(
    "addr: 0xFF",
    schema={
        "extends": "core",
        "rules": [{"pattern": "^0x[0-9a-fA-F]+$", "type": "int"}],
    },
)
```

- **`extends`** — optional base schema (`core`, `json`, `failsafe`, `yaml1.1`)
- **`rules`** — ordered list of `{pattern, type}`; first match wins
- **Supported types**: `null`, `bool`, `int`, `float`, `str`
- Built-in Core schema still uses zero-cost `match` dispatch (unaffected)

### Duplicate Keys

By default, duplicate mapping keys raise `YamlDuplicateKeyError`:

```python
pyrs_yaml.parse("key: first\nkey: second")
# pyrs_yaml.YamlDuplicateKeyError: duplicate key: key
```

Pass `allow_duplicate_keys=True` to keep the **last value**:

```python
doc = pyrs_yaml.parse("key: first\nkey: second", allow_duplicate_keys=True)
doc.get("key")  # "second"
```

The switch applies to `parse`, `safe_load`, `safe_loads`, `parse_file`, `parse_all_docs`, and `YAML(allow_duplicate_keys=True)`. In round-trip mode, documents with duplicate keys allowed serialize with the last key-value pair emitted.

### Serialization Options

`to_yaml_with_options()` controls indentation and line wrapping:

```python
yaml_str = doc.to_yaml_with_options(
    indent_size=2,  # base indent (used when per-type options omitted)
    width=80,  # line wrap width; 0 disables wrapping
    indent_mapping=4,  # block mapping indent per level
    indent_sequence=2,  # block sequence indent per level
    indent_offset=0,  # base offset for the entire document
)
```

When `indent_mapping` / `indent_sequence` / `indent_offset` are omitted, they default to `indent_size` / `indent_size` / `0` respectively, so `indent_size=4` still indents all levels by 4.

### Custom Tag Handlers

Register handlers for custom YAML tags that transform scalar values:

```python
import pyrs_yaml
```

=== "Decorator"

    ```python
    @pyrs_yaml.register_tag("!custom")
    def custom_handler(node):
        return f"custom:{node}"
    ```

=== "Imperative"

    ```python
    pyrs_yaml.register_tag("!custom", lambda node: node.upper())
    ```

```python
doc = pyrs_yaml.parse("name: !custom value")
doc.get("name")  # "custom:value"
```

- Multiple handlers for the same tag execute in ascending `priority` order; raising `YamlTagSkip` delegates to the next handler.
- Handlers must return a string, otherwise `YamlTagError` is raised.
- `remove_tag("!custom")` and `clear_tag_handlers()` unregister handlers.

### Community Plugins

Define custom YAML node types that integrate with serialization and deserialization:

```python
import pyrs_yaml
from datetime import datetime


class TimestampType(pyrs_yaml.CustomType):
    python_type = datetime

    def from_yaml(self, value: str):
        return datetime.fromisoformat(value)

    def to_yaml(self, obj) -> str:
        return obj.isoformat()


# Register imperative or decorator
pyrs_yaml.register_type("!timestamp", TimestampType())

# Load: tagged scalar → Python object
doc = pyrs_yaml.parse("when: !timestamp 2026-08-11T10:30:00")
assert isinstance(doc.get("when"), datetime)

# Dump: Python object → tagged scalar
data = {"ts": datetime(2026, 8, 11, 10, 30)}
out = pyrs_yaml.safe_dump(data)
# out contains: ts: !timestamp 2026-08-11T10:30:00
```

**Built-in plugins** (registered at import time):
`!timestamp` → `datetime`, `!date` → `datetime.date`, `!time` → `datetime.time`,
`!uuid` → `uuid.UUID`, `!decimal` → `decimal.Decimal`, `!binary` → `bytes`,
`!regex` → `re.Pattern`, `!set` → `str`

| Method | Description |
|--------|-------------|
| `can_parse(node)` | Whether this type handles a given AST node |
| `from_yaml(value)` | Convert YAML string → Python object |
| `to_yaml(obj)` | Convert Python object → YAML string |
| `validate(obj)` | Validate a Python object (returns `bool`) |

### Pydantic Integration

Parse YAML directly into Pydantic models, or serialize models to YAML:

```python
from pydantic import BaseModel
import pyrs_yaml


class User(BaseModel):
    name: str
    age: int


# Parse YAML into a Pydantic model
user = pyrs_yaml.parse_as(User, "name: Alice\nage: 30")
print(user.name)  # Alice

# Serialize a model to YAML string
yaml_str = pyrs_yaml.dump_pydantic(user)
print(yaml_str)
```

### Incremental Re-parse

Re-parse stored source text in place with different options:

```python
doc = pyrs_yaml.parse("x: on")
print(doc.get("x"))  # "on" (string, core schema)

doc.reparse(schema="yaml1.1")
print(doc.get("x"))  # True (bool, yaml1.1 schema)
```

### In-Place Editing

Edit a parsed document **without losing any formatting metadata** — comments, anchors, tags, scalar styles, and flow/block style all survive:

```python
doc = pyrs_yaml.parse("""
server:
  host: localhost  # bind address
  ports:
    - 8080
""")

doc.set("$.server.host", "0.0.0.0")  # replace by path
doc.insert("$.server.ports", 0, 80)  # insert into a sequence
doc.append("$.server.ports", 443)  # append to a sequence
doc.rename("$.server", "srv")  # rename a mapping key
del doc["server"]  # or: doc.delete("$.server")
```

- **Path API** — JSONPath-style paths (`$.a.b[0]`) with root sugar (`doc["k"] = v`, `del doc["k"]`)
- **Node API** — `doc.node().find(path)` returns `Node` objects with `set_value` / `insert` / `append` / `delete` / `rename`, plus tree traversal (`parent`, `children`, `walk`, `filter`)
- **Atomicity** — failed edits leave the document (and its revision) untouched
- **Metadata preservation** — replaced scalars keep their comment/anchor/tag/quoting; renamed keys keep position and comments
- **Alias-aware** — setting an alias's own path replaces it in place; editing *through* an alias raises `YamlEditError`

See the [In-Place Editing guide](guides/editing.md) for details.

### NumPy ndarray Support

pyrs-yaml can serialize `numpy.ndarray` objects of any dimension directly to YAML:

```python
import numpy as np
import pyrs_yaml

# 1-D array
arr = np.array([1, 2, 3], dtype="int32")
yaml_str = pyrs_yaml.safe_dump(arr)
# - 1
# - 2
# - 3

# 2-D matrix
matrix = np.array([[1, 2], [3, 4]], dtype="float64")
yaml_str = pyrs_yaml.safe_dump(matrix)
# -
#   - 1.0
#   - 2.0
# -
#   - 3.0
#   - 4.0

# Round-trip
loaded = pyrs_yaml.safe_load(yaml_str)
assert loaded == [[1.0, 2.0], [3.0, 4.0]]
```

#### Supported dtypes

| Type | Rust Backend | YAML Output |
|------|-------------|-------------|
| `int8/16/32/64` | `PyUntypedArray` → `PyArrayDyn<i8/i16/i32/i64>` | Plain integer (quoted if negative) |
| `uint8/16/32/64` | `PyUntypedArray` → `PyArrayDyn<u8/u16/u32/u64>` | Plain integer |
| `float32/64` | `PyUntypedArray` → `PyArrayDyn<f32/f64>` | Plain float (quoted if negative) |
| `complex64/128` | `PyUntypedArray` → `PyArrayDyn<Complex64/Complex32>` | `(re+imj)` string |
| `bool` | `PyUntypedArray` → `PyArrayDyn<bool>` | `true` / `false` |
| `nan` / `inf` | — | `NaN` / `.inf` / `-.inf` |

#### Notes

- **Zero-copy**: Uses the `numpy` Rust crate's `PyUntypedArray` for type-erased array access, then dispatches to the correct typed `PyArrayDyn<T>` for zero-copy slice iteration
- **GIL released**: Slice iteration runs outside the GIL for maximum performance on large arrays

!!! warning "Negative scalars in block sequences"
    YAML 1.2 block sequences cannot contain plain scalars starting with `-`;
    negative values are automatically quoted during serialization and correctly
    parsed back during round-trip.

- **Negative numbers**: YAML 1.2 block sequences cannot contain plain scalars starting with `-`; negative values are automatically quoted and correctly parsed back during round-trip
- **0-D arrays**: Reshaped to 1-D and serialized as a single-item list
- **Complex numbers**: YAML has no native complex type; serialized as `(re+imj)` strings. `safe_load` returns them as strings, not Python `complex`
- **Markdown frontmatter extraction** — `read_markdown()` for blog/content tools
- **JSON ↔ YAML conversion** — `from_json()` / `from_dict()`
- **Pydantic integration** — `parse_as()` / `dump_pydantic()`
- **Multi-document parsing** — `parse_all_docs()`
- **i18n error messages** — `set_language("zh-CN")` for bilingual errors
- **Type hints** — PEP 561 typed package marker (`py.typed`) for mypy support

### Supported YAML Constructs

| Feature | Support |
|---------|---------|
| YAML 1.2 spec | ✅ Full |
| Comments (standalone) | ✅ Preserved |
| Comments (inline) | ✅ Preserved |
| Anchors & aliases | ✅ Preserved |
| Tags (explicit) | ✅ Preserved |
| Block scalars (`\|`, `>`) | ✅ Preserved |
| Chomping indicators | ✅ Preserved |
| Flow collections (`{}`, `[]`) | ✅ Preserved |
| Merge keys (`<<`) | ✅ Resolved |
| Complex keys | ✅ Supported |
| Escape sequences | ✅ Supported |
| Multi-document | ✅ Supported |
| **Async I/O** | **✅ `safe_*_async`** |
| **JSON Schema validation** | **✅ `doc.validate()`** |
| **Incremental re-parse** | **✅ `doc.reparse()`** |
| **In-place editing** | **✅ `doc.set()` / `insert()` / `append()` / `delete()` / `rename()`** |
| **JSON export** | **✅ `doc.to_json()`** |
