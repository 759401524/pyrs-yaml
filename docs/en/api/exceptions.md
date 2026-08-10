---
title: Exceptions
description: Custom exception classes defined by pyrs-yaml for error handling, with i18n support and best practices.
tags:
  - docs
status: new
---

## Exceptions

pyrs-yaml defines custom exception classes for error handling.

### YamlParseError

Raised when YAML parsing fails.

```python
class YamlParseError(ValueError):
    """YAML parsing error (inherits from ValueError)."""
```

**Inherits from:** `ValueError`

**Example:**

```python
try:
    doc = pyrs_yaml.parse("invalid: yaml: [")
except pyrs_yaml.YamlParseError as e:
    print(f"Parse error: {e}")
    # Output: "YAML parse error: line 1, column 14: unexpected token..."
```

**Can be caught as:**

```python
except ValueError as e:  # Also works
```

### YamlSerializeError

Raised when YAML serialization fails.

```python
class YamlSerializeError(ValueError):
    """YAML serialization error (inherits from ValueError)."""
```

**Inherits from:** `ValueError`

**Example:**

```python
try:
    yaml_str = pyrs_yaml.safe_dump(some_unsupported_type)
except pyrs_yaml.YamlSerializeError as e:
    print(f"Serialize error: {e}")
```

### YamlTypeError

Raised when a type conversion error occurs.

```python
class YamlTypeError(TypeError):
    """YAML type conversion error (inherits from TypeError)."""
```

**Inherits from:** `TypeError`

**Example:**

```python
try:
    pyrs_yaml.parse(123)  # Expected str or bytes
except pyrs_yaml.YamlTypeError as e:
    print(f"Type error: {e}")
```

### YamlValidateError

Raised when JSON Schema validation fails via `YamlDocument.validate()`.

```python
class YamlValidateError(ValueError):
    """JSON Schema validation error (inherits from ValueError)."""
```

**Inherits from:** `ValueError`

**Example:**

```python
doc = pyrs_yaml.parse("name: Alice")
schema = {"type": "object", "required": ["name", "email"]}
try:
    doc.validate(schema)
except pyrs_yaml.YamlValidateError as e:
    print(f"Validation error: {e}")
    # Output: "Validation error: 'email' is a required property"
```

### YamlEditError

Raised when an in-place edit cannot be applied: unsupported value types (`tuple`), negative indices, edits through aliases, renaming the root or complex keys, navigation into a scalar, or out-of-bounds indices.

```python
class YamlEditError(ValueError):
    """In-place edit error (inherits from ValueError)."""
```

**Inherits from:** `ValueError`

**Example:**

```python
doc = pyrs_yaml.parse("a:\n  b: 1")

try:
    doc.set("$.a.b.c", 2)  # Navigates into a scalar
except pyrs_yaml.YamlEditError as e:
    print(f"Edit error: {e}")
```

### YamlPathError

Raised when a JSONPath-style path is malformed or not editable: paths not starting with `$`, wildcard (`[*]`) or deep-scan (`..`) segments used in edit operations.

```python
class YamlPathError(ValueError):
    """YAML path error (inherits from ValueError)."""
```

**Inherits from:** `ValueError`

**Example:**

```python
doc = pyrs_yaml.parse("items: [1, 2]")

try:
    doc.set("$.items[*]", 3)  # Wildcards are not editable
except pyrs_yaml.YamlPathError as e:
    print(f"Path error: {e}")
```

### YamlDocumentError

Raised when a `Node` becomes stale — the document was modified (or released) after the node was created.

```python
class YamlDocumentError(Exception):
    """Raised when a Node's parent YamlDocument is stale."""
```

**Inherits from:** `Exception`

**Example:**

```python
node = doc.node().find("$.a")
doc.set("$.b", 2)  # Bumps the document revision
node.set_value(99)  # RuntimeWarning + YamlDocumentError
```

### YamlDuplicateKeyError

Raised when a duplicate mapping key is detected in the input.

```python
class YamlDuplicateKeyError(ValueError):
    """Duplicate mapping key error (inherits from ValueError)."""
```

**Inherits from:** `ValueError`

**Example:**

```python
try:
    pyrs_yaml.parse("key: 1\nkey: 2")
except pyrs_yaml.YamlDuplicateKeyError as e:
    print(f"Duplicate key: {e}")
```

### YamlMaxDepthError

Raised when the YAML document exceeds the maximum nesting depth.

```python
class YamlMaxDepthError(ValueError):
    """Maximum nesting depth exceeded (inherits from ValueError)."""
```

**Inherits from:** `ValueError`

**Example:**

```python
try:
    pyrs_yaml.parse("a:\n  b:\n    c:\n      ...", max_depth=2)
except pyrs_yaml.YamlMaxDepthError as e:
    print(f"Max depth exceeded: {e}")
```

### YamlTagError

Raised when a tag handler is registered with an invalid name or signature.

```python
class YamlTagError(ValueError):
    """Tag handler error (inherits from ValueError)."""
```

**Inherits from:** `ValueError`

### YamlTagSkip

Sentinel exception raised by a tag handler to skip a node. The parser moves to the next node instead of raising an error. This is not a real error — it is an intentional control-flow signal.

```python
class YamlTagSkip(Exception):
    """Tag handler skip sentinel (inherits from Exception)."""
```

**Inherits from:** `Exception`

**Example:**

```python
@pyrs_yaml.register_tag("!skip_me")
def handler(node):
    raise pyrs_yaml.YamlTagSkip
```

### Error Message Format

All error messages include contextual information:

| Error | Format |
|-------|--------|
| Parse error | `"YAML parse error: line N, column M: <message>"` |
| File not found | `"File read error: <path> — <OS error>"` |
| Invalid UTF-8 | `"Invalid UTF-8: <detail>"` |
| Key not found | `"Key not found: <key>"` |
| Index out of range | `"Index out of range: <index> (len: <len>)"` |
| Unsupported type | `"Unsupported type for YAML conversion"` |
| ndarray unsupported dtype | `"Unsupported type for YAML conversion"` |
| Schema validation failure | `"<jsonschema error message>"` |
| Edit failure | `"YAML edit error: <detail>"` |
| Malformed path | `"YAML path error: <detail>"` |

### i18n Support

Error messages can be localized:

```python
pyrs_yaml.set_language("zh-CN")

try:
    pyrs_yaml.parse("invalid: [")
except pyrs_yaml.YamlParseError as e:
    print(e)
    # Chinese: "YAML 解析错误: 第 1 行, 第 14 列: ..."
```

### Best Practices

```python
import pyrs_yaml


def load_yaml(path):
    try:
        doc = pyrs_yaml.parse_file(path)
        return doc.to_dict()
    except pyrs_yaml.YamlParseError as e:
        print(f"Failed to parse {path}: {e}")
        return None
    except OSError as e:
        print(f"Failed to read {path}: {e}")
        return None
```
