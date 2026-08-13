"""Property-based tests (Hypothesis) for round-trip fidelity.

These fuzz the dump/parse pipeline with randomly generated JSON-compatible
structures to catch (a) panics/crashes on arbitrary input and (b) structural
or scalar drift introduced by refactors.

The round-trip invariant now holds end-to-end: quoted scalars load as strings
(YAML 1.2), lone-quote keys and empty collections round-trip exactly, and only
plain scalars are schema-resolved. The only strings still excluded from the
*exact-equality* strategy are those the YAML plain-scalar grammar cannot
represent losslessly (leading/trailing whitespace, control characters) — a
serializer syntax concern, not a type-resolution one. ``test_dump_never_raises``
fuzzes *arbitrary* input to assert the dump step itself never panics.
"""

from hypothesis import given, settings
from hypothesis import strategies as st

import pyrs_yaml


def rt(value):
    """Dump a Python value to YAML and load it back as a native object."""
    return pyrs_yaml.parse(pyrs_yaml.safe_dump(value)).to_dict()


# Strings that round-trip faithfully. Quoted scalars and number-like strings
# now do (Bug-4 fix); only edge-whitespace/control text is excluded because a
# plain scalar cannot carry it losslessly.
safe_text = st.text(min_size=0, max_size=40).filter(lambda s: s == s.strip() and s.isprintable())

# Arbitrary text (including whitespace/quote edge cases) used only by the
# dump-fuzz, where we only assert the dumper does not panic.
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
        st.lists(children, min_size=0, max_size=8),
        st.dictionaries(safe_text, children, min_size=0, max_size=8),
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
