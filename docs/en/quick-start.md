# Quick Start

This guide will get you up and running with pyyaml-rs in minutes.

## 1. Install

The package is not yet on PyPI. Install from source:

```bash
git clone https://github.com/759401524/pyyaml-rs.git
cd pyyaml-rs
uv run --frozen maturin develop --release
```

## 2. Parse YAML

```python
import pyyaml_rs

# Parse a YAML string
doc = pyyaml_rs.parse("""
name: Alice
age: 30
email: alice@example.com
""")

# Access values
print(doc.get("name"))    # Alice
print(doc.get("age"))     # 30
print(doc.get("email"))   # alice@example.com
```

## 3. Convert to Python Objects

```python
# Use safe_load for PyYAML-compatible behavior
data = pyyaml_rs.safe_load("""
users:
  - name: Alice
    role: admin
  - name: Bob
    role: user
""")

# Returns native Python types (dict, list, str, int, etc.)
print(data["users"][0]["name"])  # Alice
print(type(data["users"]))       # <class 'list'>
```

## 4. Serialize to YAML

```python
# Convert a Python dict back to YAML
yaml_str = pyyaml_rs.safe_dump({
    "database": {
        "host": "localhost",
        "port": 5432,
        "name": "mydb"
    }
})
print(yaml_str)
# database:
#   host: localhost
#   port: 5432
#   name: mydb
```

## 5. Preserve Formatting (Round-Trip)

```python
# The key advantage of pyyaml-rs
original = """
# Server configuration
server:
  host: 0.0.0.0
  port: 8080

# Database settings
database: &db
  host: localhost
  port: 5432

# Use the database anchor
api:
  <<: *db
  endpoint: /api/v1
"""

# Parse and re-serialize — comments and anchors preserved
doc = pyyaml_rs.parse(original)
output = doc.to_yaml()

# The output matches the input (or is semantically equivalent)
assert "# Server configuration" in output
assert "&db" in output
```

## 6. Read YAML from Files

```python
# Parse a YAML file directly
doc = pyyaml_rs.parse_file("config.yaml")
print(doc.get("name"))
```

## 7. Multiple Documents

```python
# Parse multiple YAML documents
yaml_text = """
---
name: config1
value: 1
---
name: config2
value: 2
"""

docs = pyyaml_rs.parse_all_docs(yaml_text)
print(len(docs))  # 2
print(docs[0].get("name"))  # config1
```

## 8. NumPy ndarray Support

pyyaml-rs can serialize `numpy.ndarray` objects directly to YAML. This is useful for saving scientific data, model weights, or any multi-dimensional array to a human-readable format.

```python
import numpy as np
import pyyaml_rs

# 1-D array
arr = np.array([1, 2, 3], dtype="int32")
yaml_str = pyyaml_rs.safe_dump(arr)
print(yaml_str)
# - 1
# - 2
# - 3

# 2-D matrix
matrix = np.array([[1.0, 2.0], [3.0, 4.0]], dtype="float64")
yaml_str = pyyaml_rs.safe_dump(matrix)
print(yaml_str)
# -
#   - 1.0
#   - 2.0
# -
#   - 3.0
#   - 4.0

# Round-trip preserves values
loaded = pyyaml_rs.safe_load(yaml_str)
assert loaded == [[1.0, 2.0], [3.0, 4.0]]
```

### Supported NumPy dtypes

| NumPy dtype | YAML output | Notes |
|-------------|-------------|-------|
| `int8/16/32/64` | Plain integer | Quoted if negative |
| `uint8/16/32/64` | Plain integer | — |
| `float32/64` | Plain float | Quoted if negative |
| `complex64/128` | `(re+imj)` string | No native YAML complex type |
| `bool` | `true` / `false` | — |

## Next Steps

- **[Features](features.md)** — Explore all supported YAML features
- **[Parsing Guide](guides/parsing.md)** — Advanced parsing options
- **[API Reference](api/reference.md)** — Complete API documentation
