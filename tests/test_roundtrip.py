"""Round-trip preservation tests — parse → serialize → parse identity."""

import pytest

import pyrs_yaml
from tests.data import yaml_samples as yaml


class TestRoundTrip:
    """Test perfect round-trip preservation"""

    @pytest.mark.parametrize(
        "original",
        [
            yaml.ROUNDTRIP_SIMPLE,
            yaml.ROUNDTRIP_NESTED,
            yaml.ROUNDTRIP_SEQUENCE,
            yaml.ROUNDTRIP_MIXED,
            yaml.ROUNDTRIP_EMPTY_MAP,
            yaml.ROUNDTRIP_EMPTY_SEQ,
            yaml.ROUNDTRIP_FLOW_MAP,
            yaml.ROUNDTRIP_FLOW_SEQ,
            yaml.ROUNDTRIP_FLOW_NESTED,
            yaml.ROUNDTRIP_INLINE_FLOW,
        ],
        ids=[
            "simple",
            "nested",
            "sequence",
            "mixed",
            "empty-map",
            "empty-seq",
            "flow-map",
            "flow-seq",
            "flow-nested",
            "inline-flow",
        ],
    )
    def test_roundtrips_exact_output(self, original):
        assert pyrs_yaml.parse(original).to_yaml() == original

    @pytest.mark.parametrize(
        "original",
        [
            yaml.ROUNDTRIP_COMMENT,
            yaml.ROUNDTRIP_INLINE_COMMENT,
        ],
        ids=["top-level", "inline"],
    )
    def test_roundtrips_with_comments(self, original):
        assert pyrs_yaml.parse(original).to_yaml() == original

    @pytest.mark.parametrize(
        "indicator",
        ["|-", "|", "|+", ">-", ">", ">+"],
        ids=["strip", "literal", "keep", "folded-strip", "folded", "folded-keep"],
    )
    def test_preserves_chomping_indicator(self, indicator):
        if indicator.startswith("|"):
            original = f"key: {indicator}\n  line1\n  line2\n"
        else:
            original = f"key: {indicator}\n  line1\n  line2\n"
        assert indicator in pyrs_yaml.parse(original).to_yaml()

    @pytest.mark.parametrize(
        "original,expected",
        [
            (yaml.ROUNDTRIP_ANCHOR, "&defaults"),
            (yaml.ROUNDTRIP_TAG, "!!str"),
            (yaml.ROUNDTRIP_CUSTOM_TAG, "!custom"),
            (yaml.ROUNDTRIP_MULTI_ANCHOR, "&anchor2"),
        ],
        ids=["anchor", "tag", "custom-tag", "multi-anchor"],
    )
    def test_preserves_annotation(self, original, expected):
        assert expected in pyrs_yaml.parse(original).to_yaml()

    def test_preserves_empty_values(self):
        output = pyrs_yaml.parse(yaml.ROUNDTRIP_EMPTY_KEY).to_yaml()
        assert "key1:" in output
        assert "key2: value" in output

    def test_preserves_alias_reference(self):
        doc = pyrs_yaml.parse(yaml.ROUNDTRIP_MERGE, resolve_merges=False)
        assert "*defaults" in doc.to_yaml()

    @pytest.mark.parametrize(
        "original,expected",
        [
            ("key: plain_value\n", "plain_value"),
            ("key: 'single quoted'\n", "'single quoted'"),
            ('key: "double quoted"\n', '"double quoted"'),
        ],
        ids=["plain", "single-quoted", "double-quoted"],
    )
    def test_roundtrips_scalar_style(self, original, expected):
        assert expected in pyrs_yaml.parse(original).to_yaml()

    def test_roundtrips_complex_key(self):
        output = pyrs_yaml.parse(yaml.ROUNDTRIP_EXPLICIT_KEY).to_yaml()
        assert "?" in output or "key1" in output

    def test_preserves_merge_key_unresolved(self):
        doc = pyrs_yaml.parse(yaml.ROUNDTRIP_MERGE_KEYS, resolve_merges=False)
        assert "<<" in doc.to_yaml()

    def test_resolves_merge_keys_by_default(self):
        child = pyrs_yaml.parse(yaml.ROUNDTRIP_MERGE_KEYS).get("child")
        assert child["x"] == 1
        assert child["y"] == 2


class TestAliasCycles:
    """Self-referential anchors/aliases must not recurse infinitely.

    Alias resolution short-circuits on cycles via a visited set
    (node_to_pyobject_with_anchors), yielding None instead of runaway
    recursion. Forward references are rejected by the parser.
    """

    def test_self_referential_sequence(self):
        doc = pyrs_yaml.parse("&a [*a]\n")
        data = doc.to_dict()
        assert data == [[None]]

    def test_self_referential_mapping(self):
        doc = pyrs_yaml.parse("a: &x\n  self: *x\n")
        data = doc.to_dict()
        assert data == {"a": {"self": {"self": None}}}

    def test_mutual_reference_resolves_via_visited(self):
        doc = pyrs_yaml.parse("x: &x {a: 1}\ny: &y\n  x: *x\n  y: *y\n")
        data = doc.to_dict()
        # The self-reference terminates (visited set) instead of recursing;
        # one level of expansion happens before aliases collapse to None.
        assert data == {
            "x": {"a": 1},
            "y": {"x": {"a": 1}, "y": {"x": None, "y": None}},
        }

    def test_forward_anchor_reference_rejected(self):
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.parse("a: &a {b: *b}\n")

    def test_merge_key_cycle_does_not_recurse(self):
        doc = pyrs_yaml.parse("base: &base\n  child: *base\n")
        data = doc.to_dict()
        assert data == {"base": {"child": {"child": None}}}
