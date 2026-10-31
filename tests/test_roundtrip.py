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

    Alias resolution short-circuits on cycles via a path-scoped guard
    (`in_progress` in `convert.rs`), yielding None instead of runaway
    recursion. The guard is pushed only for the duration of one expansion,
    so a *repeated* (non-cyclic) reference expands fully - only a genuine
    cycle collapses to None. Forward references are rejected by the parser.
    """

    def test_self_referential_sequence(self):
        doc = pyrs_yaml.parse("&a [*a]\n")
        data = doc.to_dict()
        assert data == [[None]]

    def test_self_referential_mapping(self):
        doc = pyrs_yaml.parse("a: &x\n  self: *x\n")
        data = doc.to_dict()
        assert data == {"a": {"self": {"self": None}}}

    def test_mutual_reference_resolves_via_cycle_guard(self):
        doc = pyrs_yaml.parse("x: &x {a: 1}\ny: &y\n  x: *x\n  y: *y\n")
        data = doc.to_dict()
        # `*y` re-enters `&y` while it is still being expanded, so only that
        # one edge collapses to None; every other edge expands fully. The
        # pre-fix accumulation set stopped after a single level.
        assert data == {
            "x": {"a": 1},
            "y": {"x": {"a": 1}, "y": {"x": {"a": 1}, "y": None}},
        }

    def test_forward_anchor_reference_rejected(self):
        with pytest.raises(pyrs_yaml.YamlParseError):
            pyrs_yaml.parse("a: &a {b: *b}\n")

    def test_merge_key_cycle_does_not_recurse(self):
        doc = pyrs_yaml.parse("base: &base\n  child: *base\n")
        data = doc.to_dict()
        assert data == {"base": {"child": {"child": None}}}


class TestBlankLineFidelity:
    """An empty line between entries is layout the node model has no slot for.

    Characterization, not aspiration: `engine:entry-blank-lines` in
    `.ci/known-residuals.json`. Values, key order, anchors and notes survive; the
    blank line does not, at every position measured here, and the output is already
    a fixed point - so a second round cannot be blamed for the loss.
    """

    @pytest.mark.known_residual("engine:entry-blank-lines")
    def test_entry_blank_lines_are_dropped_and_the_round_is_a_fixed_point(self):
        positions = {
            "between two root entries": ("a: 1\n\nb: 2\n", "a: 1\nb: 2\n"),
            "between two nested keys": ("root:\n  x: 1\n\n  y: 2\n", "root:\n  x: 1\n  y: 2\n"),
            "between two sequence items": ("seq:\n  - 1\n\n  - 2\n", "seq:\n  - 1\n  - 2\n"),
            "before a standalone note": ("a: 1\n\n# note\nb: 2\n", "a: 1\n# note\nb: 2\n"),
            "two consecutive blanks": ("a: 1\n\n\nb: 2\n", "a: 1\nb: 2\n"),
            "after a block scalar": ("text: |\n  line\n\nb: 2\n", "text: |\n  line\nb: 2\n"),
            "before the first key": ("\na: 1\n", "a: 1\n"),
            "after the last key": ("a: 1\n\n", "a: 1\n"),
        }
        for label, (src, expected) in positions.items():
            out = pyrs_yaml.parse(src).to_yaml()
            assert out == expected, f"{label}: {out!r} != {expected!r}"
            assert pyrs_yaml.parse(out).to_yaml() == out, f"{label}: not a fixed point"

        # The loss is layout only: the mapping the document carries is unchanged.
        assert pyrs_yaml.safe_load("a: 1\n\nb: 2\n") == {"a": 1, "b": 2}
        # The note still lands on its own line, so the blank is not what carries it.
        assert "# note" in pyrs_yaml.parse("a: 1\n\n# note\nb: 2\n").to_yaml()
        # Editing does not preserve it either, so no shipped route keeps the line today.
        doc = pyrs_yaml.parse("a: 1\n\nb: 2\n")
        doc.set("$.a", 9)
        assert doc.to_yaml() == "a: 9\nb: 2\n"


class TestKeySpellingProjection:
    """Three of the five scalar styles share one key spelling, and the reader does not know.

    Characterization, not aspiration: `engine:block-styled-key-spelling-collides` in
    `.ci/known-residuals.json`. A block header has to end its line, so the writer spells a
    `|`- or `>`-styled key, and a bare-empty one, as the double-quoted text. A document that
    names two of them holds two entries on the read and emits one key twice on the write.
    """

    @pytest.mark.known_residual("engine:block-styled-key-spelling-collides")
    def test_two_keys_the_writer_spells_alike_emit_one_text(self):
        emitted = pyrs_yaml.parse("? !? |\n: 1\n? !?\n: 2\n").to_yaml()
        assert emitted == '!? "": 1\n!? "": 2\n'
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError):
            pyrs_yaml.parse(emitted)

    def test_the_spelling_that_survives_is_the_quote_character(self):
        # The control: where a quote is what separates the pair, both entries survive the
        # round trip - so the residual above is the key line's layout limit, not a general
        # loss of tagged keys.
        emitted = pyrs_yaml.parse("? !? ''\n: 1\n? !? \"\"\n: 2\n").to_yaml()
        assert emitted == "!? '': 1\n!? \"\": 2\n"
        assert pyrs_yaml.parse(emitted).to_yaml() == emitted
