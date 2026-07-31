"""
Python Node API — AST navigation with path-based resolution.

Provides a Node class that wraps a YamlDocument and a path, enabling
tree traversal, query, and mutation operations.
"""

from __future__ import annotations

import warnings
from typing import Any, Callable, Iterator


class YamlDocumentError(Exception):
    """Raised when a Node's parent YamlDocument has been garbage collected."""


class Node:
    """A node in the YAML AST, backed by a YamlDocument and a path.

    Each Node stores a reference to its parent YamlDocument and a
    path tuple that navigates to the target node within the document's AST.
    """

    def __init__(self, document: Any, path: tuple = ()):
        self._doc = document
        self._path = path
        self._alive = True

    def _get_doc(self) -> Any:
        if not self._alive:
            warnings.warn(
                "Accessing a stale Node whose parent YamlDocument has been released",
                RuntimeWarning,
                stacklevel=3,
            )
            raise YamlDocumentError("parent document has been released")
        if self._doc is None:
            self._alive = False
            warnings.warn(
                "Accessing a stale Node whose parent YamlDocument has been released",
                RuntimeWarning,
                stacklevel=3,
            )
            raise YamlDocumentError("parent document has been released")
        return self._doc

    def _resolve(self) -> Any:
        """Navigate from the document root through the path to get the value."""
        doc = self._get_doc()
        data = doc.to_dict()
        current = data
        for segment in self._path:
            current = current[segment]
        return current

    def is_valid(self) -> bool:
        """Check if the parent document is still alive."""
        return self._alive and self._doc is not None

    def release(self) -> None:
        """Release the reference to the parent document, marking this node as stale.

        After calling release(), any access to this node will emit a RuntimeWarning
        and raise YamlDocumentError.
        """
        self._alive = False
        self._doc = None

    @property
    def root_type(self) -> str:
        """Get the type of this node: scalar, mapping, sequence, null, alias."""
        resolved = self._resolve()
        if isinstance(resolved, dict):
            return "mapping"
        if isinstance(resolved, list):
            return "sequence"
        if resolved is None:
            return "null"
        return "scalar"

    @property
    def value(self) -> Any | None:
        """Get the scalar value of this node. None for non-scalars."""
        if self.root_type not in ("scalar", "null"):
            return None
        return self._resolve()

    def to_yaml(self) -> str:
        """Serialize this subtree to YAML string."""
        resolved = self._resolve()
        if isinstance(resolved, (dict, list)):
            from pyrs_yaml import from_dict

            return from_dict(resolved)
        if resolved is None:
            return "null\n"
        if isinstance(resolved, str):
            return resolved + "\n"
        return str(resolved) + "\n"

    @property
    def parent(self) -> Node | None:
        """Get the parent Node, or None if this is the root."""
        if not self._path:
            return None
        return Node(self._doc, self._path[:-1])

    @property
    def children(self) -> list[Node]:
        """Get the child nodes of this node."""
        resolved = self._resolve()
        if isinstance(resolved, dict):
            return [Node(self._doc, (*self._path, k)) for k in resolved]
        if isinstance(resolved, list):
            return [Node(self._doc, (*self._path, i)) for i in range(len(resolved))]
        return []

    def walk(self) -> Iterator[Node]:
        """Walk all descendant nodes (depth-first pre-order)."""
        yield self
        for child in self.children:
            yield from child.walk()

    def filter(self, predicate: Callable[[Node], bool]) -> list[Node]:
        """Filter descendant nodes by a predicate function."""
        return [node for node in self.walk() if predicate(node)]

    def find(self, path: str) -> Any:
        """Find a node by JSONPath-like path.

        Supports:
            $.key           - Root key
            $.key.subkey    - Nested key
            $.arr[0]        - Index into sequence
            $.arr[*]        - All items in sequence
            $..key          - Deep search for key at any depth
        """
        segments = _parse_jsonpath(path)
        current = self
        for seg in segments:
            if isinstance(seg, int):
                doc = current._get_doc()
                idx_path = (*current._path, seg)
                current = Node(doc, idx_path)
            elif seg == "*":
                children = current.children
                return [Node(current._get_doc(), child._path) for child in children]
            elif isinstance(seg, str) and seg.startswith(".."):
                doc = current._get_doc()
                key = seg[2:]
                if key == "*":
                    all_nodes = list(current.walk())
                    return [Node(doc, n._path) for n in all_nodes]
                results = []
                for node in current.walk():
                    try:
                        val = node._resolve()
                        if isinstance(val, dict) and key in val:
                            results.append(Node(doc, (*node._path, key)))
                    except (KeyError, IndexError, TypeError):
                        continue
                return results
            elif isinstance(seg, str):
                doc = current._get_doc()
                val_path = (*current._path, seg)
                current = Node(doc, val_path)
        return current

    def __repr__(self) -> str:
        if not self._alive:
            return "Node(released)"
        try:
            return f"Node(root_type={self.root_type}, path={self._path})"
        except (YamlDocumentError, Exception):
            return "Node(invalid)"

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Node):
            return NotImplemented
        return self._path == other._path and self._doc is other._doc and self._alive == other._alive


def _parse_jsonpath(path: str) -> list:
    """Parse a JSONPath-like path into a list of segments.

    Returns a list where each segment is either:
        str     - key lookup
        int     - index lookup
        "*"     - wildcard
        "..key" - deep scan
    """
    if not path.startswith("$"):
        raise ValueError("Path must start with $")

    rest = path[1:]
    # Remove the first dot after $, but preserve ..
    if rest.startswith(".."):
        rest = rest  # keep .. for deep scan
    elif rest.startswith("."):
        rest = rest[1:]

    segments = []
    i = 0
    while i < len(rest):
        if i + 1 < len(rest) and rest[i] == "." and rest[i + 1] == ".":
            # Deep scan (..)
            i += 2
            key = ""
            while i < len(rest) and rest[i] not in (".", "["):
                key += rest[i]
                i += 1
            segments.append(f"..{key}")
        elif rest[i] == ".":
            i += 1
            continue
        elif rest[i] == "[":
            i += 1
            if i < len(rest) and rest[i] == "*":
                segments.append("*")
                i += 1
                if i < len(rest) and rest[i] == "]":
                    i += 1
                continue
            num = ""
            while i < len(rest) and rest[i].isdigit():
                num += rest[i]
                i += 1
            if i < len(rest) and rest[i] == "]":
                i += 1
            segments.append(int(num))
        else:
            key = ""
            while i < len(rest) and rest[i] not in (".", "["):
                key += rest[i]
                i += 1
            segments.append(key)

    return segments
