---
title: Serialization
description: Convert Python objects and YamlDocument instances to YAML strings, including options and round-trip preservation.
tags:
  - docs
status: new
---

## Serialization

Convert Python objects and `YamlDocument` instances to YAML strings.

### Basic Serialization

#### YamlDocument.to_yaml()

```python title="to_yaml()"
doc = pyrs_yaml.parse("key: value")
yaml_str = doc.to_yaml()  # (1)!
print(yaml_str)  # key: value\n
```

1. `to_yaml()` serializes back with all comments, anchors, and formatting preserved.

#### YamlDocument.to_yaml_with_options()

```python title="to_yaml_with_options()"
doc = pyrs_yaml.parse("key: value")

# Custom indentation and document markers
yaml_str = doc.to_yaml_with_options(
    indent_size=4,  # 4 spaces per indent level
    explicit_start=True,  # Add "---" at start
    explicit_end=True,  # Add "..." at end
    sort_keys=True,  # Sort keys alphabetically
)
```

#### PyYAML-Compatible Serialization

```python title="PyYAML-compatible serialization"
# Dict to YAML string
yaml_str = pyrs_yaml.safe_dump({"database": {"host": "localhost", "port": 5432}})

# Also available as safe_dumps (alias)
yaml_str = pyrs_yaml.safe_dumps({"key": "value"})
```

### Convert Python Objects to YAML

#### from_dict()

```python title="from_dict()"
yaml_str = pyrs_yaml.from_dict({"name": "Alice", "age": 30, "tags": ["admin", "user"]})
```

#### from_json()

```python title="from_json()"
yaml_str = pyrs_yaml.from_json('{"key": "value"}')
```

#### dump_file()

```python title="dump_file()"
# Write Python object directly to YAML file
pyrs_yaml.dump_file({"config": {"debug": True, "log_level": "info"}}, "output.yaml")
```

### Output Formats

pyrs-yaml can serialize to different destinations:

=== "string"

    ```python title="YAML string"
    yaml_str = pyrs_yaml.safe_dump({"key": "value"})
    ```

=== "file"

    ```python title="YAML file"
    pyrs_yaml.dump_file({"key": "value"}, "output.yaml")
    ```

=== "document"

    ```python title="YamlDocument"
    doc = pyrs_yaml.parse("key: value")
    yaml_str = doc.to_yaml()
    ```

### Supported Input Types

| Python Type | YAML Output |
|-------------|-------------|
| :material-language-python: `dict` | YAML mapping |
| :material-format-list-numbered: `list` | YAML sequence |
| :material-format-text: `str` | Plain or quoted scalar |
| :material-numeric: `int` | Plain integer |
| :material-decimal: `float` | Plain float |
| :material-toggle-switch: `bool` | `true` / `false` |
| :material-null: `None` | `null` |

### Round-Trip Preservation

```python title="Round-trip preservation"
# The key advantage: formatting is preserved
original = """
# Server config
server:
  host: 0.0.0.0
  port: 8080  # main port

database: &db
  host: localhost

api:
  <<: *db
"""

doc = pyrs_yaml.parse(original)
output = doc.to_yaml()

# Comments, anchors, and merge keys preserved
assert "# Server config" in output
assert "&db" in output
assert "<<: *db" in output
```
