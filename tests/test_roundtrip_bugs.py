"""Regression tests for the four round-trip bugs surfaced by the property
tests (previously documented as known limitations in test_property_roundtrip).

- Bug 1: lone quote characters as keys broke round-trip.
- Bug 2: empty collections dumped to an empty document (re-parsed as None).
- Bug 3: get() heuristics made dotted/bracket keys unreachable as literal keys.
- Bug 4: quoted scalars were schema-resolved instead of staying strings
  (YAML 1.2: only plain scalars are resolved).
"""

import pytest

import pyrs_yaml


class TestLoneQuoteKeys:
    def test_lone_single_quote_key_roundtrips(self):
        data = {"'": None}
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump(data)) == data

    def test_lone_double_quote_key_roundtrips(self):
        data = {'"': 1}
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump(data)) == data

    def test_document_roundtrip(self):
        doc = pyrs_yaml.parse("'': 1\n")
        assert doc.to_dict() == {"": 1}


class TestEmptyCollections:
    def test_empty_list_roundtrips(self):
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump([])) == []

    def test_empty_dict_roundtrips(self):
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump({})) == {}

    def test_nested_empty_collections(self):
        data = {"a": [], "b": {}}
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump(data)) == data

    def test_document_empty_collections(self):
        doc = pyrs_yaml.parse("a:\n  b: []\n")
        assert doc.to_dict() == {"a": {"b": []}}

    def test_from_dict_empty(self):
        assert pyrs_yaml.safe_load(pyrs_yaml.from_dict({})) == {}
        assert pyrs_yaml.safe_load(pyrs_yaml.from_dict([])) == []


class TestGetLiteralKey:
    def test_setitem_then_get_literal_key(self):
        doc = pyrs_yaml.parse("c[0]: 99\n")
        doc["c[0]"] = 100
        assert doc.get("c[0]") == 100
        assert doc.to_dict() == {"c[0]": 100}

    def test_dotted_key_reachable(self):
        doc = pyrs_yaml.parse("a.b: 1\n")
        assert doc.get("a.b") == 1

    def test_bracket_key_reachable(self):
        doc = pyrs_yaml.parse("c[0]: 2\n")
        assert doc.get("c[0]") == 2

    def test_path_access_via_find(self):
        doc = pyrs_yaml.parse("arr:\n  - 1\n  - 2\n")
        assert doc.find("$.arr[1]").value == 2
        with pytest.raises(IndexError):
            _ = doc.find("$.arr[9]").value


class TestQuotedScalarIsString:
    def test_safe_dump_true_roundtrips_as_string(self):
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump("true")) == "true"

    def test_quoted_number_stays_string(self):
        assert pyrs_yaml.safe_load('"42"') == "42"
        assert pyrs_yaml.safe_load("'42'") == "42"

    def test_quoted_bool_and_null_stay_strings(self):
        assert pyrs_yaml.safe_load('"true"') == "true"
        assert pyrs_yaml.safe_load('"null"') == "null"

    def test_plain_negative_roundtrips_as_int(self):
        doc = pyrs_yaml.parse("x: -1\n")
        assert doc.to_dict() == {"x": -1}
        assert pyrs_yaml.parse(doc.to_yaml()).to_dict() == {"x": -1}

    def test_plain_bool_roundtrips(self):
        doc = pyrs_yaml.parse("x: true\n")
        assert doc.to_dict() == {"x": True}
        assert pyrs_yaml.parse(doc.to_yaml()).to_dict() == {"x": True}


class TestLongPlainScalarWrap:
    """Long plain scalars must not be folded at a mid-token break (the fold
    would re-parse as an inserted space and corrupt the value)."""

    def test_long_token_without_spaces_roundtrips(self):
        val = "x" * 200
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump(val)) == val

    def test_long_token_with_nbsp_roundtrips(self):
        # Original CI regression: a long string containing only NBSP as
        # whitespace used to be wrapped mid-token, corrupting the value.
        val = "a" * 40 + "\xa0" * 15 + "b" * 120
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump(val)) == val

    def test_long_value_inside_nested_sequence(self):
        val = [["00000000000000" + "\xa0" * 14 + "\u0800" * 3 + "\U00010000" * 7]]
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump(val)) == val

    def test_short_tokens_still_wrap_losslessly(self):
        # Multi-token strings wrap at spaces and fold back faithfully.
        val = " ".join(["word"] * 40)
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump(val)) == val


class TestBackslashInQuotedScalar:
    """granit-parser mishandles `\\<escape>` inside double-quoted scalars, so
    values containing a backslash must be emitted single-quoted."""

    def test_backslash_zero_value(self):
        val = "'\\0"
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump(val)) == val

    def test_backslash_followed_by_escape_letters(self):
        for tail in ("0", "x", "t", "n", '"', "q"):
            val = "a\\" + tail + "b"
            assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump(val)) == val, tail

    def test_backslash_value_in_mapping(self):
        val = {"k": "a\\0b"}
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump(val)) == val

    def test_backslash_properties_string(self):
        val = r"C:\Program Files\app\0\temp"
        assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump(val)) == val


class TestNestedBlockScalarIndent:
    """Regression: a nested block scalar's body used to be indented from a
    fixed single step instead of its parent line, so `to_yaml` emitted text
    whose body lines dedented below the header (`a:\n  b: |\n  x`) — every
    nested literal/folded shape re-parsed as an error or a wrong value.
    """

    @pytest.mark.parametrize(
        "src",
        [
            "b: |\n  x\n  y\n",  # top level (worked before — control)
            "a:\n  b: |\n    x\n    y\n",  # nested pair
            "a:\n  b:\n    c: |\n      x\n      y\n",  # two levels deep
            "- |\n  x\n  y\n",  # sequence item
            "a:\n  - |\n    x\n    y\n",  # nested sequence item
            "a:\n  - k: |\n      x\n      y\n",  # compact dash mapping
        ],
    )
    def test_block_scalar_roundtrip_preserves_text_and_value(self, src: str):
        loaded = pyrs_yaml.safe_load(src)
        out = pyrs_yaml.parse(src).to_yaml()
        # Style-preserving serialization must reproduce the source exactly…
        assert out == src
        # …and at minimum re-parse back to the same data.
        assert pyrs_yaml.safe_load(out) == loaded

    def test_folded_nested_roundtrips_by_value(self):
        # `>` folds `x\ny` into the value "x y\n"; re-serializing may
        # re-wrap the folded body differently while carrying the same value.
        src = "a:\n  b: >\n    x\n    y\n"
        loaded = pyrs_yaml.safe_load(src)
        out = pyrs_yaml.parse(src).to_yaml()
        assert pyrs_yaml.safe_load(out) == loaded

    def test_flow_block_scalar_demoted_to_quoted(self):
        # A literal scalar inside a flow container has no legal home (the
        # body needs its own indented region); it demotes to a quoted
        # scalar instead of emitting unparseable text.
        doc = pyrs_yaml.parse("a: [x]\n")
        pyrs_yaml.Node(doc).find("$.a[0]").set_scalar_style("literal")
        text = doc.to_yaml()
        assert pyrs_yaml.safe_load(text) == {"a": ["x"]}

    def test_edge_space_plain_scalars_are_quoted(self):
        # Plain tokens with edge whitespace are stripped on re-parse; the
        # serializer must quote them so the value survives.
        for val in [" ", " %", "x ", "\t"]:
            assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump({"k": val})) == {"k": val}

    def test_flow_comma_plain_scalars_are_quoted(self):
        # `,` `[` `]` `{` `}` end a plain token inside a flow collection;
        # embedding one must not corrupt the flow structure.
        for val in ["a,b", "x[", "y]"]:
            assert pyrs_yaml.safe_load(pyrs_yaml.safe_dump([val])) == [val]
