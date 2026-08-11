"""CustomType base class and type registry for Community Plugins."""

from __future__ import annotations

from typing import Any, Callable, TypeVar, overload

from .pyrs_yaml import register_type as _rust_register_type

_T = TypeVar("_T", bound="CustomType")


class CustomType:
    """Base class for custom YAML node types.

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

    def can_parse(self, node: Any) -> bool:
        """Return True if this type should handle the given node.

        Defaults to True: the tag match alone is sufficient. Override to
        gate handling on node content (e.g. value prefix).
        """
        return True

    def from_yaml(self, value: str) -> Any:
        """Convert a YAML string value to a Python object."""
        return value

    def to_yaml(self, obj: Any) -> str:
        """Convert a Python object to a YAML string value."""
        return str(obj)

    def validate(self, obj: Any) -> bool:
        """Validate a Python object's type and value."""
        return True


@overload
def register_type(name: str, handler: _T) -> _T: ...


@overload
def register_type(name: str, handler: None = None) -> Callable[[type[_T] | _T], type[_T] | _T]: ...


def register_type(name, handler=None):
    """Register a CustomType handler.

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
        _rust_register_type(name, handler)
        return handler

    # Decorator form
    def decorator(cls_or_instance):
        # If it's a class (not an instance), instantiate it
        instance = cls_or_instance() if isinstance(cls_or_instance, type) else cls_or_instance
        _rust_register_type(name, instance)
        return cls_or_instance

    return decorator
