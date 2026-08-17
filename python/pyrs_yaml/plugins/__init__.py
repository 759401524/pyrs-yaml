""" "Built-in plugins package for pyrs-yaml (Community Plugins).

Third-party packages can register custom types by defining an entry point
in their `pyproject.toml`:

    [project.entry-points."pyrs_yaml.plugins"]
    myplugin = "myplugin:register"

The callable receives no arguments and should call `register_type()` to
register custom types.
"""

import contextlib
import logging

from ._builtin import _register_builtins

_register_builtins()

# ── Discovery error tracking ────────────────────────────────────────────────

_discovery_errors: list[str] = []


def _discover() -> None:
    """Auto-discover third-party plugins via entry_points."""
    import importlib.metadata as _importlib_metadata

    # entry_points() API varies by Python version:
    # - 3.12+: entry_points(group="...")
    # - 3.9-3.11: entry_points().select(group="...")
    # - 3.8: entry_points()["..."]
    if hasattr(_importlib_metadata.entry_points(), "select"):
        _eps = _importlib_metadata.entry_points().select(group="pyrs_yaml.plugins")
    elif hasattr(_importlib_metadata, "entry_points"):
        _eps = _importlib_metadata.entry_points().get("pyrs_yaml.plugins", [])
    else:
        _eps = ()

    for _ep in _eps:
        try:
            _register = _ep.load()
            if callable(_register):
                _register()
        except Exception as _exc:
            msg = f"Failed to load pyrs_yaml plugin '{_ep.name}': {_exc}"
            logging.getLogger(__name__).warning(msg)
            _discovery_errors.append(msg)


with contextlib.suppress(Exception):
    _discover()


def discover_plugins(force: bool = False) -> None:
    """Re-discover third-party plugins via entry_points.

    By default, only plugins not yet loaded are processed. Set ``force=True``
    to re-run discovery for all plugins (including those that previously
    failed or were already loaded).
    """
    _discovery_errors.clear()
    _discover()


def get_discovery_errors() -> list[str]:
    """Return a list of error messages from the last plugin discovery run."""
    return list(_discovery_errors)


__all__ = ["discover_plugins", "get_discovery_errors"]
