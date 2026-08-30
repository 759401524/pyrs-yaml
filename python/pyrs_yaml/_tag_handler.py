"""Unified TagHandler base class for pyrs-yaml.

Replaces the separate ``CustomType`` and ``register_tag`` protocols with a
single handler that can control parsing, serialization, and validation of
YAML tags — including container tags (Mapping/Sequence).

Phase 1 (additive): TagHandler coexists with CustomType and register_tag.
Phase 2 (future): CustomType deprecated in favor of TagHandler.
Phase 3 (v2.0): Old APIs removed.
"""

from __future__ import annotations

import contextlib
from dataclasses import dataclass, field
from typing import Any

from .pyrs_yaml import YamlTagSkip


class TagHandler:
    """Base class for custom YAML tag handlers.

    Implement the methods you need. Default implementations are no-ops/identity::

        class TimestampHandler(TagHandler):
            python_type = datetime

            def from_yaml(self, value):
                return datetime.fromisoformat(value)

            def to_yaml(self, obj):
                return obj.isoformat()

        register_tag_handler("!timestamp", TimestampHandler())

    **Scalar handlers** (run during parse, before schema resolution)::

        def resolve(self, value: str, tag: str) -> str:
            \"\"\"Transform raw scalar string. Return transformed string or raise YamlTagSkip.\"\"\"
            raise YamlTagSkip

    **Type handlers** (run during load/save, after schema resolution)::

        python_type: type | None = None

        def can_parse(self, value: Any) -> bool: ...
        def from_yaml(self, value: Any) -> Any: ...
        def to_yaml(self, obj: Any) -> str: ...
        def validate(self, obj: Any) -> bool: ...

    **Container handlers** (NEW — for tagged Mapping/Sequence nodes)::

        def from_yaml_mapping(self, tag: str, data: dict) -> Any:
            \"\"\"Convert tagged mapping to Python object. Raise YamlTagSkip for default dict.\"\"\"
            raise YamlTagSkip

        def from_yaml_sequence(self, tag: str, data: list) -> Any:
            \"\"\"Convert tagged sequence to Python object. Raise YamlTagSkip for default list.\"\"\"
            raise YamlTagSkip

        def to_yaml_mapping(self, obj: Any) -> dict | None:
            \"\"\"Convert Python object to mapping for YAML output. Return None to skip.\"\"\"
            return None

        def to_yaml_sequence(self, obj: Any) -> list | None:
            \"\"\"Convert Python object to sequence for YAML output. Return None to skip.\"\"\"
            return None

    **Lifecycle hooks** (optional — run at parse/serialize boundaries)::

        def on_parse_start(self, yaml_str: str, context: ParseContext) -> str:
            return yaml_str

        def on_parse_end(self, document: Any, context: ParseContext) -> None:
            pass

        def on_serialize_start(self, document: Any, context: SerializeContext) -> None:
            pass

        def on_serialize_end(self, yaml_str: str, context: SerializeContext) -> str:
            return yaml_str
    """

    # --- Scalar handlers ---

    def resolve(self, value: str, tag: str) -> str:
        """Parse-time: transform raw scalar string before schema resolution.

        Return the transformed string, or raise ``YamlTagSkip`` to skip
        this handler and try the next one (for priority-based chaining).
        """
        raise YamlTagSkip

    # --- Type handlers ---

    python_type: Any = None
    """Python type for ``isinstance`` matching during serialization.
    Set to ``None`` to disable serialization matching (parse-only handler)."""

    def can_parse(self, value: Any) -> bool:
        """Gate: should ``from_yaml`` handle this schema-resolved value?

        ``value`` is the schema-resolved Python object (str, int, dict, list,
        etc.). Default: ``True`` — the tag match alone is sufficient.
        """
        return True

    def from_yaml(self, value: Any) -> Any:
        """Load-time: convert schema-resolved Python value to target object."""
        return value

    def to_yaml(self, obj: Any) -> str:
        """Save-time: convert Python object to YAML scalar string."""
        return str(obj)

    def validate(self, obj: Any) -> bool:
        """Validate a Python object against this type. Return True if valid."""
        return True

    # --- Container handlers ---

    def from_yaml_mapping(self, tag: str, data: dict[str, Any]) -> Any:
        """Load-time: convert a tagged mapping node to a Python object.

        ``data`` is the already-converted dict. Return the converted object,
        or raise ``YamlTagSkip`` to use the default dict.
        """
        raise YamlTagSkip

    def from_yaml_sequence(self, tag: str, data: list[Any]) -> Any:
        """Load-time: convert a tagged sequence node to a Python object.

        ``data`` is the already-converted list. Return the converted object,
        or raise ``YamlTagSkip`` to use the default list.
        """
        raise YamlTagSkip

    def to_yaml_mapping(self, obj: Any) -> dict[str, Any] | None:
        """Save-time: convert a Python object to a mapping for YAML output.

        Return a dict (keys become YAML keys), or ``None`` to skip this
        handler and fall through to default serialization.
        """
        return None

    def to_yaml_sequence(self, obj: Any) -> list[Any] | None:
        """Save-time: convert a Python object to a sequence for YAML output.

        Return a list, or ``None`` to skip this handler.
        """
        return None

    # --- Lifecycle hooks ---

    def on_parse_start(self, yaml_str: str, context: ParseContext) -> str:
        """Called before parsing. Can modify the input YAML string.

        Return the (possibly modified) YAML string.
        """
        return yaml_str

    def on_parse_end(self, document: Any, context: ParseContext) -> None:
        """Called after parsing and tag resolution. Can inspect/modify the AST."""
        pass

    def on_serialize_start(self, document: Any, context: SerializeContext) -> None:
        """Called before serialization. Can inspect/modify the AST."""
        pass

    def on_serialize_end(self, yaml_str: str, context: SerializeContext) -> str:
        """Called after serialization. Can modify the output YAML string.

        Return the (possibly modified) YAML string.
        """
        return yaml_str


@dataclass(frozen=True)
class ParseContext:
    """Metadata passed to lifecycle hooks during parsing."""

    schema: str = "core"
    max_depth: int = 1000
    resolve_merges: bool = True
    source: str | None = None


@dataclass(frozen=True)
class SerializeContext:
    """Metadata passed to lifecycle hooks during serialization."""

    indent: int = 2
    flow_style: bool | None = None
    sort_keys: bool = False
    default_style: str | None = None


@dataclass
class PluginInfo:
    """Metadata descriptor returned by a plugin's ``register()`` function."""

    name: str
    version: str = "0.0.0"
    requires: list[str] = field(default_factory=list)
    conflicts: list[str] = field(default_factory=list)
    priority: int = 0
    tags: list[str] = field(default_factory=list)


# ── Python-level handler registry (for lifecycle hooks + introspection) ─────

_handler_registry: dict[str, TagHandler] = {}
"""Maps tag name → TagHandler instance. Used for lifecycle hooks and
``list_tag_handlers`` / ``get_tag_handler`` / ``remove_tag_handler``."""


def store_handler(name: str, handler: TagHandler) -> None:
    """Store a TagHandler instance in the Python-level registry."""
    _handler_registry[name] = handler


def get_stored_handler(name: str) -> TagHandler | None:
    """Retrieve a TagHandler instance from the Python-level registry."""
    return _handler_registry.get(name)


def remove_stored_handler(name: str) -> None:
    """Remove a TagHandler instance from the Python-level registry."""
    _handler_registry.pop(name, None)


def clear_stored_handlers() -> None:
    """Remove all TagHandler instances from the Python-level registry."""
    _handler_registry.clear()


def all_stored_handlers() -> dict[str, TagHandler]:
    """Return a copy of all stored TagHandler instances."""
    return dict(_handler_registry)


def call_parse_start(yaml_str: str, context: ParseContext) -> str:
    """Call ``on_parse_start`` on all registered handlers. Returns modified YAML."""
    for handler in _handler_registry.values():
        with contextlib.suppress(Exception):
            yaml_str = handler.on_parse_start(yaml_str, context)
    return yaml_str


def call_parse_end(document: Any, context: ParseContext) -> None:
    """Call ``on_parse_end`` on all registered handlers."""
    for handler in _handler_registry.values():
        with contextlib.suppress(Exception):
            handler.on_parse_end(document, context)


def call_serialize_start(document: Any, context: SerializeContext) -> None:
    """Call ``on_serialize_start`` on all registered handlers."""
    for handler in _handler_registry.values():
        with contextlib.suppress(Exception):
            handler.on_serialize_start(document, context)


def call_serialize_end(yaml_str: str, context: SerializeContext) -> str:
    """Call ``on_serialize_end`` on all registered handlers. Returns modified YAML."""
    for handler in _handler_registry.values():
        with contextlib.suppress(Exception):
            yaml_str = handler.on_serialize_end(yaml_str, context)
    return yaml_str
