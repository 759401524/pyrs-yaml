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

A scenario the file keys by runner image declares the row it quotes, not the flat column:

    `load_toml_medium` 144,902,515<!-- ir-floor: load_toml_medium @ 20261023.334.3 -->

because for those scenarios the flat number is the one *no enforcing run compares against* — `ir_gate.py`
takes its floor from `image_floors[image][scenario]` on an image the file measured. The two columns are
not close: `load_toml_medium` carries 144,902,515 on `20261023.334.3` and 142,467,241 on
`20261026.354.1`, a 1.71 % difference, and which one the flat column holds depends on the image the last
refresh happened to land on. Marking a keyed floor without its image therefore lets a paragraph pass the
gate while quoting a floor that belongs to a different machine than the leg the sentence describes.

Five things this refuses, and one it cannot see:

- a marker naming a scenario the baseline does not contain (a rename or a typo);
- a marker whose adjacent digits differ from the floor it declares (stale by any amount) — the named
  image's row for a keyed scenario, the flat column otherwise;
- a marker that is not adjacent to a grouped number at all (the declaration promises a reading);
- a keyed scenario marked without an image, since the file cannot tell the reader which row its sentence
  is about;
- an image marked against a scenario the file does not key, or against an image the file has no row for.
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

MARKER = re.compile(
    r"<!--\s*ir-floor:\s*(?P<scenario>[A-Za-z0-9_]+)"
    r"(?:\s*@\s*(?P<image>[^\s>]+))?\s*-->"
)
# Counting markers, not number-marker pairs: a pair pattern let an orphan marker silently drop out of
# the scan and still print OK.
DIGITS_BEFORE = re.compile(r"[\d,]+$")
GROUPED = re.compile(r"^\d{1,3}(?:,\d{3})+$")
# A row key is the whole runner description (`github-runner ubuntu24, image 20261023.334.3`); a mark
# names the part that distinguishes it, so the file stays the authority on the key's shape.
IMAGE_PREFIX = "image "


def load_baseline() -> tuple[dict[str, int], dict[str, dict[str, int]]] | None:
    """The flat column and the keyed rows, or None when nothing is measurable."""
    if not BASELINE.is_file():
        return None
    try:
        data = json.loads(BASELINE.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None
    scenarios = data.get("scenarios")
    if not isinstance(scenarios, dict) or not scenarios:
        return None
    floors = {str(k): int(v) for k, v in scenarios.items()}
    rows: dict[str, dict[str, int]] = {}
    for image, row in (data.get("image_floors") or {}).items():
        if isinstance(row, dict):
            rows[str(image)] = {str(k): int(v) for k, v in row.items()}
    return floors, rows


def row_of(rows: dict[str, dict[str, int]], image: str) -> list[str]:
    """The row keys a mark's `@<image>` names, so an ambiguous or absent one is a refusal."""
    return [key for key, row in rows.items() if row and key.endswith(f"{IMAGE_PREFIX}{image}")]


def version_of(key: str) -> str:
    """The part of a row key a mark names: `image 20261023.334.3` reads as `20261023.334.3`."""
    return key.rsplit(IMAGE_PREFIX, 1)[-1] if IMAGE_PREFIX in key else key


def check(path: pathlib.Path, floors: dict[str, int], rows: dict[str, dict[str, int]]) -> list[str]:
    """Every complaint about one page, with the file's 1-based line number."""
    text = path.read_text(encoding="utf-8")
    keyed = {scenario for row in rows.values() for scenario in row}
    available = sorted(version_of(key) for key in rows)
    problems: list[str] = []
    for match in MARKER.finditer(text):
        line = text.count("\n", 0, match.start()) + 1
        scenario = match.group("scenario")
        image = match.group("image")
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
        if scenario not in floors and scenario not in keyed:
            problems.append(f"{where} marker names {scenario!r}, which .ci/ir-baseline.json does not contain")
            continue
        value = int(digits.replace(",", ""))
        if image is not None and scenario not in keyed:
            problems.append(
                f"{where} marks {scenario}@{image} but the file keys no floor for it: its only floor "
                f"is the flat column, which an enforcing run on any image compares against"
            )
            continue
        if image is None and scenario in keyed:
            problems.append(
                f"{where} quotes {scenario}, a floor the file keys by runner image, naming no image: "
                f"the flat column is what no enforcing run compares against there. "
                f"Name one of the rows the file carries: {', '.join(available) or '(none)'}"
            )
            continue
        if image is not None:
            matches = row_of(rows, image)
            if not matches:
                problems.append(
                    f"{where} marks {scenario}@{image}, which .ci/ir-baseline.json has no row for; "
                    f"the rows are {', '.join(available) or '(none)'}"
                )
                continue
            if len(matches) > 1:
                problems.append(
                    f"{where} marks {scenario}@{image}, which names {len(matches)} rows: "
                    f"{', '.join(sorted(matches))} — the mark has to be addressable to one floor"
                )
                continue
            floor = rows[matches[0]].get(scenario)
            if floor is None:
                problems.append(
                    f"{where} marks {scenario}@{image} but that image's row carries no floor for it; "
                    f"the row covers {', '.join(sorted(rows[matches[0]]))}"
                )
                continue
            if value != floor:
                problems.append(f"{where} quotes {scenario} on {image} as {digits} but the committed row is {floor:,}")
            continue
        if value != floors[scenario]:
            problems.append(f"{where} quotes {scenario} as {digits} but the committed floor is {floors[scenario]:,}")
    return problems


def main(argv: list[str]) -> int:
    baseline = load_baseline()
    if baseline is None:
        print("cannot measure: .ci/ir-baseline.json is missing, unreadable, or has no scenarios")
        return 2
    floors, rows = baseline
    paths = [pathlib.Path(a) for a in argv] if argv else list(GOVERNED)
    readable = [p for p in paths if p.is_file()]
    if not readable:
        print(f"cannot measure: none of the {len(paths)} governed page(s) is a readable file")
        return 2
    texts = {p: p.read_text(encoding="utf-8") for p in readable}
    marked = sum(len(MARKER.findall(text)) for text in texts.values())
    if marked == 0:
        print(
            f"cannot measure: {marked} ir-floor markers across {len(readable)} governed page(s); "
            f"a page that quotes no current floor is a scope change, not a pass"
        )
        return 2
    problems: list[str] = []
    for path in readable:
        try:
            problems.extend(check(path, floors, rows))
        except (OSError, UnicodeDecodeError) as exc:
            print(f"cannot read {path.as_posix()}: {exc}")
            return 2
    keyed = {scenario for row in rows.values() for scenario in row}
    quoted = sum(1 for text in texts.values() for match in MARKER.finditer(text) if match.group("image") is not None)
    if problems:
        print(f"{len(problems)} marked floor reading(s) disagree with .ci/ir-baseline.json:")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print(
        f"OK: {marked} marked floor reading(s) across {len(readable)} page(s) equal the committed "
        f"baseline ({len(floors)} scenarios, {len(keyed)} keyed by image in {len(rows)} row(s), "
        f"{quoted} of them quoted from a row)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
