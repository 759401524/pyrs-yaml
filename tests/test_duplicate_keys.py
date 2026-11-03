"""Duplicate key tests — YamlDuplicateKeyError and allow_duplicate_keys override."""

import pytest
from hypothesis import given, settings
from hypothesis import strategies as st

import pyrs_yaml
from tests.data import yaml_samples as yaml


class TestDuplicateKeys:
    """Test duplicate key detection and override."""

    def test_duplicate_keys_default_false_raises_error(self):
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError):
            pyrs_yaml.YAML().parse(yaml.DUPLICATE_KEYS_OVERRIDE)

    def test_duplicate_keys_true_allows_override(self):
        doc = pyrs_yaml.YAML(allow_duplicate_keys=True).parse(yaml.DUPLICATE_KEYS_OVERRIDE)
        assert doc.get("key") == "second"

    def test_top_level_parse_allow_duplicate_keys(self):
        doc = pyrs_yaml.parse(yaml.DUPLICATE_KEYS_OVERRIDE, allow_duplicate_keys=True)
        assert doc.get("key") == "second"

    def test_top_level_parse_raises_by_default(self):
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError):
            pyrs_yaml.parse(yaml.DUPLICATE_KEYS_OVERRIDE)

    def test_safe_load_allow_duplicate_keys_last_wins(self):
        data = pyrs_yaml.safe_load(yaml.DUPLICATE_KEYS_OVERRIDE, allow_duplicate_keys=True)
        assert data["key"] == "second"

    def test_safe_load_raises_by_default(self):
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError):
            pyrs_yaml.safe_load(yaml.DUPLICATE_KEYS_OVERRIDE)

    def test_roundtrip_keeps_last_occurrence(self):
        doc = pyrs_yaml.YAML(typ="rt", allow_duplicate_keys=True).parse(yaml.DUPLICATE_KEYS_OVERRIDE)
        output = doc.to_yaml()
        assert "second" in output
        assert "first" not in output

    def test_nested_duplicate_keys(self):
        nested = "outer:\n  dup: a\n  dup: b"
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError):
            pyrs_yaml.parse(nested)
        doc = pyrs_yaml.parse(nested, allow_duplicate_keys=True)
        assert doc.get("outer")["dup"] == "b"

    def test_null_keys_are_not_duplicate(self):
        # yaml-test-suite 2JQS: duplicate empty/null keys are valid
        doc = pyrs_yaml.parse(": a\n: b")
        assert doc.get("~") == "b"
        doc = pyrs_yaml.parse("~: a\n~: b")
        assert doc.get("~") == "b"
        doc = pyrs_yaml.parse("null: a\nnull: b")
        assert doc.get("null") == "b"

    def test_null_key_then_real_duplicate_still_errors(self):
        yaml_text = "~: a\nkey: first\nkey: second"
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError):
            pyrs_yaml.parse(yaml_text)

    def test_parse_all_docs_duplicate_keys(self):
        multi = "---\nkey: first\nkey: second\n---\nother: value"
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError):
            pyrs_yaml.parse_all_docs(multi)
        docs = pyrs_yaml.parse_all_docs(multi, allow_duplicate_keys=True)
        assert len(docs) == 2
        assert docs[0].get("key") == "second"

    def test_safe_loads_duplicate_keys(self):
        multi = "---\nkey: first\nkey: second\n---\nother: value"
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError):
            pyrs_yaml.safe_loads(multi)
        docs = pyrs_yaml.safe_loads(multi, allow_duplicate_keys=True)
        assert docs[0]["key"] == "second"


SCALAR = "a: 1\na: 2\n"
NULL = ": 1\n: 2\n"
TAGGED = "0set: 1\n!base 0set: 2\n"
QUOTED = "'a': 1\na: 2\n"
POLICIES = ("as_is", "error", "last_wins", "first_wins")


class TestTheDuplicateKeysPolicy:
    """`duplicate_keys` names what the boolean could only half ask for.

    Measured against the shipped object on four shapes. `as_is` is the default and gives each shape
    the answer it has always got, which is why the flag adds no migration: the reader's own
    behaviour differs by identity class, and a single default value could not have described it.
    The tag-distinct pair is not a duplicate under any policy - YAML counts two keys there - so
    what that document still refuses is `to_dict()`, not the read.
    """

    def test_scalar_and_quoted_collisions_are_refused_by_default(self):
        for text in (SCALAR, QUOTED):
            with pytest.raises(pyrs_yaml.YamlDuplicateKeyError):
                pyrs_yaml.parse(text, duplicate_keys="as_is")

    def test_the_null_class_folds_to_the_last_pair_by_default(self):
        doc = pyrs_yaml.parse(NULL, duplicate_keys="as_is")
        assert doc.to_yaml() == "~: 2\n"

    def test_a_tag_tells_two_keys_apart_under_every_policy(self):
        for policy in POLICIES:
            doc = pyrs_yaml.parse(TAGGED, duplicate_keys=policy)
            assert doc.to_yaml() == TAGGED, policy
            with pytest.raises(pyrs_yaml.YamlTypeError):
                doc.to_dict()

    def test_error_refuses_the_class_the_default_exempts(self):
        with pytest.raises(pyrs_yaml.YamlDuplicateKeyError, match="~"):
            pyrs_yaml.parse(NULL, duplicate_keys="error")

    def test_last_wins_agrees_with_the_legacy_boolean(self):
        for text in (SCALAR, NULL, QUOTED):
            named = pyrs_yaml.parse(text, duplicate_keys="last_wins")
            legacy = pyrs_yaml.parse(text, allow_duplicate_keys=True)
            assert named.to_yaml() == legacy.to_yaml(), text

    def test_first_wins_keeps_the_pair_the_document_said_first(self):
        assert pyrs_yaml.parse(SCALAR, duplicate_keys="first_wins").to_yaml() == "a: 1\n"
        assert pyrs_yaml.parse(NULL, duplicate_keys="first_wins").to_yaml() == "~: 1\n"
        assert pyrs_yaml.parse(QUOTED, duplicate_keys="first_wins").to_yaml() == "'a': 1\n"

    def test_first_wins_carries_the_note_of_the_pair_it_folds_away(self):
        doc = pyrs_yaml.parse("a: 1\n# carried\na: 2\n", duplicate_keys="first_wins")
        assert "# carried" in doc.to_yaml(), doc.to_yaml()

    def test_both_readers_answer_alike_for_every_policy(self):
        # safe_load takes the direct route; parse answers through the AST. The two used to
        # disagree on adjacent key pairs, so this flag has to land on both sides at once.
        for policy in POLICIES:
            for text in (SCALAR, NULL, TAGGED):
                fast = _maybe(pyrs_yaml.safe_load, text, policy)
                slow = _maybe(_ast_dict, text, policy)
                assert fast == slow, (policy, text)

    def test_passing_both_spellings_of_one_choice_is_its_own_refusal(self):
        with pytest.raises(pyrs_yaml.YamlParseError, match="pass one"):
            pyrs_yaml.parse(SCALAR, allow_duplicate_keys=True, duplicate_keys="first_wins")

    def test_an_unrecognised_value_names_the_values_it_means(self):
        with pytest.raises(pyrs_yaml.YamlParseError, match="unknown duplicate_keys"):
            pyrs_yaml.parse(SCALAR, duplicate_keys="keep_everything")

    def test_positional_callers_keep_their_meaning(self):
        # (yaml, resolve_merges, schema, max_depth, allow_duplicate_keys) is the old spelling; the
        # new parameter is appended, so nothing shifts.
        doc = pyrs_yaml.parse(SCALAR, True, None, 1000, True)
        assert doc.to_dict() == {"a": 2}


def _ast_dict(text, duplicate_keys):
    """The projection the AST route answers with, refusals included."""
    return pyrs_yaml.parse(text, duplicate_keys=duplicate_keys).to_dict()


def _maybe(call, text, duplicate_keys):
    """The answer, or the type name of the refusal, so two routes can be compared either way."""
    try:
        return call(text, duplicate_keys=duplicate_keys)
    except Exception as exc:
        return type(exc).__name__


MAX_EXAMPLES = 75

# A plain key, always prefixed so it cannot resolve as a null, bool or number, and two distinct
# values: the smallest document shape every policy has to answer.
_KEYS = st.text(alphabet="abcdefghijklmnopqrstuvwxyz", min_size=1, max_size=5)
_PAIR = st.lists(st.integers(0, 9), min_size=2, max_size=2, unique=True)


class TestThePoliciesAreOneAnotherReversed:
    """`first_wins` is `last_wins` read from the other end, and what a policy keeps it can read back.

    The fixed samples in ``TestTheDuplicateKeysPolicy`` pin the four answers on four spellings; this
    says the fold is a rule about position rather than about those samples, over generated keys and
    values, on both readers - the direct loader holds its own copy of the decision.
    """

    @settings(max_examples=MAX_EXAMPLES, deadline=None)
    @given(_KEYS, _PAIR)
    def test_swapping_the_pair_flips_one_policy_into_the_other(self, key, values):
        name = f"k{key}"
        first, second = values
        forward = f"{name}: {first}\n{name}: {second}\n"
        backward = f"{name}: {second}\n{name}: {first}\n"
        assert pyrs_yaml.parse(forward, duplicate_keys="first_wins").to_dict() == {name: first}
        assert pyrs_yaml.parse(backward, duplicate_keys="last_wins").to_dict() == {name: first}
        assert pyrs_yaml.parse(forward, duplicate_keys="last_wins").to_dict() == {name: second}
        assert pyrs_yaml.parse(backward, duplicate_keys="first_wins").to_dict() == {name: second}

    @settings(max_examples=MAX_EXAMPLES, deadline=None)
    @given(_KEYS, _PAIR)
    def test_a_folded_document_is_a_fixed_point_for_both_readers(self, key, values):
        name = f"k{key}"
        first, second = values
        text = f"{name}: {first}\n{name}: {second}\n"
        for policy in ("last_wins", "first_wins"):
            kept = pyrs_yaml.parse(text, duplicate_keys=policy)
            emitted = kept.to_yaml()
            assert pyrs_yaml.parse(emitted, duplicate_keys=policy).to_dict() == kept.to_dict(), policy
            assert pyrs_yaml.safe_load(emitted, duplicate_keys=policy) == kept.to_dict(), policy
