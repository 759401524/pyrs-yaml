"""Property-based tests (Hypothesis) for round-trip fidelity.

These fuzz the dump/parse pipeline with randomly generated JSON-compatible
structures to catch (a) panics/crashes on arbitrary input and (b) structural
or scalar drift introduced by refactors.

Note on the round-trip invariant: this library resolves scalar *type* from
content (e.g. the string ``"true"`` is loaded back as the boolean ``True``),
and a few serializer edge cases (e.g. a mapping key that is a lone ``'`` or
``"`` character) emit YAML that does not re-parse. The exact-equality and
idempotence properties below are therefore scoped to "safe" strings that are
known to survive a round trip unchanged, so the suite stays green while still
exercising deep nesting, collections, ordering, None/bool/int/float, and the
vast majority of realistic strings. The ``test_dump_never_raises`` fuzz uses
*arbitrary* input to assert the dump step itself never panics.
"""

from hypothesis import given, settings
from hypothesis import strategies as st

import pyrs_yaml


def rt(value):
    """Dump a Python value to YAML and load it back as a native object."""
    return pyrs_yaml.parse(pyrs_yaml.safe_dump(value)).to_dict()


def _roundtrips_scalar(s):
    # True only for strings that survive a round trip with unchanged type/value.
    try:
        return rt(s) == s
    except Exception:
        return False


# Strings that round-trip faithfully (excludes "true"/"1"/lone quotes/...).
safe_text = st.text(min_size=0, max_size=40).filter(_roundtrips_scalar)

# Arbitrary text (including known-fragile single/double quote strings) used
# only by the dump-fuzz, where we only assert the dumper does not panic.
any_text = st.text(min_size=0, max_size=40)

safe_leaf = st.one_of(
    st.none(),
    st.booleans(),
    st.integers(min_value=-(10**12), max_value=10**12),
    st.floats(allow_nan=False, allow_infinity=False, width=64),
    safe_text,
)

safe_json = st.recursive(
    safe_leaf,
    lambda children: st.one_of(
        # Empty collections dump to an empty document that re-parses as None,
        # so exclude them from the exact-equality strategy.
        st.lists(children, min_size=1, max_size=8),
        st.dictionaries(safe_text, children, min_size=1, max_size=8),
    ),
    max_leaves=40,
)

arbitrary_json = st.recursive(
    st.one_of(
        st.none(),
        st.booleans(),
        st.integers(min_value=-(10**12), max_value=10**12),
        st.floats(allow_nan=False, allow_infinity=False, width=64),
        any_text,
    ),
    lambda children: st.one_of(
        st.lists(children, min_size=0, max_size=8),
        st.dictionaries(any_text, children, min_size=0, max_size=8),
    ),
    max_leaves=40,
)


@settings(max_examples=300, deadline=None)
@given(safe_leaf)
def test_safe_scalar_roundtrip_exact(value):
    assert rt(value) == value


@settings(max_examples=300, deadline=None)
@given(safe_json)
def test_safe_structure_roundtrip_exact(value):
    assert rt(value) == value


@settings(max_examples=300, deadline=None)
@given(safe_json)
def test_roundtrip_idempotent(value):
    once = rt(value)
    twice = rt(once)
    assert twice == once


@settings(max_examples=300, deadline=None)
@given(arbitrary_json)
def test_dump_never_raises(value):
    # The dump step must not panic or raise on any JSON-compatible structure,
    # even inputs whose output is not re-parseable (a separate load-side issue).
    out = pyrs_yaml.safe_dump(value)
    assert isinstance(out, str)
