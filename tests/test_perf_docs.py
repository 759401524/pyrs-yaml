"""Fail when a performance page disagrees with the committed measurement snapshot.

The tables once cited millisecond figures that no run produced: a docs pass rewrote every
row, the absolute values moved by roughly 22x while the ratios stayed about the same, and
nothing read the pages, so nothing noticed. Every wall-clock number on the four locales'
performance and feature pages now comes from `.ci/perf-snapshot.json`, written by
`scripts/bench_snapshot.py`. This gate compares the rendered cells against that file, so a
page can only change by re-measuring first, and a stale snapshot fails by naming the row.
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
OPERATIONS = ("parse", "serialize", "roundtrip")
FIXTURES = {"small": BENCHMARK_SMALL, "medium": BENCHMARK_MEDIUM, "large": BENCHMARK_LARGE}

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
SPEEDUP_CELL = re.compile(r"(\d+)\u00d7")


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


def speedup(cell: str) -> int:
    match = SPEEDUP_CELL.search(cell)
    assert match, f"not a speedup cell: {cell!r}"
    return int(match.group(1))


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


@pytest.fixture(scope="module")
def data() -> dict:
    return snapshot()


def test_snapshot_records_the_protocol_the_pages_describe(data):
    assert data["parameters"] == {"rounds": 9, "target_seconds": 0.1}
    assert data["generated_at"]
    assert set(data["libraries"]) >= {"pyrs-yaml", "PyYAML", "ruamel.yaml"}
    assert set(data["operations"]) == set(OPERATIONS)
    for operation in OPERATIONS:
        assert set(data["operations"][operation]) == set(FIXTURES)
        for size in FIXTURES:
            row = data["operations"][operation][size]
            assert set(row) == {"pyrs-yaml", "PyYAML", "ruamel.yaml"}
            assert all(value > 0 for value in row.values())


def test_snapshot_fixtures_match_the_benchmark_inputs(data):
    assert {name: entry["bytes"] for name, entry in data["fixtures"].items()} == {
        name: len(text.encode("utf-8")) for name, text in FIXTURES.items()
    }


def test_benchmark_tables_match_snapshot(data):
    for path in (page_path for page_path in PAGES if page_path.name == "benchmarks.md"):
        found = [table for table in tables(path) if len(table[0]) == 5 and table[0][1] == "pyrs-yaml"]
        assert len(found) == 3, f"{page(path)}: expected three parse/serialize/round-trip tables"
        for operation, (header, rows) in zip(OPERATIONS, found):
            assert header[1:4] == ["pyrs-yaml", "PyYAML", "ruamel.yaml"], page(path)
            for row in rows:
                key = size_key(row[0])
                expected_row = data["operations"][operation][key]
                for column, library in enumerate(header[1:4], start=1):
                    assert microseconds(row[column]) == pytest.approx(expected_row[library], rel=TIME_TOLERANCE), (
                        f"{page(path)}: {operation}/{key}/{library} shows {row[column]}, snapshot records {expected_row[library]} µs"
                    )
                expected_speedup = expected_row["PyYAML"] / expected_row["pyrs-yaml"]
                assert abs(speedup(row[4]) - expected_speedup) <= SPEEDUP_TOLERANCE, (
                    f"{page(path)}: {operation}/{key} shows {row[4]}, snapshot implies {expected_speedup:.1f}x"
                )


def test_comparison_tables_match_snapshot(data):
    for path in (page_path for page_path in PAGES if page_path.name == "comparison.md"):
        found = [
            table for table in tables(path) if len(table[0]) == 3 and table[0][1] in {"Time", "時間", "时间", "시간"}
        ]
        assert len(found) == 3, f"{page(path)}: expected three parse/serialize/round-trip tables"
        for operation, (_, rows) in zip(OPERATIONS, found):
            assert [row[0].replace("*", "") for row in rows] == ["pyrs-yaml", "PyYAML", "ruamel.yaml"], page(path)
            expected_row = data["operations"][operation]["large"]
            assert microseconds(rows[0][1]) == pytest.approx(expected_row["pyrs-yaml"], rel=TIME_TOLERANCE), (
                f"{page(path)}: {operation}/pyrs-yaml shows {rows[0][1]}"
            )
            baseline = expected_row["pyrs-yaml"]
            for row in rows[1:]:
                library = row[0].replace("*", "")
                assert microseconds(row[1]) == pytest.approx(expected_row[library], rel=TIME_TOLERANCE), (
                    f"{page(path)}: {operation}/{library} shows {row[1]}, snapshot records {expected_row[library]} µs"
                )
                assert abs(speedup(row[2]) - expected_row[library] / baseline) <= SPEEDUP_TOLERANCE, (
                    f"{page(path)}: {operation}/{library} shows {row[2]}, snapshot implies {expected_row[library] / baseline:.1f}x"
                )


def test_features_table_matches_snapshot(data):
    for path in (page_path for page_path in PAGES if page_path.name == "features.md"):
        found = [table for table in tables(path) if len(table[0]) == 3 and table[0][1] == "pyrs-yaml"]
        assert len(found) == 1, f"{page(path)}: expected one performance table"
        _, rows = found[0]
        assert len(rows) == 3, page(path)
        for operation, row in zip(OPERATIONS, rows):
            expected_row = data["operations"][operation]["large"]
            for column, library in ((1, "pyrs-yaml"), (2, "PyYAML")):
                assert microseconds(row[column]) == pytest.approx(expected_row[library], rel=TIME_TOLERANCE), (
                    f"{page(path)}: {operation}/{library} shows {row[column]}, snapshot records {expected_row[library]} µs"
                )


def test_pages_cite_the_snapshot_and_no_withdrawn_measurement():
    for path in PAGES:
        text = path.read_text(encoding="utf-8")
        for stale in WITHDRAWN:
            assert stale not in text, f"{page(path)} still cites {stale!r}"
    for path in (page_path for page_path in PAGES if page_path.name == "benchmarks.md"):
        text = path.read_text(encoding="utf-8")
        assert "scripts/bench_snapshot.py" in text, page(path)
        assert ".ci/perf-snapshot.json" in text, page(path)
