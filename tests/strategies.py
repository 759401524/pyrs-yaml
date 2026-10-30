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

# The families the capped leaves above can never draw, measured on 2026-10-30 by counting the leaves
# of 25,000 draws of the pre-widening `arbitrary_json`: 270,602 leaves, and every one of them is
# `None` (58,064), `str` (53,396), a finite `float` (53,251), `bool` (52,978) or an `int` inside
# +/- 10^12 (52,913) — no other type appears even once. The same walk over 20,000 draws of the
# widened generator finds 134,145 of its 229,798 leaves outside that set: ~10,000 of each of the ten
# new families, 22,108 integers past i64 and 10,470 non-finite floats. So the cap, not the rarity of
# a draw, is what kept whole families out of the tier.
#
# What each one does at the binding is pinned by name in `test_property_roundtrip.py`, measured there
# rather than assumed here: an integer past i64 is the string the hub calls it (digits out, `str`
# back), a tuple is the sequence the loader hands back, bytes reach `!!binary` and come back as
# bytes, the temporals and `Decimal` keep their tag, and a `set`, `frozenset`, `complex` or
# unregistered object raises the binding's own `YamlTypeError` — a typed refusal, never a crash. The
# property therefore widens from "the dump step does not panic" to "every value either reaches text
# or raises that one error type", which is the claim a fuzz tier has to make to be able to fail.
unrepresentable_leaves = st.one_of(
    st.builds(set, st.just([1, 2])),
    st.builds(frozenset, st.just([1])),
    st.just(complex(1, 2)),
    st.builds(lambda: type("Point", (), {"x": 1})()),
)

hard_leaf = st.one_of(
    st.integers(),  # unbounded: both directions cross i64, which the hub spells as a string
    st.floats(width=64),  # NaN, ±inf and -0.0 are YAML scalars this tier never reached
    # Those two branches cross i64, or land on a non-finite value, only a few times per thousand
    # leaves, so a guard that asks for them inside 400 draws reads a coin flip — measured: two
    # interpreter legs missed a different one each, 3.10 the non-finite float and 3.8 the wide int.
    # These branches give the same two shapes a cheap way in, so the guard asks the strategy and not
    # the draw.
    st.one_of(
        st.integers(min_value=2**63, max_value=2**70),
        st.integers(min_value=-(2**70), max_value=-(2**63)),
    ),
    st.sampled_from([float("nan"), float("inf"), float("-inf")]),
    st.tuples(st.integers(), st.integers()),
    st.binary(max_size=16),
    st.dates(),
    st.times(),
    st.datetimes(),
    st.decimals(allow_nan=False, allow_infinity=False, places=3),
    unrepresentable_leaves,
)

arbitrary_json = st.recursive(
    st.one_of(
        st.none(),
        st.booleans(),
        st.integers(min_value=-(10**12), max_value=10**12),
        st.floats(allow_nan=False, allow_infinity=False, width=64),
        any_text,
        hard_leaf,
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
