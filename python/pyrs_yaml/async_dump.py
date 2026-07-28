"""
Async I/O wrappers for pyrs_yaml.

Provides ``async`` counterparts to the synchronous ``safe_dump`` / ``safe_loads``
API using ``asyncio.to_thread`` so that blocking serialization can run off the
event loop thread without changing the underlying Rust implementation.

Example:
    >>> import asyncio
    >>> import pyrs_yaml
    >>>
    >>> async def main():
    ...     await pyrs_yaml.safe_dump_async({"a": 1})

Example:
    >>> async def main():
    ...     data = await pyrs_yaml.safe_loads_async("a: 1")
    ...     data == {"a": 1}
    ...     True
"""

from __future__ import annotations

import asyncio
from typing import Any

from .pyrs_yaml import safe_dump, safe_load, safe_loads


def _safe_dump_sync(data: Any) -> str:
    return safe_dump(data)


def _safe_loads_sync(yaml: str, schema: str = "core") -> Any:
    return safe_loads(yaml, schema=schema)


def _safe_load_sync(yaml: str, schema: str = "core") -> Any:
    return safe_load(yaml, schema=schema)


async def safe_dump_async(data: Any) -> str:
    """Serialize *data* to a YAML string (async).

    Mirrors :func:`pyrs_yaml.safe_dump`.
    """
    loop = asyncio.get_running_loop()
    return await loop.run_in_executor(None, _safe_dump_sync, data)


async def safe_loads_async(yaml: str, schema: str = "core") -> Any:
    """Parse a YAML string into native Python objects (async).

    Mirrors :func:`pyrs_yaml.safe_loads`.
    """
    loop = asyncio.get_running_loop()
    return await loop.run_in_executor(None, _safe_loads_sync, yaml, schema)


async def safe_load_async(yaml: str, schema: str = "core") -> Any:
    """Parse a YAML string into native Python objects (async).

    Mirrors :func:`pyrs_yaml.safe_load`.
    """
    loop = asyncio.get_running_loop()
    return await loop.run_in_executor(None, _safe_load_sync, yaml, schema)
