"""
MergedView — read-only view of a YAML document with merge keys resolved.

Provides a dict-like interface that resolves `<<: *anchor` merge keys
without mutating the original AST. The view is built lazily from
`YamlDocument.to_dict()`.
"""

from __future__ import annotations

from typing import Any, Mapping, Sequence

from typing_extensions import override


class MergedView(Mapping[str, Any]):
    """Read-only view of a YAML document with merge keys resolved.

    Built from `YamlDocument.to_dict()`, which already resolves anchors
    and merge keys during serialization. This view provides a dict-like
    interface for inspection without mutating the original AST.
    """

    def __init__(self, document):
        data = document.to_dict()
        if isinstance(data, list):
            self._data = {i: item for i, item in enumerate(data)}
        else:
            self._data = data

    @override
    def __getitem__(self, key) -> Any:
        return self._wrap(self._data[key])

    @override
    def __iter__(self):
        return iter(self._data)

    @override
    def __len__(self) -> int:
        return len(self._data)

    @override
    def __repr__(self) -> str:
        return f"MergedView({self._data!r})"

    def _wrap(self, value):
        return MergedView._wrap_value(value)

    @staticmethod
    def _wrap_value(value: Any) -> Any:
        if isinstance(value, dict):
            return MergedView._DictView(value)
        if isinstance(value, list):
            return MergedView._ListView(value)
        return value

    class _DictView(Mapping[str, Any]):
        """Read-only dict view that wraps child values recursively."""

        def __init__(self, data):
            self._data = data

        @override
        def __getitem__(self, key):
            return MergedView._wrap_value(self._data[key])

        @override
        def __iter__(self):
            return iter(self._data)

        @override
        def __len__(self):
            return len(self._data)

        @override
        def __repr__(self):
            return repr(self._data)

    class _ListView(Sequence[Any]):
        """Read-only list view that wraps child values recursively."""

        def __init__(self, data):
            self._data = data

        @override
        def __getitem__(self, index):
            return MergedView._wrap_value(self._data[index])

        @override
        def __len__(self):
            return len(self._data)

        @override
        def __repr__(self):
            return repr(self._data)
