---
title: Plugin Development
description: Create third-party plugins for pyrs-yaml using the Community Plugins API.
tags:
  - docs
status: new
---

## Plugin Development

This guide explains how to create a third-party plugin for pyrs-yaml using
the Community Plugins API.

### Anatomy of a Plugin

A plugin is a Python module that defines a `CustomType` subclass, registers
it, and (optionally) exposes an entry point for automatic discovery.

**Minimal plugin:**

```python title="my_timestamp_plugin.py"
# my_timestamp_plugin.py
import pyrs_yaml
from datetime import datetime


class MyTimestampType(pyrs_yaml.CustomType):
    python_type = datetime

    def from_yaml(self, value: str):
        return datetime.fromisoformat(value)

    def to_yaml(self, obj) -> str:
        return obj.isoformat()


def register():
    pyrs_yaml.register_type("!mytimestamp", MyTimestampType())
```

### Automatic Discovery via Entry Points

To make your plugin auto-discoverable, add an entry point in your
`pyproject.toml`:

```toml title="pyproject.toml"
[project.entry-points."pyrs_yaml.plugins"]
mytimestamp = "my_timestamp_plugin:register"
```

When pyrs-yaml is imported, it scans `pyrs_yaml.plugins` entry points and
calls each registered callable. The callable receives no arguments and
should call `register_type()` to register custom types.

**The name's spelling is part of the contract.** `register_type("!x", ...)` answers the local tag
`!x`, and `register_type("!!x", ...)` answers YAML's standard tag - where both spellings of that
tag reach it, because `!!x` and the verbatim `!<tag:yaml.org,2002:x>` are one tag written two ways
(YAML 1.2 §6.8). A URI outside `tag:yaml.org,2002:` is not YAML's to name, so a scalar carrying it
keeps the reading an untagged scalar has.

### Creating a CustomType

The `CustomType` base class provides four methods:

| Method | Purpose | Default |
|--------|---------|---------|
| :material-function: `can_parse(node)` | Gate: should this type handle the node? | `True` (tag match sufficient) |
| :material-swap-horizontal: `from_yaml(value)` | Convert YAML string → Python object | Returns `value` unchanged |
| :material-swap-horizontal: `to_yaml(obj)` | Convert Python object → YAML string | Returns `str(obj)` |
| :material-check-decagram: `validate(obj)` | Validate the Python object | Returns `True` |

**`python_type`** — Optional attribute. When set, the serializer uses
`isinstance(obj, python_type)` to detect objects that should be serialized
with this type's `to_yaml()` method.

**The handler is asked before the object is read as a number.** A class that answers Python's number
protocol (`__index__`, `__float__`) still reaches its own `to_yaml()`, as a value, as a key and as a
sequence item. One family is exempt, and the exemption is the same on both writers: an `int` or `int`
subclass — and a `bool`, which is one — keeps the integer spelling a reader resolves. So a money type
carrying `__index__` reaches the handler, while `42` and `True` never do.

**What the handler owns is a scalar.** The registry is asked before either writer reaches its own
spelling rules, so a value only `to_yaml()` can spell keeps the line opened for it — under a key,
under a dash, and inside a compact sequence item — and an object a handler claims reaches the
handler before the array route asks it for a buffer. Two families keep their own spelling under the
same rule, because they already have one a reader resolves: a `str`, or a subclass of it, is written
as a string, so a handler registered for `str` shapes the reading side and does not rewrite text on
the way out; and a `dict` or a `list` is written as the container the caller's data says it is.

### Example: UUID Plugin

```python title="uuid_plugin.py"
import uuid
import pyrs_yaml


class UUIDType(pyrs_yaml.CustomType):
    python_type = uuid.UUID

    def from_yaml(self, value: str):
        return uuid.UUID(value)

    def to_yaml(self, obj) -> str:
        return str(obj)


def register():
    pyrs_yaml.register_type("!uuid", UUIDType())
```

### Example: `can_parse` Gate

```python title="Conditional parse gate"
class ConditionalType(pyrs_yaml.CustomType):
    python_type = str

    def can_parse(self, value: str) -> bool:
        # Only handle values starting with a prefix
        return value.startswith("myprefix_")

    def from_yaml(self, value: str):
        return value[len("myprefix_") :]

    def to_yaml(self, obj) -> str:
        return obj
```

### Validation

```python title="Validated int type"
class PositiveIntType(pyrs_yaml.CustomType):
    python_type = int

    def from_yaml(self, value: str):
        n = int(value)
        if n < 0:
            raise ValueError("must be positive")
        return n

    def to_yaml(self, obj) -> str:
        return str(obj)

    def validate(self, obj) -> bool:
        return isinstance(obj, int) and obj > 0
```

### Testing Your Plugin

```python title="test_my_plugin.py"
def test_my_plugin():
    # Clear any built-in types that might conflict
    pyrs_yaml.clear_type_handlers()

    import my_timestamp_plugin

    my_timestamp_plugin.register()

    doc = pyrs_yaml.parse("when: !mytimestamp 2026-08-11T10:30:00")
    from datetime import datetime

    assert isinstance(doc.get("when"), datetime)
```

### Publishing

1. :material-package-variant-closed: Package your plugin as a Python package (wheel)
2. :material-file-code: Include the `pyproject.toml` entry point
3. :material-rocket-launch: Publish to PyPI
4. :material-magnify: Users install it and it's auto-discovered

### API Reference

| Function | Description |
|----------|-------------|
| :material-code-braces: `register_type(name, handler)` | Register a `CustomType` instance |
| :material-close: `clear_type_handlers()` | Remove all registered types |
| :material-close: `remove_type(name)` | Remove a specific type |
| :material-check-decagram: `validate_custom_types(obj)` | Validate an object against all registered types |

---

### See Also

- [Community Plugins](community-plugins.md) — Built-in types you can extend
- [Custom Schemas](custom-schema.md) — Define type resolution rules
- [Tag Registry API](../api/reference.md#tag-registry) — `register_tag()` and related functions
