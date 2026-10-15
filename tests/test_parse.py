"""Core YAML parsing tests — parsing only (no serialization)."""

import math

import pytest

import pyrs_yaml
from tests.data import yaml_samples as yaml


class TestBasicParsing:
    """Test basic YAML parsing functionality"""

    def test_parses_plain_string_as_scalar(self):
        assert pyrs_yaml.parse(yaml.PLAIN_STRING).root_type() == "scalar"

    def test_parses_mapping_from_string(self):
        assert pyrs_yaml.parse(yaml.SIMPLE_MAPPING).root_type() == "mapping"

    def test_parses_sequence_from_string(self):
        assert pyrs_yaml.parse(yaml.SEQUENCE).root_type() == "sequence"

    def test_parses_nested_mapping(self):
        assert pyrs_yaml.parse(yaml.NESTED_MAPPING).root_type() == "mapping"

    def test_parses_empty_value_as_none(self):
        assert pyrs_yaml.parse(yaml.EMPTY_VALUE).get("key") is None

    @pytest.mark.parametrize("variant", ["null", "Null", "NULL", "~"], ids=["null", "Null", "NULL", "tilde"])
    def test_parses_null_variant_as_none(self, variant):
        assert pyrs_yaml.parse(f"key: {variant}").get("key") is None

    @pytest.mark.parametrize("variant", ["true", "True", "TRUE"], ids=["true", "True", "TRUE"])
    def test_parses_boolean_true_variant(self, variant):
        assert pyrs_yaml.parse(f"key: {variant}").get("key") is True

    @pytest.mark.parametrize("variant", ["false", "False", "FALSE"], ids=["false", "False", "FALSE"])
    def test_parses_boolean_false_variant(self, variant):
        assert pyrs_yaml.parse(f"key: {variant}").get("key") is False

    @pytest.mark.parametrize(
        "yaml_str,key,expected",
        [
            ("key: 42", "key", 42),
            ("key: -17", "key", -17),
        ],
        ids=["positive", "negative"],
    )
    def test_parses_integer(self, yaml_str, key, expected):
        assert pyrs_yaml.parse(yaml_str).get(key) == expected

    @pytest.mark.parametrize(
        "yaml_str,key,expected",
        [
            ("key: 3.14", "key", 3.14),
            ("key: -0.5", "key", -0.5),
        ],
        ids=["positive", "negative"],
    )
    def test_parses_float(self, yaml_str, key, expected):
        assert abs(pyrs_yaml.parse(yaml_str).get(key) - expected) < 1e-10


class TestQuoteStyles:
    """Test different quote style parsing"""

    @pytest.mark.parametrize(
        "yaml_str,expected",
        [
            ("key: value", "value"),
            ("key: 'value'", "value"),
            ('key: "value"', "value"),
        ],
        ids=["plain", "single-quoted", "double-quoted"],
    )
    def test_parses_quoted_value(self, yaml_str, expected):
        assert pyrs_yaml.parse(yaml_str).get("key") == expected

    def test_parses_special_chars_in_quoted_value(self):
        assert pyrs_yaml.parse(yaml.SPECIAL_CHARS).get("key") == "value:with:colons"


class TestYaml12Types:
    """Test YAML 1.2 type resolution"""

    @pytest.mark.parametrize(
        "yaml_str,key,expected",
        [
            ("key: 0o14", "key", 12),
            ("key: 0x0C", "key", 12),
            ("key: 6.022e23", "key", 6.022e23),
        ],
        ids=["octal", "hex", "scientific"],
    )
    def test_parses_numeric_type(self, yaml_str, key, expected):
        value = pyrs_yaml.parse(yaml_str).get(key)
        if isinstance(expected, float):
            assert abs(value - expected) < 1e10
        else:
            assert value == expected

    def test_parses_infinity(self):
        assert math.isinf(pyrs_yaml.parse(yaml.INFINITY).get("key"))

    def test_parses_nan(self):
        assert math.isnan(pyrs_yaml.parse(yaml.NAN).get("key"))


class TestTags:
    """Test YAML tag parsing"""

    @pytest.mark.parametrize(
        "yaml_str,tag",
        [
            (yaml.TAG_STR, "!!str"),
            (yaml.TAG_CUSTOM, "!custom"),
            (yaml.TAG_INT, "!!int"),
            (yaml.TAG_VERBATIM, "!<tag:yaml.org,2002:str>"),
        ],
        ids=["primary", "local", "int", "verbatim"],
    )
    def test_preserves_tag_in_output(self, yaml_str, tag):
        assert tag in pyrs_yaml.parse(yaml_str).to_yaml()


class TestYamlDirective:
    """`version()` reports what the source's `%YAML` directive declared.

    Every document used to answer the reader's own version, declared or not, because the AST
    path dropped the directive granit handed over. Reporting is the whole change: the schema
    profile decides how a scalar resolves, not the directive, and the writer does not re-emit
    it, so `to_yaml()` of a declared document reads back as undeclared.
    """

    @pytest.mark.parametrize("declared", ["1.1", "1.2", "1.3"])
    def test_declared_version_is_reported(self, declared):
        doc = pyrs_yaml.parse(f"%YAML {declared}\n---\na: 1\n")
        assert doc.version() == declared

    def test_undeclared_document_keeps_the_readers_own_version(self):
        assert pyrs_yaml.parse("a: 1\n").version() == "1.2"

    def test_a_directive_does_not_change_resolution(self):
        assert pyrs_yaml.parse("%YAML 1.1\n---\nflag: yes\n").get("flag") == "yes"

    def test_each_document_of_a_stream_has_its_own_version(self):
        docs = pyrs_yaml.parse_all_docs("%YAML 1.1\n---\na: 1\n...\n%YAML 1.2\n---\nb: 2\n")
        assert [doc.version() for doc in docs] == ["1.1", "1.2"]

    def test_reparse_reports_the_directive_too(self):
        doc = pyrs_yaml.parse("%YAML 1.1\n---\na: 1\n")
        doc.reparse()
        assert doc.version() == "1.1"

    def test_the_directive_is_not_re_emitted(self):
        emitted = pyrs_yaml.parse("%YAML 1.1\n---\na: 1\n").to_yaml()
        assert emitted == "a: 1\n"
        assert pyrs_yaml.parse(emitted).version() == "1.2"

    def test_a_tag_handle_is_expanded_not_carried(self):
        emitted = pyrs_yaml.parse("%TAG !u! tag:yaml.org,2002:\n---\nu: !u!str x\n").to_yaml()
        assert emitted == "u: !!str x\n"


class TestBlockScalars:
    """Test block scalar parsing"""

    @pytest.mark.parametrize(
        "yaml_str,indicator",
        [
            (yaml.LITERAL_BLOCK, "|"),
            (yaml.FOLDED_BLOCK, ">"),
            (yaml.STRIP_BLOCK, "|-"),
            (yaml.KEEP_BLOCK, "|+"),
            (yaml.FOLDED_STRIP, ">-"),
        ],
        ids=["literal", "folded", "literal-strip", "literal-keep", "folded-strip"],
    )
    def test_preserves_block_indicator(self, yaml_str, indicator):
        assert indicator in pyrs_yaml.parse(yaml_str).to_yaml()


class TestComplexKeys:
    """Test complex key parsing"""

    @pytest.mark.parametrize(
        "yaml_str", [yaml.EXPLICIT_KEY_SEQ, yaml.EXPLICIT_KEY_MAP], ids=["sequence-key", "mapping-key"]
    )
    def test_parses_explicit_key(self, yaml_str):
        assert "?" in pyrs_yaml.parse(yaml_str).to_yaml()

    # The dict projection either carries the key or refuses it; it never invents one.
    # A sequence key gets the form ruamel.yaml produces -- measured 2026-10-14, its safe
    # loader gives `(1, 2)` for `? [1, 2]` and `(1, 'x')` for `? [1, "x"]`. Before this,
    # such a key came back as the node's Rust debug text, which re-emitted stably, so the
    # corruption looked like a round trip.

    @pytest.mark.parametrize(
        ("yaml_str", "expected"),
        [
            pytest.param("? [1, 2]\n: v\n", {(1, 2): "v"}, id="sequence-key"),
            pytest.param('? [1, "x"]\n: v\n', {(1, "x"): "v"}, id="mixed-sequence-key"),
        ],
    )
    def test_sequence_key_is_projected_as_a_tuple(self, yaml_str, expected):
        assert pyrs_yaml.safe_load(yaml_str) == expected

    @pytest.mark.parametrize(
        "yaml_str",
        [
            pytest.param("? {a: 1}\n: v\n", id="mapping-key"),
            pytest.param("? ?\n", id="empty-nested-key"),
            pytest.param("a: &x [1, 2]\n? *x\n: v\n", id="alias-key"),
        ],
    )
    def test_a_key_with_no_hashable_form_is_refused_not_replaced(self, yaml_str):
        with pytest.raises(TypeError, match="unhashable key") as raised:
            pyrs_yaml.safe_load(yaml_str)
        assert "Mapping {" not in str(raised.value)
        assert "Sequence {" not in str(raised.value)

    def test_the_refusal_does_not_point_at_a_route_that_also_refuses(self):
        # The message first shipped told the reader to project the document with
        # to_json()/to_toml(). Both raise on that same key — `json-object-key` and
        # `toml-keys-must-be-scalars`, measured here — so the advice sent the reader into two
        # more errors. Only the AST route survives, and the message now says that.
        with pytest.raises(TypeError, match="unhashable key") as raised:
            pyrs_yaml.safe_load("? {a: 1}\n: v\n")
        text = str(raised.value)
        assert "or project it with" not in text
        assert "to_json() and to_toml() refuse this key as well." in text

    def test_a_nested_sequence_key_keeps_pythons_own_refusal(self):
        # Only the outer container becomes a tuple, exactly as ruamel does it, so a nested
        # list is refused by Python rather than by a message this engine adds for it.
        with pytest.raises(TypeError, match="unhashable type: 'list'"):
            pyrs_yaml.safe_load("? [[1], [2]]\n: v\n")

    @pytest.mark.parametrize(
        "yaml_str",
        [
            pytest.param("? [1, 2]\n: v\n", id="sequence-key"),
            pytest.param("? {a: 1}\n: v\n", id="mapping-key"),
            pytest.param("? ?\n", id="empty-nested-key"),
        ],
    )
    def test_the_ast_still_holds_the_key_the_document_wrote(self, yaml_str):
        text = pyrs_yaml.parse(yaml_str).to_yaml()
        assert "?" in text
        assert pyrs_yaml.parse(text).to_yaml() == text

    def test_an_alias_key_is_written_back_instead_of_dropped_to_null(self):
        # The writer used to resolve an alias key away, so `? *x` + `: v` came back as `null: v` —
        # a different mapping, one entry shorter than the tree that produced it. The compact
        # `*x: v` is not an available alternative: this engine's reader refuses it ("simple key
        # expected ':'") and so does ruamel.yaml, which is why the marker form is what we emit,
        # in block style on the marker line and inside `{ ? *x : v }` for a flow mapping.
        block = "a: &x [1, 2]\n? *x\n: v\n"
        assert pyrs_yaml.parse(block).to_yaml() == block
        flow = "{a: &x [1, 2], ? *x : v}\n"
        assert pyrs_yaml.parse(flow).to_yaml() == flow
        # The name of the failure, spelled out: the key must not survive as a null scalar.
        assert "null:" not in pyrs_yaml.parse(block).to_yaml()

    def test_a_note_on_a_flow_member_is_written_and_not_dropped(self):
        # The flow writer used to answer a member's note with no note at all: `{A: x  # V}` read
        # into a tree that kept `V` and wrote `{A: x}`, so the note lived only in memory and
        # whoever read the text next never saw it. A flow member owns exactly one slot — the
        # line-end comment this engine's reader hands back to that member — and a `#` closes its
        # line, so the rest of the pair continues one column deeper than the bracket: at column 0 a
        # value under `- {` or `k: {` is a wrongly indented flow continuation.
        for src, expected in [
            ("{A: x  # V\n}\n", "{A: x  # V\n }\n"),
            ("{A: # K\n  x}\n", "{A: # K\n x}\n"),
            ("[x  # V\n]\n", "[x  # V\n ]\n"),
            ("{A: 1, B: x  # V\n}\n", "{A: 1, B: x  # V\n }\n"),
            ("{A: x  # U\n, B: 2}\n", "{A: x  # U\n , B: 2}\n"),
            ("k:\n  {A: x  # V\n  }\n", "k: {A: x  # V\n    }\n"),
            ("- {A: x  # V\n}\n", "- {A: x  # V\n   }\n"),
            ("{A: [x  # V\n]}\n", "{A: [x  # V\n     ]}\n"),
        ]:
            emitted = pyrs_yaml.parse(src).to_yaml()
            assert emitted == expected, src
            assert pyrs_yaml.parse(emitted).to_yaml() == expected, src

    def test_a_container_key_in_a_flow_mapping_keeps_its_marker(self):
        # `write_scalar_for_key` has nothing to say about a mapping or a sequence, so the pair came
        # back keyed by `null` — the corruption PR #410 closed for an alias key, in the sibling
        # branch of the same call. The marker form is measured readable here and by ruamel.
        for src in ["{? {} : v}\n", "{? {A: 1} : v}\n", "{? [1, 2] : v}\n"]:
            emitted = pyrs_yaml.parse(src).to_yaml()
            assert "? " in emitted, src
            assert pyrs_yaml.parse(emitted).to_yaml() == emitted, src


class TestAnchorsAliases:
    """Test anchor and alias parsing"""

    def test_parses_anchor_definition(self):
        assert "&defaults" in pyrs_yaml.parse(yaml.ANCHOR_SIMPLE).to_yaml()

    def test_resolves_alias_reference(self):
        doc = pyrs_yaml.parse(yaml.ANCHOR_MERGE)
        assert doc.get("prod")["v"] == 1

    def test_resolves_alias_with_override(self):
        prod = pyrs_yaml.parse(yaml.ANCHOR_MERGE_OVERRIDE).get("prod")
        assert prod["timeout"] == 30
        assert prod["host"] == "prod.com"


class TestComments:
    """Test comment parsing"""

    @pytest.mark.parametrize(
        "yaml_str,expected",
        [
            (yaml.COMMENT_TOP, "# This is a comment"),
            (yaml.COMMENT_INLINE, "# inline comment"),
            (yaml.COMMENT_BOTH, "# Comment"),
        ],
        ids=["standalone", "inline", "both"],
    )
    def test_preserves_comment_in_output(self, yaml_str, expected):
        assert expected in pyrs_yaml.parse(yaml_str).to_yaml()


class TestParseAllDocs:
    """Test parse_all_docs function"""

    @pytest.mark.parametrize(
        "yaml_str,expected_count,expected_values",
        [
            ("key: value", 1, {"key": "value"}),
            (yaml.MULTI_DOC_TWO, 2, {"a": 1, "b": 2}),
            ("", 0, None),
        ],
        ids=["single", "multiple", "empty"],
    )
    def test_parses_multiple_documents(self, yaml_str, expected_count, expected_values):
        docs = pyrs_yaml.parse_all_docs(yaml_str)
        assert len(docs) == expected_count
        if expected_values:
            for key, value in expected_values.items():
                assert docs[0 if key in ("a", "key") else 1].get(key) == value
