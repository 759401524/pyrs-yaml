"""
pyrs-yaml: High-performance Python YAML library with perfect round-trip support.

This module provides a Rust-backed YAML parser and serializer that preserves
comments, anchors, tags, and formatting during round-trip conversions.

Example:
    >>> import pyrs_yaml
    >>> doc = pyrs_yaml.parse("key: value")
    >>> print(doc.to_yaml())
    key: value
    >>> print(doc.get("key"))
    value
"""

from .async_dump import (
    safe_dump_async,
    safe_load_async,
    safe_loads_async,
)
from .node import Node, YamlDocumentError
from .pyrs_yaml import (
    YAML,
    StreamIterator,
    YamlDocument,
    YamlMaxDepthError,
    YamlParseError,
    YamlSerializeError,
    YamlTypeError,
    YamlValidateError,
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
    parse_stream,
    read_markdown,
    read_markdown_str,
    safe_dump,
    safe_load,
    safe_loads,
    set_language,
)


# Monkey-patch YamlDocument with node() and find() methods
def _yaml_document_node(self):
    from .node import Node

    return Node(self)


def _yaml_document_find(self, path):
    from .node import Node

    return Node(self).find(path)


YamlDocument.node = _yaml_document_node
YamlDocument.find = _yaml_document_find

__all__ = [
    "YAML",
    "Node",
    "StreamIterator",
    "YamlDocument",
    "YamlDocumentError",
    "YamlMaxDepthError",
    "YamlParseError",
    "YamlSerializeError",
    "YamlTypeError",
    "YamlValidateError",
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
    "parse_stream",
    "read_markdown",
    "read_markdown_str",
    "safe_dump",
    "safe_dump_async",
    "safe_load",
    "safe_load_async",
    "safe_loads",
    "safe_loads_async",
    "set_language",
]

try:
    from importlib.metadata import version as _version
except Exception:

    def _version(name: str) -> str:
        return "unknown"


def __getattr__(name: str) -> str:
    if name == "__version__":
        return _version("pyrs-yaml")
    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
