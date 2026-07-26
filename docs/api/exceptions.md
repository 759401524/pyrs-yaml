# Exceptions

pyyaml-rs defines three custom exception classes for error handling.

## YamlParseError

Raised when YAML parsing fails.

```python
class YamlParseError(ValueError):
    """YAML parsing error (inherits from ValueError)."""
```

**Inherits from:** `ValueError`

**Example:**

```python
try:
    doc = pyyaml_rs.parse("invalid: yaml: [")
except pyyaml_rs.YamlParseError as e:
    print(f"Parse error: {e}")
    # Output: "YAML parse error: line 1, column 14: unexpected token..."
```

**Can be caught as:**

```python
except ValueError as e:  # Also works
```

## YamlSerializeError

Raised when YAML serialization fails.

```python
class YamlSerializeError(ValueError):
    """YAML serialization error (inherits from ValueError)."""
```

**Inherits from:** `ValueError`

**Example:**

```python
try:
    yaml_str = pyyaml_rs.safe_dump(some_unsupported_type)
except pyyaml_rs.YamlSerializeError as e:
    print(f"Serialize error: {e}")
```

## YamlTypeError

Raised when a type conversion error occurs.

```python
class YamlTypeError(TypeError):
    """YAML type conversion error (inherits from TypeError)."""
```

**Inherits from:** `TypeError`

**Example:**

```python
try:
    pyyaml_rs.parse(123)  # Expected str or bytes
except pyyaml_rs.YamlTypeError as e:
    print(f"Type error: {e}")
```

## Error Message Format

All error messages include contextual information:

| Error | Format |
|-------|--------|
| Parse error | `"YAML parse error: line N, column M: <message>"` |
| File not found | `"File read error: <path> — <OS error>"` |
| Invalid UTF-8 | `"Invalid UTF-8: <detail>"` |
| Key not found | `"Key not found: <key>"` |
| Index out of range | `"Index out of range: <index> (len: <len>)"` |
| Unsupported type | `"Unsupported type for YAML conversion"` |

## i18n Support

Error messages can be localized:

```python
pyyaml_rs.set_language("zh-CN")

try:
    pyyaml_rs.parse("invalid: [")
except pyyaml_rs.YamlParseError as e:
    print(e)
    # Chinese: "YAML 解析错误: 第 1 行, 第 14 列: ..."
```

## Best Practices

```python
import pyyaml_rs

def load_yaml(path):
    try:
        doc = pyyaml_rs.parse_file(path)
        return doc.to_dict()
    except pyyaml_rs.YamlParseError as e:
        print(f"Failed to parse {path}: {e}")
        return None
    except OSError as e:
        print(f"Failed to read {path}: {e}")
        return None
```
