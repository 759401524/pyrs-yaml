"""Fail when a performance page disagrees with the committed measurement snapshot.

The tables once cited millisecond figures that no run produced: a docs pass rewrote every
row, the absolute values moved by roughly 22x while the ratios stayed about the same, and
nothing read the pages, so nothing noticed. Every wall-clock number on the four locales'
performance and feature pages now comes from `.ci/perf-snapshot.json`, written by
`scripts/bench_snapshot.py`. This gate compares the rendered cells against that file, so a
page can only change by re-measuring first, and a stale snapshot fails by naming the row.
Every page that quotes a speedup as prose - the index, round-trip, PyYAML-compatibility,
benchmark, comparison and feature pages - has its ranges derived here too, because
`scripts/bench_snapshot.py --docs` rewrites tables only and a withdrawn figure could
otherwise survive beside the table that contradicts it.
"""

from __future__ import annotations

import json
import re
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

# yaml-rs is not one of the snapshot's libraries: this range comes from the cross-library
# benchmark in tests/test_benchmark_crosslib.py, so it is named rather than derived.
EXTRA_CLAIMS = {"comparison.md": {"2\u20134"}}

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
SPEEDUP_CELL = re.compile(r"(\d+(?:\.\d+)?)\u00d7")
RANGE_CLAIM = re.compile(r"(\d+)\s*[\u2013\u301c~-]\s*(\d+)\s*(?:\u00d7|\u500d|\ubc30)")


def snapshot() -> dict:
    return json.loads(SNAPSHOT_PATH.read_text(encoding="utf-8"))


def cells(line: str) -> list[str]:
    return [cell.strip() for cell in line.strip().strip("|").split("|")]


def tables(path: Path) -> list[tuple[list[str], list[list[str]]]]:
    """Header cells and body rows of every Markdown table in `path`, in file order."""
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
            found.append((cells(header), rows))
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


def assert_speedup(cell: str, expected: float, message: str) -> None:
    """Whole multiples above ten, one decimal below - the same rule the writer applies.

    A 1.5x win rounded to an integer prints as 1x and a 0.4x loss prints as 0x, so the small
    cells keep their decimal and this check keeps the distinction instead of forgiving it.
    """
    shown = speedup(cell)
    if expected >= 10:
        assert abs(shown - expected) <= SPEEDUP_TOLERANCE, f"{message}: shows {cell}, snapshot implies {expected:.1f}x"
    else:
        assert abs(shown - expected) <= 0.06 * expected, f"{message}: shows {cell}, snapshot implies {expected:.2f}x"


def size_key(label: str) -> str:
    match = SIZE_CELL.search(label)
    assert match, f"not a size label: {label!r}"
    value = float(match.group(1)) * (1000 if match.group(2) == "KB" else 1)
    key = min(FIXTURES, key=lambda name: abs(len(FIXTURES[name].encode("utf-8")) - value))
    documented = len(FIXTURES[key].encode("utf-8"))
    assert abs(documented - value) / documented <= 0.1, f"size {label!r} names no fixture"
    return key


def page(path: Path) -> str:
    return str(path.relative_to(REPO_ROOT))


def speedup_range(data: dict, operation: str, library: str) -> str:
    values = [times[library] / times["pyrs-yaml"] for times in yaml_operations(data)[operation].values()]
    return f"{round(min(values))}\u2013{round(max(values))}"


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
    for path in PAGES:
        if not markers(path):
            continue
        lines = path.read_text(encoding="utf-8").splitlines()
        for attrs, header_line in markers(path):
            if attrs["layout"] != "sizes":
                continue
            header = cells(lines[header_line])
            times_by_size = data["formats"][attrs["format"]]["operations"][attrs["operation"]]
            sizes = attrs["sizes"].split(",")
            for index, size in enumerate(sizes):
                offset = header_line + 2 + index
                assert offset < len(lines) and lines[offset].startswith("|"), (
                    f"{page(path)}: {attrs['format']}/{attrs['operation']} names {len(sizes)} fixtures, "
                    f"its table has no row {index + 1}"
                )
                row = cells(lines[offset])
                assert len(row) == len(header), f"{page(path)}: {attrs['format']}/{size} row shape"
                expected = times_by_size[size]
                for column, library in enumerate(header[1:-1], start=1):
                    assert microseconds(row[column]) == pytest.approx(expected[library], rel=TIME_TOLERANCE), (
                        f"{page(path)}: {attrs['format']}/{attrs['operation']}/{size}/{library}"
                    )
                assert_speedup(
                    row[-1],
                    expected[attrs["baseline"]] / expected["pyrs-yaml"],
                    f"{page(path)}: {attrs['format']}/{attrs['operation']}/{size}",
                )


def test_snapshot_records_the_protocol_the_pages_describe(data):
    assert data["parameters"] == {"rounds": 9, "target_seconds": 0.1}
    assert data["generated_at"]
    assert set(data["libraries"]) >= {
        "pyrs-yaml",
        "PyYAML",
        "ruamel.yaml",
        "json",
        "orjson",
        "tomllib",
        "pyjson5",
    }
    assert set(yaml_operations(data)) == set(OPERATIONS)
    for operation in OPERATIONS:
        assert set(yaml_operations(data)[operation]) == set(FIXTURES)
        for size in FIXTURES:
            row = yaml_operations(data)[operation][size]
            assert set(row) == {"pyrs-yaml", "PyYAML", "ruamel.yaml"}
            assert all(value > 0 for value in row.values())
    assert set(data["formats"]) >= {"yaml", "json", "toml", "json5"}
    # A writer half is absent because no counterparty does that job, not because nobody asked.
    assert set(data["unpublished_writers"]) == {"json", "toml", "json5"}


def test_snapshot_fixtures_match_the_benchmark_inputs(data):
    assert {name: entry["bytes"] for name, entry in data["formats"]["yaml"]["fixtures"].items()} == {
        name: len(text.encode("utf-8")) for name, text in FIXTURES.items()
    }


def test_benchmark_tables_match_snapshot(data):
    for path in (page_path for page_path in PAGES if page_path.name == "benchmarks.md"):
        found = [
            table
            for table in tables(path)
            if len(table[0]) == 5 and table[0][1] == "pyrs-yaml" and "PyYAML" in table[0]
        ]
        assert len(found) == 3, f"{page(path)}: expected three parse/serialize/round-trip tables"
        for operation, (header, rows) in zip(OPERATIONS, found):
            assert header[1:4] == ["pyrs-yaml", "PyYAML", "ruamel.yaml"], page(path)
            for row in rows:
                key = size_key(row[0])
                expected_row = yaml_operations(data)[operation][key]
                for column, library in enumerate(header[1:4], start=1):
                    assert microseconds(row[column]) == pytest.approx(expected_row[library], rel=TIME_TOLERANCE), (
                        f"{page(path)}: {operation}/{key}/{library} shows {row[column]}, snapshot records {expected_row[library]} µs"
                    )
                assert_speedup(
                    row[4],
                    expected_row["PyYAML"] / expected_row["pyrs-yaml"],
                    f"{page(path)}: {operation}/{key}",
                )


def test_comparison_tables_match_snapshot(data):
    for path in (page_path for page_path in PAGES if page_path.name == "comparison.md"):
        found = [
            table for table in tables(path) if len(table[0]) == 3 and table[0][1] in {"Time", "時間", "时间", "시간"}
        ]
        assert len(found) == 3, f"{page(path)}: expected three parse/serialize/round-trip tables"
        for operation, (_, rows) in zip(OPERATIONS, found):
            assert [row[0].replace("*", "") for row in rows] == ["pyrs-yaml", "PyYAML", "ruamel.yaml"], page(path)
            expected_row = yaml_operations(data)[operation]["large"]
            assert microseconds(rows[0][1]) == pytest.approx(expected_row["pyrs-yaml"], rel=TIME_TOLERANCE), (
                f"{page(path)}: {operation}/pyrs-yaml shows {rows[0][1]}"
            )
            baseline = expected_row["pyrs-yaml"]
            for row in rows[1:]:
                library = row[0].replace("*", "")
                assert microseconds(row[1]) == pytest.approx(expected_row[library], rel=TIME_TOLERANCE), (
                    f"{page(path)}: {operation}/{library} shows {row[1]}, snapshot records {expected_row[library]} µs"
                )
                assert_speedup(
                    row[2],
                    expected_row[library] / baseline,
                    f"{page(path)}: {operation}/{library}",
                )


def test_features_table_matches_snapshot(data):
    for path in (page_path for page_path in PAGES if page_path.name == "features.md"):
        found = [table for table in tables(path) if len(table[0]) == 3 and table[0][1] == "pyrs-yaml"]
        assert len(found) == 1, f"{page(path)}: expected one performance table"
        _, rows = found[0]
        assert len(rows) == 3, page(path)
        for operation, row in zip(OPERATIONS, rows):
            expected_row = yaml_operations(data)[operation]["large"]
            for column, library in ((1, "pyrs-yaml"), (2, "PyYAML")):
                assert microseconds(row[column]) == pytest.approx(expected_row[library], rel=TIME_TOLERANCE), (
                    f"{page(path)}: {operation}/{library} shows {row[column]}, snapshot records {expected_row[library]} µs"
                )


def test_prose_claims_match_the_snapshot(data):
    ranges = {
        "index.md": (
            speedup_range(data, "parse", "PyYAML"),
            speedup_range(data, "serialize", "PyYAML"),
            speedup_range(data, "parse", "ruamel.yaml"),
            speedup_range(data, "serialize", "ruamel.yaml"),
        ),
        "round-trip.md": (
            speedup_range(data, "roundtrip", "PyYAML"),
            speedup_range(data, "roundtrip", "ruamel.yaml"),
        ),
        "pyyaml-compat.md": (
            speedup_range(data, "parse", "PyYAML"),
            speedup_range(data, "serialize", "PyYAML"),
        ),
        "benchmarks.md": (
            speedup_range(data, "parse", "PyYAML"),
            speedup_range(data, "serialize", "PyYAML"),
            speedup_range(data, "roundtrip", "PyYAML"),
            speedup_range(data, "parse", "ruamel.yaml"),
            speedup_range(data, "serialize", "ruamel.yaml"),
            speedup_range(data, "roundtrip", "ruamel.yaml"),
        ),
        "comparison.md": (
            speedup_range(data, "parse", "PyYAML"),
            speedup_range(data, "serialize", "PyYAML"),
        ),
        "features.md": (
            speedup_range(data, "parse", "PyYAML"),
            speedup_range(data, "serialize", "PyYAML"),
        ),
    }
    for path in CLAIM_PAGES + PAGES:
        text = path.read_text(encoding="utf-8")
        claimed = {f"{match.group(1)}\u2013{match.group(2)}" for match in RANGE_CLAIM.finditer(text)}
        expected = set(ranges[path.name]) | EXTRA_CLAIMS.get(path.name, set())
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
