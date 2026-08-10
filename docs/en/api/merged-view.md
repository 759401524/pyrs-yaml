# MergedView Class

The `MergedView` class provides a read-only view of a `YamlDocument` with merge keys (`<<: *anchor`) resolved. It is accessed via `doc.merged()`.

## Overview

```python
class MergedView(Mapping):
    """Read-only view of a YAML document with merge keys resolved."""
```

The view is built lazily from `YamlDocument.to_dict()`, which resolves anchors and merge keys during serialization. The original AST is never mutated.

## Constructor

### `MergedView.__init__()`

```python
MergedView.__init__(document: YamlDocument) -> None
```

**Parameters:**

- `document` — A `YamlDocument` instance

If the document root is a sequence, the view converts it to an integer-keyed mapping (`{0: item0, 1: item1, ...}`).

## Methods

### `__getitem__()`

Access a value by key.

```python
__getitem__(key: str | int) -> Any
```

Child dicts and lists are wrapped recursively in `MergedView._DictView` and `MergedView._ListView` respectively.

**Example:**

```python
doc = pyrs_yaml.parse("""
defaults: &defaults
  timeout: 30
  retries: 3

config:
  <<: *defaults
  timeout: 60
""")

view = doc.merged()
print(view["config"]["timeout"])   # 60 (overrides merged value)
print(view["config"]["retries"])   # 3 (inherited from merge)
```

### `__len__()`

Return the number of top-level items.

```python
__len__() -> int
```

### `__iter__()`

Iterate over top-level keys.

```python
__iter__() -> Iterator[str | int]
```

### `__repr__()`

```python
__repr__() -> str
```

Returns `MergedView({...})` with the internal dict representation.

### `get()`

`get()` is inherited from `collections.abc.Mapping` — it provides `get(key, default=None)`.

```python
get(key: str | int, default: Any = None) -> Any
```

## Merge Key Resolution

Keys are resolved with the following precedence (highest wins):

1. Keys defined directly in the merging document
2. Keys from merged anchors (in the order they appear in `<<:`)
3. Later anchors override earlier ones

## Root Type Support

| Root Type | Behavior |
| --- | --- |
| Mapping | Keys are the mapping keys |
| Sequence | Keys are integer indices (`0`, `1`, ...) |
| Scalar/Null | `__len__()` returns `0`; `__getitem__()` raises `KeyError` |

## Example

```python
import pyrs_yaml

doc = pyrs_yaml.parse("""
base: &base
  host: localhost
  port: 8080

prod:
  <<: *base
  host: prod.example.com
  debug: false
""")

merged = doc.merged()
assert merged["base"]["host"] == "localhost"
assert merged["prod"]["host"] == "prod.example.com"  # overridden
assert merged["prod"]["port"] == 8080                  # inherited
assert merged["prod"]["debug"] is False                # own key
```
