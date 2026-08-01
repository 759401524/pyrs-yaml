# pyrs-yaml

[![CodSpeed](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://app.codspeed.io/759401524/pyrs-yaml?utm_source=badge)

A high-performance Python YAML library with perfect round-trip support, built with Rust and PyO3.

## Features

- **YAML 1.2 compliant** - Uses saphyr-parser for full YAML 1.2 support
- **Perfect Round-Trip** - Preserves comments, anchors, tags, chomping, scalar styles, and flow/block formatting
- **In-Place Editing** - Edit parsed documents via JSONPath-style paths (`doc.set("$.a.b", v)`) or the `Node` tree API, without losing formatting
- **High Performance** - Rust backend, see [benchmarks](benches/yaml_bench.rs)
- **NumPy ndarray support** - `safe_dump()` / `safe_dumps()` / `from_dict()` / `dump_file()` serialize `numpy.ndarray` of any dimension (0-D through N-D) with zero-copy Rust dispatch
- **JSON Schema validation** - `YamlDocument.validate(schema)` validates parsed documents against JSON Schema; `YamlValidateError` for failures
- **Async I/O** - `safe_dumps_async` / `safe_dump_async` / `safe_loads_async` / `safe_load_async` via `asyncio.run_in_executor`
- **Incremental re-parse** - `doc.source()` + `doc.reparse()` for re-parsing stored YAML in-place with different options
- **JSON serialization** - `doc.to_json()` exports documents to standard JSON
- **Duplicate keys** - `allow_duplicate_keys=True` opts into last-value-wins; `YamlDuplicateKeyError` otherwise
- **Custom tag handlers** - `register_tag` with priority-based chaining, `YamlTagSkip`, `remove_tag`/`clear_tag_handlers`
- **Pydantic models** - `parse_as(Model, yaml)` validates parsed YAML against Pydantic v2 models
- **Custom AST** - Extensible AST for advanced YAML manipulation
- **PyYAML Compatible** - Drop-in replacement with `safe_load`/`safe_dump` API

## Installation

```bash
pip install pyrs-yaml
```

## Quick Start

```python
import pyrs_yaml

# Parse YAML
doc = pyrs_yaml.parse("key: value")
print(doc.to_yaml())  # key: value

# PyYAML compatible API
data = pyrs_yaml.safe_load("key: value")
print(data)  # {'key': 'value'}

# Round-trip preserves comments
original = "# Comment\nkey: value  # inline\n"
doc = pyrs_yaml.parse(original)
assert doc.to_yaml() == original  # True

# Edit in place without losing formatting
doc.set("$.key", "edited")  # key: edited  # inline
doc.set("$.new", 1)  # add a new key
print(doc.to_yaml())
```

### JSON Schema validation

```python
doc = pyrs_yaml.parse("name: Alice\nage: 30")
doc.validate({"type": "object", "properties": {"name": {"type": "string"}}})
# None — validation passed

# Invalid — raises YamlValidateError
doc.validate({"type": "object", "required": ["email"]})
# pyrs_yaml.YamlValidateError: "Email" is a required property
```

### Async serialization

```python
import asyncio
import pyrs_yaml


async def main():
    yaml = await pyrs_yaml.safe_dumps_async({"a": 1})
    data = await pyrs_yaml.safe_loads_async(yaml)
    print(data)  # {'a': 1}


asyncio.run(main())
```

### Incremental re-parse

```python
doc = pyrs_yaml.parse("x: on")
print(doc.get("x"))  # "on" (core schema: string)

doc.reparse(schema="yaml1.1")
print(doc.get("x"))  # True (yaml1.1 schema: bool)
```

### JSON export

```python
doc = pyrs_yaml.parse("a: 1\nb: hello")
json_str = doc.to_json()  # '{"a": 1, "b": "hello"}'
```

### NumPy ndarray support

```python
import numpy as np
import pyrs_yaml

# 1-D array
arr = np.array([1, 2, 3], dtype="int32")
yaml_str = pyrs_yaml.safe_dump(arr)
print(yaml_str)
# - 1
# - 2
# - 3

# 2-D matrix
matrix = np.array([[1, 2], [3, 4]], dtype="float64")
yaml_str = pyrs_yaml.safe_dump(matrix)
print(yaml_str)
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

### Duplicate keys

Duplicate mapping keys raise `YamlDuplicateKeyError` by default:

```python
pyrs_yaml.parse("key: first\nkey: second")
# pyrs_yaml.YamlDuplicateKeyError: duplicate key: key
```

Pass `allow_duplicate_keys=True` to keep the **last value** instead:

```python
doc = pyrs_yaml.parse("key: first\nkey: second", allow_duplicate_keys=True)
doc.get("key")  # "second"
```

The flag is available on `parse`, `safe_load`, `safe_loads`, `parse_file`, `parse_all_docs`, and `YAML(allow_duplicate_keys=True)`. In round-trip mode, serializing a document with allowed duplicate keys emits the last occurrence.

### Serialization options

`to_yaml_with_options()` controls indentation and line wrapping:

```python
yaml_str = doc.to_yaml_with_options(
    indent_size=2,  # legacy base indent (used when the per-type options are omitted)
    width=80,  # line-wrap width; 0 disables wrapping
    indent_mapping=4,  # indent per block-mapping level
    indent_sequence=2,  # indent per block-sequence level
    indent_offset=0,  # base offset applied to the whole document
)
```

`indent_mapping` / `indent_sequence` / `indent_offset` default to `indent_size` / 0 when omitted, so `indent_size=4` still indents everything by 4.

### Tag handlers

Register a handler for a custom YAML tag to transform scalar values:

```python
import pyrs_yaml


# Decorator form
@pyrs_yaml.register_tag("!custom")
def custom_handler(node):
    return f"custom:{node}"


# Imperative form
pyrs_yaml.register_tag("!custom", lambda node: node.upper())

doc = pyrs_yaml.parse("name: !custom value")
doc.get("name")  # "custom:value"
```

- Multiple handlers per tag run in ascending `priority` order; raising `YamlTagSkip` passes control to the next handler.
- A handler must return a string — anything else raises `YamlTagError`.
- `remove_tag("!custom")` and `clear_tag_handlers()` unregister handlers.

### Pydantic models

Parse YAML directly into a Pydantic v2 model:

```python
from pydantic import BaseModel
import pyrs_yaml


class Config(BaseModel):
    name: str
    age: int


cfg = pyrs_yaml.parse_as(Config, "name: Alice\nage: 30")
cfg.name  # "Alice"
```

`parse_as` raises `TypeError` for non-`BaseModel` targets and propagates Pydantic's `ValidationError` when the YAML does not match the model.

## Features Supported

| Feature | Support |
|---------|---------|
| YAML 1.2 | Full |
| Comments (standalone + inline) | Preserved |
| Anchors (`&`) and aliases (`*`) | Preserved |
| Tags (`!!str`, `!!int`, etc.) | Preserved |
| Chomping (`\|-`, `\|+`, `>-`, `>+`) | Preserved |
| Complex keys (sequence/mapping as key) | Supported |
| Escape sequences (`\n`, `\t`, `\uXXXX`) | Supported |
| Flow collections (`{}`, `[]`) | Preserved |
| Block scalars (`\|`, `>`) | Preserved |
| Merge keys (`<<: *alias`) | Resolved (opt-out via `resolve_merges=False`) |
| **NumPy ndarray** | **Full (0-D through N-D)** |
| **JSON Schema validation** | **Full** |
| **Async I/O** | **Full** |
| **Incremental re-parse** | **Full** |
| **JSON export** | **Full** |
| **Duplicate keys** | **Configurable (`YamlDuplicateKeyError` / last-wins)** |
| **Custom tag handlers** | **Priority-chained `register_tag`** |
| **Pydantic models** | **`parse_as()` validation** |

## API Reference

### Core Functions

```python
# Parse YAML string (accepts str or bytes)
doc = pyrs_yaml.parse(yaml_str)
doc = pyrs_yaml.parse(yaml_bytes)

# Parse with options
doc = pyrs_yaml.parse(yaml_str, resolve_merges=False)

# Parse YAML file
doc = pyrs_yaml.parse_file("config.yaml")

# Parse multiple YAML documents
docs = pyrs_yaml.parse_all_docs(yaml_str)

# Convert to YAML string (with options)
yaml_str = doc.to_yaml()
yaml_str = doc.to_yaml_with_options(indent_size=4, explicit_start=True, sort_keys=True)

# Get value by key (with default)
value = doc.get("key")
value = doc.get("missing_key", "default")

# Get root type
doc.root_type()  # "mapping", "sequence", "scalar", "null"

# Check containment and length
"key" in doc
len(doc)

# Iterate
for key in doc:
    print(key, doc[key])
```

### PyYAML Compatible API

```python
# Load YAML to dict
data = pyrs_yaml.safe_load(yaml_str)

# Load multiple documents
docs = pyrs_yaml.safe_loads(yaml_str)

# Dump dict to YAML
yaml_str = pyrs_yaml.safe_dump(data)

# Convert dict to YAML
yaml_str = pyrs_yaml.from_dict(data)

# Convert JSON to YAML
yaml_str = pyrs_yaml.from_json(json_str)

# Dump to file
pyrs_yaml.dump_file(data, "output.yaml")

# Extract YAML frontmatter from markdown
frontmatter, content = pyrs_yaml.read_markdown("post.md")
frontmatter, content = pyrs_yaml.read_markdown_str(markdown_text)

# i18n language management
pyrs_yaml.set_language("zh-CN")
pyrs_yaml.get_language()  # "zh-CN"
pyrs_yaml.list_languages()  # ["en", "zh-CN"]
pyrs_yaml.detect_language()  # auto-detect from environment
pyrs_yaml.negotiate_language(["zh-CN", "en"], "en")  # "zh-CN"
```

## Performance

Criterion benchmarks in `benches/yaml_bench.rs` (Rust) + `pytest-codspeed` in `tests/test_benchmark_crosslib.py` (Python):

| Operation | Time |
|-----------|------|
| Parse (small, ~2 keys) | ~1.7 µs |
| Parse (medium, ~30 keys) | ~12 µs |
| Parse (large, ~60 keys) | ~38 µs |
| Serialize (small) | ~4.4 µs |
| Serialize (medium) | ~4.7 µs |
| Serialize (large) | ~5.5 µs |
| Roundtrip (small) | ~5.9 µs |
| Roundtrip (large) | ~45 µs |

## Development

```bash
# Install dependencies
uv sync

# Build Python extension
maturin develop --release

# Run tests
cargo test
pytest tests/

# Run benchmarks (Rust)
cargo bench

# Run benchmarks (Python)
uv run pytest tests/test_benchmark_crosslib.py tests/test_benchmark_api.py --codspeed

# Performance sanity checks
uv run pytest tests/test_performance.py -v

# Run clippy
cargo clippy -- -D warnings

# Format code
cargo fmt
```

## License

Licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
