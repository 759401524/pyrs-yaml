"""Built-in plugins for pyrs-yaml (Community Plugins, Spiral 3).

These are demonstration CustomType implementations installed by default.
They show how third-party modules can register custom node types via the
same `register_type` API.
"""

from datetime import datetime

from .._type_registry import CustomType, register_type


class TimestampType(CustomType):
    """`!timestamp` — serialize/deserialize `datetime` objects."""

    python_type = datetime

    def from_yaml(self, value):
        return datetime.fromisoformat(value)

    def to_yaml(self, obj):
        return obj.isoformat()


class SetType(CustomType):
    """`!set` — a YAML set maps unique keys to null values."""

    def from_yaml(self, value):
        return value

    def to_yaml(self, obj):
        return str(obj)


def _register_builtins():
    """Register built-in plugins idempotently."""
    register_type("!timestamp", TimestampType())
    register_type("!set", SetType())


_register_builtins()
