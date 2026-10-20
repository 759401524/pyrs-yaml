"""Pin that an anchor the duplicate-key fold removes still reaches the emitted document.

libFuzzer `edit_yaml` crash-68617eaf (34 bytes). `: abase: &x 1\nuse: *x\n:` is accepted, and the
third line's empty key duplicates the first, so the fold drops the entry whose *value subtree*
defines `&x` while `use: *x` survives. The read path used to hand the repair pass only the
dropped entry's own anchor, so the name lost its definition and the emission was `use: *x` —
text this library's own reader refuses ("found unknown anchor"). It now hands over every
definition leaving the tree, so the use is inlined and the output reads back.
"""

import pyrs_yaml

SOURCE = ": abase: &x 1\nuse: *x\n:"


def test_a_folded_away_anchor_definition_still_reaches_the_output():
    emitted = pyrs_yaml.parse(SOURCE).to_yaml()
    assert "*x" not in emitted, f"an alias still names a definition the tree lost: {emitted!r}"
    again = pyrs_yaml.parse(emitted)
    assert again.to_yaml() == emitted, f"not a fixed point in one round: {emitted!r} -> {again.to_yaml()!r}"
