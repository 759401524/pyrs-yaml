# ---
---

title: Serialization
lang: ja-JP

# シリアル化

Convert Python objects and `YamlDocument` instances to YAML strings.

## Basic シリアル化

### YamlDocument.to_yaml()

```python
doc = pyyaml_rs.parse("key: value")
yaml_str = doc.to_yaml()
print(yaml_str)  # key: value\n
```

### YamlDocument.to_yaml_with_options()

```python
doc = pyyaml_rs.parse("key: value")

# Custom indentation and document markers
yaml_str = doc.to_yaml_with_options(
    indent_size=4,           # 4 spaces per indent level
    explicit_start=True,     # Add "---" at start
    explicit_end=True,       # Add "..." at end
    sort_keys=True,          # Sort keys alphabetically
)
```

### PyYAML-Compatible シリアル化

```python
# Dict to YAML string
yaml_str = pyyaml_rs.safe_dump({
    "database": {
        "host": "localhost",
        "port": 5432
    }
})

# Also available as safe_dumps (alias)
yaml_str = pyyaml_rs.safe_dumps({"key": "value"})
```

## Convert Python Objects to YAML

### from_dict()

```python
yaml_str = pyyaml_rs.from_dict({
    "name": "Alice",
    "age": 30,
    "tags": ["admin", "user"]
})
```

### from_json()

```python
yaml_str = pyyaml_rs.from_json('{"key": "value"}')
```

### dump_file()

```python
# Write Python object directly to YAML file
pyyaml_rs.dump_file({
    "config": {
        "debug": True,
        "log_level": "info"
    }
}, "output.yaml")
```

## Supported Input Types

| Python Type | YAML Output |
|-------------|-------------|
| `dict` | YAML mapping |
| `list` | YAML sequence |
| `str` | Plain or quoted scalar |
| `int` | Plain integer |
| `float` | Plain float |
| `bool` | `true` / `false` |
| `None` | `null` |

## リバース保存

```python
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

doc = pyyaml_rs.parse(original)
output = doc.to_yaml()

# Comments, anchors, and merge keys preserved
assert "# Server config" in output
assert "&db" in output
assert "<<: *db" in output
```
