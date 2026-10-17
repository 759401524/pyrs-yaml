"""One document, one answer: the bridges must read a standard tag as the loader does.

Issue #340. Since #335 the loader treats `!!str`/`!!int`/`!!float`/`!!bool` as stating the type
(YAML 1.2 §6.1), but `to_json`, `to_jsonc`, `to_json5` and `to_toml` resolved the scalar *text* and
ignored `meta.tag`, so the same AST answered two different questions. Measured on `main` before the
fix, for `v: !!str 1.20`: the loader said `{'v': '1.20'}` while `to_json` emitted `{"v": 1.2}` and
`to_toml` emitted `v = 1.2` - the type changed *and* the trailing zero the document wrote on purpose
was gone.

Each case below is asserted against the loader, which is the reference the bridges have to agree
with; the two reference libraries were already measured on these texts in `test_standard_tags.py`.
"""

import json

import pytest

import pyrs_yaml

# (document, what the loader says it is) - the pairs the bridges must reproduce.
CASES = [
    ("str-over-number", "v: !!str 1.20\n", {"v": "1.20"}),
    ("str-over-bool", "v: !!str yes\n", {"v": "yes"}),
    ("bool-legacy", "v: !!bool yes\n", {"v": True}),
    ("bool-off", "v: !!bool off\n", {"v": False}),
    ("int-binary", "v: !!int 0b101\n", {"v": 5}),
    ("int-hex", "v: !!int 0x1F\n", {"v": 31}),
    ("int-legacy-octal", "v: !!int 010\n", {"v": 8}),
    ("int-underscore", "v: !!int 1_000\n", {"v": 1000}),
    ("float-from-int-text", "v: !!float 1\n", {"v": 1.0}),
    ("null-word", "v: !!null ~\n", {"v": None}),
    ("verbatim-uri", "v: !<tag:yaml.org,2002:str> 1.20\n", {"v": "1.20"}),
    # The non-specific tag is not a standard tag, but for a scalar it states the same thing:
    # YAML 1.2 6.8 leaves the node's tag as `!` and 10.5.2 restricts application resolution
    # rules to the `?` tag, so the scalar is its own text (spec Example 6.28, suite "S4JQ").
    ("non-specific-over-number", "v: ! 1.20\n", {"v": "1.20"}),
    ("non-specific-over-bool", "v: ! yes\n", {"v": "yes"}),
    ("non-specific-over-null", "v: ! ~\n", {"v": "~"}),
]


def _scalar_of(payload):
    return next(iter(payload.values()))


class TestJsonBridgesAgreeWithTheLoader:
    @pytest.mark.parametrize(("case", "text", "want"), CASES, ids=[c[0] for c in CASES])
    def test_to_json_projects_the_tagged_type(self, case, text, want):
        emitted = json.loads(pyrs_yaml.parse(text).to_json())
        assert emitted == want, f"{case}: to_json said {emitted!r}, the loader says {want!r}"

    @pytest.mark.parametrize(("case", "text", "want"), CASES, ids=[c[0] for c in CASES])
    def test_to_jsonc_and_json5_agree(self, case, text, want):
        doc = pyrs_yaml.parse(text)
        assert json.loads(doc.to_jsonc()) == want, case
        assert json.loads(doc.to_json5()) == want, case

    def test_a_str_tag_outranks_the_number_spelling_heuristic(self):
        """`1e3` passes through verbatim as a number when untagged (PR #121 fidelity); tagged
        `!!str` it is text, and a spelling heuristic must not win over the document."""
        assert json.loads(pyrs_yaml.parse("v: !!str 1e3\n").to_json()) == {"v": "1e3"}
        assert json.loads(pyrs_yaml.parse("v: 1e3\n").to_json()) == {"v": 1000.0}


class TestTomlBridgeAgreesWithTheLoader:
    @pytest.mark.parametrize(
        ("case", "text", "want"),
        [
            ("str-over-number", "v: !!str 1.20\n", 'v = "1.20"\n'),
            ("str-over-bool", "v: !!str yes\n", 'v = "yes"\n'),
            ("bool-legacy", "v: !!bool yes\n", "v = true\n"),
            ("int-binary", "v: !!int 0b101\n", "v = 0b101\n"),
            ("int-hex", "v: !!int 0x1F\n", "v = 0x1F\n"),
        ],
        ids=["str-over-number", "str-over-bool", "bool-legacy", "int-binary", "int-hex"],
    )
    def test_to_toml_projects_the_tagged_type(self, case, text, want):
        assert pyrs_yaml.to_toml(text) == want, case

    def test_a_tagged_binary_literal_keeps_its_spelling_and_its_type(self):
        """Tagged `!!int`, the binary text is a legal TOML literal *and* an integer, so the existing
        fidelity pass-through can finally apply: untagged, YAML Core reads `0b101` as a string and
        TOML receives it quoted."""
        emitted = pyrs_yaml.to_toml("v: !!int 0b101\n")
        assert emitted == "v = 0b101\n"
        assert pyrs_yaml.load_toml(emitted) == {"v": 5}


class TestRefusalRatherThanReTyping:
    """A tag the text cannot satisfy has no honest projection.

    Resolving `!!int hello` back to a string is how a bridge ends up agreeing with no reference
    implementation; the loader raises `YamlTypeError` for the same input, and #328 set the precedent
    that a strict format refuses what it cannot spell.
    """

    @pytest.mark.parametrize("text", ["v: !!int hello\n", "v: !!float banana\n", "v: !!bool maybe\n"])
    def test_json_refuses_a_tag_the_text_cannot_satisfy(self, text):
        with pytest.raises(pyrs_yaml.YamlSerializeError) as caught:
            pyrs_yaml.parse(text).to_json()
        assert "tag-text-mismatch" in str(caught.value), str(caught.value)

    def test_toml_refuses_it_too(self):
        with pytest.raises(pyrs_yaml.YamlSerializeError) as caught:
            pyrs_yaml.to_toml("v: !!int hello\n")
        assert "tag-text-mismatch" in str(caught.value), str(caught.value)


class TestWhatIsDeliberatelyNotChanged:
    """The boundaries this change leaves where they were, asserted so they stay that way."""

    def test_a_local_tag_is_still_the_plugins_address(self):
        """`!int` is an application tag, not YAML's, so nothing may read it as one.

        Both sides fall back to the implicit resolution - the lenient policy #335 chose, because a
        local tag is how the plugin system is addressed - and the value that falls out is the
        text's own type. Asserted as a relation as well as a value: a bridge must not drift from
        the loader here in either direction.
        """
        text = "v: !int 7\n"
        assert json.loads(pyrs_yaml.parse(text).to_json()) == {"v": 7}
        assert json.loads(pyrs_yaml.parse(text).to_json()) == pyrs_yaml.safe_load(text)

    def test_an_unknown_standard_suffix_is_left_alone(self):
        text = "v: !!weird 7\n"
        assert json.loads(pyrs_yaml.parse(text).to_json()) == {"v": 7}
        assert json.loads(pyrs_yaml.parse(text).to_json()) == pyrs_yaml.safe_load(text)

    def test_binary_still_projects_as_its_base64_text(self):
        """JSON and TOML have no byte-string type, so what `!!binary` should become is an open
        mapping decision recorded in #340 - not a type-resolution question, and not something this
        fix changes silently."""
        assert json.loads(pyrs_yaml.parse("v: !!binary aGk=\n").to_json()) == {"v": "aGk="}
        assert _scalar_of(pyrs_yaml.safe_load("v: !!binary aGk=\n")) == b"hi"

    def test_untagged_documents_are_untouched(self):
        assert json.loads(pyrs_yaml.parse("v: 1.20\n").to_json()) == {"v": 1.2}
        assert json.loads(pyrs_yaml.parse("v: 0x1F\n").to_json()) == {"v": 31}
        assert json.loads(pyrs_yaml.parse("v: yes\n").to_json()) == {"v": "yes"}


class TestTheNonSpecificTagIsReadTheSameEverywhere:
    """The loader side of the rule the bridges above have to reproduce.

    Suite case `S4JQ` / YAML 1.2 spec Example 6.28 is the corpus claim: `! 12` is the string
    `"12"`. PyYAML and ruamel both resolve a `!`-tagged plain scalar implicitly and answer `12`,
    so both readings are recorded rather than reconciled by silence - ruamel has an open ticket
    for exactly this (#573). The divergence is toward the specification, and the price of taking
    it is stated in the pull request rather than discovered by a user.
    """

    def test_a_bang_is_text_for_a_value_and_for_a_key(self):
        assert pyrs_yaml.safe_load("- ! 12\n") == ["12"]
        assert pyrs_yaml.safe_load("v: ! 12\n") == {"v": "12"}
        assert pyrs_yaml.safe_load("! 12: a\n") == {"12": "a"}

    def test_a_local_tag_with_a_suffix_is_still_the_authors_own(self):
        """`!foo` names an application tag, not the non-specific one: nothing about it states a
        type, so the schema resolves its text as it would for an untagged scalar."""
        assert pyrs_yaml.safe_load("- !foo 12\n") == [12]
        assert pyrs_yaml.safe_load("- ! 12\n") == ["12"]

    def test_the_reading_survives_a_round_trip(self):
        src = "v: ! 1.20\n"
        doc = pyrs_yaml.parse(src)
        assert doc.to_yaml() == src
        assert pyrs_yaml.safe_load(doc.to_yaml()) == {"v": "1.20"}
        assert json.loads(doc.to_json()) == {"v": "1.20"}

    def test_pyyaml_still_resolves_the_non_specific_tag_implicitly(self):
        yaml = pytest.importorskip("yaml")
        assert yaml.safe_load("- ! 12\n") == [12]
        assert pyrs_yaml.safe_load("- ! 12\n") == ["12"]
