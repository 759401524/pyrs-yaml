"""Property-based tests (Hypothesis) for the deep editing / metadata APIs
added since 0.14 (set_many wildcards, metadata setters, sort_keys).

Each test fuzzes a core invariant on random JSON-compatible documents:

- ``set_many`` with a wildcard path equals applying the same value to every
  matched path individually (single splice burst vs N bursts).
- Adding/removing metadata (comment/anchor/tag) never corrupts the document
  and never changes the resolved value.
- ``sort_keys`` is idempotent and preserves the document's value.
"""

from urllib.parse import quote

from hypothesis import given, settings
from hypothesis import strategies as st

import pyrs_yaml
from tests.strategies import roundtrip_safe_json, roundtrip_safe_leaf

MAX_EXAMPLES = 75


def as_doc(value):
    """Parse a native value into a YamlDocument via safe_dump + parse."""
    return pyrs_yaml.parse(pyrs_yaml.safe_dump(value))


def _safe_keys(value):
    """Yield mapping keys that are non-empty ASCII alphabetic strings — safe
    for bare JSONPath segments (numeric / non-ASCII / symbol keys would be
    parsed as an index or require quoting, which is a JSONPath syntax concern
    outside the reach of these editing invariants)."""
    if isinstance(value, dict):
        for k in value:
            if isinstance(k, str) and k and k.isascii() and k.isalpha():
                yield k


class TestSetManyWildcardProperty:
    @settings(max_examples=MAX_EXAMPLES, deadline=None)
    @given(st.data())
    def test_wildcard_set_many_equals_individual_sets(self, data):
        value = data.draw(roundtrip_safe_json)
        # Only target lists whose elements are all dicts — descending into a
        # `.key` inside a scalar/None element requires create_missing, which is
        # a different (already unit-tested) path, not this wildcard invariant.
        keys = [
            k
            for k in _safe_keys(value)
            if isinstance(value[k], list) and value[k] and all(isinstance(e, dict) for e in value[k])
        ]
        if not keys:
            return
        key = keys[0]
        path = f"$.{quote(key)}[*].__touched__"
        touched = data.draw(roundtrip_safe_leaf)
        doc = as_doc(value)
        doc.set_many({path: touched})
        before = doc.to_dict()

        doc2 = as_doc(value)
        for i in range(len(value[key])):
            doc2.set(f"$.{quote(key)}[{i}].__touched__", touched)
        assert before == doc2.to_dict()

    @settings(max_examples=MAX_EXAMPLES, deadline=None)
    @given(roundtrip_safe_json)
    def test_set_many_never_corrupts_roundtrip(self, value):
        keys = list(_safe_keys(value))
        if not keys:
            return
        key = keys[0]
        doc = as_doc(value)
        doc.set_many({f"$.{quote(key)}": 0})
        reparsed = pyrs_yaml.parse(doc.to_yaml()).to_dict()
        assert key in reparsed


class TestMetadataProperty:
    @settings(max_examples=MAX_EXAMPLES, deadline=None)
    @given(roundtrip_safe_json)
    def test_comment_edit_preserves_value(self, value):
        """A standalone comment on a child key never changes the value."""
        keys = list(_safe_keys(value))
        if not keys:
            return
        k = keys[0]
        doc = as_doc(value)
        doc.node().find(f"$.{quote(k)}").set_comment("a property comment")
        assert doc.to_dict() == value

    @settings(max_examples=MAX_EXAMPLES, deadline=None)
    @given(roundtrip_safe_json)
    def test_anchor_edit_preserves_value(self, value):
        keys = list(_safe_keys(value))
        if not keys:
            return
        k = keys[0]
        doc = as_doc(value)
        doc.node().find(f"$.{quote(k)}").set_anchor("amp")
        assert doc.to_dict() == value

    @settings(max_examples=MAX_EXAMPLES, deadline=None)
    @given(roundtrip_safe_json)
    def test_tag_edit_preserves_value(self, value):
        keys = list(_safe_keys(value))
        if not keys:
            return
        k = keys[0]
        doc = as_doc(value)
        child = doc.node().find(f"$.{quote(k)}")
        if child.scalar_style is None:
            return  # only scalars carry explicit tags
        child.set_tag("!custom")
        assert doc.node().find(f"$.{quote(k)}").value == value[k]

    @settings(max_examples=MAX_EXAMPLES, deadline=None)
    @given(roundtrip_safe_json)
    def test_comment_edit_stays_valid_yaml(self, value):
        """After a comment edit the document must still parse (round-trips)."""
        keys = list(_safe_keys(value))
        if not keys:
            return
        k = keys[0]
        doc = as_doc(value)
        doc.node().find(f"$.{quote(k)}").set_comment("then remove")
        doc2 = pyrs_yaml.parse(doc.to_yaml())
        assert doc2.to_dict() == value


class TestSortKeysProperty:
    @settings(max_examples=MAX_EXAMPLES, deadline=None)
    @given(roundtrip_safe_json)
    def test_sort_keys_idempotent_and_value_preserving(self, value):
        if not isinstance(value, dict):
            return
        doc = as_doc(value)
        doc.sort_keys()
        first = doc.to_yaml()
        doc.sort_keys()
        assert doc.to_yaml() == first
        assert pyrs_yaml.parse(first).to_dict() == value
