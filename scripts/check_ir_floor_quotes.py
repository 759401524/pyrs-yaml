#!/usr/bin/env python3
"""Assert that a floor reading quoted as current in `QUALITY_MATRIX.md` equals the committed baseline.

Why this exists, named: the Ir baseline refresh rewrites all forty-seven numbers in
`.ci/ir-baseline.json`, and every copy of one of those numbers in undated prose becomes a historical
fact that nobody marked as history. Measured on 2026-10-26, nine minutes after a refresh landed:
`QUALITY_MATRIX.md` quoted nine floors, and nine of nine disagreed with the file it describes —
`serialize_note_flow` off by 15 instructions, `documents_short_strings` and `documents_long_strings`
off by 31,977 each, four `load_*`/`from_*_notes` readings off by hundreds to thousands. Only the two
numbers the refresh job's own PR had just re-pointed were right, because a human touched exactly
those. Nothing read the other nine.

The rule is a declaration, not a parser. A number quoted as *current* carries the scenario it comes
from immediately after it:

    `serialize_note_flow` 62,066,033<!-- ir-floor: serialize_note_flow -->

and this checker re-reads `.ci/ir-baseline.json` and compares. A reading with no marker is not
claimed to be current, which is what makes the sentence about a past measurement legal in the same
page: prose says "measured +1.44% with the unpinned pair", "28 instructions out of 16,020,906",
"event counts", `ITERATIONS = 2_000`, fixture widths. An earlier attempt found the same shape by
scanning for a scenario name near a number and produced ~110 hits in `docs/dev/perf.md`, nearly all
of them legitimate history — which is why the author declares instead of the checker guessing.

Three things this refuses, and one it cannot see:

- a marker naming a scenario the baseline does not contain (a rename or a typo);
- a marker whose adjacent digits differ from that scenario's committed floor (stale by any amount);
- a marker that is not adjacent to a grouped number at all (the declaration promises a reading).
- it cannot see an *unmarked* hand-copy of a live floor: absence of a marker reads as "this is
  history", which is the author's claim to make and the residual this gate leaves open.

Usage:
    python scripts/check_ir_floor_quotes.py                # the governed pages
    python scripts/check_ir_floor_quotes.py path [...]     # explicit, for tests

Exit code 0 = every marked reading equals the file; 1 = a marked reading is stale, malformed, or
names an unknown scenario; 2 = nothing was measurable (no baseline, no governed page, or a page that
cannot be read), which is never allowed to read as a pass.
"""

from __future__ import annotations

import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
BASELINE = REPO / ".ci" / "ir-baseline.json"
# The live matrix page is the one place that quotes floors as current facts. The dated records
# (`docs/dev/quality-ledger.md`, a released changelog section) name what was true on their date and
# are not re-pointed by a refresh.
GOVERNED = (REPO / "QUALITY_MATRIX.md",)

MARKER = re.compile(r"<!--\s*ir-floor:\s*(?P<scenario>[A-Za-z0-9_]+)\s*-->")
# Counting markers, not number-marker pairs: a pair pattern let an orphan marker silently drop out of
# the scan and still print OK.
DIGITS_BEFORE = re.compile(r"[\d,]+$")
GROUPED = re.compile(r"^\d{1,3}(?:,\d{3})+$")


def load_floors() -> dict[str, int] | None:
    if not BASELINE.is_file():
        return None
    try:
        data = json.loads(BASELINE.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None
    scenarios = data.get("scenarios")
    if not isinstance(scenarios, dict) or not scenarios:
        return None
    return {str(k): int(v) for k, v in scenarios.items()}


def check(path: pathlib.Path, floors: dict[str, int]) -> list[str]:
    """Every complaint about one page, with the file's 1-based line number."""
    text = path.read_text(encoding="utf-8")
    problems: list[str] = []
    for match in MARKER.finditer(text):
        line = text.count("\n", 0, match.start()) + 1
        scenario = match.group("scenario")
        before = DIGITS_BEFORE.search(text, 0, match.start())
        digits = before.group(0) if before else ""
        where = f"{path.as_posix()}:{line}"
        if not digits:
            problems.append(
                f"{where} marker for {scenario} carries no reading: the declaration "
                f"promises one and nothing precedes it"
            )
            continue
        if not GROUPED.match(digits):
            problems.append(
                f"{where} marker for {scenario} is not attached to a grouped thousands-separated reading: {digits!r}"
            )
            continue
        if scenario not in floors:
            problems.append(f"{where} marker names {scenario!r}, which .ci/ir-baseline.json does not contain")
            continue
        if int(digits.replace(",", "")) != floors[scenario]:
            problems.append(f"{where} quotes {scenario} as {digits} but the committed floor is {floors[scenario]:,}")
    return problems


def main(argv: list[str]) -> int:
    floors = load_floors()
    if floors is None:
        print("cannot measure: .ci/ir-baseline.json is missing, unreadable, or has no scenarios")
        return 2
    paths = [pathlib.Path(a) for a in argv] if argv else list(GOVERNED)
    readable = [p for p in paths if p.is_file()]
    if not readable:
        print(f"cannot measure: none of the {len(paths)} governed page(s) is a readable file")
        return 2
    marked = sum(len(MARKER.findall(p.read_text(encoding="utf-8"))) for p in readable)
    if marked == 0:
        print(
            f"cannot measure: {marked} ir-floor markers across {len(readable)} governed page(s); "
            f"a page that quotes no current floor is a scope change, not a pass"
        )
        return 2
    problems: list[str] = []
    for path in readable:
        try:
            problems.extend(check(path, floors))
        except (OSError, UnicodeDecodeError) as exc:
            print(f"cannot read {path.as_posix()}: {exc}")
            return 2
    if problems:
        print(f"{len(problems)} marked floor reading(s) disagree with .ci/ir-baseline.json:")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print(
        f"OK: {marked} marked floor reading(s) across {len(readable)} page(s) equal the committed "
        f"baseline ({len(floors)} scenarios)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
