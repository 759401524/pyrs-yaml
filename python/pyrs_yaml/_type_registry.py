"""CustomType base class and type registry for Community Plugins.

.. deprecated::
    `CustomType` and `register_type` are deprecated (Phase 2). Use the unified
    `TagHandler` API (`register_tag_handler`) instead. `register_type` now
    auto-wraps a `CustomType` as a `TagHandler` so existing code keeps working,
    but new plugins should subclass `TagHandler` directly. Removal is scheduled
    for v2.0.
"""

from __future__ import annotations

import warnings
from typing import Any, Callable, TypeVar, overload

from typing_extensions import override

from ._tag_handler import TagHandler, store_handler
from .pyrs_yaml import register_tag as _rust_register_tag
from .pyrs_yaml import register_type as _rust_register_type

_T = TypeVar("_T", bound="CustomType")

_DEPRECATION_MSG = "CustomType/register_type is deprecated; use TagHandler/register_tag_handler instead"


def _warn_deprecation() -> None:
    warnings.warn(_DEPRECATION_MSG, DeprecationWarning, stacklevel=2)


class CustomType:
    """Base class for custom YAML node types.

    .. deprecated::
        Use `TagHandler` instead (see `register_tag_handler`). `CustomType` is
        retained for backward compatibility and will be removed in v2.0.

    Subclass this to define a custom type that can be used with YAML tags:

        class TimestampType(CustomType):
            python_type = datetime  # optional: for serialization isinstance check

            def from_yaml(self, value):
                return datetime.fromisoformat(value)

            def to_yaml(self, obj):
                return obj.isoformat()

            def can_parse(self, node):
                return True

        register_type("!timestamp", TimestampType())
    """

    # Optional: set this to a Python type for serialization isinstance checks.
    python_type: Any = None

    def can_parse(self, value: Any) -> bool:
        """Return True if this type should handle the given value.

        ``value`` is the schema-resolved Python value (str, int, dict, list,
        etc.). Defaults to True: the tag match alone is sufficient. Override
        to gate handling on value content (e.g. value prefix).
        """
        return True

    def from_yaml(self, value: Any) -> Any:
        """Convert a schema-resolved Python value to the target Python object.

        ``value`` is the already-resolved Python value (str, int, dict, list,
        None, etc.). For scalar tags this is a str; for structured tags
        (e.g. ``!set``) it can be a dict or list.
        """
        return value

    def to_yaml(self, obj: Any) -> str:
        """Convert a Python object to a YAML string value."""
        return str(obj)

    def validate(self, obj: Any) -> bool:
        """Validate a Python object's type and value."""
        return True


class _CustomTypeAdapter(TagHandler):
    """Wrap a legacy `CustomType` as a `TagHandler` (Phase 2 compatibility).

    Preserves the old scalar `from_yaml`/`to_yaml`/`can_parse`/`validate`
    semantics while exposing the new `TagHandler` protocol so the handler
    participates in container dispatch and lifecycle hooks.
    """

    def __init__(self, ct: CustomType) -> None:
        self._ct = ct
        self.python_type = ct.python_type

    @override
    def resolve(self, value: str, tag: str) -> str:
        # CustomType never rewrites the scalar string; identity transform.
        return value

    @override
    def can_parse(self, value: Any) -> bool:
        return self._ct.can_parse(value)

    @override
    def from_yaml(self, value: Any) -> Any:
        return self._ct.from_yaml(value)

    @override
    def to_yaml(self, obj: Any) -> str:
        return self._ct.to_yaml(obj)

    @override
    def validate(self, obj: Any) -> bool:
        return self._ct.validate(obj)


def _register_via_core(name: str, instance: CustomType | TagHandler, priority: int = 0) -> None:
    """Register a handler into both Rust registries + the Python handler registry.

    Low-level: does NOT emit deprecation warnings. A legacy `CustomType` is
    auto-wrapped as a `TagHandler` so both legacy and new handlers share one
    registration backend without recursion or duplicate warnings.
    """
    handler: TagHandler = instance if isinstance(instance, TagHandler) else _CustomTypeAdapter(instance)

    def _resolve_wrapper(value: str) -> str:
        return handler.resolve(value, name)

    _rust_register_tag(name, _resolve_wrapper, priority)
    _rust_register_type(name, handler)
    store_handler(name, handler)


@overload
def register_type(name: str, handler: _T) -> _T: ...


@overload
def register_type(name: str, handler: None = None) -> Callable[[type[_T] | _T], type[_T] | _T]: ...


def register_type(name, handler=None):
    """Register a CustomType handler.

    .. deprecated::
        Use `register_tag_handler` with a `TagHandler` instead. This function
        now auto-wraps the `CustomType` as a `TagHandler` and will be removed in
        v2.0.

    Supports both imperative and decorator forms:

        register_type("!timestamp", TimestampType())

        @register_type("!timestamp")
        class TimestampType(CustomType):
            def from_yaml(self, value):
                return datetime.fromisoformat(value)

    Args:
        name: The tag name (e.g. "!timestamp").
        handler: A CustomType instance, or None for decorator form.
    """
    if handler is not None:
        _warn_deprecation()
        _register_via_core(name, _CustomTypeAdapter(handler))
        return handler

    # Decorator form
    def decorator(cls_or_instance):
        _warn_deprecation()
        # If it's a class (not an instance), instantiate it
        instance = cls_or_instance() if isinstance(cls_or_instance, type) else cls_or_instance
        _register_via_core(name, _CustomTypeAdapter(instance))
        return cls_or_instance

    return decorator
