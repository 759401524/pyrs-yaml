"""pydantic-settings integration: a YAML settings source backed by pyrs-yaml.

``PyrsYamlConfigSettingsSource`` is a drop-in replacement for
``pydantic_settings.YamlConfigSettingsSource`` that uses pyrs-yaml as the
YAML parser instead of PyYAML. It is defined lazily (PEP 562 module
``__getattr__``) so ``import pyrs_yaml`` never requires pydantic-settings.
"""

from __future__ import annotations

from functools import lru_cache
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from pathlib import Path

    from pydantic_settings.sources.types import Traversable

from typing_extensions import override

__all__ = ["PyrsYamlConfigSettingsSource"]  # noqa: F822 — provided lazily via __getattr__


@lru_cache(maxsize=1)
def _settings_source_class() -> type[Any]:
    try:
        from pydantic_settings import YamlConfigSettingsSource
    except ImportError as exc:
        raise ImportError(
            "pydantic-settings is required for PyrsYamlConfigSettingsSource. Install with: uv add 'pyrs-yaml[settings]'"
        ) from exc

    from .pyrs_yaml import safe_load

    class PyrsYamlConfigSettingsSource(YamlConfigSettingsSource):
        """Load settings from a YAML file using pyrs-yaml instead of PyYAML.

        Only the file-reading step differs from
        ``YamlConfigSettingsSource``; ``yaml_file``, ``yaml_file_encoding``,
        ``yaml_config_section`` (dot-notation paths included) and
        ``deep_merge`` behave identically. Values are resolved with the YAML
        1.2 core schema (e.g. ``on`` stays a string) rather than PyYAML's
        YAML 1.1 rules.
        """

        @override
        def _read_file(self, file_path: Path | Traversable) -> dict[str, Any]:
            with file_path.open(encoding=self.yaml_file_encoding) as yaml_file:
                data = safe_load(yaml_file.read())
            return data if isinstance(data, dict) else {}

    return PyrsYamlConfigSettingsSource


def __getattr__(name: str) -> Any:
    if name == "PyrsYamlConfigSettingsSource":
        return _settings_source_class()
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
