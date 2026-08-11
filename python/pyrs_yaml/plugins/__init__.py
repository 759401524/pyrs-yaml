"""Built-in plugins package for pyrs-yaml (Community Plugins).

Third-party packages can register custom types by defining an entry point
in their `pyproject.toml`:

    [project.entry-points."pyrs_yaml.plugins"]
    myplugin = "myplugin:register"

The callable receives no arguments and should call `register_type()` to
register custom types.
"""

import logging

from ._builtin import _register_builtins

_register_builtins()

# Auto-discover third-party plugins via entry_points
try:
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
            logging.getLogger(__name__).warning("Failed to load pyrs_yaml plugin '%s': %s", _ep.name, _exc)
except Exception:  # intentional: importlib.metadata API varies by Python version (3.8-3.12+)
    pass

__all__ = []
