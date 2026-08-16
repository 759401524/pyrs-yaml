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
    try:
        node = Node(self)
        for seg in _parse_jsonpath(path):
            if seg == "*" or (isinstance(seg, str) and seg.startswith("..")):
                raise YamlPathError("wildcard/deep-scan paths are not editable")
            node = Node(self, (*node._path, seg))
        return node
    except ValueError as e:
        raise YamlPathError(str(e)) from e


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


def _yaml_document_sort_keys(self, path: str = "$") -> None:
    """Sort the mapping keys at ``path`` (default root) in place."""
    return _path_node(self, path).sort_keys()


def _expand_wildcard_path(doc: Any, path: str) -> list[tuple[Any, ...]]:
    """Expand a JSONPath with wildcards/deep-scan into concrete path tuples.

    Returns every leaf path matching the pattern. Unlike ``Node.find``, this
    continues descending after ``[*]`` so ``$.items[*].active`` yields the
    ``.active`` field of each item.
    """
    segments = _parse_jsonpath(path)
    results: list[tuple[Any, ...]] = []

    def walk(current_path: tuple[Any, ...], segs: list[Any]) -> None:
        if not segs:
            results.append(current_path)
            return
        seg, rest = segs[0], segs[1:]
        if seg == "*":
            node = Node(doc, current_path)
            for child in node.children:
                walk(child._path, rest)
        elif isinstance(seg, str) and seg.startswith(".."):
            key = seg[2:]
            entry = Node(doc, current_path)
            for descendant in entry.walk():
                if key == "*":
                    walk(descendant._path, rest)
                else:
                    # descendant must be a mapping containing `key`
                    val = _safe_value(doc, descendant._path)
                    if isinstance(val, dict) and key in val:
                        walk((*descendant._path, key), rest)
        elif isinstance(seg, int):
            walk((*current_path, seg), rest)
        else:
            walk((*current_path, seg), rest)

    walk((), segments)
    return sorted(dict.fromkeys(results))


def _safe_value(doc: Any, path: tuple[Any, ...]) -> Any:
    """Resolve a node's value without raising on stale/missing nodes."""
    try:
        return Node(doc, path)._resolve()
    except Exception:
        return None


def _yaml_document_set_many(self, pairs: dict[str, Any]) -> None:
    """Set multiple values at once (single splice burst).

    Paths may include wildcards (``$.items[*].active``); every matching node
    is set. Accepts ``{"path": value, ...}``.
    """
    segs = []
    values = []
    for path, value in pairs.items():
        if "*" in path or ".." in path:
            for concrete in _expand_wildcard_path(self, path):
                segs.append(list(concrete))
                values.append(value)
        else:
            n = _path_node(self, path)
            segs.append([s for s in n._path])
            values.append(value)
    if segs:
        self._set_many_path(list(zip(segs, values)))
