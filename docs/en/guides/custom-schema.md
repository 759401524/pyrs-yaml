---
title: Custom Schemas
description: Define and use custom YAML schemas for type resolution control.
tags:
  - docs
status: new
---

## Custom Schemas

By default, pyrs-yaml uses the YAML 1.2 Core schema for implicit type
resolution. With the YAML Schema Language, you can define custom schemas
that control how plain scalars are resolved to Python types.

### Why Custom Schemas?

The Core schema resolves `0xFF` to `int(255)`, `2026-08-11` to `int(2026)`
and `hello` to `"hello"`. Sometimes you want different behavior:

- Keep dates as strings (`"2026-08-11"` instead of `2026`)
- Parse hex/binary literals as integers
- Add YAML 1.1-style boolean lexemes (`yes`/`no`)
- Use a JSON-only subset (no `inf`, `nan`, `0x`)

### Schema Definition Format

A schema is defined as a YAML file with a `rules` list. Each rule has a
`pattern` (regex) and a `type` (one of `null`, `bool`, `int`, `float`, `str`).

```yaml title="hex_schema.yaml"
# hex_schema.yaml
name: hex
extends: core
rules:
  - pattern: ^0x[0-9a-fA-F]+$
    type: int
  - pattern: ^0b[01]+$
    type: int
```

**`extends`** — Optional base schema. Rules are checked first; if none
match, the `extends` schema handles resolution. Default: `core`.

**`rules`** — Ordered list. The first matching pattern determines the type.
Supported types and their values:

| `type` | Python result | Example |
|--------|--------------|---------|
| :material-null: `null` | `None` | `~` |
| :material-toggle-switch: `bool` | `True` / `False` | `true`, `yes`, `on` |
| :material-numeric: `int` | `int` | `42`, `0xFF`, `0o77`, `0b1010` |
| :material-decimal: `float` | `float` | `3.14`, `1e10` |
| :material-format-text: `str` | `str` | `2026-08-11` |

### Registering and Using a Schema

```python title="Register and use a schema"
import pyrs_yaml

# Register from a YAML string
pyrs_yaml.register_schema("hex", """
name: hex
extends: core
rules:
  - pattern: ^0x[0-9a-fA-F]+$
    type: int
""")

# Use with YAML instance
y = pyrs_yaml.YAML(schema="hex")
doc = y.parse("addr: 0xFF")
assert doc.get("addr") == 255

# Use with module-level functions
d = pyrs_yaml.safe_load("addr: 0x1F", schema="hex")
assert d["addr"] == 31
```

#### Loading a schema from a file

`load_schema()` reads a schema definition from a file path and registers it:

```python title="Load schema from file"
# hex.yaml contains the schema YAML shown above
pyrs_yaml.load_schema("hex", "path/to/hex.yaml")
```

#### Listing registered schemas

`list_schemas()` returns all registered schema names (built-in + custom):

```python title="List registered schemas"
print(pyrs_yaml.list_schemas())
# ['failsafe', 'json', 'core', 'yaml1.1', 'hex', ...]
```

#### Structural validation

A `validate` section in a schema definition adds structural checks on top of
scalar type resolution. Use it with `validate_against_schema()` to check a
document before using it:

```yaml title="Schema with validation"
name: app
extends: core
validate:
  - path: $.port
    type: int
    required: true
  - path: $.tags[*]
    type: str
  - path: $.numbers
    sequence_of: int
  - path: $.config
    mapping_of: str
```

```python
import pyrs_yaml

schema = """\
name: app
extends: core
validate:
  - path: $.port
    type: int
    required: true
"""

pyrs_yaml.validate_against_schema("port: 80\n", schema)          # OK
# Raises YamlValidateError listing every failure:
pyrs_yaml.validate_against_schema("port: abc\n", schema)
```

- `path` — JSONPath-like location (`$.key`, `$.a.b`, `$.tags[*]`); omit for all scalars
- `type` — the scalar must resolve to this YAML type (`null`/`bool`/`int`/`float`/`str`)
- `sequence_of` / `mapping_of` — every element / value must be of the given type
- `required` — the path must be present and non-null (combines with `type`)

### Inline Dict Schema

Instead of registering separately, pass a dict directly:

```python title="Inline dict schema"
d = pyrs_yaml.safe_load(
    "addr: 0xFF",
    schema={
        "extends": "core",
        "rules": [{"pattern": "^0x[0-9a-fA-F]+$", "type": "int"}],
    },
)
assert d["addr"] == 255
```

### Common Patterns

=== "Keep dates as strings"

    ```python
    schema = {
        "extends": "core",
        "rules": [{"pattern": "^\\d{4}-\\d{2}-\\d{2}$", "type": "str"}],
    }
    ```

=== "Add YAML 1.1 booleans"

    ```python
    schema = {
        "extends": "core",
        "rules": [{"pattern": "^(yes|no|Yes|No|YES|NO)$", "type": "bool"}],
    }
    ```

=== "Strict JSON mode"

    ```python
    schema = {
        "extends": "failsafe",
        "rules": [
            {"pattern": "^null$|^~$", "type": "null"},
            {"pattern": "^(true|false)$", "type": "bool"},
            {"pattern": "^-?\\d+$", "type": "int"},
            {"pattern": "^-?\\d+\\.\\d+$", "type": "float"},
        ],
    }
    ```

### Performance

Custom schemas use a regex-based rule engine. For each scalar, rules are
checked in order until a match is found. For best performance:

- Keep rule count under 20
- Put the most common patterns first
- Use `extends: core` to avoid re-implementing the full Core resolution

The built-in Core schema is unaffected — it still uses the zero-cost
`match` dispatch and is not impacted by custom schema registration.
