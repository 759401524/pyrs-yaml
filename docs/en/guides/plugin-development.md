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

A plugin teaches pyrs-yaml to understand a custom YAML tag. There are two
interfaces:

- **`TagHandler`** (recommended, newer) — a single base class registered with
  `register_tag_handler()` that supports scalar conversion, container tags
  (mappings / sequences), scalar string transforms, and parse / serialize
  lifecycle hooks.
- **`CustomType`** (classic) — registers a scalar type with `register_type()`;
  still fully supported and documented below.

A minimal plugin defines a class, registers it, and (optionally) exposes an
entry point for automatic discovery.

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

### The Unified TagHandler API

`TagHandler` is a single base class that replaces both `register_tag` (scalar
string transform) and `register_type` (object conversion). A handler implements
only the methods it needs — you are not required to override all of them.

```python
import pyrs_yaml
from typing import Any


class TagHandler:
    python_type = None  # for isinstance-based serialization dispatch

    # Scalar: parse-time string transform (alternative to register_tag)
    def resolve(self, value: str, tag: str) -> str: ...

    # Scalar: bidirectional type conversion (alternative to CustomType)
    def from_yaml(self, value: Any, tag: str) -> Any: ...
    def to_yaml(self, obj: Any, tag: str) -> str: ...

    # Container: mapping / sequence tag conversion
    def from_yaml_mapping(self, mapping: dict, tag: str) -> Any: ...
    def from_yaml_sequence(self, sequence: list, tag: str) -> Any: ...
    def to_yaml_mapping(self, obj: Any, tag: str) -> dict: ...
    def to_yaml_sequence(self, obj: Any, tag: str) -> list: ...

    # Lifecycle hooks (run once per document)
    def on_parse_start(self, yaml: str, ctx) -> str: ...
    def on_parse_end(self, doc, ctx) -> None: ...
    def on_serialize_start(self, doc, ctx) -> None: ...
    def on_serialize_end(self, result: str, ctx) -> str: ...
```

#### Registering a TagHandler

`register_tag_handler()` accepts an instance, a class (instantiated
automatically), or a decorator:

```python title="hex_plugin.py"
import pyrs_yaml


class HexInt(pyrs_yaml.TagHandler):
    python_type = int

    def from_yaml(self, value, tag):
        return int(value, 16)

    def to_yaml(self, obj, tag):
        return hex(obj)


pyrs_yaml.register_tag_handler("!hex", HexInt())
```

#### Container tags

A `TagHandler` can convert an entire mapping or sequence node, not just scalars.
Override `from_yaml_mapping` / `from_yaml_sequence` (load path) and
`to_yaml_mapping` / `to_yaml_sequence` (save path):

```python title="omap_plugin.py"
import pyrs_yaml


class OrderedMap(pyrs_yaml.TagHandler):
    python_type = dict

    def from_yaml_mapping(self, mapping, tag):
        return list(mapping.items())  # preserve order as pairs

    def to_yaml_mapping(self, obj, tag):
        return dict(obj)


pyrs_yaml.register_tag_handler("!omap", OrderedMap())
```

!!! note "Container tags and `safe_dump`"
    Container handlers (`from_yaml_mapping` / `from_yaml_sequence` /
    `to_yaml_mapping` / `to_yaml_sequence`) only run in the AST round-trip path
    (`parse` → `to_yaml_with_options` / `serialize`). The `safe_dump` /
    `from_dict` / `dump_file` fast path has no AST context, so it always emits
    plain containers.

#### Lifecycle hooks

`on_parse_start` / `on_parse_end` run around parsing; `on_serialize_start` /
`on_serialize_end` run around `serialize()` (the module-level wrapper around
`to_yaml_with_options`). Hooks are best-effort: an exception in one handler is
suppressed so other handlers still run. `ParseContext` and `SerializeContext`
expose `schema`, `max_depth`, `indent`, and `sort_keys` for introspection.

#### Introspection

```python
pyrs_yaml.list_tag_handlers()  # -> list[(tag, priority)]
pyrs_yaml.get_tag_handler("!hex")  # -> TagHandler | None
pyrs_yaml.remove_tag_handler("!hex")  # -> None
```

### See Also

- [Community Plugins](community-plugins.md) — Built-in types you can extend
- [Custom Schemas](custom-schema.md) — Define type resolution rules
- [Tag Registry API](../api/reference.md#tag-registry) — `register_tag()` and related functions
