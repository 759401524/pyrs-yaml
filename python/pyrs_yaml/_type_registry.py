"""CustomType base class and type registry for Community Plugins."""

from .pyrs_yaml import register_type as _rust_register_type


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
    python_type = None

    def can_parse(self, node) -> bool:
        """Return True if this type should handle the given node."""
        return False

    def from_yaml(self, value: str):
        """Convert a YAML string value to a Python object."""
        return value

    def to_yaml(self, obj) -> str:
        """Convert a Python object to a YAML string value."""
        return str(obj)

    def validate(self, obj) -> bool:
        """Validate a Python object's type and value."""
        return True


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
