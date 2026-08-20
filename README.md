# pyrs-yaml

[![PyPI version](https://img.shields.io/pypi/v/pyrs-yaml)](https://pypi.org/project/pyrs-yaml/)
[![Python versions](https://img.shields.io/pypi/pyversions/pyrs-yaml)](https://pypi.org/project/pyrs-yaml/)
[![Downloads](https://img.shields.io/pypi/dm/pyrs-yaml)](https://pypi.org/project/pyrs-yaml/)
[![License](https://img.shields.io/github/license/759401524/pyrs-yaml)](LICENSE-MIT)
[![CI](https://img.shields.io/github/actions/workflow/status/759401524/pyrs-yaml/ci.yml?branch=main)](https://github.com/759401524/pyrs-yaml/actions)
[![GitHub release](https://img.shields.io/github/v/release/759401524/pyrs-yaml)](https://github.com/759401524/pyrs-yaml/releases)
[![Docs](https://img.shields.io/website?url=https%3A%2F%2F759401524.github.io%2Fpyrs-yaml%2F&label=docs&color=blue)](https://759401524.github.io/pyrs-yaml)
[![GitHub stars](https://img.shields.io/github/stars/759401524/pyrs-yaml)](https://github.com/759401524/pyrs-yaml)
[![CodSpeed](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://app.codspeed.io/759401524/pyrs-yaml?utm_source=badge)
[![zread](https://img.shields.io/badge/Ask_Zread-_.svg?style=flat-square&color=00b0aa&labelColor=000000&logo=data%3Aimage%2Fsvg%2Bxml%3Bbase64%2CPHN2ZyB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIHZpZXdCb3g9IjAgMCAxNiAxNiIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHBhdGggZD0iTTQuOTYxNTYgMS42MDAxSDIuMjQxNTZDMS44ODgxIDEuNjAwMSAxLjYwMTU2IDEuODg2NjQgMS42MDE1NiAyLjI0MDFWNC45NjAxQzEuNjAxNTYgNS4zMTM1NiAxLjg4ODEgNS42MDAxIDIuMjQxNTYgNS42MDAxSDQuOTYxNTZDNS4zMTUwMiA1LjYwMDEgNS42MDE1NiA1LjMxMzU2IDUuNjAxNTYgNC45NjAxVjIuMjQwMUM1LjYwMTU2IDEuODg2NjQgNS4zMTUwMiAxLjYwMDEgNC45NjE1NiAxLjYwMDFaIiBmaWxsPSIjZmZmIi8%2BCjxwYXRoIGQ9Ik00Ljk2MTU2IDEwLjM5OTlIMi4yNDE1NkMxLjg4ODEgMTAuMzk5OSAxLjYwMTU2IDEwLjY4NjQgMS42MDE1NiAxMS4wMzk5VjEzLjc1OTlDMS42MDE1NiAxNC4xMTM0IDEuODg4MSAxNC4zOTk5IDIuMjQxNTYgMTQuMzk5OUg0Ljk2MTU2QzUuMzE1MDIgMTQuMzk5OSA1LjYwMTU2IDE0LjExMzQgNS42MDE1NiAxMy43NTk5VjExLjAzOTlDNS42MDE1NiAxMC42ODY0IDUuMzE1MDIgMTAuMzk5OSA0Ljk2MTU2IDEwLjM5OTlaIiBmaWxsPSIjZmZmIi8%2BCjxwYXRoIGQ9Ik0xMy43NTg0IDEuNjAwMUgxMS4wMzg0QzEwLjY4NSAxLjYwMDEgMTAuMzk4NCAxLjg4NjY0IDEwLjM5ODQgMi4yNDAxVjQuOTYwMUMxMC4zOTg0IDUuMzEzNTYgMTAuNjg1IDUuNjAwMSAxMS4wMzg0IDUuNjAwMUgxMy43NTg0QzE0LjExMTkgNS42MDAxIDE0LjM5ODQgNS4zMTM1NiAxNC4zOTg0IDQuOTYwMVYyLjI0MDFDMTQuMzk4NCAxLjg4NjY0IDE0LjExMTkgMS42MDAxIDEzLjc1ODQgMS42MDAxWiIgZmlsbD0iI2ZmZiIvPgo8cGF0aCBkPSJNNCAxMkwxMiA0TDQgMTJaIiBmaWxsPSIjZmZmIi8%2BCjxwYXRoIGQ9Ik00IDEyTDEyIDQiIHN0cm9rZT0iI2ZmZiIgc3Ryb2tlLXdpZHRoPSIxLjUiIHN0cm9rZS1saW5lY2FwPSJyb3VuZCIvPgo8L3N2Zz4K&logoColor=ffffff)](https://zread.ai/759401524/pyrs-yaml)

**English** | [简体中文](README.zh-CN.md)

A high-performance Python YAML library with perfect round-trip support, built with Rust and PyO3.

## Features

- **YAML 1.2 compliant** - Uses granit-parser for full YAML 1.2 support with native comment preservation
- **Perfect Round-Trip** - Preserves comments, anchors, tags, chomping, scalar styles, and flow/block formatting
- **In-Place Editing** - Edit parsed documents via JSONPath-style paths (`doc.set("$.a.b", v)`) or the `Node` tree API, without losing formatting
- **High Performance** - Rust backend, 7× faster `safe_dump`/`from_dict` vs v0.10 (direct writer, no intermediate AST); fast-path `safe_load`/`safe_loads` skips anchor tracking when none present
- **Depth-limited parsing** - `max_depth` (default 1000) on `parse`, `parse_file`, `parse_all_docs`, `parse_stream`, `safe_load`, `safe_loads`, `read_markdown`, `read_markdown_str` to prevent deep nesting attacks
- **NumPy ndarray support** - `safe_dump()` / `safe_dumps()` / `from_dict()` / `dump_file()` serialize `numpy.ndarray` of any dimension (0-D through N-D) with zero-copy Rust dispatch
- **JSON Schema validation** - `YamlDocument.validate(schema)` validates parsed documents against JSON Schema; `YamlValidateError` for failures
- **Async I/O** - `safe_dumps_async` / `safe_dump_async` / `safe_loads_async` / `safe_load_async` via `asyncio.run_in_executor`
- **Incremental re-parse** - `doc.source()` + `doc.reparse()` for re-parsing stored YAML in-place with different options (e.g. `schema="yaml1.1"`)
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

Or with uv:

```bash
uv pip install pyrs-yaml
```

## Requirements

- **Supported Python versions** (installing wheels): Python 3.8+ (CPython; PyPy and free-threaded 3.14t wheels are also published). abi3 wheels mean one wheel covers all supported Python versions.
- **Rust toolchain** (building from source only): Rust 1.96 or later (MSRV, edition 2024). This is above PyO3's own baseline (rustc 1.83+ for PyO3 0.29) and is chosen deliberately for std API headroom — it keeps current stable APIs (e.g. `assert_matches!`, stabilized in 1.96) available without waiting for a future MSRV bump. End users installing wheels never need Rust.

## Documentation

Full documentation (English, 简体中文, 日本語, 한국어) is available at [https://759401524.github.io/pyrs-yaml](https://759401524.github.io/pyrs-yaml). See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

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

# Parse with options (max_depth, schema, allow_duplicate_keys)
doc = pyrs_yaml.parse(yaml_str, resolve_merges=False, max_depth=500, schema="yaml1.1")

# Parse YAML file
doc = pyrs_yaml.parse_file("config.yaml")

# Parse multiple YAML documents
docs = pyrs_yaml.parse_all_docs(yaml_str)


# Stream parsing (on_event callback)
def handler(event):
    print(event)
    return True  # return False to stop


iter = pyrs_yaml.parse_stream(yaml_str, on_event=handler, max_depth=1000)

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

# Dump to YAML from dict
yaml_str = pyrs_yaml.from_dict(data)

# i18n language management
pyrs_yaml.set_language("zh-CN")
pyrs_yaml.get_language()  # "zh-CN"
pyrs_yaml.list_languages()  # ["en", "zh-CN"]
pyrs_yaml.detect_language()  # auto-detect from environment
pyrs_yaml.negotiate_language(["zh-CN", "en"], "en")  # "zh-CN"
```

### PyYAML Compatible API

```python
# Load YAML to dict (supports schema and max_depth)
data = pyrs_yaml.safe_load(yaml_str)
data = pyrs_yaml.safe_load(yaml_str, schema="yaml1.1", max_depth=500)

# Load multiple documents
docs = pyrs_yaml.safe_loads(yaml_str)
docs = pyrs_yaml.safe_loads(yaml_str, allow_duplicate_keys=True)

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
frontmatter, content = pyrs_yaml.read_markdown_str(markdown_text, max_depth=200)
```

## Performance

divan benchmarks in `crates/pyrs-yaml/benches/yaml_bench.rs` (Rust) + `pytest-codspeed` in `tests/test_benchmark_crosslib.py` (Python). See [benchmarks docs](docs/en/performance/benchmarks.md) for the full cross-library comparison against PyYAML and ruamel.yaml.

**v0.11 highlights (vs v0.10):**

- `safe_dump` / `from_dict` / `dump_file` / `dump_iterable`: **7× faster** — direct writer eliminates intermediate `CustomNode` AST
- `safe_load` / `safe_loads` / `to_dict`: **fast-path** — skips anchor tracking when input has no `&` characters
- `resolve_core_type`: **first-byte dispatch** — non-numeric/boolean scalars return `Str` immediately

## Development

```bash
# Install dependencies
uv sync

# Build Python extension
uv run maturin develop --release

# Run tests (Rust: cargo nextest; Python: uv run pytest)
cargo nextest run --all
uv run pytest tests/ -v --ignore=tests/benchmark_compare.py

# Lint and format (Rust + Python)
cargo clippy -- -D warnings
cargo fmt
uv run ruff check .
uv run ruff format .

# Run benchmarks (Rust)
cargo bench

# Run benchmarks (Python)
uv run pytest tests/test_benchmark_crosslib.py tests/test_benchmark_api.py --codspeed

# Performance sanity checks
uv run pytest tests/test_performance.py -v

# Git hooks
prek install --prepare-hooks
prek run --all-files
```

## License

Licensed under either of:

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
