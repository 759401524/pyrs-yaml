#!/usr/bin/env python3
"""Assert the quality ledger's `### (xy)` sections run in increasing letter order, with no repeat.

The letters are the ledger's citation scheme: `(details: quality-ledger (bf))` names a section only if
the sequence increases and each letter appears once. Both halves fail invisibly in the file itself,
because the ledger grows by insertion and each branch anchors wherever its neighbour happened to be -
`(bh)` was found after `(bi)`, `(ch)` after `(cj)`.

Why a hook rather than only a test: measured on 2026-10-26, a head whose only defect was a section
placed one letter early passed `prek hook set on the full tree`, the docs gates, `rust-lint` and the
Ir leg, and then reddened all twelve `test` legs (three OSes, 3.8 through 3.15) for it. The rule was
judged only by the Python tier, ten minutes and twelve runners away from the edit.

Gaps are *not* reported, because they are expected and have two distinct causes: a branch can claim a
letter and close by another route, and `ak` through `an` are occupied by the older entry form in the
same file, which uses a bold lead-in instead of a heading. A scheme with no registry produces both,
and neither makes a citation wrong. The rule's scope is the `### (xy)` heading form only for that same
reason - folding the bold lead-ins in would assert a convention the file does not have.

Usage:
    python scripts/check_ledger_order.py                    # the live ledger
    python scripts/check_ledger_order.py path [...]         # explicit, for tests

Exit code 0 = letters increase and none repeats; 1 = an out-of-order or repeated section, named; 2 =
nothing was measurable (an unreadable file, or one carrying no `### (xy)` section at all), which is
never allowed to read as a pass: a heading pattern that matches nothing would otherwise report a
clean tree forever.
"""

from __future__ import annotations

import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
DEFAULT = REPO / "docs" / "dev" / "quality-ledger.md"
SECTION = re.compile(r"^### \(([a-z]{2})\)", re.M)


def letters_in(path: pathlib.Path) -> list[str]:
    return SECTION.findall(path.read_text(encoding="utf-8"))


def violations(letters: list[str]) -> list[str]:
    """The two ways a citation stops resolving, named separately.

    A repeated letter is also a step backwards, so it appears in both lists by design: the order line
    says which section sits where, the repeat line says which letter no longer names one section.
    """
    out = [f"({after}) follows ({before})" for before, after in zip(letters, letters[1:]) if before >= after]
    repeats = sorted({letter for letter in letters if letters.count(letter) > 1})
    out += [f"({letter}) is used twice" for letter in repeats]
    return out


def main(argv: list[str]) -> int:
    paths = [pathlib.Path(arg) for arg in argv] or [DEFAULT]
    found: list[list[str]] = []
    for path in paths:
        try:
            letters = letters_in(path)
        except OSError as exc:
            print(f"nothing measured: cannot read {path} ({exc})", file=sys.stderr)
            return 2
        if not letters:
            print(f"nothing measured: {path} has no '### (xy)' section", file=sys.stderr)
            return 2
        found.append(letters)

    problems = [problem for letters in found for problem in violations(letters)]
    if problems:
        print(f"{len(problems)} section(s) break the letter order: " + ", ".join(problems))
        return 1
    print(f"OK: {sum(len(letters) for letters in found)} lettered section(s) read in order across {len(paths)} file(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
