"""The quality ledger's `### (xy)` sections stay in letter order, and no letter repeats.

The ledger grows by insertion, and each branch anchors wherever its neighbour happened to be. Two
sections ended up behind letters that came later - `(bh)` sat after `(bi)`, `(ch)` after `(cj)` - found
by reading the file's own heading sequence rather than by chance: a reader following a
`(details: quality-ledger (ch))` pointer from a changelog entry expects the sections to be where the
letters say they are, and two orderings in one file means the letters are decoration.

Repeating a letter is the worse failure, because the citation is how any reader resolves a claim to its
evidence: with two sections called `(bf)`, a pointer names neither. Measured while writing this, no
letter in the file was repeated - and this section's own letter, `(ck)`, is why the count is not quoted
here: a test that recites the size of the file it guards goes stale the first time someone adds an
entry to it.

Gaps are *not* reported, because they are expected and there are two distinct reasons for them. A
branch can claim a letter and close by another route (`bs`, `bu`, `cd` are skipped between present
neighbours), and a wider stretch (`ak` through `an`) is simply occupied by the older form of the same
kind of entry, which uses a bold lead-in instead of a heading. A scheme with no registry produces both,
and neither makes a citation wrong.

Scope of the ordering rule is the `### (xy)` heading form only. The older entries in the same file use
a bold lead-in (`**(am) The Ir gate's headroom…**`) under family headings, and folding them in would
assert a convention the file does not have.
"""

from __future__ import annotations

import re
from collections import Counter
from pathlib import Path

LEDGER = Path(__file__).resolve().parent.parent / "docs" / "dev" / "quality-ledger.md"
SECTION = re.compile(r"^### \(([a-z]{2})\)", re.M)


def _letters() -> list[str]:
    return SECTION.findall(LEDGER.read_text(encoding="utf-8"))


def test_the_lettered_sections_run_in_order() -> None:
    letters = _letters()
    assert letters, "the heading pattern matched nothing; the ledger's section form changed"
    out_of_order = [(before, after) for before, after in zip(letters, letters[1:]) if before >= after]
    assert not out_of_order, f"{len(out_of_order)} section(s) out of letter order: " + ", ".join(
        f"({a}) follows ({b})" for b, a in out_of_order
    )


def test_no_letter_is_used_twice() -> None:
    """A citation like `(details: quality-ledger (bf))` has to name one section.

    Two sections sharing a letter makes every pointer to it ambiguous, and the ambiguity is invisible
    in the file itself - which is why this is checked separately from the order.

    Proven by mutation, both directions: putting `(ch)` back where it was found, after `(cj)`, reddens
    `test_the_lettered_sections_run_in_order` alone; renaming the last section to `(cg)` reddens this
    test *and* the order one, because a repeated letter is also a step backwards - this is the test
    whose message names the cause, which is the reason for keeping the two rules apart at all.
    Restoring the file gives 2 passed.
    """
    repeats = sorted(letter for letter, n in Counter(_letters()).items() if n > 1)
    assert not repeats, f"sections share a letter: {repeats}"
