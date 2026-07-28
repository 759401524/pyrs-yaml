# ---

---

title: YamlDocument Class
lang: ja-JP

## YamlDocument クラス

The `YamlDocument` class represents a parsed YAML document with full metadata preservation.

### 概要

```python
class YamlDocument:
    """A parsed YAML document with perfect round-trip support."""
```

### Methods

#### `to_yaml()`

Convert the document back to a YAML string.

```python
to_yaml() -> str
```

**Returns:** The complete YAML document string, ending with a newline.

**Example:**

```python
doc = pyyaml_rs.parse("key: value")
print(doc.to_yaml())  # key: value\n
```

#### `to_yaml_with_options()`

Convert to YAML with custom options.

```python
to_yaml_with_options(
    indent_size: int = 2,
    explicit_start: bool = False,
    explicit_end: bool = False,
    sort_keys: bool = False,
) -> str
```

**Parameters:**

- `indent_size` — Spaces per indent level (default: 2)
- `explicit_start` — Add `---` at document start (default: False)
- `explicit_end` — Add `...` at document end (default: False)
- `sort_keys` — Sort keys alphabetically (default: False)

**Example:**

```python
yaml_str = doc.to_yaml_with_options(
    indent_size=4,
    explicit_start=True,
    sort_keys=True,
)
```

#### `to_dict()`

Convert to a Python dict/list, resolving alias references.

```python
to_dict() -> dict[str, Any] | list[Any]
```

**Returns:** Native Python types. Anchors (`&name`) are inlined, aliases (`*name`) are replaced with actual values. Scalars are converted to Python native types (bool/int/float/str/None).

**Example:**

```python
data = doc.to_dict()
print(data["key"])  # value
print(type(data))   # <class 'dict'>
```

#### `get()`

Get a value by key (for mapping roots).

```python
get(key: str, default: Any = None) -> Any
```

**Parameters:**

- `key` — The key to look up
- `default` — Value to return if key not found (default: None)

**Returns:** The value, or `default` if not found (or if root is not a mapping).

**Example:**

```python
value = doc.get("key")
value = doc.get("missing", "fallback")
```

#### `root_type()`

Get the root node type as a string.

```python
root_type() -> str
```

**Returns:** One of `"scalar"`, `"mapping"`, `"sequence"`, `"null"`, `"alias"`.

**Example:**

```python
print(doc.root_type())  # "mapping"
```

#### `to_json()`

Serialize the document to a JSON string.

```python
to_json(indent: int = 2) -> str
```

**Parameters:**

- `indent` — JSON indentation spaces (default: 2)

**Returns:** A JSON string of the document contents.

**Example:**

```python
doc = pyyaml_rs.parse("a: 1\nb: hello")
json_str = doc.to_json()  # '{"a": 1, "b": "hello"}'
```

#### `validate()`

Validate the document contents against a JSON Schema.

```python
validate(schema: str | dict[str, Any]) -> None
```

**Parameters:**

- `schema` — JSON Schema as either a JSON string or a Python dict

**Returns:** `None` on success.

**Raises:**

- `YamlValidateError` — Document does not conform to the schema

**Example:**

```python
doc = pyyaml_rs.parse("name: Alice\nage: 30")
doc.validate({"type": "object", "properties": {"name": {"type": "string"}}})

# From JSON string
doc.validate('{"type": "object", "required": ["name"]}')
```

#### `source()`

Return the original YAML source text used to create this document.

```python
source() -> str | None
```

**Returns:** The original YAML string, or `None` if the document was not created via `parse()` (e.g. from `from_dict()`).

**Example:**

```python
doc = pyyaml_rs.parse("key: value")
print(doc.source())  # "key: value"
```

#### `reparse()`

Re-parse the stored source text in place, allowing schema or merge behavior changes.

```python
reparse(resolve_merges: bool = True, schema: str = "core") -> None
```

**Parameters:**

- `resolve_merges` — Whether to resolve `<<: *alias` merge keys (default: `True`)
- `schema` — Type resolution schema: `"core"`, `"json"`, `"failsafe"`, or `"yaml1.1"` (default: `"core"`)

**Raises:**

- `TypeError` — No source text stored
- `YamlParseError` — Re-parsing failed

**Example:**

```python
doc = pyyaml_rs.parse("x: on")
print(doc.get("x"))  # "on" (string, core schema)

doc.reparse(schema="yaml1.1")
print(doc.get("x"))  # True (bool, yaml1.1 schema)
```

### Dunder Methods

#### `__getitem__()`

Access by key (mapping) or index (sequence).

```python
doc["key"]      # For mappings
doc[0]          # For sequences
```

**Raises:**

- `KeyError` — Key not found in mapping
- `IndexError` — Index out of range for sequence
- `TypeError` — Document not subscriptable

#### `__contains__()`

Check if a key exists.

```python
"key" in doc  # Returns bool
```

#### `__len__()`

Get the number of items.

```python
len(doc)  # Number of keys (mapping) or items (sequence)
```

#### `__iter__()`

Iterate over keys (mapping) or values (sequence).

```python
for key in doc:
    print(key, doc[key])
```

#### `__repr__()`

Debug representation.

```python
repr(doc)  # "YamlDocument(<yaml>...)"
```

#### `__str__()`

String representation.

```python
str(doc)  # Same as doc.to_yaml()
```

### Example

```python
import pyyaml_rs

doc = pyyaml_rs.parse("""
name: Alice
age: 30
""")

print(doc.get("name"))    # Alice
print(doc.root_type())    # mapping
print(len(doc))           # 2
print("name" in doc)      # True
for key in doc:
    print(key, doc[key])  # name Alice, age 30
```
