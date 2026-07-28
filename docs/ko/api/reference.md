---

title: Module Reference
lang: ko-KR

## 모듈 참조

Complete API reference for the `pyyaml_rs` module.

### Core Functions

#### `parse()`

YAML 파arsing or bytes into a `YamlDocument`.

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
doc = pyyaml_rs.parse("key: value")
doc = pyyaml_rs.parse(b"key: value")
doc = pyyaml_rs.parse(yaml_str, resolve_merges=False)
```

#### `parse_file()`

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
doc = pyyaml_rs.parse_file("config.yaml")
```

#### `parse_all_docs()`

여러 YAML 문서 구문 분석 from a string.

```python
parse_all_docs(yaml: str) -> list[YamlDocument]
```

**Returns:** A list of `YamlDocument` objects

**Example:**

```python
docs = pyyaml_rs.parse_all_docs("a: 1\n---\nb: 2")
```

### PyYAML-Compatible Functions

#### `safe_load()`

Parse YAML and return native Python types.

```python
safe_load(yaml: str) -> dict[str, Any] | list[Any]
```

**Equivalent to:** `yaml.safe_load()` in PyYAML

**Example:**

```python
data = pyyaml_rs.safe_load("key: value")  # {'key': 'value'}
```

#### `safe_loads()`

여러 YAML 문서 구문 분석.

```python
safe_loads(yaml: str) -> list[dict[str, Any] | list[Any]]
```

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

YAML 파arsing into native Python objects (async).

```python
async def safe_loads_async(yaml: str, schema: str = "core") -> Any
```

#### `safe_load_async()`

YAML 파arsing into native Python objects (async).

```python
async def safe_load_async(yaml: str, schema: str = "core") -> Any
```

**Example:**

```python
import asyncio, pyyaml_rs

async def main():
    yaml = await pyyaml_rs.safe_dumps_async({"a": 1})
    data = await pyyaml_rs.safe_loads_async(yaml)
    print(data)  # {'a': 1}

asyncio.run(main())
```

### Markdown 프론트메터

#### `read_markdown()`

Extract YAML frontmatter from a Markdown file.

```python
read_markdown(path: str) -> tuple[dict[str, Any] | None, str]
```

**Returns:** `(frontmatter_dict, content_string)`. If no frontmatter, `frontmatter` is `None`.

#### `read_markdown_str()`

Extract YAML frontmatter from a Markdown string.

```python
read_markdown_str(content: str) -> tuple[dict[str, Any] | None, str]
```

### i18n Functions

#### `set_language()`

Set the language for error messages.

```python
set_language(lang: str) -> None
```

Supported: `"en"`, `"zh-CN"`

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

### 예외

- `YamlParseError` — YAML parsing error (inherits from `ValueError`)
- `YamlSerializeError` — YAML serialization error (inherits from `ValueError`)
- `YamlTypeError` — Type conversion error (inherits from `TypeError`)
- `YamlValidateError` — JSON Schema validation error (inherits from `ValueError`)

### Version

```python
__version__ = "0.6.0"
```
