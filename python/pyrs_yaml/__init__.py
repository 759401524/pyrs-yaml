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
from .compliance import compliance_report
from .merged_view import MergedView
from .node import Node, YamlDocumentError
from .pydantic import parse_as
from .pyrs_yaml import (
    YAML,
    StreamIterator,
    YamlDocument,
    YamlDuplicateKeyError,
    YamlEditError,
    YamlMaxDepthError,
    YamlParseError,
    YamlPathError,
    YamlSerializeError,
    YamlTagError,
    YamlTagSkip,
    YamlTypeError,
    YamlValidateError,
    clear_tag_handlers,
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
    remove_tag,
    safe_dump,
    safe_load,
    safe_loads,
    set_language,
)
from .pyrs_yaml import register_tag as _rust_register_tag


def register_tag(name, handler=None, priority=0):
    """Register a tag handler.

    Supports both decorator and imperative forms:
        @register_tag("!custom")
        def handler(node):
            ...

        @register_tag("!custom", priority=1)
        def handler(node):
            ...

        register_tag("!custom", handler_fn)
        register_tag("!custom", handler_fn, priority=1)
    """
    if handler is not None:
        _rust_register_tag(name, handler, priority)
        return handler

    # Decorator form
    def decorator(fn):
        _rust_register_tag(name, fn, priority)
        return fn

    return decorator


# Monkey-patch YamlDocument with node() and find() methods
def _yaml_document_node(self):
    from .node import Node

    return Node(self)


def _yaml_document_find(self, path):
    from .node import Node

    return Node(self).find(path)


def _yaml_document_walk(self):
    from .node import Node

    for path in self._walk_paths():
        yield Node(self, path)


def _yaml_document_scalars(self):
    from .node import Node

    for path in self._scalar_paths():
        yield Node(self, path)


YamlDocument.node = _yaml_document_node
YamlDocument.find = _yaml_document_find
YamlDocument.walk = _yaml_document_walk
YamlDocument.scalars = _yaml_document_scalars


def _yaml_document_merged(self):
    from .merged_view import MergedView

    return MergedView(self)


YamlDocument.merged = _yaml_document_merged


from . import editing as _editing  # noqa: E402  (monkeypatches must run after Rust module loads)

YamlDocument.set = _editing._yaml_document_set
YamlDocument.insert = _editing._yaml_document_insert
YamlDocument.append = _editing._yaml_document_append
YamlDocument.delete = _editing._yaml_document_delete
YamlDocument.rename = _editing._yaml_document_rename

__all__ = [
    "YAML",
    "MergedView",
    "Node",
    "StreamIterator",
    "YamlDocument",
    "YamlDocumentError",
    "YamlDuplicateKeyError",
    "YamlEditError",
    "YamlMaxDepthError",
    "YamlParseError",
    "YamlPathError",
    "YamlSerializeError",
    "YamlTagError",
    "YamlTagSkip",
    "YamlTypeError",
    "YamlValidateError",
    "clear_tag_handlers",
    "compliance_report",
    "detect_language",
    "dump_file",
    "from_dict",
    "from_json",
    "get_language",
    "list_languages",
    "negotiate_language",
    "parse",
    "parse_all_docs",
    "parse_as",
    "parse_file",
    "parse_stream",
    "read_markdown",
    "read_markdown_str",
    "register_tag",
    "remove_tag",
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
