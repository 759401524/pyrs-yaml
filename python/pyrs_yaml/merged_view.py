"""
MergedView — read-only view of a YAML document with merge keys resolved.

Provides a dict-like interface that resolves `<<: *anchor` merge keys
without mutating the original AST. The view is built lazily from
`YamlDocument.to_dict()`.
"""

from collections.abc import Mapping, Sequence


class MergedView(Mapping):
    """Read-only view of a YAML document with merge keys resolved.

    Built from `YamlDocument.to_dict()`, which already resolves anchors
    and merge keys during serialization. This view provides a dict-like
    interface for inspection without mutating the original AST.
    """

    def __init__(self, document):
        self._data = document.to_dict()

    def __getitem__(self, key):
        return self._wrap(self._data[key])

    def __iter__(self):
        return iter(self._data)

    def __len__(self):
        return len(self._data)

    def __repr__(self):
        return f"MergedView({self._data!r})"

    def _wrap(self, value):
        """Wrap a value in a MergedView if it's a dict, or a MergedList if it's a list."""
        if isinstance(value, dict):
            return MergedView._DictView(value)
        if isinstance(value, list):
            return MergedView._ListView(value)
        return value

    class _DictView(Mapping):
        """Read-only dict view that wraps child values recursively."""

        def __init__(self, data):
            self._data = data

        def __getitem__(self, key):
            value = self._data[key]
            if isinstance(value, dict):
                return MergedView._DictView(value)
            if isinstance(value, list):
                return MergedView._ListView(value)
            return value

        def __iter__(self):
            return iter(self._data)

        def __len__(self):
            return len(self._data)

        def __repr__(self):
            return repr(self._data)

    class _ListView(Sequence):
        """Read-only list view that wraps child values recursively."""

        def __init__(self, data):
            self._data = data

        def __getitem__(self, index):
            value = self._data[index]
            if isinstance(value, dict):
                return MergedView._DictView(value)
            if isinstance(value, list):
                return MergedView._ListView(value)
            return value

        def __len__(self):
            return len(self._data)

        def __repr__(self):
            return repr(self._data)
