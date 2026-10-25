#!/usr/bin/env python3
"""Measure the cross-library benchmarks and record them in .ci/perf-snapshot.json.

Batch timing is used because a single call to the Rust-backed reader/writer can be
faster than the clock resolution: each case is measured as several whole batches of
many calls, and the reported value is the fastest per-call time across batches.

The snapshot has a format axis. YAML is measured on parse, serialize and round-trip
against PyYAML, ruamel.yaml and the Rust-backed `yaml-rs` and `ryaml`; JSON on parse against
`json` and `orjson`; TOML on parse against `tomllib` and `rtoml`; JSON5 on parse against
`pyjson5`. A write column is published only
where a counterparty does the same job from the same starting object: pyrs emits
JSON/TOML/JSON5 from its own AST rather than from a Python object, so `json.dumps` and
`tomli_w.dumps` are a different operation, and those writer halves stay priced by the
committed instruction floors in `.ci/ir-baseline.json`. The snapshot names them in
`unpublished_writers` so the absence is a record, not a silence.

Every case also records whether the columns decode to the same object (`same_object`).
Where they do not - some reader types a scalar differently on the large
fixture - the row still times the same bytes through every reader, and the disagreement
is the fidelity fact the round-trip suite already owns rather than a reason to drop it.

`--docs` rewrites the wall-clock tables on the four locales' performance and feature
pages from the snapshot. Each generated table carries a `<!-- bench: ... -->` marker that
names its format, operation, fixtures and speedup baseline, so a table is bound to a
measurement by its own declaration instead of by counting columns.
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

try:  # Counterparty readers: needed by --update only, and marker-bounded off several matrix legs.
    import orjson
    import pyjson5
    import rtoml
    import ryaml
    import tomllib
    import yaml_rs
except ImportError:  # pragma: no cover - every leg has to be able to import this script
    orjson = pyjson5 = rtoml = ryaml = tomllib = yaml_rs = None

#: The peers `--update` measures, bound to `None` where an interpreter cannot import them;
#: `_require_counterparties` turns that into a refusal at the point the work is asked for.
COUNTERPARTIES = {
    "orjson": orjson,
    "pyjson5": pyjson5,
    "rtoml": rtoml,
    "ryaml": ryaml,
    "tomllib": tomllib,
    "yaml_rs": yaml_rs,
}

import pyrs_yaml  # noqa: E402
from tests.data.yaml_samples import (  # noqa: E402
    BENCHMARK_CONFIG_JSON,
    BENCHMARK_CONFIG_JSON5,
    BENCHMARK_CONFIG_JSON_LARGE,
    BENCHMARK_CONFIG_TOML,
    BENCHMARK_LARGE,
    BENCHMARK_MEDIUM,
    BENCHMARK_SMALL,
    BENCHMARK_TOML_HOT,
)

SNAPSHOT = ROOT / ".ci" / "perf-snapshot.json"
LOCALES = ("en", "zh", "ja", "ko")
PAGES = tuple(
    ROOT / "docs" / locale / name
    for locale in LOCALES
    for name in ("performance/benchmarks.md", "performance/comparison.md", "features.md")
)
ROUNDS = 9
TARGET_SECONDS = 0.1
MAX_BATCH = 500_000
SELF = "pyrs-yaml"
RUAMEL = YAML()


def ruamel_dump(data):
    stream = io.StringIO()
    RUAMEL.dump(data, stream)
    return stream.getvalue()


#: One entry per published format. A library names the reader that turns the fixture text
#: into the object it starts from, and the writer that turns it back - `write` is present
#: only where that library can do the outbound half, which is what keeps a published
#: serialize column a like-for-like race.
FORMATS = {
    "yaml": {
        "fixtures": {"small": BENCHMARK_SMALL, "medium": BENCHMARK_MEDIUM, "large": BENCHMARK_LARGE},
        "columns": {
            SELF: {"read": pyrs_yaml.parse, "write": lambda doc: doc.to_yaml()},
            "PyYAML": {"read": yaml.safe_load, "write": yaml.safe_dump},
            "ruamel.yaml": {"read": RUAMEL.load, "write": ruamel_dump},
            # The two Rust-backed Python readers. They belong in the same column group because
            # they do the same job on the same bytes, and the headline ratio stays PyYAML's so the
            # published rows remain comparable with the ones they replace. Their attributes are read
            # inside a lambda, like every other optional peer: the module has to import on a leg
            # where the marker kept it out, and `None.loads` would not let it.
            "yaml-rs": {"read": lambda text: yaml_rs.loads(text), "write": lambda doc: yaml_rs.dumps(doc)},
            "ryaml": {"read": lambda text: ryaml.load(io.StringIO(text)), "write": lambda doc: ryaml.dumps(doc)},
        },
        "operations": ("parse", "serialize", "roundtrip"),
    },
    "json": {
        "fixtures": {"config": BENCHMARK_CONFIG_JSON, "large": BENCHMARK_CONFIG_JSON_LARGE},
        "columns": {
            SELF: {"read": pyrs_yaml.load_json},
            "json": {"read": json.loads},
            "orjson": {"read": lambda text: orjson.loads(text)},
        },
        "operations": ("parse",),
    },
    "toml": {
        "fixtures": {"hot": BENCHMARK_TOML_HOT, "config": BENCHMARK_CONFIG_TOML},
        "columns": {
            SELF: {"read": pyrs_yaml.load_toml},
            "tomllib": {"read": lambda text: tomllib.loads(text)},
            # `rtoml` is the `toml-rs` crate's binding, so it is the only TOML counterparty that is
            # Rust against Rust; the speedup column is measured against it rather than against the
            # standard library, because beating a pure-Python reader is not the interesting claim.
            "rtoml": {"read": lambda text: rtoml.loads(text)},
        },
        "operations": ("parse",),
    },
    "json5": {
        "fixtures": {"config": BENCHMARK_CONFIG_JSON5},
        "columns": {
            SELF: {"read": pyrs_yaml.load_json5},
            "pyjson5": {"read": lambda text: pyjson5.loads(text)},
        },
        "operations": ("parse",),
    },
}

STDLIB = ("json", "tomllib")

VERSIONS = {
    SELF: lambda: pyrs_yaml.__version__,
    "PyYAML": lambda: package_version("PyYAML"),
    "ruamel.yaml": lambda: package_version("ruamel.yaml"),
    "yaml-rs": lambda: package_version("yaml-rs"),
    "ryaml": lambda: package_version("ryaml"),
    "orjson": lambda: package_version("orjson"),
    "pyjson5": lambda: package_version("pyjson5"),
    "rtoml": lambda: package_version("rtoml"),
}


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


def warm_up(seconds=1.5):
    end = time.perf_counter() + seconds
    value = 0
    while time.perf_counter() < end:
        value = (value * 1103515245 + 12345) & 0x7FFFFFFF
    return value


def _work(column, operation, text):
    """The callable a column is timed on for one operation, with its setup already applied."""
    read, write = column["read"], column.get("write")
    if operation == "parse":
        return lambda: read(text)
    if operation == "serialize":
        if write is None:
            raise SystemExit(f"{operation} needs a write column; {column!r} has none")
        document = read(text)
        return lambda: write(document)
    if operation == "roundtrip":
        if write is None:
            raise SystemExit(f"{operation} needs a write column; {column!r} has none")
        return lambda: write(read(text))
    raise SystemExit(f"unknown operation {operation!r}")


def _decoded(column, text):
    return column["read"](text)


def _same_object(columns, text):
    """Whether every column decodes the fixture to an equal object, compared as plain data."""
    values = [_plain(_decoded(column, text)) for column in columns.values()]
    return all(value == values[0] for value in values[1:])


def _plain(value):
    to_plain = getattr(value, "to_dict", None)
    if to_plain is not None:
        value = to_plain()
    if isinstance(value, dict):
        return {key: _plain(item) for key, item in value.items()}
    if isinstance(value, list):
        return [_plain(item) for item in value]
    return value


def _require_counterparties():
    """Stop a re-measure that would publish a table with a hole in it, and name what is missing.

    This asks rather than raising at import, because `tests/test_scripts_import_on_supported_python.py`
    imports this file on every leg the matrix runs and the peers are marker-bounded - no `tomllib`
    below 3.11, no `orjson` at 3.14, no `yaml-rs` at 3.15 - so an import-time refusal is five red jobs
    for a command nobody ran. `--docs` reads the committed snapshot and needs none of them.
    """
    missing = sorted(name for name, module in COUNTERPARTIES.items() if module is None)
    if missing:
        raise SystemExit(
            f"--update measures every published column, and the counterparty readers are not all importable "
            f"on this interpreter (left unbound: {', '.join(missing)}); run it where `uv sync --group test` "
            "installs them together, rather than publishing a partial table"
        )


def collect():
    _require_counterparties()
    warm_up()
    formats = {}
    libraries = {}
    for format_name, spec in FORMATS.items():
        columns = spec["columns"]
        operations = {operation: {} for operation in spec["operations"]}
        fixtures = {}
        same = {}
        for size, text in spec["fixtures"].items():
            fixtures[size] = {"bytes": len(text.encode("utf-8"))}
            for operation in spec["operations"]:
                work = {name: _work(column, operation, text) for name, column in columns.items()}
                operations[operation][size] = {name: measure(fn) for name, fn in work.items()}
            same[size] = _same_object(columns, text)
        formats[format_name] = {"fixtures": fixtures, "same_object": same, "operations": operations}
        for name in columns:
            if name in STDLIB:
                libraries[name] = f"stdlib (Python {platform.python_version()})"
            else:
                libraries[name] = VERSIONS[name]()
    return {
        "generated_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "environment": {
            "python": platform.python_version(),
            "platform": platform.platform(),
            "machine": platform.machine(),
            "processor": platform.processor(),
        },
        "libraries": libraries,
        "parameters": {"rounds": ROUNDS, "target_seconds": TARGET_SECONDS},
        "formats": formats,
        "unpublished_writers": sorted(
            name for name, spec in FORMATS.items() if not any("write" in c for c in spec["columns"].values())
        ),
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


def _speedup(times, library):
    """Whole multiples above ten, one decimal below it.

    Rounding to an integer is what the YAML tables always did, and at 20x to 70x it reads true.
    On the JSON and JSON5 rows the same rounding would print a 1.5x win as 1x and a 0.65x loss
    as 0x, which is not a measurement, so the small end keeps its first decimal.
    """
    value = times[library] / times[SELF]
    return f"{round(value)}\u00d7" if value >= 10 else f"{value:.1f}\u00d7"


MARKER = re.compile(r"<!--\s*bench:\s*(?P<body>[^>]*?)\s*-->")


def _attrs(body):
    pairs = dict(item.split("=", 1) for item in body.split() if "=" in item)
    for required in ("layout", "format", "operation"):
        if required not in pairs:
            raise ValueError(f"bench marker needs {required}=: {body!r}")
    return pairs


def _split(line):
    return [part.strip() for part in line.strip().strip("|").split("|")]


def _join(cells):
    return "| " + " | ".join(cells) + " |"


def _table_at(lines, start):
    """The table starting at `start`, as (header cells, indexes of its row lines)."""
    if start + 1 >= len(lines) or not set(lines[start + 1]) <= set("|-: ") or "-" not in lines[start + 1]:
        raise ValueError(f"no table under a bench marker: {lines[start]!r}")
    index = start + 2
    rows = []
    while index < len(lines) and lines[index].startswith("|"):
        rows.append(index)
        index += 1
    return _split(lines[start]), rows


def _times(snapshot, attrs):
    format_name = attrs["format"]
    if format_name not in snapshot["formats"]:
        raise ValueError(f"the snapshot records no format {format_name!r}")
    operations = snapshot["formats"][format_name]["operations"]
    operation = attrs["operation"]
    if operation not in operations:
        raise ValueError(f"{format_name} records no {operation!r} operation")
    return operations[operation]


def _rewrite_sizes(snapshot, attrs, lines, start):
    operation = _times(snapshot, attrs)
    sizes = attrs["sizes"].split(",")
    baseline = attrs["baseline"]
    speedup = attrs.get("speedup", "yes") == "yes"
    header, rows = _table_at(lines, start)
    if len(rows) != len(sizes):
        raise ValueError(
            f"{attrs['format']}/{attrs['operation']}: marker names {len(sizes)} fixtures, table has {len(rows)}"
        )
    libraries = header[1:-1] if speedup else header[1:]
    if speedup and baseline not in libraries:
        raise ValueError(f"{attrs['format']}/{attrs['operation']}: baseline {baseline} is not a time column")
    for index, size in zip(rows, sizes):
        if size not in operation:
            raise ValueError(f"{attrs['format']}/{attrs['operation']}: no measurement for {size!r}")
        cells = _split(lines[index])
        if len(cells) != len(header):
            raise ValueError(
                f"{attrs['format']}/{attrs['operation']}/{size}: {len(cells)} cells, header has {len(header)}"
            )
        for column, library in enumerate(libraries, start=1):
            cells[column] = _format_time(operation[size][library])
        if speedup:
            cells[-1] = f"**{_speedup(operation[size], baseline)}**"
        lines[index] = _join(cells)


def _rewrite_libraries(snapshot, attrs, lines, start):
    operation = _times(snapshot, attrs)
    size = attrs["size"]
    times = operation[size]
    # The header names the value columns; the rows are the libraries themselves.
    _, rows = _table_at(lines, start)
    if len(rows) != len(times):
        raise ValueError(
            f"{attrs['format']}/{attrs['operation']}/{size}: table has {len(rows)} rows, snapshot {len(times)}"
        )
    for index in rows:
        cells = _split(lines[index])
        library = cells[0].replace("*", "")
        if library not in times:
            raise ValueError(f"{attrs['format']}/{attrs['operation']}: table row {library!r} is not measured")
        cells[1] = _format_time(times[library])
        cells[2] = "—" if library == SELF else _behind(times, library)
        lines[index] = _join(cells)


def _behind(times, library):
    """How far the peer sits from us, in the direction the number actually points.

    The column always said "slower", which was true while every counterparty on this page was
    Python. A Rust-backed reader can be the faster one, and a fraction printed beside "slower" is
    not a measurement - so the word follows the ratio, and the magnitude is quoted from the side it
    belongs to.
    """
    ratio = times[library] / times[SELF]
    if ratio >= 1:
        return f"{_speedup(times, library)} slower"
    value = 1 / ratio
    return f"{round(value)}× faster" if value >= 10 else f"{value:.1f}× faster"  # noqa: RUF001


def _rewrite_operations(snapshot, attrs, lines, start):
    operations = {name: _times(snapshot, {**attrs, "operation": name}) for name in attrs["operation"].split(",")}
    size = attrs["size"]
    header, rows = _table_at(lines, start)
    names = attrs["operation"].split(",")
    if len(rows) != len(names):
        raise ValueError(f"{attrs['format']}: marker names {len(names)} operations, table has {len(rows)}")
    for index, name in zip(rows, names):
        cells = _split(lines[index])
        for column, library in enumerate(header[1:], start=1):
            cells[column] = _format_time(operations[name][size][library])
        lines[index] = _join(cells)
    return names


def rewrite_docs(snapshot):
    writers = {"sizes": _rewrite_sizes, "libraries": _rewrite_libraries, "operations": _rewrite_operations}
    for path in PAGES:
        lines = path.read_text(encoding="utf-8").split("\n")
        found = 0
        for index, line in enumerate(lines):
            match = MARKER.search(line)
            if not match:
                continue
            cursor = index + 1
            while cursor < len(lines) and not lines[cursor].strip():
                cursor += 1
            attrs = _attrs(match.group("body"))
            writer = writers.get(attrs["layout"])
            if writer is None:
                raise ValueError(f"unknown layout {attrs['layout']!r} in {path.name}")
            writer(snapshot, attrs, lines, cursor)
            found += 1
        if not found:
            raise ValueError(f"{path}: no bench markers, so its tables are no longer generated")
        path.write_text("\n".join(lines), encoding="utf-8", newline="\n")
        print(f"{path.relative_to(ROOT)}: {found} table(s)")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--update", action="store_true", help=f"write {SNAPSHOT.relative_to(ROOT)}")
    parser.add_argument(
        "--docs",
        action="store_true",
        help=f"rewrite the wall-clock tables in {len(PAGES)} pages from {SNAPSHOT.relative_to(ROOT)}",
    )
    parser.add_argument("--matrix", action="store_true", help="print the measured format/operation matrix")
    parser.add_argument("--check", action="store_true", help="import the counterparty readers, then exit")
    args = parser.parse_args()

    if args.docs:
        rewrite_docs(json.loads(SNAPSHOT.read_text(encoding="utf-8")))
        return

    if args.check:
        for format_name, spec in FORMATS.items():
            print(f"{format_name:6s} {len(spec['fixtures'])} fixture(s), operations: {', '.join(spec['operations'])}")
        return

    snapshot = collect()
    if args.update:
        SNAPSHOT.write_text(json.dumps(snapshot, indent=2) + "\n", encoding="utf-8", newline="\n")
        print(f"wrote {SNAPSHOT.relative_to(ROOT)}")
        return

    if args.matrix:
        for format_name, data in snapshot["formats"].items():
            for operation, sizes in data["operations"].items():
                columns = ", ".join(sorted(next(iter(sizes.values()))))
                print(f"{format_name:6s} {operation:10s} {len(sizes)} fixture(s): {columns}")
        print("writer halves not published for:", ", ".join(snapshot["unpublished_writers"]))
        return

    for format_name, data in snapshot["formats"].items():
        for operation, sizes in data["operations"].items():
            for size, libraries in sizes.items():
                cells = "  ".join(f"{name} {value:.3f} us" for name, value in libraries.items())
                print(f"{format_name:6s} {operation:9s} {size:7s} {cells}")


if __name__ == "__main__":
    main()
