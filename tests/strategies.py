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
        # `<<` is reserved merge-extension syntax: with a mapping value the
        # round-trip hub consumes it by design (PR #187 contract), so it
        # cannot appear as a random key in exact-equality domains.
        st.dictionaries(roundtrip_safe_text.filter(lambda k: k != "<<"), children, min_size=0, max_size=8),
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
        # Same `<<` merge-extension carve-out as roundtrip_safe_json above.
        st.dictionaries(any_text.filter(lambda k: k != "<<"), children, min_size=0, max_size=8),
    ),
    max_leaves=40,
)


# ── dialect grammar fuzz ──────────────────────────────────────────────────────
# Random byte/text mostly trips the loaders' "expected a value" early exit and
# never reaches the interesting grammar. `dialect_text` assembles real JSONC /
# JSON5 / TOML tokens -- comments, trailing commas, unquoted keys, hex /
# leading-dot / Infinity / NaN spellings, leading zeros, underscores, invalid
# UTF-8 (lone surrogate, NUL, NBSP, lone CR), unterminated strings -- so the
# fuzzer exercises the exact constructs the objective calls out (JSON5 trailing
# comma / comment / unquoted key, huge numbers, illegal UTF-8) instead of
# floundering on the first byte. The invariant these feed into is pure
# no-panic: every loader/serializer must return a value or raise a typed
# parse/serialize error, never abort the process.
_DIALECT_FRAGMENTS = st.sampled_from(
    [
        "// line\n",
        "/* block */",
        "# toml note\n",
        "{",
        "}",
        "[",
        "]",
        "(",
        ")",
        ",",
        ":",
        "=",
        ".",
        "e",
        "E",
        "'",
        '"',
        "\\",
        "\\n",
        "\\u",
        "\\u00",
        "\\ud800",
        "0x1F",
        "0XFF",
        "0o755",
        "0b101",
        "+.5",
        "5.",
        ".5",
        "Infinity",
        "-Infinity",
        "NaN",
        "007",
        "-01",
        "1_0",
        "1__0",
        "1_",
        "true",
        "false",
        "null",
        "nil",
        "key",
        "a.b.c",
        '"k"',
        "1e999",
        "1e-999",
        "-0",
        "99999999999999999999999999",
        "\t",
        "\r\n",
        "\x00",
        "\u00a0",
        "\u2028",
        "\ufeff",
        " ",
        "\n",
    ]
)
dialect_text = st.lists(_DIALECT_FRAGMENTS, min_size=0, max_size=28).map("".join)
