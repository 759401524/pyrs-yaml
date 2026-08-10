---
title: Quick Start
description: Get up and running with pyrs-yaml in minutes, covering parsing, serialization, round-trip, and in-place editing.
tags:
  - docs
status: new
---

## Quick Start

This guide will get you up and running with pyrs-yaml in minutes.

### 1. Install

The package is not yet on PyPI. Install from source:

```bash
git clone https://github.com/759401524/pyrs-yaml.git
cd pyrs-yaml
uv run --frozen maturin develop --release
```

### 2. Parse YAML

```python
import pyrs_yaml

# Parse a YAML string
doc = pyrs_yaml.parse("""
name: Alice
age: 30
email: alice@example.com
""")

# Access values
print(doc.get("name"))  # Alice
print(doc.get("age"))  # 30
print(doc.get("email"))  # alice@example.com
```

### 3. Convert to Python Objects

```python
# Use safe_load for PyYAML-compatible behavior
data = pyrs_yaml.safe_load("""
users:
  - name: Alice
    role: admin
  - name: Bob
    role: user
""")

# Returns native Python types (dict, list, str, int, etc.)
print(data["users"][0]["name"])  # Alice
print(type(data["users"]))  # <class 'list'>
```

### 4. Serialize to YAML

```python
# Convert a Python dict back to YAML
yaml_str = pyrs_yaml.safe_dump({"database": {"host": "localhost", "port": 5432, "name": "mydb"}})
print(yaml_str)
# database:
#   host: localhost
#   port: 5432
#   name: mydb
```

### 5. Preserve Formatting (Round-Trip)

```python
# The key advantage of pyrs-yaml
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
doc = pyrs_yaml.parse(original)
output = doc.to_yaml()

# The output matches the input (or is semantically equivalent)
assert "# Server configuration" in output
assert "&db" in output
```

### 6. Edit In Place

```python
# Edit a parsed document without losing comments or formatting
doc = pyrs_yaml.parse("""
server:
  host: localhost  # bind address
  ports:
    - 8080
""")

doc.set("$.server.host", "0.0.0.0")  # replace by path
doc.append("$.server.ports", 443)  # append to a sequence

print(doc.to_yaml())
# server:
#   host: 0.0.0.0  # bind address
#   ports:
#     - 8080
#     - 443
```

See the [In-Place Editing guide](guides/editing.md) for the full API.

### 7. Read YAML from Files

```python
# Parse a YAML file directly
doc = pyrs_yaml.parse_file("config.yaml")
print(doc.get("name"))
```

### 8. Multiple Documents

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

docs = pyrs_yaml.parse_all_docs(yaml_text)
print(len(docs))  # 2
print(docs[0].get("name"))  # config1
```

### 9. NumPy ndarray Support

pyrs-yaml can serialize `numpy.ndarray` objects directly to YAML. This is useful for saving scientific data, model weights, or any multi-dimensional array to a human-readable format.

```python
import numpy as np
import pyrs_yaml

# 1-D array
arr = np.array([1, 2, 3], dtype="int32")
yaml_str = pyrs_yaml.safe_dump(arr)
print(yaml_str)
# - 1
# - 2
# - 3

# 2-D matrix
matrix = np.array([[1.0, 2.0], [3.0, 4.0]], dtype="float64")
yaml_str = pyrs_yaml.safe_dump(matrix)
print(yaml_str)
# -
#   - 1.0
#   - 2.0
# -
#   - 3.0
#   - 4.0

# Round-trip preserves values
loaded = pyrs_yaml.safe_load(yaml_str)
assert loaded == [[1.0, 2.0], [3.0, 4.0]]
```

#### Supported NumPy dtypes

| NumPy dtype | YAML output | Notes |
|-------------|-------------|-------|
| `int8/16/32/64` | Plain integer | Quoted if negative |
| `uint8/16/32/64` | Plain integer | — |
| `float32/64` | Plain float | Quoted if negative |
| `complex64/128` | `(re+imj)` string | No native YAML complex type |
| `bool` | `true` / `false` | — |

### Next Steps

- **[Features](features.md)** — Explore all supported YAML features
- **[Parsing Guide](guides/parsing.md)** — Advanced parsing options
- **[In-Place Editing](guides/editing.md)** — Edit documents without losing formatting
- **[API Reference](api/reference.md)** — Complete API documentation
