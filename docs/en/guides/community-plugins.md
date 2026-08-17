---
title: Community Plugins
description: Extend pyrs-yaml with custom node types using the Community Plugins API.
tags:
  - docs
status: new
---

## Community Plugins

The Community Plugins API lets you define custom YAML node types that
integrate with pyrs-yaml's serialization and deserialization. A custom type
can convert between YAML tagged scalars and arbitrary Python objects.

### Built-in Plugins

pyrs-yaml ships with built-in plugins registered at import time:

| Tag | Python type | Description |
|-----|-------------|-------------|
| `!timestamp` | `datetime` | ISO 8601 datetime round-trip |
| `!date` | `datetime.date` | ISO 8601 date (no time) |
| `!time` | `datetime.time` | ISO 8601 time (no date) |
| `!uuid` | `uuid.UUID` | UUID string ↔ object |
| `!decimal` | `decimal.Decimal` | Arbitrary-precision decimal |
| `!binary` | `bytes` | Base64-encoded binary data |
| `!regex` | `re.Pattern` | Compiled regex pattern |
| `!set` | `str` | YAML set (unkeyed mapping) — experimental, no round-trip serialization |

### Creating a Custom Type

Subclass `CustomType` and implement `from_yaml()` and `to_yaml()`:

```python
import pyrs_yaml
from datetime import datetime


class TimestampType(pyrs_yaml.CustomType):
    python_type = datetime

    def from_yaml(self, value: str):
        return datetime.fromisoformat(value)

    def to_yaml(self, obj) -> str:
        return obj.isoformat()
```

**`python_type`** — Optional attribute used during serialization. When
`safe_dump()` encounters a Python object, it checks registered types via
`isinstance(obj, type.python_type)` and calls `to_yaml()` if matched.

### Registering

**Imperative form:**

```python
pyrs_yaml.register_type("!timestamp", TimestampType())
```

**Decorator form:**

```python
@pyrs_yaml.register_type("!timestamp")
class TimestampType(pyrs_yaml.CustomType):
    ...
```

### Usage

**Loading a tagged scalar:**

```python
doc = pyrs_yaml.parse("when: !timestamp 2026-08-11T10:30:00")
val = doc.get("when")
assert isinstance(val, datetime)  # val is a datetime object
```

The tag `!timestamp` triggers `from_yaml()` which returns a `datetime`.

**Dumping a Python object:**

```python
data = {"ts": datetime(2026, 8, 11, 10, 30)}
out = pyrs_yaml.safe_dump(data)
# out contains: ts: !timestamp 2026-08-11T10:30:00
```

`safe_dump()` checks each value against registered types. When a `datetime`
object is found, `TimestampType.to_yaml()` is called and the output includes
the `!timestamp` tag.

### API Reference

| Method | Description |
|--------|-------------|
| `can_parse(value)` | Whether this type handles a given scalar value (string) |
| `from_yaml(value)` | Convert YAML string → Python object |
| `to_yaml(obj)` | Convert Python object → YAML string |
| `validate(obj)` | Validate a Python object (returns `bool`) |

### Third-Party Plugins

Third-party packages can register custom types by calling `register_type()`
at import time. Automatic discovery via `importlib.metadata.entry_points`
(group `pyrs_yaml.plugins`) is supported. Plugins are discovered at module
import time; errors are logged and do not block startup.

### Example: UUID Type

```python
import uuid
import pyrs_yaml


class UUIDType(pyrs_yaml.CustomType):
    python_type = uuid.UUID

    def from_yaml(self, value):
        return uuid.UUID(value)

    def to_yaml(self, obj):
        return str(obj)


pyrs_yaml.register_type("!uuid", UUIDType())

doc = pyrs_yaml.parse("id: !uuid 550e8400-e29b-41d4-a716-446655440000")
assert isinstance(doc.get("id"), uuid.UUID)
```
