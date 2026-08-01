# YamlDocument Class

The `YamlDocument` class represents a parsed YAML document with full metadata preservation.

## Overview

```python
class YamlDocument:
    """A parsed YAML document with perfect round-trip support."""
```

## Methods

### `to_yaml()`

Convert the document back to a YAML string.

```python
to_yaml() -> str
```

**Returns:** The complete YAML document string, ending with a newline.

**Example:**

```python
doc = pyrs_yaml.parse("key: value")
print(doc.to_yaml())  # key: value\n
```

### `to_yaml_with_options()`

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

### `to_dict()`

Convert to a Python dict/list, resolving alias references.

```python
to_dict() -> dict[str, Any] | list[Any]
```

**Returns:** Native Python types. Anchors (`&name`) are inlined, aliases (`*name`) are replaced with actual values. Scalars are converted to Python native types (bool/int/float/str/None).

**Example:**

```python
data = doc.to_dict()
print(data["key"])  # value
print(type(data))  # <class 'dict'>
```

### `get()`

Get a value by key (for mapping roots) or by JSONPath-like path.

```python
get(key: str, default: Any = None) -> Any
```

**Parameters:**

- `key` — The key to look up, or a path. A key containing `.`, `[`, or starting with `$` is treated as a path: `$.a.b` (nested keys) and `$.arr[0]` / `$.arr[-1]` (sequence indexes, negative counts from the end) are resolved via the same navigation rules as the edit methods
- `default` — Value to return if the key/path is not found (default: None)

**Returns:** The value, or `default` if not found (or if root is not a mapping).

**Raises:** `YamlPathError` — malformed path (e.g. `$[bad`, or a wildcard/deep-scan path)

**Example:**

```python
value = doc.get("key")
value = doc.get("$.a.b")  # nested path
value = doc.get("$.items[-1]")  # last element
value = doc.get("missing", "fallback")
```

### `root_type()`

Get the root node type as a string.

```python
root_type() -> str
```

**Returns:** One of `"scalar"`, `"mapping"`, `"sequence"`, `"null"`, `"alias"`.

**Example:**

```python
print(doc.root_type())  # "mapping"
```

### `to_json()`

Serialize the document to a JSON string.

```python
to_json(indent: int = 2) -> str
```

**Parameters:**

- `indent` — JSON indentation spaces (default: 2)

**Returns:** A JSON string of the document contents.

**Example:**

```python
doc = pyrs_yaml.parse("a: 1\nb: hello")
json_str = doc.to_json()  # '{"a": 1, "b": "hello"}'
```

### `validate()`

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
doc = pyrs_yaml.parse("name: Alice\nage: 30")
doc.validate({"type": "object", "properties": {"name": {"type": "string"}}})

# From JSON string
doc.validate('{"type": "object", "required": ["name"]}')
```

### `source()`

Return the original YAML source text used to create this document. If the document has been edited in place, the source is lazily re-serialized from the current tree on first access.

```python
source() -> str
```

**Returns:** The YAML string. Empty string if the document was not created via `parse()` (e.g. from `from_dict()`).

**Example:**

```python
doc = pyrs_yaml.parse("key: value")
print(doc.source())  # "key: value"
```

### `reparse()`

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
doc = pyrs_yaml.parse("x: on")
print(doc.get("x"))  # "on" (string, core schema)

doc.reparse(schema="yaml1.1")
print(doc.get("x"))  # True (bool, yaml1.1 schema)
```

## Editing Methods

All edits are atomic — a failed edit leaves the document (and its revision) untouched. On success, the stored source is marked dirty and the next `source()` / `to_yaml()` / `to_yaml_with_options()` / `reparse()` call re-serializes from the updated tree. See the [In-Place Editing guide](../guides/editing.md) for the full path syntax and semantics.

### `set()`

Set the value at a path, preserving the target's metadata (comment, anchor, tag, style). Setting a path on an **empty document** (parsed from `""`) auto-creates a mapping root.

```python
set(path: str, value: Any) -> None
```

```python
doc = pyrs_yaml.parse("a:\n  b: 1")
doc.set("$.a.b", 42)  # replace existing
doc.set("$.a.c", True)  # create new key
doc.set("$", {"x": 1})  # replace the root

empty = pyrs_yaml.parse("")
empty.set("$.a", 1)  # auto-creates a mapping root: {a: 1}
```

**Raises:**

- `YamlPathError` — Malformed path (wildcards/`..` are rejected)
- `YamlEditError` — Navigation failure, unsupported value type (`tuple`), etc.

### `insert()`

Insert a value into a sequence at an index. The path must resolve to a sequence. Negative indexes count from the end (`-1` inserts before the last element).

```python
insert(path: str, index: int, value: Any) -> None
```

```python
doc = pyrs_yaml.parse("items: [a, c]")
doc.insert("$.items", 1, "b")  # items: [a, b, c]
doc.insert("$.items", -1, "x")  # items: [a, b, x, c]
```

### `append()`

Append a value to a sequence. The path must resolve to a sequence.

```python
append(path: str, value: Any) -> None
```

```python
doc = pyrs_yaml.parse("items: [a, b]")
doc.append("$.items", "c")
```

### `delete()`

Delete the node at a path, preserving mapping/sequence order.

```python
delete(path: str) -> None
```

```python
doc = pyrs_yaml.parse("a: 1\nb: 2\nc: 3")
doc.delete("$.b")
# a: 1
# c: 3
```

### `rename()`

Rename a mapping key in place, preserving position and key metadata.

```python
rename(path: str, new_key: str) -> None
```

```python
doc = pyrs_yaml.parse("old: 1  # comment")
doc.rename("$.old", "new")
# new: 1  # comment
```

### `node()`

Get the root `Node` of the document for tree navigation and editing.

```python
node() -> Node
```

```python
node = doc.node().find("$.a.b")
node.set_value(42)
```

### `find()`

Query the document by JSONPath-like path. Supports wildcards (`[*]`) and deep-scan (`..`), returning a list when multiple nodes match.

```python
find(path: str) -> Node | list[Node]
```

### `__setitem__()` / `__delitem__()` — root sugar

```python
doc["key"] = value  # equivalent to doc.set("$.key", value)
del doc["key"]  # equivalent to doc.delete("$.key")
```

## Dunder Methods

### `__getitem__()`

Access by key (mapping) or index (sequence).

```python
doc["key"]  # For mappings
doc[0]  # For sequences
```

**Raises:**

- `KeyError` — Key not found in mapping
- `IndexError` — Index out of range for sequence
- `TypeError` — Document not subscriptable

### `__contains__()`

Check if a key exists.

```python
"key" in doc  # Returns bool
```

### `__len__()`

Get the number of items.

```python
len(doc)  # Number of keys (mapping) or items (sequence)
```

### `__iter__()`

Iterate over keys (mapping) or values (sequence).

```python
for key in doc:
    print(key, doc[key])
```

### `__repr__()`

Debug representation.

```python
repr(doc)  # "YamlDocument(<yaml>...)"
```

### `__str__()`

String representation.

```python
str(doc)  # Same as doc.to_yaml()
```

## Example

```python
import pyrs_yaml

doc = pyrs_yaml.parse("""
name: Alice
age: 30
""")

print(doc.get("name"))  # Alice
print(doc.root_type())  # mapping
print(len(doc))  # 2
print("name" in doc)  # True
for key in doc:
    print(key, doc[key])  # name Alice, age 30
```
