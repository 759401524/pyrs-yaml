"""
Async I/O wrappers for pyyaml_rs.

Provides ``async`` counterparts to the synchronous ``safe_dump`` / ``safe_dumps``
API using ``asyncio.to_thread`` so that blocking serialization can run off the
event loop thread without changing the underlying Rust implementation.

Example:
    >>> import asyncio
    >>> import pyyaml_rs
    >>>
    >>> async def main():
    ...     await pyyaml_rs.safe_dump_async({"a": 1})

Example:
    >>> async def main():
    ...     data = await pyyaml_rs.safe_loads_async("a: 1")
    ...     data == {"a": 1}
    ...     True
"""

from __future__ import annotations

import asyncio
from typing import Any

from .pyyaml_rs import safe_dump, safe_dumps, safe_load, safe_loads


def _safe_dumps_sync(data: Any) -> str:
    return safe_dumps(data)


def _safe_dump_sync(data: Any) -> None:
    return safe_dump(data)


def _safe_loads_sync(yaml: str, schema: str = "core") -> Any:
    return safe_loads(yaml, schema=schema)


def _safe_load_sync(yaml: str, schema: str = "core") -> Any:
    return safe_load(yaml, schema=schema)


async def safe_dumps_async(data: Any) -> str:
    """Serialize *data* to a YAML string (async).

    Mirrors :func:`pyyaml_rs.safe_dumps`.
    """
    loop = asyncio.get_running_loop()
    return await loop.run_in_executor(None, _safe_dumps_sync, data)


async def safe_dump_async(data: Any) -> None:
    """Serialize *data* to stdout as YAML (async).

    Mirrors :func:`pyyaml_rs.safe_dump`.
    """
    loop = asyncio.get_running_loop()
    return await loop.run_in_executor(None, _safe_dump_sync, data)


async def safe_loads_async(yaml: str, schema: str = "core") -> Any:
    """Parse a YAML string into native Python objects (async).

    Mirrors :func:`pyyaml_rs.safe_loads`.
    """
    loop = asyncio.get_running_loop()
    return await loop.run_in_executor(None, _safe_loads_sync, yaml, schema)


async def safe_load_async(yaml: str, schema: str = "core") -> Any:
    """Parse a YAML string into native Python objects (async).

    Mirrors :func:`pyyaml_rs.safe_load`.
    """
    loop = asyncio.get_running_loop()
    return await loop.run_in_executor(None, _safe_load_sync, yaml, schema)
