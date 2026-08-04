"""In-place editing orchestration for YamlDocument (path API + sugar)."""

from __future__ import annotations

from typing import Any

from .node import Node, _parse_jsonpath
from .pyrs_yaml import YamlPathError


def _path_node(self, path: str) -> Node:
    """Resolve a JSONPath string to a single editable Node.

    Wildcard (*) and deep-scan (..) segments raise YamlPathError since
    edits target exactly one node.
    """
    node = Node(self)
    for seg in _parse_jsonpath(path):
        if seg == "*" or (isinstance(seg, str) and seg.startswith("..")):
            raise YamlPathError("wildcard/deep-scan paths are not editable")
        node = Node(self, (*node._path, seg))
    return node


def _yaml_document_set(self, path: str, value: Any, create_missing: bool = False) -> None:
    return _path_node(self, path).set_value(value, create_missing=create_missing)


def _yaml_document_insert(self, path: str, index: int, value: Any) -> None:
    return _path_node(self, path).insert(index, value)


def _yaml_document_append(self, path: str, value: Any) -> None:
    return _path_node(self, path).append(value)


def _yaml_document_delete(self, path: str) -> None:
    return _path_node(self, path).delete()


def _yaml_document_rename(self, path: str, new_key: str) -> None:
    return _path_node(self, path).rename(new_key)
