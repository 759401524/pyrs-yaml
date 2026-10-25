"""Fail when a performance page disagrees with the committed measurement snapshot.

The tables once cited millisecond figures that no run produced: a docs pass rewrote every
row, the absolute values moved by roughly 22x while the ratios stayed about the same, and
nothing read the pages, so nothing noticed. Every wall-clock number on the four locales'
performance and feature pages now comes from `.ci/perf-snapshot.json`, written by
`scripts/bench_snapshot.py`. This gate compares the rendered cells against that file, so a
page can only change by re-measuring first, and a stale snapshot fails by naming the row.
Which tables those are, and against which libraries, is the snapshot's answer rather than
this file's: the tables are found through the `bench:` marker above them, in the three
layouts `--docs` writes, and a comparison table that carries no marker is the same failure
as a stale cell - a number no run produced.
Every page that quotes a speedup as prose - the index, round-trip, PyYAML-compatibility,
benchmark, comparison and feature pages - has its ranges derived here too, because
`scripts/bench_snapshot.py --docs` rewrites tables only and a withdrawn figure could
otherwise survive beside the table that contradicts it.
"""

from __future__ import annotations

import json
import re
from functools import partial
from pathlib import Path

import pytest

from tests.data.yaml_samples import BENCHMARK_LARGE, BENCHMARK_MEDIUM, BENCHMARK_SMALL

REPO_ROOT = Path(__file__).resolve().parent.parent
SNAPSHOT_PATH = REPO_ROOT / ".ci" / "perf-snapshot.json"
LOCALES = ("en", "zh", "ja", "ko")
PAGES = tuple(
    REPO_ROOT / "docs" / locale / name
    for locale in LOCALES
    for name in ("performance/benchmarks.md", "performance/comparison.md", "features.md")
)
CLAIM_PAGES = tuple(
    REPO_ROOT / "docs" / locale / name
    for locale in LOCALES
    for name in ("index.md", "guides/round-trip.md", "guides/pyyaml-compat.md")
)
OPERATIONS = ("parse", "serialize", "roundtrip")
FIXTURES = {"small": BENCHMARK_SMALL, "medium": BENCHMARK_MEDIUM, "large": BENCHMARK_LARGE}
SELF = "pyrs-yaml"

# Two significant digits in the cells allow about 2% of rounding at the 10-24 range; 5%
# still refuses the scale of drift that motivated the gate (absolute values moved ~22x).
TIME_TOLERANCE = 0.05
SPEEDUP_TOLERANCE = 1

# Withdrawn readings: the 2026-08-17 provenance line and the ranges it produced. Kept as
# literal strings because prose has no other oracle - a page that still cites one of these
# is quoting a run that cannot be reproduced.
WITHDRAWN = (
    "2026-08-17",
    "21\u201343",
    "55\u2013177",
    "48\u2013100",
    "123\u2013371",
    "21-43",
    "55-177",
    "48-100",
    "123-371",
    "21\u301c43",
    "55\u301c177",
    "20\u201340\u00d7",
    "20\u201340 倍",
    "20〜40 倍",
    "20~40배",
    "~2\u20133\u00d7",
    "2〜3 倍",
    "2~3배",
)

TIME_CELL = re.compile(r"^\**([\d.]+)\s*(ns|µs|μs|us|ms)\**$")
SCALE = {"ns": 0.001, "µs": 1.0, "μs": 1.0, "us": 1.0, "ms": 1000.0}
SIZE_CELL = re.compile(r"([\d.]+)\s*(B|KB)")
SPEEDUP_CELL = re.compile(r"(\d+(?:\.\d+)?)\u00d7\s*(faster|slower)?")
RANGE_CLAIM = re.compile(r"(\d+(?:\.\d+)?)\s*[\u2013\u301c~-]\s*(\d+(?:\.\d+)?)\s*(?:\u00d7|\u500d|\ubc30)")


def snapshot() -> dict:
    return json.loads(SNAPSHOT_PATH.read_text(encoding="utf-8"))


def cells(line: str) -> list[str]:
    return [cell.strip() for cell in line.strip().strip("|").split("|")]


def tables(path: Path) -> list[tuple[int, list[str], list[list[str]]]]:
    """Every Markdown table in `path`: its header line index, header cells and body rows."""
    lines = path.read_text(encoding="utf-8").splitlines()
    found = []
    index = 0
    while index < len(lines) - 1:
        header, rule = lines[index], lines[index + 1]
        if header.startswith("|") and set(rule) <= set("|-: ") and "-" in rule:
            rows = []
            cursor = index + 2
            while cursor < len(lines) and lines[cursor].startswith("|"):
                rows.append(cells(lines[cursor]))
                cursor += 1
            found.append((index, cells(header), rows))
            index = cursor
        else:
            index += 1
    return found


def microseconds(cell: str) -> float:
    match = TIME_CELL.match(cell)
    assert match, f"not a time cell: {cell!r}"
    value, unit = match.groups()
    return float(value) * SCALE[unit]


def speedup(cell: str) -> float:
    match = SPEEDUP_CELL.search(cell)
    assert match, f"not a speedup cell: {cell!r}"
    return float(match.group(1))


def shown(value: float) -> str:
    """Whole multiples above ten, one decimal below it - the writer's own rounding rule."""
    return f"{round(value)}" if value >= 10 else f"{value:.1f}"


def assert_speedup(cell: str, expected: float, message: str) -> None:
    """The trailing column of a size table, which always reads baseline over us.

    A 1.5x win rounded to an integer prints as 1x and a 0.4x loss prints as 0x, so the small
    cells keep their decimal and this check keeps the distinction instead of forgiving it.
    """
    shown_value = speedup(cell)
    if expected >= 10:
        assert abs(shown_value - expected) <= SPEEDUP_TOLERANCE, (
            f"{message}: shows {cell}, snapshot implies {expected:.1f}x"
        )
    else:
        assert abs(shown_value - expected) <= 0.06 * expected, (
            f"{message}: shows {cell}, snapshot implies {expected:.2f}x"
        )


def assert_relative(cell: str, ratio: float, message: str) -> None:
    """The peer-vs-us column, where the word has to follow the direction the number points.

    Every counterparty on this page used to be a Python reader, so the column said "slower" and
    needed no reading. A Rust-backed peer can be the faster one, and a gate that only compared
    magnitudes would accept "2.6x slower" for a peer that runs 2.6x faster - so the word is
    checked against the ratio, not assumed.
    """
    match = SPEEDUP_CELL.search(cell)
    assert match, f"not a relative-speed cell: {cell!r}"
    word = match.group(2)
    expected_word = "slower" if ratio >= 1 else "faster"
    assert word == expected_word, f"{message}: shows {cell!r}, snapshot implies {expected_word}"
    assert_speedup(cell, max(ratio, 1 / ratio), message)


def label_bytes(label: str) -> int:
    """The document size a row label prints, in bytes."""
    match = SIZE_CELL.search(label)
    assert match, f"not a size label: {label!r}"
    return round(float(match.group(1)) * (1000 if match.group(2) == "KB" else 1))


def page(path: Path) -> str:
    return str(path.relative_to(REPO_ROOT))


def ratio_range(data: dict, format_name: str, operation: str, library: str, behind: bool = False) -> str:
    """The range a page may quote for one peer, in the direction its sentence points.

    `behind=True` is the honest-loss reading - how much faster the peer is - which is the same
    ratio inverted, rounded by the rule the tables already use.
    """
    values = [times[library] / times[SELF] for times in data["formats"][format_name]["operations"][operation].values()]
    if behind:
        values = [1 / value for value in values]
    return f"{shown(min(values))}\u2013{shown(max(values))}"


@pytest.fixture(scope="module")
def data() -> dict:
    return snapshot()


def yaml_operations(data: dict) -> dict:
    """The YAML half of the snapshot, which is what the pages' prose claims are about."""
    return data["formats"]["yaml"]["operations"]


MARKER = re.compile(r"<!--\s*bench:\s*(?P<body>[^>]*?)\s*-->")


def markers(path: Path) -> list[tuple[dict, int]]:
    """Every bench marker in a page, with the line index of the table header beneath it."""
    lines = path.read_text(encoding="utf-8").splitlines()
    found = []
    for index, line in enumerate(lines):
        match = MARKER.search(line)
        if match is None:
            continue
        cursor = index + 1
        while cursor < len(lines) and not lines[cursor].strip():
            cursor += 1
        attrs = dict(item.split("=", 1) for item in match.group("body").split() if "=" in item)
        found.append((attrs, cursor))
    return found


def test_every_measured_case_is_published(data):
    """The snapshot records what was measured; the pages are where it is published.

    A case measured and never published is how a format quietly leaves the public record -
    the state the JSON and TOML readers were in while only YAML had a table. The question is
    asked in both directions, because a marker naming nothing measured is just as wrong.
    """
    published = {
        (attrs["format"], operation)
        for path in PAGES
        for attrs, _ in markers(path)
        for operation in attrs["operation"].split(",")
    }
    measured = {
        (format_name, operation) for format_name, entry in data["formats"].items() for operation in entry["operations"]
    }
    assert measured - published == set(), f"measured but on no page: {sorted(measured - published)}"
    assert published - measured == set(), f"on a page but never measured: {sorted(published - measured)}"
    for format_name in data["unpublished_writers"]:
        assert (format_name, "serialize") not in published, f"{format_name} publishes a writer row it cannot race"


def test_the_format_tables_match_the_snapshot(data):
    """Each marker's table has to carry the numbers the snapshot records for it."""
    checks = {"sizes": check_sizes, "libraries": check_libraries, "operations": check_operations}
    for path in PAGES:
        lines = path.read_text(encoding="utf-8").splitlines()
        for attrs, header_line in markers(path):
            check = checks.get(attrs["layout"])
            assert check, f"{page(path)}: marker declares a layout this gate does not read: {attrs['layout']!r}"
            check(path, lines, attrs, header_line, data)


def table_rows(lines: list[str], header_line: int, count: int, message: str) -> list[list[str]]:
    """The `count` body rows under a header, as cells - a short table is a finding, not a slice."""
    rows = []
    for index in range(header_line + 2, header_line + 2 + count):
        assert index < len(lines) and lines[index].startswith("|"), (
            f"{message}: its table has no row {index - header_line - 1}"
        )
        rows.append(cells(lines[index]))
    return rows


def check_sizes(path: Path, lines: list[str], attrs: dict, header_line: int, data: dict) -> None:
    """One row per fixture size; one column per library; the marker's baseline in the ratio."""
    format_name = attrs["format"]
    operations = data["formats"][format_name]["operations"][attrs["operation"]]
    fixtures = data["formats"][format_name]["fixtures"]
    header = cells(lines[header_line])
    sizes = attrs["sizes"].split(",")
    with_ratio = attrs.get("speedup", "yes") == "yes"
    libraries = header[1:-1] if with_ratio else header[1:]
    if with_ratio:
        # A ratio column has to say which library it is measured against. The TOML and JSON5 rows
        # print a fraction - we are the slower reader there - and a column headed only "Speedup"
        # over "0.5x" is the shape a reader misreads.
        assert attrs["baseline"] in header[-1], (
            f"{page(path)}: {format_name}/{attrs['operation']} divides by {attrs['baseline']}, "
            f"whose name is not in the ratio column {header[-1]!r}"
        )
    rows = table_rows(lines, header_line, len(sizes), f"{page(path)}: {format_name}/{attrs['operation']}")
    for size, row in zip(sizes, rows):
        assert size in operations, f"{page(path)}: {format_name} measures no {size!r} {attrs['operation']}"
        times = operations[size]
        message = f"{page(path)}: {format_name}/{attrs['operation']}/{size}"
        assert len(row) == len(header), f"{message}: {len(row)} cells, header has {len(header)}"
        assert set(libraries) == set(times), f"{message}: table races {sorted(libraries)}, snapshot {sorted(times)}"
        documented = fixtures[size]["bytes"]
        assert abs(label_bytes(row[0]) - documented) <= 0.1 * documented, (
            f"{message}: label {row[0]!r} is not the {documented} B fixture"
        )
        for column, library in enumerate(libraries, start=1):
            assert microseconds(row[column]) == pytest.approx(times[library], rel=TIME_TOLERANCE), (
                f"{message}/{library} shows {row[column]}, snapshot records {times[library]} µs"
            )
        if with_ratio:
            assert_speedup(row[-1], times[attrs["baseline"]] / times[SELF], message)


def check_libraries(path: Path, lines: list[str], attrs: dict, header_line: int, data: dict) -> None:
    """One row per library on one fixture, with the distance from us in the direction it points."""
    times = data["formats"][attrs["format"]]["operations"][attrs["operation"]][attrs["size"]]
    header = cells(lines[header_line])
    rows = table_rows(lines, header_line, len(times), f"{page(path)}: {attrs['format']}/{attrs['operation']}")
    assert len(header) == 3, f"{page(path)}: a library table has three columns, header has {len(header)}"
    names = [row[0].replace("*", "") for row in rows]
    assert set(names) == set(times), (
        f"{page(path)}: {attrs['operation']} rows {sorted(names)}, snapshot {sorted(times)}"
    )
    for row, library in zip(rows, names):
        message = f"{page(path)}: {attrs['format']}/{attrs['operation']}/{library}"
        assert microseconds(row[1]) == pytest.approx(times[library], rel=TIME_TOLERANCE), (
            f"{message} shows {row[1]}, snapshot records {times[library]} µs"
        )
        if library == SELF:
            assert row[2] == "—", f"{page(path)}: the row that is us should carry no ratio, shows {row[2]!r}"
        else:
            assert_relative(row[2], times[library] / times[SELF], message)


def check_operations(path: Path, lines: list[str], attrs: dict, header_line: int, data: dict) -> None:
    """One row per operation on one fixture - the summary a feature page carries."""
    format_name = attrs["format"]
    names = attrs["operation"].split(",")
    header = cells(lines[header_line])
    libraries = header[1:]
    assert SELF in libraries, f"{page(path)}: the summary table does not name itself"
    rows = table_rows(lines, header_line, len(names), f"{page(path)}: {format_name} summary")
    for operation, row in zip(names, rows):
        times = data["formats"][format_name]["operations"][operation][attrs["size"]]
        message = f"{page(path)}: {format_name}/{operation}"
        assert len(row) == len(header), f"{message}: {len(row)} cells, header has {len(header)}"
        for column, library in enumerate(libraries, start=1):
            assert library in times, f"{message}: {library} is not measured for {format_name}"
            assert microseconds(row[column]) == pytest.approx(times[library], rel=TIME_TOLERANCE), (
                f"{message}/{library} shows {row[column]}, snapshot records {times[library]} µs"
            )


def test_snapshot_records_the_protocol_the_pages_describe(data):
    assert data["parameters"] == {"rounds": 9, "target_seconds": 0.1}
    assert data["generated_at"]
    assert set(data["libraries"]) >= {
        "pyrs-yaml",
        "PyYAML",
        "ruamel.yaml",
        "yaml-rs",
        "ryaml",
        "json",
        "orjson",
        "tomllib",
        "rtoml",
        "pyjson5",
    }
    assert set(yaml_operations(data)) == set(OPERATIONS)
    for operation in OPERATIONS:
        by_size = yaml_operations(data)[operation]
        assert set(by_size) == set(FIXTURES)
        # One column set across the sizes: a peer measured on two fixtures but not the third would
        # leave the page publishing the hole as if it were a result.
        assert len({frozenset(row) for row in by_size.values()}) == 1, f"{operation} sizes name different libraries"
        assert set(by_size["large"]) == {"pyrs-yaml", "PyYAML", "ruamel.yaml", "yaml-rs", "ryaml"}, operation
        assert all(value > 0 for row in by_size.values() for value in row.values())
    assert set(data["formats"]) >= {"yaml", "json", "toml", "json5"}
    # A writer half is absent because no counterparty does that job, not because nobody asked.
    assert set(data["unpublished_writers"]) == {"json", "toml", "json5"}


def test_snapshot_fixtures_match_the_benchmark_inputs(data):
    assert {name: entry["bytes"] for name, entry in data["formats"]["yaml"]["fixtures"].items()} == {
        name: len(text.encode("utf-8")) for name, text in FIXTURES.items()
    }


def test_every_library_comparison_carries_a_marker(data):
    """A table that races us against a measured library is generated, or it is a finding.

    `--docs` rewrites the cells under a marker and nothing else, so the marked tables are the ones
    a re-measure reaches. An unmarked comparison table is either hand-typed or left behind by the
    last snapshot, and either way it publishes a number no run produced - the divan leg's own
    table stays out of this, because it names no library the snapshot measures.
    """
    measured = set(data["libraries"]) - {SELF}
    for path in PAGES:
        marked = {header_line for _, header_line in markers(path)}
        for header_line, header, rows in tables(path):
            names = set(header) | {row[0].replace("*", "") for row in rows}
            timed = any(TIME_CELL.match(cell) for row in rows for cell in row)
            if not (SELF in names and names & measured and timed):
                continue
            peers = "/".join(sorted(names & measured))
            assert header_line in marked, (
                f"{page(path)}: the {peers} table on line {header_line + 1} has no bench marker"
            )


def test_prose_claims_match_the_snapshot(data):
    ahead = partial(ratio_range, data)
    behind = partial(ratio_range, data, behind=True)
    ranges = {
        "index.md": (
            ahead("yaml", "parse", "PyYAML"),
            ahead("yaml", "serialize", "PyYAML"),
            ahead("yaml", "parse", "ruamel.yaml"),
            ahead("yaml", "serialize", "ruamel.yaml"),
        ),
        "round-trip.md": (
            ahead("yaml", "roundtrip", "PyYAML"),
            ahead("yaml", "roundtrip", "ruamel.yaml"),
        ),
        "pyyaml-compat.md": (
            ahead("yaml", "parse", "PyYAML"),
            ahead("yaml", "serialize", "PyYAML"),
        ),
        "benchmarks.md": (
            ahead("yaml", "parse", "PyYAML"),
            ahead("yaml", "serialize", "PyYAML"),
            ahead("yaml", "roundtrip", "PyYAML"),
            ahead("yaml", "parse", "ruamel.yaml"),
            ahead("yaml", "serialize", "ruamel.yaml"),
            ahead("yaml", "roundtrip", "ruamel.yaml"),
            behind("yaml", "parse", "yaml-rs"),
            behind("yaml", "roundtrip", "yaml-rs"),
            ahead("yaml", "serialize", "yaml-rs"),
        ),
        "comparison.md": (
            ahead("yaml", "parse", "PyYAML"),
            ahead("yaml", "serialize", "PyYAML"),
            ahead("yaml", "serialize", "yaml-rs"),
        ),
        "features.md": (
            ahead("yaml", "parse", "PyYAML"),
            ahead("yaml", "serialize", "PyYAML"),
        ),
    }
    for path in CLAIM_PAGES + PAGES:
        text = path.read_text(encoding="utf-8")
        claimed = {f"{match.group(1)}\u2013{match.group(2)}" for match in RANGE_CLAIM.finditer(text)}
        expected = set(ranges[path.name])
        assert claimed <= expected, f"{page(path)} cites ranges outside the snapshot: {sorted(claimed - expected)}"
        assert expected <= claimed, f"{page(path)} omits the snapshot ranges: {sorted(expected - claimed)}"


def test_pages_cite_the_snapshot_and_no_withdrawn_measurement():
    for path in PAGES + CLAIM_PAGES:
        text = path.read_text(encoding="utf-8")
        for stale in WITHDRAWN:
            assert stale not in text, f"{page(path)} still cites {stale!r}"
    for path in (page_path for page_path in PAGES if page_path.name == "benchmarks.md"):
        text = path.read_text(encoding="utf-8")
        assert "scripts/bench_snapshot.py" in text, page(path)
        assert ".ci/perf-snapshot.json" in text, page(path)


def test_a_page_that_names_the_measurement_names_its_date(data):
    # Every table here is a re-measured snapshot, so the date a page prints is a reading too.
    # `--docs` rewrites the cells and nothing else, which left eight provenance lines naming an
    # older run than the numbers above them; this is the check that would have caught that edit.
    stamped = data["generated_at"][:10]
    for path in PAGES:
        text = path.read_text(encoding="utf-8")
        if "bench_snapshot.py" in text:
            assert stamped in text, f"{page(path)} names the script but not the run it recorded ({stamped})"
