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

Install pyrs-yaml from PyPI:

```bash title="Install from PyPI"
pip install pyrs-yaml
```

### 2. Parse YAML

```python title="Parse and access values"
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

```python title="safe_load for native types"
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

```python title="safe_dump a dict"
# Convert a Python dict back to YAML
yaml_str = pyrs_yaml.safe_dump({"database": {"host": "localhost", "port": 5432, "name": "mydb"}})
print(yaml_str)
# database:
#   host: localhost
#   port: 5432
#   name: mydb
```

### 5. Preserve Formatting (Round-Trip)

```python title="Comments and anchors survive round-trip"
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
doc = pyrs_yaml.parse(original)  # (1)!
output = doc.to_yaml()  # (2)!

# The output matches the input (or is semantically equivalent)
assert "# Server configuration" in output  # (3)!
assert "&db" in output  # (4)!
```

1. :material-arrow-down: `parse` builds a `YamlDocument` that retains every comment, anchor, tag, and style.
2. :material-arrow-down: `to_yaml` re-serializes from the AST, preserving formatting — no string manipulation.
3. :material-arrow-down: Standalone comments survive round-trip intact.
4. :material-arrow-down: Anchors (`&db`) and aliases (`*db`) and merge keys (`<<`) are preserved.

### 6. Edit In Place

```python title="Edit by JSONPath"
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

```python title="parse_file"
# Parse a YAML file directly
doc = pyrs_yaml.parse_file("config.yaml")
print(doc.get("name"))
```

### 8. Multiple Documents

```python title="parse_all_docs"
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

??? note "Optional: requires NumPy"

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

### 10. Manipulate Metadata (comment, anchor, tag)

YAML metadata — comments, anchors, and tags — survive round-trip by default;
you can also **read and edit them** through the `Node` API:

```python title="Set comment, anchor, tag"
doc = pyrs_yaml.parse("key: value")
node = doc.node().find("$.key")

# Set a comment (standalone: own line above value)
node.set_comment("a note")

# Set an anchor
node.set_anchor("cfg")

# Set a tag
node.set_tag("!custom")

print(doc.to_yaml())
# key: &cfg !custom value  # a note
```

Metadata can be removed as well:

```python title="Remove metadata"
node.remove_comment()
node.remove_anchor()
node.remove_tag()
```

### 11. Control Formatting (scalar style, flow style, chomping)

pyrs-yaml preserves the **scalar style** (plain, single-quoted, double-quoted,
literal, folded), **flow style** (block vs. JSON-like `{}`/`[]`), and
**chomping indicator** (strip, clip, keep) of every node:

```python title="Style, flow, chomping"
doc = pyrs_yaml.parse("key: value")

# Switch scalar style to single-quoted
doc.node().find("$.key").set_scalar_style("single_quoted")
print(doc.to_yaml())  # key: 'value'

# Switch the root document to flow style
doc.node().find("$").set_flow_style(True)
print(doc.to_yaml())  # {key: 'value'}

# Change chomping on a literal block scalar
doc = pyrs_yaml.parse("text: |\n  hello\n  world\n")
doc.node().find("$.text").set_chomping("strip")
print(doc.to_yaml())  # text: |-\n  hello\n  world
```

### 12. Validate with a Schema

??? note "Optional: YAML Schema Language"

    Define a YAML Schema Language document with structural rules and validate
    data against it:

    ```python
    import pyrs_yaml

    schema = """\
    name: app
    extends: core
    validate:
      - path: $.port
        type: int
        required: true
      - path: $.tags[*]
        type: str
    """

    # Valid document passes
    pyrs_yaml.validate_against_schema("port: 8080\ntags: [web, api]\n", schema)

    # Invalid document raises YamlValidateError with every failure
    pyrs_yaml.validate_against_schema("port: abc\n", schema)
    # YamlValidateError: $.port: expected int but got Str("abc")
    ```

### 13. Deep Editing (batch, sort, move, copy)

Edit multiple paths at once, sort keys, relocate subtrees, and deep-copy
values — all while preserving every other YAML feature:

```python title="Batch, sort, move, copy"
# Batch set with wildcards
doc = pyrs_yaml.parse("items:\n  - active: true\n  - active: true\n")
doc.set_many({"$.items[*].active": False})
print(doc.to_yaml())
# items:
#   - active: false
#   - active: false

# Sort mapping keys in place
doc = pyrs_yaml.parse("z: 1\na: 2\nm: 3\n")
doc.sort_keys()
print(doc.to_yaml())  # a: 2\nm: 3\nz: 1

# Move a subtree to a new path
doc = pyrs_yaml.parse("src:\n  x: 1\ndst: {}\n")
doc.node().find("$.src").move("$.dst")
print(doc.to_yaml())  # dst:\n  x: 1

# Deep-copy a subtree as a standalone value
node = doc.node().find("$.dst")
copied = node.copy()  # returns dict/list/scalar, detached from doc
```

### Next Steps

<div class="grid cards" markdown>

- :material-feature-search: **[Features](features.md)** — Explore all supported YAML features
- :material-file-search: **[Parsing Guide](guides/parsing.md)** — Advanced parsing options
- :material-pencil: **[In-Place Editing](guides/editing.md)** — Edit documents without losing formatting
- :material-book-open-variant: **[Configuration Management Tutorial](guides/tutorial-config-management.md)** — End-to-end walkthrough
- :material-code-braces: **[API Reference](api/reference.md)** — Complete API documentation

</div>
