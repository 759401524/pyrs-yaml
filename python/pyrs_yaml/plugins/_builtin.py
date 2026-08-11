"""Built-in plugins for pyrs-yaml (Community Plugins, Spiral 3).

These are demonstration CustomType implementations installed by default.
They show how third-party modules can register custom node types via the
same `register_type` API.
"""

import base64
import re
import uuid
from datetime import date, datetime, time
from decimal import Decimal
from typing import Any, final

from typing_extensions import override

from .._type_registry import CustomType, register_type


@final
class TimestampType(CustomType):
    """`!timestamp` — serialize/deserialize `datetime` objects."""

    python_type = datetime

    @override
    def from_yaml(self, value: str) -> Any:
        return datetime.fromisoformat(value)

    @override
    def to_yaml(self, obj: Any) -> str:
        return obj.isoformat()


@final
class DateType(CustomType):
    """`!date` — serialize/deserialize exact `datetime.date` objects (not datetime subclasses)."""

    python_type = date

    @override
    def from_yaml(self, value: str) -> Any:
        return date.fromisoformat(value)

    @override
    def to_yaml(self, obj: Any) -> str:
        return obj.isoformat()


@final
class TimeType(CustomType):
    """`!time` — serialize/deserialize `datetime.time` objects."""

    python_type = time

    @override
    def from_yaml(self, value: str) -> Any:
        return time.fromisoformat(value)

    @override
    def to_yaml(self, obj: Any) -> str:
        return obj.isoformat()


@final
class UUIDType(CustomType):
    """`!uuid` — serialize/deserialize `uuid.UUID` objects."""

    python_type = uuid.UUID

    @override
    def from_yaml(self, value: str) -> Any:
        return uuid.UUID(value)

    @override
    def to_yaml(self, obj: Any) -> str:
        return str(obj)


@final
class DecimalType(CustomType):
    """`!decimal` — serialize/deserialize `decimal.Decimal` objects."""

    python_type = Decimal

    @override
    def from_yaml(self, value: str) -> Any:
        return Decimal(value)

    @override
    def to_yaml(self, obj: Any) -> str:
        return str(obj)


@final
class BinaryType(CustomType):
    """`!binary` — base64-encoded bytes (YAML 1.1 binary tag)."""

    python_type = bytes

    @override
    def from_yaml(self, value: str) -> Any:
        return base64.b64decode(value)

    @override
    def to_yaml(self, obj: Any) -> str:
        return base64.b64encode(obj).decode("ascii")


@final
class RegexType(CustomType):
    """`!regex` — serialize/deserialize compiled `re.Pattern` objects."""

    python_type = re.Pattern

    @override
    def from_yaml(self, value: str) -> Any:
        return re.compile(value)

    @override
    def to_yaml(self, obj: Any) -> str:
        return obj.pattern


@final
class SetType(CustomType):
    """`!set` — a YAML set maps unique keys to null values.

    Note: The current CustomType protocol only supports scalar string values,
    so `from_yaml` cannot properly parse a YAML mapping-backed set.
    `to_yaml` serializes a Python set as a YAML block mapping with null values.
    """

    @override
    def from_yaml(self, value: str) -> Any:
        return value

    @override
    def to_yaml(self, obj: Any) -> str:
        items = []
        for item in sorted(obj):
            items.append(f"  {item!s}: null")
        return "!set\n" + "\n".join(items) + "\n"


def _register_builtins():
    """Register built-in plugins idempotently."""
    register_type("!date", DateType())
    register_type("!time", TimeType())
    register_type("!timestamp", TimestampType())
    register_type("!uuid", UUIDType())
    register_type("!decimal", DecimalType())
    register_type("!binary", BinaryType())
    register_type("!regex", RegexType())
    register_type("!set", SetType())
