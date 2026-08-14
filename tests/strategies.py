"""Shared Hypothesis strategies for round-trip property tests.

Extracted from ``test_property_roundtrip.py`` so that other test modules
(``test_edit``, ``test_node_api``, ``test_fidelity``) can also generate
random JSON-compatible structures and verify round-trip stability.
"""

from hypothesis import strategies as st

import pyrs_yaml

# ── round-trip safe strategies ───────────────────────────────────────────────


def rt(value):
    """Dump a Python value to YAML and load it back as a native object."""
    return pyrs_yaml.parse(pyrs_yaml.safe_dump(value)).to_dict()


# Strings that survive a round trip with unchanged value.  Quoted scalars and
# number-like strings now round-trip (Bug-4 fix); edge-whitespace and control
# characters are excluded because a plain scalar cannot carry them losslessly.
roundtrip_safe_text = st.text(
    alphabet=st.characters(blacklist_categories=("Cc", "Cs", "Cn")),
    min_size=0,
    max_size=40,
).filter(lambda s: s == s.strip())

roundtrip_safe_leaf = st.one_of(
    st.none(),
    st.booleans(),
    st.integers(min_value=-(10**12), max_value=10**12),
    st.floats(allow_nan=False, allow_infinity=False, width=64),
    roundtrip_safe_text,
)

roundtrip_safe_json = st.recursive(
    roundtrip_safe_leaf,
    lambda children: st.one_of(
        st.lists(children, min_size=0, max_size=8),
        st.dictionaries(roundtrip_safe_text, children, min_size=0, max_size=8),
    ),
    max_leaves=40,
)


# ── arbitrary strategies (no exact-equality guarantee) ───────────────────────

# Arbitrary text (including whitespace/quote edge cases) used by the dump-fuzz
# where we only assert the dumper does not panic.
any_text = st.text(min_size=0, max_size=40)

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
