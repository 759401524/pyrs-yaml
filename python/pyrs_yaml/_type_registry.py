"""Low-level tag handler registration backend.

The public API is `register_tag_handler` (see `python/pyrs_yaml/__init__.py`).
This module wires a `TagHandler` into the Rust tag registry, Rust type
registry, and the Python handler registry so a single `TagHandler` instance
powers parse-time resolution, load/save conversion, and container dispatch.
"""

from __future__ import annotations

from ._tag_handler import TagHandler, store_handler
from .pyrs_yaml import register_tag as _rust_register_tag
from .pyrs_yaml import register_type as _rust_register_type


def _register_via_core(name: str, handler: TagHandler, priority: int = 0) -> None:
    """Register a `TagHandler` into both Rust registries + the Python registry.

    Low-level: used by `register_tag_handler`. Does not emit deprecation
    warnings and does not perform duplicate registration.
    """

    def _resolve_wrapper(value: str) -> str:
        return handler.resolve(value, name)

    _rust_register_tag(name, _resolve_wrapper, priority)
    _rust_register_type(name, handler)
    store_handler(name, handler)
