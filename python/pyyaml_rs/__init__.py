"""
pyyaml-rs: High-performance Python YAML library with perfect round-trip support.

This module provides a Rust-backed YAML parser and serializer that preserves
comments, anchors, tags, and formatting during round-trip conversions.

Example:
    >>> import pyyaml_rs
    >>> doc = pyyaml_rs.parse("key: value")
    >>> print(doc.to_yaml())
    key: value
    >>> print(doc.get("key"))
    value
"""

from .pyyaml_rs import (
    YamlDocument,
    YamlParseError,
    YamlSerializeError,
    YamlTypeError,
    detect_language,
    dump_file,
    from_dict,
    from_json,
    get_language,
    list_languages,
    negotiate_language,
    parse,
    parse_all_docs,
    parse_file,
    read_markdown,
    read_markdown_str,
    safe_dump,
    safe_dumps,
    safe_load,
    safe_loads,
    set_language,
)

__all__ = [
    "YamlDocument",
    "YamlParseError",
    "YamlSerializeError",
    "YamlTypeError",
    "detect_language",
    "dump_file",
    "from_dict",
    "from_json",
    "get_language",
    "list_languages",
    "negotiate_language",
    "parse",
    "parse_all_docs",
    "parse_file",
    "read_markdown",
    "read_markdown_str",
    "safe_dump",
    "safe_dumps",
    "safe_load",
    "safe_loads",
    "set_language",
]

try:
    from importlib.metadata import version as _version
except Exception:

    def _version(name: str) -> str:
        return "unknown"


def __getattr__(name: str) -> str:
    if name == "__version__":
        return _version("pyyaml-rs")
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
