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

from __future__ import annotations

import hashlib
from typing import Any, Callable, Dict, Literal, TypeVar, overload

from typing_extensions import TypeAlias, TypedDict, TypeGuard

from . import plugins as _plugins  # ruff: ignore[F401] — registers built-in plugins
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
    list_schemas,
    load_schema,
    negotiate_language,
    parse_stream,
    register_schema,
    remove_tag,
    remove_type,
    safe_dump,
    set_language,
    validate_against_registered_schema,
    validate_custom_types,
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
from .pyrs_yaml import (
    validate_against_schema as _validate_against_schema,
)

_F = TypeVar("_F", bound=Callable[..., Any])
"""Type variable for callable handler in register_tag/register_type."""


@overload
def register_tag(name: str, handler: _F, priority: int = 0) -> _F: ...


@overload
def register_tag(name: str, handler: None = None, priority: int = 0) -> Callable[[_F], _F]: ...


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

# ── Inline schema dict typed structure ──────────────────────────────────────

_SchemaRule: TypeAlias = Dict[str, str]
"""A single schema rule: keys 'pattern' (regex) and 'type' (null/bool/int/float/str)."""


class _SchemaDict(TypedDict, total=False):
    """Type for an inline schema dict passed to `YAML(schema=...)`."""

    extends: str
    """Base schema name ('core', 'json', 'failsafe', 'yaml1.1', or custom)."""
    rules: list[_SchemaRule]
    """List of pattern → type rules."""


# ── Schema helpers ──────────────────────────────────────────────────────────


def _is_schema_dict(val: object) -> TypeGuard[_SchemaDict]:
    return isinstance(val, dict)


def _schema_to_yaml(schema: _SchemaDict) -> str:
    """Convert an inline schema dict to a YAML schema string."""
    if not isinstance(schema, dict):
        raise TypeError(f"schema must be a dict, got {type(schema).__name__}")
    if "rules" not in schema or not schema["rules"]:
        raise ValueError("schema dict must contain a non-empty 'rules' list")
    rules = schema["rules"]
    if not isinstance(rules, list):
        raise ValueError("schema 'rules' must be a list")
    lines = [f"extends: {schema.get('extends', 'core')}"]
    lines.append("rules:")
    for rule in rules:
        if not isinstance(rule, dict) or "pattern" not in rule or "type" not in rule:
            raise ValueError(f"each rule must have 'pattern' and 'type' keys, got {rule}")
        pattern = rule["pattern"]
        typ = rule["type"]
        if "'" in pattern:
            lines.append(f'  - pattern: "{pattern}"')
        else:
            lines.append(f"  - pattern: '{pattern}'")
        if "'" in typ:
            lines.append(f'    type: "{typ}"')
        else:
            lines.append(f"    type: '{typ}'")
    return "\n".join(lines) + "\n"


def _coerce_schema(schema: str | _SchemaDict) -> str:
    """Return a schema name string, registering inline dict schemas.

    Accepts either a schema name (str) or an inline schema definition (dict).
    Inline dicts are serialized to YAML, registered under a deterministic name
    derived from their payload, and the name is returned.
    """
    if isinstance(schema, str):
        return schema
    if _is_schema_dict(schema):
        yaml_str = _schema_to_yaml(schema)
        digest = hashlib.sha256(yaml_str.encode()).hexdigest()[:16]
        name = f"_inline_{digest}"
        # Register under a deterministic name derived from the payload.
        # Re-registering the same content is a cheap no-op (overwrites with an
        # identical resolver). A genuine schema error propagates as
        # YamlParseError so callers see the malformed definition.
        register_schema(name, yaml_str)
        return name
    raise TypeError(f"schema must be str or dict, got {type(schema).__name__}")


def validate_against_schema(data: str, schema: str) -> None:
    """Validate a YAML document against a schema's `validate` rules.

    ``schema`` can be either a registered schema name or a schema definition
    YAML string. Raises ``YamlValidateError`` listing every structural failure.
    """
    if "\n" in schema or "{" in schema:
        return _validate_against_schema(data, schema)
    return validate_against_registered_schema(data, schema)


# Wrap YAML to accept inline dict schemas. PyO3 validates signature at the
# C level before calling __init__, so monkey-patching doesn't work for
# argument type coercion. Use delegation instead.


class _YAMLMetaclass(type):
    """Metaclass that delegates class-level attribute access to the Rust YAML class."""

    def __getattr__(cls, name: str) -> Any:
        return getattr(_YAML, name)


class YAML(metaclass=_YAMLMetaclass):
    """Configured parser instance; `schema` accepts a name or an inline dict."""

    _impl: _YAML

    def __init__(
        self,
        typ: Literal["rt", "safe", "full"] = "rt",
        schema: str | _SchemaDict = "core",
        max_depth: int = 1000,
        allow_duplicate_keys: bool = False,
    ) -> None:
        self._impl = _YAML(typ, _coerce_schema(schema), max_depth, allow_duplicate_keys)

    def __getattr__(self, name: str) -> Any:
        return getattr(self._impl, name)


# Module-level functions with schema support — wrap to accept dict
def safe_load(
    yaml: str,
    schema: str | _SchemaDict = "core",
    max_depth: int = 1000,
    allow_duplicate_keys: bool = False,
) -> dict[str, Any] | list[Any]:
    return _safe_load(yaml, _coerce_schema(schema), max_depth, allow_duplicate_keys)


def safe_loads(
    yaml: str,
    schema: str | _SchemaDict = "core",
    max_depth: int = 1000,
    allow_duplicate_keys: bool = False,
) -> list[dict[str, Any] | list[Any]]:
    return _safe_loads(yaml, _coerce_schema(schema), max_depth, allow_duplicate_keys)


def parse(
    yaml: str | bytes,
    resolve_merges: bool = True,
    schema: str | _SchemaDict = "core",
    max_depth: int = 1000,
    allow_duplicate_keys: bool = False,
) -> Any:
    return _parse(yaml, resolve_merges, _coerce_schema(schema), max_depth, allow_duplicate_keys)


def parse_file(
    path: str,
    schema: str | _SchemaDict = "core",
    max_depth: int = 1000,
    allow_duplicate_keys: bool = False,
) -> Any:
    return _parse_file(path, _coerce_schema(schema), max_depth, allow_duplicate_keys)


def parse_all_docs(
    yaml: str,
    resolve_merges: bool = True,
    schema: str | _SchemaDict = "core",
    max_depth: int = 1000,
    allow_duplicate_keys: bool = False,
) -> list[Any]:
    return _parse_all_docs(yaml, resolve_merges, _coerce_schema(schema), max_depth, allow_duplicate_keys)


def read_markdown(
    content: str,
    schema: str | _SchemaDict = "core",
    max_depth: int = 1000,
) -> tuple[dict[str, Any] | None, str]:
    return _read_markdown(content, _coerce_schema(schema), max_depth)


def read_markdown_str(
    content: str,
    schema: str | _SchemaDict = "core",
    max_depth: int = 1000,
) -> tuple[dict[str, Any] | None, str]:
    return _read_markdown_str(content, _coerce_schema(schema), max_depth)


# Monkey-patch YamlDocument with node() and find() methods
def _yaml_document_node(self: Any) -> Any:
    return Node(self)


def _yaml_document_find(self: Any, path: str) -> Any:
    return Node(self).find(path)


def _yaml_document_walk(self: Any) -> Any:
    for path in self._walk_paths():
        yield Node(self, path)


def _yaml_document_scalars(self: Any) -> Any:
    for path in self._scalar_paths():
        yield Node(self, path)


YamlDocument.node = _yaml_document_node  # ty: ignore[unresolved-attribute]
YamlDocument.find = _yaml_document_find  # ty: ignore[unresolved-attribute]
YamlDocument.walk = _yaml_document_walk  # ty: ignore[unresolved-attribute]
YamlDocument.scalars = _yaml_document_scalars  # ty: ignore[unresolved-attribute]


def _yaml_document_merged(self):
    return MergedView(self)


YamlDocument.merged = _yaml_document_merged  # ty: ignore[unresolved-attribute]


from . import editing as _editing  # ruff: ignore[E402]  (monkeypatches must run after Rust module loads)

YamlDocument.set = _editing._yaml_document_set  # ty: ignore[unresolved-attribute]
YamlDocument.insert = _editing._yaml_document_insert  # ty: ignore[unresolved-attribute]
YamlDocument.append = _editing._yaml_document_append  # ty: ignore[unresolved-attribute]
YamlDocument.delete = _editing._yaml_document_delete  # ty: ignore[unresolved-attribute]
YamlDocument.rename = _editing._yaml_document_rename  # ty: ignore[unresolved-attribute]
YamlDocument.sort_keys = _editing._yaml_document_sort_keys  # ty: ignore[unresolved-attribute]
YamlDocument.set_many = _editing._yaml_document_set_many  # ty: ignore[unresolved-attribute]

# Module-level aliases (PyYAML compatibility)
safe_dumps = safe_dump
safe_dumps_async = safe_dump_async

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
    "list_schemas",
    "load_schema",
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
    "safe_dumps",
    "safe_load",
    "safe_load_async",
    "safe_loads",
    "safe_loads_async",
    "set_language",
    "validate_against_schema",
    "validate_custom_types",
]

try:
    from importlib.metadata import version as _version
except ImportError:

    def _version(name: str) -> str:
        return "unknown"


def __getattr__(name: str) -> str:
    if name == "__version__":
        return _version("pyrs-yaml")
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
