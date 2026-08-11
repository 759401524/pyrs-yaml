---
title: Module Reference
description: Complete API reference for the pyrs_yaml module, including core functions, PyYAML-compatible API, and async functions.
tags:
  - docs
status: new
---

## Module Reference

Complete API reference for the `pyrs_yaml` module.

!!! tip "Version compatibility"
    pyrs-yaml ships as an ABI3 wheel, so a single wheel works across Python
    3.8–3.15 — no recompilation needed when upgrading Python.

### Core Functions

#### `parse()`

Parse a YAML string or bytes into a `YamlDocument`.

```python
parse(yaml: str | bytes, resolve_merges: bool = True, schema: str | dict = "core", max_depth: int = 1000, allow_duplicate_keys: bool = False) -> YamlDocument
```

**Parameters:**

- `yaml` — YAML content as `str` or `bytes`
- `resolve_merges` — Whether to resolve merge keys (`<<: *alias`) after parsing (default: `True`)
- `schema` — Schema name (`"core"`, `"json"`, `"failsafe"`, `"yaml1.1"`, or a registered custom name), or an inline schema dict (see [YAML Schema Language](#yaml-schema-language))
- `max_depth` — Maximum nesting depth (default: `1000`)
- `allow_duplicate_keys` — Whether to allow duplicate mapping keys (default: `False`)

**Returns:** A `YamlDocument` containing the parsed YAML

**Raises:**

- `YamlParseError` — Invalid YAML syntax
- `YamlTypeError` — Schema not found
- `TypeError` — Input is not `str` or `bytes`

**Example:**

```python
doc = pyrs_yaml.parse("key: value")
doc = pyrs_yaml.parse(b"key: value")
doc = pyrs_yaml.parse(yaml_str, schema="json")
doc = pyrs_yaml.parse("addr: 0xFF", schema={"extends": "core", "rules": [{"pattern": "^0x[0-9a-fA-F]+$", "type": "int"}]})
```

#### `parse_file()`

Parse a YAML file.

```python
parse_file(path: str, schema: str | dict = "core", max_depth: int = 1000, allow_duplicate_keys: bool = False) -> YamlDocument
```

**Parameters:**

- `path` — Path to the YAML file
- `schema` — Schema name or inline dict (see [YAML Schema Language](#yaml-schema-language))

**Returns:** A `YamlDocument`

**Raises:**

- `IOError` — File not found or unreadable
- `YamlParseError` — Invalid YAML
- `YamlTypeError` — Schema not found

**Example:**

```python
doc = pyrs_yaml.parse_file("config.yaml")
```

#### `parse_all_docs()`

Parse multiple YAML documents from a string.

```python
parse_all_docs(yaml: str, resolve_merges: bool = True, schema: str | dict = "core", max_depth: int = 1000, allow_duplicate_keys: bool = False) -> list[YamlDocument]
```

**Parameters:**

- `yaml` — YAML content with one or more documents (`---` separated)
- `schema` — Schema name or inline dict (see [YAML Schema Language](#yaml-schema-language))

**Returns:** A list of `YamlDocument` objects

**Example:**

```python
docs = pyrs_yaml.parse_all_docs("a: 1\n---\nb: 2")
```

### PyYAML-Compatible Functions

#### `safe_load()`

Parse a YAML string into a Python dict or list. Uses PyYAML-compatible API.

```python
safe_load(yaml: str, schema: str | dict = "core", max_depth: int = 1000, allow_duplicate_keys: bool = False) -> dict[str, Any] | list[Any]
```

**Parameters:**

- `yaml` — YAML content as `str`
- `schema` — Schema name or inline dict (see [YAML Schema Language](#yaml-schema-language))

**Raises:** `YamlParseError`, `YamlTypeError`

**Example:**

```python
d = pyrs_yaml.safe_load("key: value")
d = pyrs_yaml.safe_load("addr: 0xFF", schema={"extends": "core", "rules": [{"pattern": "^0x[0-9a-fA-F]+$", "type": "int"}]})
```

#### `safe_loads()`

Parse multiple YAML documents from a string into Python dicts/lists.

```python
safe_loads(yaml: str, schema: str | dict = "core", max_depth: int = 1000, allow_duplicate_keys: bool = False) -> list[dict[str, Any] | list[Any]]
```

**Parameters:**

- `yaml` — YAML content with one or more documents
- `schema` — Schema name or inline dict (see [YAML Schema Language](#yaml-schema-language))

**Equivalent to:** `yaml.safe_loads()` in PyYAML

#### `safe_dump()`

Serialize a Python object to YAML.

```python
safe_dump(data: dict[str, Any] | list[Any] | ndarray) -> str
```

**Equivalent to:** `yaml.safe_dump()` in PyYAML

**Supported input types:** `dict`, `list`, `str`, `int`, `float`, `bool`, `None`, and **`numpy.ndarray`** (all dimensions and numeric dtypes: `int8/16/32/64`, `uint8/16/32/64`, `float32/64`, `complex64/128`, `bool`)

#### `safe_dumps()`

Alias for `safe_dump()`.

```python
safe_dumps(data: dict[str, Any] | list[Any] | ndarray) -> str
```

### Conversion Functions

#### `from_dict()`

Convert a Python dict to YAML string. Also accepts `numpy.ndarray` as a value inside the dict.

```python
from_dict(data: dict[str, Any]) -> str
```

#### `from_json()`

Convert a JSON string to YAML string.

```python
from_json(json_str: str) -> str
```

#### `dump_file()`

Serialize a Python object to YAML and write to file. Accepts `dict`, `list`, or `numpy.ndarray`.

```python
dump_file(data: Any, path: str) -> None
```

### Pydantic Integration

#### `dump_pydantic()`

Serialize a Pydantic model to a YAML string.

```python
dump_pydantic(model: BaseModel) -> str
```

Uses `model_dump(mode='json')` to preserve string types (e.g. a `"10001"` zip code stays a string) before delegating to `safe_dump`.

**Raises:**

- `ImportError` — pydantic is not installed
- `TypeError` — `model` is not a Pydantic `BaseModel` instance

**Example:**

```python
from pydantic import BaseModel
import pyrs_yaml


class User(BaseModel):
    name: str
    age: int


yaml_str = pyrs_yaml.dump_pydantic(User(name="Alice", age=30))
```

#### `parse_as()`

Parse a YAML string and validate it against a Pydantic model.

```python
parse_as(model: type[BaseModel], src: str, **yaml_kwargs: Any) -> BaseModel
```

**Parameters:**

- `model` — A Pydantic `BaseModel` subclass
- `src` — YAML string to parse
- `**yaml_kwargs` — Keyword arguments forwarded to the `YAML()` constructor

**Returns:** An instance of `model` validating the parsed YAML.

**Raises:**

- `ImportError` — pydantic is not installed
- `TypeError` — `model` is not a Pydantic `BaseModel` subclass
- `pydantic.ValidationError` — the parsed data fails model validation

**Example:**

```python
user = pyrs_yaml.parse_as(User, "name: Alice\nage: 30")
print(user.name)  # Alice
```

### Tag Registry

#### `register_tag()`

Register a custom tag handler. Supports both decorator and imperative forms.

```python
register_tag(name: str, handler: Callable | None = None, priority: int = 0) -> Callable
```

**Example:**

=== "Decorator"

    ```python
    @pyrs_yaml.register_tag("!custom")
    def handler(node):
        return f"custom:{node}"
    ```

=== "Imperative"

    ```python
    pyrs_yaml.register_tag("!custom", handler_fn, priority=1)
    ```

#### `remove_tag()`

Remove a tag handler.

```python
remove_tag(name: str) -> None
```

#### `clear_tag_handlers()`

Remove all registered tag handlers.

```python
clear_tag_handlers() -> None
```

### YAML Schema Language

#### `register_schema()`

Register a custom YAML schema from a schema definition string.

```python
register_schema(name: str, schema_yaml: str) -> None
```

**Parameters:**

- `name` — Schema name, used as `YAML(schema=name)`
- `schema_yaml` — Schema definition in YAML format

The schema definition supports a `rules` list mapping regex patterns to
YAML types, and an optional `extends` base schema:

```python
pyrs_yaml.register_schema("myapp", """
name: myapp
extends: core
rules:
  - pattern: ^0x[0-9a-fA-F]+$
    type: int
  - pattern: ^\\d{4}-\\d{2}-\\d{2}$
    type: str
""")

doc = pyrs_yaml.parse("addr: 0xFF", schema="myapp")
assert doc.get("addr") == 255
```

**Raises:** `YamlParseError` — Invalid schema definition

#### Schema as inline dict

The `schema` parameter of `YAML()`, `parse()`, `parse_file()`,
`parse_all_docs()`, `safe_load()`, and `safe_loads()` also accepts an inline
dict, which is serialized and registered automatically:

```python
doc = pyrs_yaml.safe_load(
    "addr: 0xFF",
    schema={
        "extends": "core",
        "rules": [{"pattern": "^0x[0-9a-fA-F]+$", "type": "int"}],
    },
)
assert doc["addr"] == 255
```

### Community Plugins

#### `CustomType`

Base class for custom YAML node types. Subclass it to define a type that can
be used with YAML tags.

```python
class CustomType:
    python_type = None  # set to a Python type for isinstance checks
    def can_parse(self, node) -> bool: ...
    def from_yaml(self, value: str): ...
    def to_yaml(self, obj) -> str: ...
    def validate(self, obj) -> bool: ...
```

**Methods:**

- `python_type` — Optional Python type used during serialization (`isinstance`)
- `can_parse(node)` — Whether this type handles a given node
- `from_yaml(value)` — Convert a YAML string to a Python object (load)
- `to_yaml(obj)` — Convert a Python object to a YAML string (dump)
- `validate(obj)` — Validate a Python object's type and value

Built-in plugins include `!timestamp` (maps to `datetime`) and `!set`.

#### `register_type()`

Register a `CustomType` instance or class.

```python
register_type(name: str, handler: CustomType | None = None) -> CustomType
```

**Imperative form:**

```python
class TimestampType(pyrs_yaml.CustomType):
    python_type = datetime
    def from_yaml(self, value):
        return datetime.fromisoformat(value)
    def to_yaml(self, obj):
        return obj.isoformat()

pyrs_yaml.register_type("!timestamp", TimestampType())
```

**Decorator form:**

```python
@pyrs_yaml.register_type("!timestamp")
class TimestampType(pyrs_yaml.CustomType):
    ...

doc = pyrs_yaml.parse("when: !timestamp 2026-08-11T10:30:00")
assert isinstance(doc.get("when"), datetime)
```

#### `remove_type()`

```python
remove_type(name: str) -> None
```

Remove a registered custom type handler.

#### `clear_type_handlers()`

```python
clear_type_handlers() -> None
```

Remove all registered custom type handlers.

### Compliance

#### `compliance_report()`

Compute the YAML Test Suite compliance report.

```python
compliance_report() -> dict
```

Returns the YAML Test Suite pass rate and per-test results.

### Streaming Events

#### `parse_stream()`

Parse YAML incrementally, yielding raw event dicts.

```python
parse_stream(yaml: str) -> StreamIterator
```

Returns a `StreamIterator` yielding one event dict per step. Unlike `YAML().load_stream()` (which resolves into Python values), this exposes the raw token stream.

#### `YamlStream` { #yamlstream }

The `YamlStream` class is a lazy event iterator returned by `YAML().load_stream()` and `YAML().load_stream_file()`. It yields parsed event dicts one at a time without loading the entire document into memory.

```python
stream = yaml.load_stream_file("large.yaml")
for event in stream:
    print(event)
```

See [`YamlStream`](yaml-instance.md) for full API details.

### Async Functions

Async I/O wrappers via `asyncio.run_in_executor`. Non-blocking in event loop context.

#### `safe_dumps_async()`

Serialize a Python object to YAML string (async).

```python
async def safe_dumps_async(data: Any) -> str
```

#### `safe_dump_async()`

Serialize a Python object to stdout as YAML (async).

```python
async def safe_dump_async(data: Any) -> None
```

#### `safe_loads_async()`

Parse a YAML string into native Python objects (async).

```python
async def safe_loads_async(yaml: str, schema: str = "core") -> Any
```

#### `safe_load_async()`

Parse a YAML string into native Python objects (async).

```python
async def safe_load_async(yaml: str, schema: str = "core") -> Any
```

**Example:**

```python
import asyncio, pyrs_yaml


async def main():
    yaml = await pyrs_yaml.safe_dumps_async({"a": 1})
    data = await pyrs_yaml.safe_loads_async(yaml)
    print(data)  # {'a': 1}


asyncio.run(main())
```

### Markdown Frontmatter

#### `read_markdown()`

Extract YAML frontmatter from a Markdown file.

```python
read_markdown(path: str, schema: str | dict = "core", max_depth: int = 1000) -> tuple[dict[str, Any] | None, str]
```

**Returns:** `(frontmatter_dict, content_string)`. If no frontmatter, `frontmatter` is `None`.

#### `read_markdown_str()`

Extract YAML frontmatter from a Markdown string.

```python
read_markdown_str(content: str, schema: str | dict = "core", max_depth: int = 1000) -> tuple[dict[str, Any] | None, str]
```

### i18n Functions

#### `set_language()`

Set the language for error messages.

```python
set_language(lang: str) -> None
```

Supported: `"en"`, `"zh-CN"`, `"ja-JP"`, `"ko-KR"`

#### `get_language()`

Get the current language.

```python
get_language() -> str
```

#### `list_languages()`

List all supported languages.

```python
list_languages() -> list[str]
```

#### `detect_language()`

Auto-detect user's preferred language from environment variables.

```python
detect_language() -> str
```

#### `negotiate_language()`

BCP 47 language negotiation.

```python
negotiate_language(user_locales: list[str], default: str = "en") -> str
```

### Exceptions

- `YamlParseError` — YAML parsing error (inherits from `ValueError`)
- `YamlSerializeError` — YAML serialization error (inherits from `ValueError`)
- `YamlTypeError` — Type conversion error (inherits from `TypeError`)
- `YamlValidateError` — JSON Schema validation error (inherits from `ValueError`)
- `YamlEditError` — In-place edit failure (inherits from `ValueError`)
- `YamlPathError` — Malformed/non-editable path (inherits from `ValueError`)
- `YamlDocumentError` — Stale `Node` access (inherits from `Exception`)
- `YamlDuplicateKeyError` — Duplicate mapping key detected (inherits from `ValueError`)
- `YamlMaxDepthError` — Exceeded maximum nesting depth (inherits from `ValueError`)
- `YamlTagError` — Invalid tag handler registration (inherits from `ValueError`)
- `YamlTagSkip` — Sentinel raised by a tag handler to skip a node (inherits from `Exception`)

See [Exceptions](exceptions.md) for full details.

### Version

```python
__version__ = "0.12.1"
```
