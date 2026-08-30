"""Built-in tag handlers for pyrs-yaml.

These `TagHandler` implementations are registered by default so that common
YAML tags (`!timestamp`, `!date`, `!uuid`, `!set`, etc.) round-trip to native
Python objects. Optional third-party libraries (pendulum, arrow, ulid) are
registered when available.
"""

import base64
import re
import uuid
from datetime import date, datetime, time
from decimal import Decimal
from typing import Any, final

from typing_extensions import override

from .._tag_handler import TagHandler
from .._type_registry import _register_via_core


@final
class TimestampHandler(TagHandler):
    """`!timestamp` — serialize/deserialize `datetime` objects."""

    python_type = datetime

    @override
    def from_yaml(self, value: str) -> Any:
        return datetime.fromisoformat(value)

    @override
    def to_yaml(self, obj: Any) -> str:
        return obj.isoformat()


@final
class DateHandler(TagHandler):
    """`!date` — exact `datetime.date` objects (not datetime subclasses)."""

    python_type = date

    @override
    def from_yaml(self, value: str) -> Any:
        return date.fromisoformat(value)

    @override
    def to_yaml(self, obj: Any) -> str:
        return obj.isoformat()


@final
class TimeHandler(TagHandler):
    """`!time` — `datetime.time` objects."""

    python_type = time

    @override
    def from_yaml(self, value: str) -> Any:
        return time.fromisoformat(value)

    @override
    def to_yaml(self, obj: Any) -> str:
        return obj.isoformat()


@final
class UUIDHandler(TagHandler):
    """`!uuid` — `uuid.UUID` objects."""

    python_type = uuid.UUID

    @override
    def from_yaml(self, value: str) -> Any:
        return uuid.UUID(value)

    @override
    def to_yaml(self, obj: Any) -> str:
        return str(obj)


@final
class DecimalHandler(TagHandler):
    """`!decimal` — `decimal.Decimal` objects."""

    python_type = Decimal

    @override
    def from_yaml(self, value: str) -> Any:
        return Decimal(value)

    @override
    def to_yaml(self, obj: Any) -> str:
        return str(obj)


@final
class BinaryHandler(TagHandler):
    """`!binary` — base64-encoded bytes (YAML 1.1 binary tag)."""

    python_type = bytes

    @override
    def from_yaml(self, value: str) -> Any:
        return base64.b64decode(value)

    @override
    def to_yaml(self, obj: Any) -> str:
        return base64.b64encode(obj).decode("ascii")


@final
class RegexHandler(TagHandler):
    """`!regex` — compiled `re.Pattern` objects."""

    python_type = re.Pattern

    @override
    def from_yaml(self, value: str) -> Any:
        return re.compile(value)

    @override
    def to_yaml(self, obj: Any) -> str:
        return obj.pattern


@final
class SetHandler(TagHandler):
    """`!set` — a YAML set (unique keys mapped to null) becomes a Python set."""

    @override
    def from_yaml_mapping(self, tag: str, data: dict[str, Any]) -> Any:
        return set(data.keys())

    @override
    def to_yaml_mapping(self, obj: Any) -> dict[str, Any]:
        return {item: None for item in obj}


@final
class DurationHandler(TagHandler):
    """`!duration` — `pendulum.Duration` objects (registered when pendulum present)."""

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
class ArrowHandler(TagHandler):
    """`!arrow` — `arrow.Arrow` objects (registered when arrow present)."""

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
class ULIDHandler(TagHandler):
    """`!ulid` — `ulid.ULID` objects (registered when ulid present)."""

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
    """Register built-in tag handlers idempotently."""
    _register_via_core("!date", DateHandler())
    _register_via_core("!time", TimeHandler())
    _register_via_core("!timestamp", TimestampHandler())
    _register_via_core("!uuid", UUIDHandler())
    _register_via_core("!decimal", DecimalHandler())
    _register_via_core("!binary", BinaryHandler())
    _register_via_core("!regex", RegexHandler())
    _register_via_core("!set", SetHandler())


def _register_third_party():
    """Register optional third-party tag handlers (no-op when library absent)."""
    try:
        import pendulum

        _register_via_core("!duration", DurationHandler(pendulum))
    except ImportError:
        pass

    try:
        import arrow

        _register_via_core("!arrow", ArrowHandler(arrow))
    except ImportError:
        pass

    try:
        from ulid import ULID

        _register_via_core("!ulid", ULIDHandler(ULID))
    except ImportError:
        pass
