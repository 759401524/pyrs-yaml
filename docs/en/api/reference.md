# Module Reference

Complete API reference for the `pyrs_yaml` module.

## Core Functions

### `parse()`

Parse a YAML string or bytes into a `YamlDocument`.

```python
parse(yaml: str | bytes, resolve_merges: bool = True) -> YamlDocument
```

**Parameters:**

- `yaml` — YAML content as `str` or `bytes`
- `resolve_merges` — Whether to resolve merge keys (`<<: *alias`) after parsing (default: `True`)

**Returns:** A `YamlDocument` containing the parsed YAML

**Raises:**

- `YamlParseError` — Invalid YAML syntax
- `TypeError` — Input is not `str` or `bytes`

**Example:**

```python
doc = pyrs_yaml.parse("key: value")
doc = pyrs_yaml.parse(b"key: value")
doc = pyrs_yaml.parse(yaml_str, resolve_merges=False)
```

### `parse_file()`

Parse a YAML file.

```python
parse_file(path: str) -> YamlDocument
```

**Parameters:**

- `path` — Path to the YAML file

**Returns:** A `YamlDocument`

**Raises:**

- `IOError` — File not found or unreadable
- `YamlParseError` — Invalid YAML

**Example:**

```python
doc = pyrs_yaml.parse_file("config.yaml")
```

### `parse_all_docs()`

Parse multiple YAML documents from a string.

```python
parse_all_docs(yaml: str) -> list[YamlDocument]
```

**Returns:** A list of `YamlDocument` objects

**Example:**

```python
docs = pyrs_yaml.parse_all_docs("a: 1\n---\nb: 2")
```

## PyYAML-Compatible Functions

### `safe_load()`

Parse YAML and return native Python types.

```python
safe_load(yaml: str) -> dict[str, Any] | list[Any]
```

**Equivalent to:** `yaml.safe_load()` in PyYAML

**Example:**

```python
data = pyrs_yaml.safe_load("key: value")  # {'key': 'value'}
```

### `safe_loads()`

Parse multiple YAML documents.

```python
safe_loads(yaml: str) -> list[dict[str, Any] | list[Any]]
```

**Equivalent to:** `yaml.safe_loads()` in PyYAML

### `safe_dump()`

Serialize a Python object to YAML.

```python
safe_dump(data: dict[str, Any] | list[Any] | ndarray) -> str
```

**Equivalent to:** `yaml.safe_dump()` in PyYAML

**Supported input types:** `dict`, `list`, `str`, `int`, `float`, `bool`, `None`, and **`numpy.ndarray`** (all dimensions and numeric dtypes: `int8/16/32/64`, `uint8/16/32/64`, `float32/64`, `complex64/128`, `bool`)

### `safe_dumps()`

Alias for `safe_dump()`.

```python
safe_dumps(data: dict[str, Any] | list[Any] | ndarray) -> str
```

## Conversion Functions

### `from_dict()`

Convert a Python dict to YAML string. Also accepts `numpy.ndarray` as a value inside the dict.

```python
from_dict(data: dict[str, Any]) -> str
```

### `from_json()`

Convert a JSON string to YAML string.

```python
from_json(json_str: str) -> str
```

### `dump_file()`

Serialize a Python object to YAML and write to file. Accepts `dict`, `list`, or `numpy.ndarray`.

```python
dump_file(data: Any, path: str) -> None
```

## Pydantic Integration

### `dump_pydantic()`

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

### `parse_as()`

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

## Tag Registry

### `register_tag()`

Register a custom tag handler. Supports both decorator and imperative forms.

```python
register_tag(name: str, handler: Callable | None = None, priority: int = 0) -> Callable
```

**Example (decorator):**

```python
@pyrs_yaml.register_tag("!custom")
def handler(node):
    return f"custom:{node}"
```

**Example (imperative):**

```python
pyrs_yaml.register_tag("!custom", handler_fn, priority=1)
```

### `remove_tag()`

Remove a tag handler.

```python
remove_tag(name: str) -> None
```

### `clear_tag_handlers()`

Remove all registered tag handlers.

```python
clear_tag_handlers() -> None
```

## Compliance

### `compliance_report()`

Compute the YAML Test Suite compliance report.

```python
compliance_report() -> dict
```

Returns the YAML Test Suite pass rate and per-test results.

## Streaming Events

### `parse_stream()`

Parse YAML incrementally, yielding raw event dicts.

```python
parse_stream(yaml: str) -> StreamIterator
```

Returns a `StreamIterator` yielding one event dict per step. Unlike `YAML().load_stream()` (which resolves into Python values), this exposes the raw token stream.

## Async Functions

Async I/O wrappers via `asyncio.run_in_executor`. Non-blocking in event loop context.

### `safe_dumps_async()`

Serialize a Python object to YAML string (async).

```python
async def safe_dumps_async(data: Any) -> str
```

### `safe_dump_async()`

Serialize a Python object to stdout as YAML (async).

```python
async def safe_dump_async(data: Any) -> None
```

### `safe_loads_async()`

Parse a YAML string into native Python objects (async).

```python
async def safe_loads_async(yaml: str, schema: str = "core") -> Any
```

### `safe_load_async()`

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

## Markdown Frontmatter

### `read_markdown()`

Extract YAML frontmatter from a Markdown file.

```python
read_markdown(path: str, schema: str = "core", max_depth: int = 1000) -> tuple[dict[str, Any] | None, str]
```

**Returns:** `(frontmatter_dict, content_string)`. If no frontmatter, `frontmatter` is `None`.

### `read_markdown_str()`

Extract YAML frontmatter from a Markdown string.

```python
read_markdown_str(content: str, schema: str = "core", max_depth: int = 1000) -> tuple[dict[str, Any] | None, str]
```

## i18n Functions

### `set_language()`

Set the language for error messages.

```python
set_language(lang: str) -> None
```

Supported: `"en"`, `"zh-CN"`, `"ja-JP"`, `"ko-KR"`

### `get_language()`

Get the current language.

```python
get_language() -> str
```

### `list_languages()`

List all supported languages.

```python
list_languages() -> list[str]
```

### `detect_language()`

Auto-detect user's preferred language from environment variables.

```python
detect_language() -> str
```

### `negotiate_language()`

BCP 47 language negotiation.

```python
negotiate_language(user_locales: list[str], default: str = "en") -> str
```

## Exceptions

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

## Version

```python
__version__ = "0.12.1"
```
