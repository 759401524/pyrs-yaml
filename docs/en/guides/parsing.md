---
title: Parsing YAML
description: Guide to all ways of parsing YAML with pyrs-yaml, including basic parsing, file parsing, and error handling.
tags:
  - docs
status: new
---

## Parsing YAML

This guide covers all ways to parse YAML with pyrs-yaml.

### Basic Parsing

#### Parse a YAML String

```python title="Parse a string"
import pyrs_yaml

doc = pyrs_yaml.parse("key: value")  # (1)!
print(doc.get("key"))  # value
```

1. `parse()` returns a [`YamlDocument`](../api/yaml-document.md) that preserves comments, anchors, and formatting.

#### Parse with Options

```python title="Parse with options"
# Disable merge key resolution (keep <<: *alias as-is)
doc = pyrs_yaml.parse(yaml_text, resolve_merges=False)
```

#### Parse a YAML File

```python title="Parse a file"
doc = pyrs_yaml.parse_file("config.yaml")
print(doc.get("name"))
```

#### Parse Multiple Documents

```python title="Parse multiple documents"
# YAML with --- separators
yaml_text = """
---
name: first
---
name: second
"""

docs = pyrs_yaml.parse_all_docs(yaml_text)
print(len(docs))  # 2
print(docs[0].get("name"))  # first
print(docs[1].get("name"))  # second
```

#### PyYAML-Compatible Parsing

!!! tip "PyYAML-compatible parsing"
    Use `safe_load` to get native Python types (`dict`, `list`, `str`, `int`,
    etc.) instead of a `YamlDocument`. For multiple documents, `safe_loads`
    returns a list of parsed objects.

```python title="PyYAML-compatible parsing"
# Returns native Python types (dict, list, str, int, etc.)
data = pyrs_yaml.safe_load("key: value")
print(data)  # {'key': 'value'}

# Multiple documents
docs = pyrs_yaml.safe_loads("a: 1\n---\nb: 2")
print(len(docs))  # 2
```

### Acceptable Input Types

pyrs-yaml accepts three input forms:

- :material-language-python: **`str`** — standard YAML string
- :material-binary: **`bytes`** — valid UTF-8 encoded bytes
- :material-format-list-bulleted: **`str` with BOM** — handled correctly

=== "str"

    ```python title="str input"
    doc = pyrs_yaml.parse("key: value")
    ```

=== "bytes"

    ```python title="bytes input"
    doc = pyrs_yaml.parse(b"key: value")
    ```

### Error Handling

```python title="Error handling"
try:
    doc = pyrs_yaml.parse("invalid: yaml: [")
except pyrs_yaml.YamlParseError as e:
    print(f"Parse error: {e}")
```

### Supported Data Types

pyrs-yaml correctly parses all YAML 1.2 scalar types:

| Type | Example | Python Type |
|------|---------|-------------|
| :material-format-text: String | `hello` | `str` |
| :material-numeric: Integer | `42`, `0x1A`, `0o77` | `int` |
| :material-decimal: Float | `3.14`, `1.23e-4` | `float` |
| :material-toggle-switch: Boolean | `true`, `false` | `bool` |
| :material-null: Null | `null`, `~` | `None` |
| :material-infinity: Infinity | `.inf`, `-.inf` | `float` |
| :material-alphabetical: NaN | `.nan` | `float` |
