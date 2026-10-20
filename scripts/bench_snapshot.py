#!/usr/bin/env python3
"""Measure the YAML cross-library benchmarks and record them in .ci/perf-snapshot.json.

Batch timing is used because a single call to the Rust-backed reader/writer can be
faster than the clock resolution: each case is measured as several whole batches of
many calls, and the reported value is the fastest per-call time across batches.
`--docs` rewrites the wall-clock tables on the four locales' performance and feature
pages from the snapshot, so no table is ever transcribed by hand.
"""

from __future__ import annotations

import argparse
import io
import json
import platform
import re
import sys
import time
from datetime import datetime, timezone
from importlib.metadata import version as package_version
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

import yaml  # noqa: E402
from ruamel.yaml import YAML  # noqa: E402

import pyrs_yaml  # noqa: E402
from tests.data.yaml_samples import (  # noqa: E402
    BENCHMARK_LARGE,
    BENCHMARK_MEDIUM,
    BENCHMARK_SMALL,
)

SNAPSHOT = ROOT / ".ci" / "perf-snapshot.json"
FIXTURES = {"small": BENCHMARK_SMALL, "medium": BENCHMARK_MEDIUM, "large": BENCHMARK_LARGE}
SIZES = tuple(FIXTURES)
OPERATIONS = ("parse", "serialize", "roundtrip")
LOCALES = ("en", "zh", "ja", "ko")
PAGES = tuple(
    ROOT / "docs" / locale / name
    for locale in LOCALES
    for name in ("performance/benchmarks.md", "performance/comparison.md", "features.md")
)
ROUNDS = 9
TARGET_SECONDS = 0.1
MAX_BATCH = 500_000
RUAMEL = YAML()


def _time_calls(fn, calls):
    start = time.perf_counter_ns()
    for _ in range(calls):
        fn()
    return time.perf_counter_ns() - start


def measure(fn):
    fn()
    calls = 1
    while True:
        elapsed = _time_calls(fn, calls)
        if elapsed >= TARGET_SECONDS * 1e9 or calls >= MAX_BATCH:
            break
        if elapsed > 0:
            calls = min(max(calls * 2, int(calls * TARGET_SECONDS * 1e9 / elapsed)), MAX_BATCH)
        else:
            calls *= 2
    samples = [_time_calls(fn, calls) / calls for _ in range(ROUNDS)]
    return round(min(samples) / 1000, 3)


def ruamel_dump(data):
    stream = io.StringIO()
    RUAMEL.dump(data, stream)
    return stream.getvalue()


def warm_up(seconds=1.5):
    end = time.perf_counter() + seconds
    value = 0
    while time.perf_counter() < end:
        value = (value * 1103515245 + 12345) & 0x7FFFFFFF
    return value


def collect():
    warm_up()
    operations = {"parse": {}, "serialize": {}, "roundtrip": {}}
    fixtures = {}
    for size, text in FIXTURES.items():
        fixtures[size] = {"bytes": len(text.encode("utf-8"))}
        pyrs_doc = pyrs_yaml.parse(text)
        pyyaml_data = yaml.safe_load(text)
        ruamel_data = RUAMEL.load(text)
        cases = {
            "parse": {
                "pyrs-yaml": lambda t=text: pyrs_yaml.parse(t),
                "PyYAML": lambda t=text: yaml.safe_load(t),
                "ruamel.yaml": lambda t=text: RUAMEL.load(t),
            },
            "serialize": {
                "pyrs-yaml": lambda doc=pyrs_doc: doc.to_yaml(),
                "PyYAML": lambda data=pyyaml_data: yaml.safe_dump(data),
                "ruamel.yaml": lambda data=ruamel_data: ruamel_dump(data),
            },
            "roundtrip": {
                "pyrs-yaml": lambda t=text: pyrs_yaml.parse(t).to_yaml(),
                "PyYAML": lambda t=text: yaml.safe_dump(yaml.safe_load(t)),
                "ruamel.yaml": lambda t=text: ruamel_dump(RUAMEL.load(t)),
            },
        }
        for operation, libraries in cases.items():
            operations[operation][size] = {name: measure(fn) for name, fn in libraries.items()}

    return {
        "generated_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "environment": {
            "python": platform.python_version(),
            "platform": platform.platform(),
            "machine": platform.machine(),
            "processor": platform.processor(),
        },
        "libraries": {
            "pyrs-yaml": pyrs_yaml.__version__,
            "PyYAML": package_version("PyYAML"),
            "ruamel.yaml": package_version("ruamel.yaml"),
        },
        "parameters": {"rounds": ROUNDS, "target_seconds": TARGET_SECONDS},
        "fixtures": fixtures,
        "operations": operations,
    }


def _format_time(microseconds):
    if microseconds >= 1000:
        milliseconds = microseconds / 1000
        return f"{milliseconds:.1f} ms" if milliseconds < 10 else f"{milliseconds:.0f} ms"
    if microseconds >= 10:
        return f"{microseconds:.0f} µs"
    if microseconds >= 1:
        return f"{microseconds:.1f} µs"
    return f"{microseconds:.2f} µs"


def _speedup(snapshot, operation, size, library):
    times = snapshot["operations"][operation][size]
    return round(times[library] / times["pyrs-yaml"])


def _size(label):
    """The fixture a localized row label names, e.g. "Small (~100 B)"."""
    match = re.search(r"([\d.]+)\s*(KB|B)", label)
    if match is None:
        raise ValueError(f"no fixture size in table row: {label!r}")
    value = float(match.group(1)) * (1024 if match.group(2) == "KB" else 1)
    return min(SIZES, key=lambda size: abs(len(FIXTURES[size].encode("utf-8")) - value))


def _tables(path):
    text = path.read_text(encoding="utf-8")
    lines = text.split("\n")
    blocks = []
    index = 0
    while index < len(lines) - 1:
        rule = lines[index + 1]
        if lines[index].startswith("|") and "-" in rule and set(rule) <= set("|-: "):
            start = index
            index += 2
            while index < len(lines) and lines[index].startswith("|"):
                index += 1
            blocks.append((start, index))
        else:
            index += 1
    return lines, blocks


def _split(line):
    return [part.strip() for part in line.strip().strip("|").split("|")]


def _join(cells):
    return "| " + " | ".join(cells) + " |"


def _write(path, lines):
    path.write_text("\n".join(lines), encoding="utf-8", newline="\n")


def _rewrite_benchmarks(snapshot, path):
    lines, blocks = _tables(path)
    tables = [block for block in blocks if len(_split(lines[block[0]])) == 5]
    if len(tables) != len(OPERATIONS):
        raise ValueError(f"{path}: expected {len(OPERATIONS)} five-column tables, found {len(tables)}")
    for operation, (start, end) in zip(OPERATIONS, tables):
        rows = range(start + 2, end)
        if len(rows) != len(SIZES):
            raise ValueError(f"{path}: expected {len(SIZES)} rows in the {operation} table, found {len(rows)}")
        for offset, size in zip(rows, SIZES):
            cells = _split(lines[offset])
            found = _size(cells[0])
            if found != size:
                raise ValueError(f"{path}: row {cells[0]!r} is {found}, expected {size}")
            times = snapshot["operations"][operation][size]
            cells[1] = _format_time(times["pyrs-yaml"])
            cells[2] = _format_time(times["PyYAML"])
            cells[3] = _format_time(times["ruamel.yaml"])
            cells[4] = f"**{_speedup(snapshot, operation, size, 'PyYAML')}\u00d7**"
            lines[offset] = _join(cells)
    _write(path, lines)


def _rewrite_comparison(snapshot, path):
    lines, blocks = _tables(path)
    tables = [block for block in blocks if len(_split(lines[block[0]])) == 3 and block[1] - block[0] == 5]
    if len(tables) != len(OPERATIONS):
        raise ValueError(f"{path}: expected {len(OPERATIONS)} performance tables, found {len(tables)}")
    for operation, (start, end) in zip(OPERATIONS, tables):
        for offset in range(start + 2, end):
            cells = _split(lines[offset])
            library = cells[0].replace("*", "")
            if library not in ("pyrs-yaml", "PyYAML", "ruamel.yaml"):
                raise ValueError(f"{path}: unexpected library row {cells[0]!r}")
            cells[1] = _format_time(snapshot["operations"][operation]["large"][library])
            cells[2] = (
                "—" if library == "pyrs-yaml" else f"{_speedup(snapshot, operation, 'large', library)}\u00d7 slower"
            )
            lines[offset] = _join(cells)
    _write(path, lines)


def _rewrite_features(snapshot, path):
    lines, blocks = _tables(path)
    tables = [
        block
        for block in blocks
        if len(_split(lines[block[0]])) == 3
        and "pyrs-yaml" in _split(lines[block[0]])
        and "PyYAML" in _split(lines[block[0]])
        and "ruamel.yaml" not in _split(lines[block[0]])
    ]
    if len(tables) != 1:
        raise ValueError(f"{path}: expected one pyrs-yaml/PyYAML table, found {len(tables)}")
    start, end = tables[0]
    rows = range(start + 2, end)
    if len(rows) != len(OPERATIONS):
        raise ValueError(f"{path}: expected {len(OPERATIONS)} rows, found {len(rows)}")
    for offset, operation in zip(rows, OPERATIONS):
        cells = _split(lines[offset])
        cells[1] = _format_time(snapshot["operations"][operation]["large"]["pyrs-yaml"])
        cells[2] = _format_time(snapshot["operations"][operation]["large"]["PyYAML"])
        lines[offset] = _join(cells)
    _write(path, lines)


def rewrite_docs(snapshot):
    for path in PAGES:
        if path.name == "benchmarks.md":
            _rewrite_benchmarks(snapshot, path)
        elif path.name == "comparison.md":
            _rewrite_comparison(snapshot, path)
        else:
            _rewrite_features(snapshot, path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--update", action="store_true", help=f"write {SNAPSHOT.relative_to(ROOT)}")
    parser.add_argument(
        "--docs",
        action="store_true",
        help=f"rewrite the wall-clock tables in {len(PAGES)} pages from {SNAPSHOT.relative_to(ROOT)}",
    )
    args = parser.parse_args()

    if args.docs:
        rewrite_docs(json.loads(SNAPSHOT.read_text(encoding="utf-8")))
        print(f"rewrote tables in {len(PAGES)} pages")
        return

    snapshot = collect()
    if args.update:
        SNAPSHOT.write_text(json.dumps(snapshot, indent=2) + "\n", encoding="utf-8", newline="\n")
        print(f"wrote {SNAPSHOT.relative_to(ROOT)}")
        return

    for operation, sizes in snapshot["operations"].items():
        for size, libraries in sizes.items():
            cells = "  ".join(f"{name} {value:.3f} us" for name, value in libraries.items())
            print(f"{operation:9s} {size:6s} {cells}")


if __name__ == "__main__":
    main()
