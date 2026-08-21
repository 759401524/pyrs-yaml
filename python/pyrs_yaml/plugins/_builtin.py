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

    When parsed, the value is a Python dict whose keys are the set members.
    ``from_yaml`` extracts the keys and returns a Python set.
    ``to_yaml`` serializes a Python set as a YAML block mapping with null values.
    """

    @override
    def from_yaml(self, value: Any) -> Any:
        if isinstance(value, dict):
            return set(value.keys())
        return {value} if value is not None else set()

    @override
    def to_yaml(self, obj: Any) -> str:
        items = []
        for item in sorted(obj):
            items.append(f"  {item!s}: null")
        return "!set\n" + "\n".join(items) + "\n"


@final
class DurationType(CustomType):
    """`!duration` — serialize/deserialize `pendulum.Duration` objects.

    `pendulum.Duration` is a `datetime.timedelta` subclass, but is only
    registered when the `pendulum` library is installed, so a plain stdlib
    `timedelta` is never matched.
    """

    def __init__(self, module: Any) -> None:
        self.python_type = module.Duration
        self._module = module

    @override
    def from_yaml(self, value: str) -> Any:
        return self._module.duration(seconds=float(value))

    @override
    def to_yaml(self, obj: Any) -> str:
        return str(obj.total_seconds())


@final
class ArrowType(CustomType):
    """`!arrow` — serialize/deserialize `arrow.Arrow` objects."""

    def __init__(self, module: Any) -> None:
        self.python_type = module.Arrow
        self._module = module

    @override
    def from_yaml(self, value: str) -> Any:
        return self._module.get(value)

    @override
    def to_yaml(self, obj: Any) -> str:
        return obj.isoformat()


@final
class ULIDType(CustomType):
    """`!ulid` — serialize/deserialize `ulid.ULID` objects."""

    def __init__(self, py_type: Any) -> None:
        self.python_type = py_type
        self._py_type = py_type

    @override
    def from_yaml(self, value: str) -> Any:
        return self._py_type.from_str(value)

    @override
    def to_yaml(self, obj: Any) -> str:
        return str(obj)


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


def _register_third_party():
    """Register optional third-party plugins (no-op when library is absent)."""
    # pendulum.Duration → !duration
    try:
        import pendulum

        register_type("!duration", DurationType(pendulum))
    except ImportError:
        pass

    # arrow.Arrow → !arrow
    try:
        import arrow

        register_type("!arrow", ArrowType(arrow))
    except ImportError:
        pass

    # ulid.ULID → !ulid
    try:
        from ulid import ULID

        register_type("!ulid", ULIDType(ULID))
    except ImportError:
        pass
