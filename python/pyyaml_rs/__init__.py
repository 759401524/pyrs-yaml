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
    parse,
    parse_file,
    parse_all_docs,
    safe_load,
    safe_loads,
    safe_dump,
    safe_dumps,
    from_dict,
    from_json,
    dump_file,
    read_markdown,
    read_markdown_str,
)

__all__ = [
    "YamlDocument",
    "parse",
    "parse_file",
    "parse_all_docs",
    "safe_load",
    "safe_loads",
    "safe_dump",
    "safe_dumps",
    "from_dict",
    "from_json",
    "dump_file",
    "read_markdown",
    "read_markdown_str",
]

__version__ = "0.2.0"
