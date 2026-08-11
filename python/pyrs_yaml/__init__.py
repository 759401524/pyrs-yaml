"""
pyrs-yaml: High-performance Python YAML library with perfect round-trip support.

This module provides a Rust-backed YAML parser and serializer that preserves
comments, anchors, tags, and formatting during round-trip conversions.

Example:
    >>> import pyrs_yaml
    >>> doc = pyrs_yaml.parse("key: value")
    >>> print(doc.to_yaml())
    key: value
    >>> print(doc.get("key"))
    value
"""

import contextlib
import hashlib

from . import plugins as _plugins  # noqa: F401 — registers built-in plugins
from ._type_registry import CustomType, register_type
from .async_dump import (
    safe_dump_async,
    safe_load_async,
    safe_loads_async,
)
from .compliance import compliance_report
from .merged_view import MergedView
from .node import Node, YamlDocumentError
from .pydantic import dump_pydantic, parse_as
from .pyrs_yaml import (
    YAML as _YAML,
)
from .pyrs_yaml import (
    StreamIterator,
    YamlDocument,
    YamlDuplicateKeyError,
    YamlEditError,
    YamlMaxDepthError,
    YamlParseError,
    YamlPathError,
    YamlSerializeError,
    YamlTagError,
    YamlTagSkip,
    YamlTypeError,
    YamlValidateError,
    clear_tag_handlers,
    clear_type_handlers,
    detect_language,
    dump_file,
    from_dict,
    from_json,
    get_language,
    list_languages,
    negotiate_language,
    parse_stream,
    register_schema,
    remove_tag,
    remove_type,
    safe_dump,
    set_language,
)
from .pyrs_yaml import (
    parse as _parse,
)
from .pyrs_yaml import (
    parse_all_docs as _parse_all_docs,
)
from .pyrs_yaml import (
    parse_file as _parse_file,
)
from .pyrs_yaml import (
    read_markdown as _read_markdown,
)
from .pyrs_yaml import (
    read_markdown_str as _read_markdown_str,
)
from .pyrs_yaml import (
    register_tag as _rust_register_tag,
)
from .pyrs_yaml import (
    safe_load as _safe_load,
)
from .pyrs_yaml import (
    safe_loads as _safe_loads,
)


def register_tag(name, handler=None, priority=0):
    """Register a tag handler.

    Supports both decorator and imperative forms:
        @register_tag("!custom")
        def handler(node):
            ...

        @register_tag("!custom", priority=1)
        def handler(node):
            ...

        register_tag("!custom", handler_fn)
        register_tag("!custom", handler_fn, priority=1)
    """
    if handler is not None:
        _rust_register_tag(name, handler, priority)
        return handler

    # Decorator form
    def decorator(fn):
        _rust_register_tag(name, fn, priority)
        return fn

    return decorator


# CustomType / register_type imported from ._type_registry


def _schema_to_yaml(schema):
    """Convert an inline schema dict to a YAML schema string."""
    if not isinstance(schema, dict):
        raise TypeError(f"schema must be a dict, got {type(schema).__name__}")
    if "rules" not in schema or not schema["rules"]:
        raise ValueError("schema dict must contain a non-empty 'rules' list")
    rules = schema["rules"]
    if not isinstance(rules, list):
        raise ValueError("schema 'rules' must be a list")
    digest = hashlib.sha256(str(schema).encode()).hexdigest()[:16]
    lines = [f"name: _inline_{digest}"]
    lines.append(f"extends: {schema.get('extends', 'core')}")
    lines.append("rules:")
    for rule in rules:
        if not isinstance(rule, dict) or "pattern" not in rule or "type" not in rule:
            raise ValueError(f"each rule must have 'pattern' and 'type' keys, got {rule}")
        lines.append(f"  - pattern: {rule['pattern']}")
        lines.append(f"    type: {rule['type']}")
    return "\n".join(lines) + "\n"


def _coerce_schema(schema):
    """Return a schema name string, registering inline dict schemas.

    Accepts either a schema name (str) or an inline schema definition (dict).
    Inline dicts are serialized to YAML, registered under a deterministic name
    derived from their JSON payload, and the name is returned.
    """
    if isinstance(schema, str):
        return schema
    if isinstance(schema, dict):
        digest = hashlib.sha256(_schema_to_yaml(schema).encode()).hexdigest()[:16]
        name = f"_inline_{digest}"
        with contextlib.suppress(Exception):
            # Already registered under this name (same content) — ignore.
            register_schema(name, _schema_to_yaml(schema))
        return name
    raise TypeError(f"schema must be str or dict, got {type(schema).__name__}")


# Wrap YAML to accept inline dict schemas. PyO3 validates signature at the
# C level before calling __init__, so monkey-patching doesn't work for
# argument type coercion. Use delegation instead.


class _YAMLMetaclass(type):
    """Metaclass that delegates class-level attribute access to the Rust YAML class."""

    def __getattr__(cls, name):
        return getattr(_YAML, name)


class YAML(metaclass=_YAMLMetaclass):
    """Configured parser instance; `schema` accepts a name or an inline dict."""

    def __init__(self, typ="rt", schema="core", max_depth=1000, allow_duplicate_keys=False):
        self._impl = _YAML(typ, _coerce_schema(schema), max_depth, allow_duplicate_keys)

    def __getattr__(self, name):
        return getattr(self._impl, name)


# Module-level functions with schema support — wrap to accept dict
def safe_load(yaml, schema="core", max_depth=1000, allow_duplicate_keys=False):
    return _safe_load(yaml, _coerce_schema(schema), max_depth, allow_duplicate_keys)


def safe_loads(yaml, schema="core", max_depth=1000, allow_duplicate_keys=False):
    return _safe_loads(yaml, _coerce_schema(schema), max_depth, allow_duplicate_keys)


def parse(yaml, resolve_merges=True, schema="core", max_depth=1000, allow_duplicate_keys=False):
    return _parse(yaml, resolve_merges, _coerce_schema(schema), max_depth, allow_duplicate_keys)


def parse_file(path, schema="core", max_depth=1000, allow_duplicate_keys=False):
    return _parse_file(path, _coerce_schema(schema), max_depth, allow_duplicate_keys)


def parse_all_docs(yaml, resolve_merges=True, schema="core", max_depth=1000, allow_duplicate_keys=False):
    return _parse_all_docs(yaml, resolve_merges, _coerce_schema(schema), max_depth, allow_duplicate_keys)


def read_markdown(content, schema="core", max_depth=1000):
    return _read_markdown(content, _coerce_schema(schema), max_depth)


def read_markdown_str(content, schema="core", max_depth=1000):
    return _read_markdown_str(content, _coerce_schema(schema), max_depth)


# Monkey-patch YamlDocument with node() and find() methods
def _yaml_document_node(self):
    from .node import Node

    return Node(self)


def _yaml_document_find(self, path):
    from .node import Node

    return Node(self).find(path)


def _yaml_document_walk(self):
    from .node import Node

    for path in self._walk_paths():
        yield Node(self, path)


def _yaml_document_scalars(self):
    from .node import Node

    for path in self._scalar_paths():
        yield Node(self, path)


YamlDocument.node = _yaml_document_node
YamlDocument.find = _yaml_document_find
YamlDocument.walk = _yaml_document_walk
YamlDocument.scalars = _yaml_document_scalars


def _yaml_document_merged(self):
    from .merged_view import MergedView

    return MergedView(self)


YamlDocument.merged = _yaml_document_merged


from . import editing as _editing  # noqa: E402  (monkeypatches must run after Rust module loads)

YamlDocument.set = _editing._yaml_document_set
YamlDocument.insert = _editing._yaml_document_insert
YamlDocument.append = _editing._yaml_document_append
YamlDocument.delete = _editing._yaml_document_delete
YamlDocument.rename = _editing._yaml_document_rename

__all__ = [
    "YAML",
    "CustomType",
    "MergedView",
    "Node",
    "StreamIterator",
    "YamlDocument",
    "YamlDocumentError",
    "YamlDuplicateKeyError",
    "YamlEditError",
    "YamlMaxDepthError",
    "YamlParseError",
    "YamlPathError",
    "YamlSerializeError",
    "YamlTagError",
    "YamlTagSkip",
    "YamlTypeError",
    "YamlValidateError",
    "clear_tag_handlers",
    "clear_type_handlers",
    "compliance_report",
    "detect_language",
    "dump_file",
    "dump_pydantic",
    "from_dict",
    "from_json",
    "get_language",
    "list_languages",
    "negotiate_language",
    "parse",
    "parse_all_docs",
    "parse_as",
    "parse_file",
    "parse_stream",
    "read_markdown",
    "read_markdown_str",
    "register_schema",
    "register_tag",
    "register_type",
    "remove_tag",
    "remove_type",
    "safe_dump",
    "safe_dump_async",
    "safe_load",
    "safe_load_async",
    "safe_loads",
    "safe_loads_async",
    "set_language",
]

try:
    from importlib.metadata import version as _version
except Exception:

    def _version(name: str) -> str:
        return "unknown"


def __getattr__(name: str) -> str:
    if name == "__version__":
        return _version("pyrs-yaml")
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
