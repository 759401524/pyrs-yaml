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

# ── Unified TagHandler API ───────────────────────────────────────────────────
from ._tag_handler import (
    ParseContext,
    PluginInfo,
    SerializeContext,
    TagHandler,
    call_parse_end,
    call_parse_start,
    call_serialize_end,
    call_serialize_start,
    clear_stored_handlers,
    get_stored_handler,
    remove_stored_handler,
)
from ._type_registry import _register_via_core
from .async_dump import (
    safe_dump_async,
    safe_load_async,
    safe_loads_async,
)
from .compliance import compliance_report
from .merged_view import MergedView
from .node import Node, YamlDocumentError
from .plugins import discover_plugins, get_discovery_errors, get_plugin_info, list_plugin_info
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
    YamlStream,
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
    get_plugin,
    list_languages,
    list_plugins,
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
    safe_load as _safe_load,
)
from .pyrs_yaml import (
    safe_loads as _safe_loads,
)
from .pyrs_yaml import (
    validate_against_schema as _validate_against_schema,
)

_THandler = TypeVar("_THandler", bound=TagHandler)


@overload
def register_tag_handler(name: str, handler: _THandler, priority: int = 0) -> _THandler: ...


@overload
def register_tag_handler(name: str, handler: None = None, priority: int = 0) -> Callable[[_THandler], _THandler]: ...


def register_tag_handler(name, handler=None, priority=0):
    """Register a unified TagHandler for a YAML tag.

    Supports both imperative and decorator forms::

        register_tag_handler("!timestamp", TimestampHandler())

        @register_tag_handler("!custom", priority=1)
        class CustomHandler(TagHandler):
            def from_yaml(self, value):
                return custom_parse(value)

    The handler can implement any combination of:
    - ``resolve()`` — parse-time scalar string transformation
    - ``from_yaml()`` / ``to_yaml()`` — type conversion
    - ``from_yaml_mapping()`` / ``from_yaml_sequence()`` — container conversion
    - Lifecycle hooks (``on_parse_start``, ``on_parse_end``, etc.)
    """
    from ._tag_handler import TagHandler as _TH  # noqa: N814

    def _register(name: str, instance: _TH, priority: int) -> None:
        # Delegate to the shared low-level backend (Rust tag + type registries
        # plus the Python handler registry). This avoids double registration.
        _register_via_core(name, instance, priority)

    if handler is not None:
        if not isinstance(handler, _TH):
            raise TypeError(f"handler must be a TagHandler instance, got {type(handler).__name__}")
        _register(name, handler, priority)
        return handler

    # Decorator form
    def decorator(cls_or_instance):
        instance = cls_or_instance() if isinstance(cls_or_instance, type) else cls_or_instance
        if not isinstance(instance, _TH):
            raise TypeError(
                f"decorated object must be a TagHandler subclass or instance, got {type(instance).__name__}"
            )
        _register(name, instance, priority)
        return cls_or_instance

    return decorator


def list_tag_handlers() -> list[tuple[str, int]]:
    """List all registered tag handlers.

    Returns a list of ``(tag_name, priority)`` tuples.
    """
    from .pyrs_yaml import tag_registry_list_tags as _list_tags

    tags = _list_tags()
    result = []
    for tag in tags:
        from .pyrs_yaml import tag_registry_get_handler_info as _get_info

        handlers = _get_info(tag)
        if handlers:
            for priority, _handler in handlers:
                result.append((tag, priority))
        else:
            result.append((tag, 0))
    return result


def get_tag_handler(name: str) -> TagHandler | None:
    """Get the highest-priority handler for a tag.

    Returns the TagHandler instance, or ``None`` if no handler is registered.
    """
    return get_stored_handler(name)


def remove_tag_handler(name: str) -> None:
    """Remove all handlers for a tag."""
    remove_tag(name)
    remove_type(name)
    remove_stored_handler(name)


# Wrap clear_tag_handlers to also clear the Python-level registry.
# clear_tag_handlers is imported from Rust at line 51.
_rust_clear_tag_handlers = clear_tag_handlers  # type: ignore[assignment]


def clear_tag_handlers() -> None:
    """Remove all tag handlers from both Rust and Python registries."""
    _rust_clear_tag_handlers()
    clear_stored_handlers()


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
    schema_str = _coerce_schema(schema)
    ctx = ParseContext(schema=schema_str, max_depth=max_depth, resolve_merges=resolve_merges)
    if isinstance(yaml, str):
        yaml = call_parse_start(yaml, ctx)
    doc = _parse(yaml, resolve_merges, schema_str, max_depth, allow_duplicate_keys)
    call_parse_end(doc, ctx)
    return doc


def parse_file(
    path: str,
    schema: str | _SchemaDict = "core",
    max_depth: int = 1000,
    allow_duplicate_keys: bool = False,
) -> Any:
    schema_str = _coerce_schema(schema)
    ctx = ParseContext(schema=schema_str, max_depth=max_depth, source=path)
    doc = _parse_file(path, schema_str, max_depth, allow_duplicate_keys)
    call_parse_end(doc, ctx)
    return doc


def parse_all_docs(
    yaml: str,
    resolve_merges: bool = True,
    schema: str | _SchemaDict = "core",
    max_depth: int = 1000,
    allow_duplicate_keys: bool = False,
) -> list[Any]:
    schema_str = _coerce_schema(schema)
    ctx = ParseContext(schema=schema_str, max_depth=max_depth, resolve_merges=resolve_merges)
    yaml_str = call_parse_start(yaml, ctx)
    docs = _parse_all_docs(yaml_str, resolve_merges, schema_str, max_depth, allow_duplicate_keys)
    for doc in docs:
        call_parse_end(doc, ctx)
    return docs


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


def serialize(
    document: YamlDocument,
    indent: int = 2,
    sort_keys: bool = False,
    explicit_start: bool = False,
    explicit_end: bool = False,
    max_depth: int = 1000,
    width: int = 80,
) -> str:
    """Serialize a YamlDocument to YAML string with lifecycle hooks.

    Calls ``on_serialize_start`` before and ``on_serialize_end`` after
    the underlying ``to_yaml_with_options``.
    """
    ctx = SerializeContext(indent=indent, sort_keys=sort_keys)
    call_serialize_start(document, ctx)
    result = document.to_yaml_with_options(
        indent_size=indent,
        sort_keys=sort_keys,
        explicit_start=explicit_start,
        explicit_end=explicit_end,
        max_depth=max_depth,
        width=width,
    )
    return call_serialize_end(result, ctx)


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
    "MergedView",
    "Node",
    "ParseContext",
    "PluginInfo",
    "PyrsYamlConfigSettingsSource",
    "SerializeContext",
    "StreamIterator",
    "TagHandler",
    "YamlDocument",
    "YamlDocumentError",
    "YamlDuplicateKeyError",
    "YamlEditError",
    "YamlMaxDepthError",
    "YamlParseError",
    "YamlPathError",
    "YamlSerializeError",
    "YamlStream",
    "YamlTagError",
    "YamlTagSkip",
    "YamlTypeError",
    "YamlValidateError",
    "clear_tag_handlers",
    "clear_type_handlers",
    "compliance_report",
    "detect_language",
    "discover_plugins",
    "dump_file",
    "dump_pydantic",
    "from_dict",
    "from_json",
    "get_discovery_errors",
    "get_language",
    "get_plugin",
    "get_plugin_info",
    "get_tag_handler",
    "list_languages",
    "list_plugin_info",
    "list_plugins",
    "list_schemas",
    "list_tag_handlers",
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
    "register_tag_handler",
    "remove_tag",
    "remove_tag_handler",
    "remove_type",
    "safe_dump",
    "safe_dump_async",
    "safe_dumps",
    "safe_load",
    "safe_load_async",
    "safe_loads",
    "safe_loads_async",
    "serialize",
    "set_language",
    "validate_against_schema",
    "validate_custom_types",
]

try:
    from importlib.metadata import version as _version
except ImportError:

    def _version(name: str) -> str:
        return "unknown"


def __getattr__(name: str) -> Any:
    if name == "__version__":
        return _version("pyrs-yaml")
    if name in ("dump_pydantic", "parse_as"):
        from .pydantic import dump_pydantic, parse_as

        return {"dump_pydantic": dump_pydantic, "parse_as": parse_as}[name]
    if name == "PyrsYamlConfigSettingsSource":
        from .settings import PyrsYamlConfigSettingsSource

        return PyrsYamlConfigSettingsSource
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
