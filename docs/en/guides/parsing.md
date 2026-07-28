# Parsing YAML

This guide covers all ways to parse YAML with pyrs-yaml.

## Basic Parsing

### Parse a YAML String

```python
import pyrs_yaml

doc = pyrs_yaml.parse("key: value")
print(doc.get("key"))  # value
```

### Parse with Options

```python
# Disable merge key resolution (keep <<: *alias as-is)
doc = pyrs_yaml.parse(yaml_text, resolve_merges=False)
```

### Parse a YAML File

```python
doc = pyrs_yaml.parse_file("config.yaml")
print(doc.get("name"))
```

### Parse Multiple Documents

```python
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

### PyYAML-Compatible Parsing

```python
# Returns native Python types (dict, list, str, int, etc.)
data = pyrs_yaml.safe_load("key: value")
print(data)  # {'key': 'value'}

# Multiple documents
docs = pyrs_yaml.safe_loads("a: 1\n---\nb: 2")
print(len(docs))  # 2
```

## Acceptable Input Types

- `str` — standard YAML string
- `bytes` — valid UTF-8 encoded bytes
- `str` with BOM — handled correctly

```python
# Accepts both str and bytes
doc1 = pyrs_yaml.parse("key: value")
doc2 = pyrs_yaml.parse(b"key: value")
```

## Error Handling

```python
try:
    doc = pyrs_yaml.parse("invalid: yaml: [")
except pyrs_yaml.YamlParseError as e:
    print(f"Parse error: {e}")
```

## Supported Data Types

pyrs-yaml correctly parses all YAML 1.2 scalar types:

| Type | Example | Python Type |
|------|---------|-------------|
| String | `hello` | `str` |
| Integer | `42`, `0x1A`, `0o77` | `int` |
| Float | `3.14`, `1.23e-4` | `float` |
| Boolean | `true`, `false` | `bool` |
| Null | `null`, `~` | `None` |
| Infinity | `.inf`, `-.inf` | `float` |
| NaN | `.nan` | `float` |
