---
title: TOML, JSON, CSV/TSV & INI Formats
description: Exchange TOML, JSON, CSV/TSV and INI with pyrs-yaml while keeping YAML as the single editable representation.
tags:
  - docs
status: new
---

Read and write TOML, JSON, CSV/TSV and INI — with YAML as the hub.

## The Hub Model

pyrs-yaml speaks several config formats, but **YAML is the only editable
representation**: round-trip editing (comments, anchors, in-place splices)
is YAML-exclusive by design. The exchange formats convert in and out:

```text
load_toml / load_ini / load_csv / JSON text ──▶ values & YAML ──▶ parse() / edit() / dump
to_toml  ◀── YAML text ◀──────────────────────────────┘
```

## JSON: Native Already

YAML 1.2 is a superset of JSON, so every JSON document loads everywhere a
YAML document does — no separate entry point:

```python title="JSON input"
import pyrs_yaml

data = pyrs_yaml.safe_load('{"a": [1, 2], "b": true}')
# {'a': [1, 2], 'b': True}
```

The dedicated `json` schema (see [Custom Schemas](custom-schema.md)) also
restricts scalar resolution to JSON-compatible rules, and
`from_json` / the CLI `to-json` convert explicitly.

### Line-delimited JSON: `load_json_lines` { #json-lines }

A `.jsonl` / NDJSON file is not one JSON document, so nothing that takes a
document reads it — `load_json` over a two-record file refuses the second line
as trailing characters. `load_json_lines` is the route that does, and it returns
a `JsonLines` iterator that resolves one document per `__next__`, so only one
line is live at a time:

```python title="JSON Lines"
import pyrs_yaml

records = pyrs_yaml.load_json_lines('{"a": 1}\n{"b": 2}\n')
next(records)  # {'a': 1}
list(pyrs_yaml.load_json_lines('{"a": 1}\n{"b": 2}\n'))
# [{'a': 1}, {'b': 2}]
```

Each value is what `load_json` returns for that line, and the stream keeps the
engine's contract: a line the format does not define raises `YamlParseError`
naming its position in the whole input, and the cursor does not move, so asking
again reports the same record rather than the one after it.

```python title="What a refused line says"
list(pyrs_yaml.load_json_lines('{"a": 1}\n\n'))
# pyrs_yaml.YamlParseError: a blank line carries no document in
# a line-delimited JSON stream at line 2 column 1

next(pyrs_yaml.load_json_lines("// note\n"))
# pyrs_yaml.YamlParseError: expected a JSON value at line 1 column 1

next(pyrs_yaml.load_json_lines('{"a": 1,\n'))
# pyrs_yaml.YamlParseError: expected a quoted object key at line 1 column 9
```

A blank line is refused wherever it sits, including the extra one a file ends
with, because a stream that answered `[]` for `{"a": 1}\n\n` would be guessing
which line the author meant to drop. A line carrying one complete value is
required and sufficient: no separator, no trailing comma, one document per line.

## TOML

Three functions, mirroring the JSON conversion family:

```python title="TOML in, TOML out"
import pyrs_yaml

# TOML text -> Python values (fast path, no intermediate document)
config = pyrs_yaml.load_toml('s = "true"\nn = 42\n')
# {'s': 'true', 'n': 42}   <- TOML strings never re-resolve: "true" stays a string

# TOML text -> YAML text (then edit as usual)
yaml_text = pyrs_yaml.from_toml('title = "app"\nport = 8080\n')
# 'title: "app"\nport: 8080\n'

# YAML text -> TOML text
toml_text = pyrs_yaml.to_toml("name: app\ncount: 3\nnested:\n  a: 1\n  b: two\n")
# 'name = "app"\ncount = 3\nnested = { a = 1, b = "two" }\n'
```

!!! note "Datetimes"

    TOML datetimes arrive as real `datetime.datetime` objects through the
    built-in `!!timestamp` plugin:

    ```python
    pyrs_yaml.load_toml("when = 2026-01-02T03:04:05Z\n")
    # {'when': datetime.datetime(2026, 1, 2, 3, 4, 5, tzinfo=datetime.timezone.utc)}
    ```

### What `to_toml` rejects

TOML cannot express some YAML shapes; these raise `ValueError` with stable
messages instead of silently losing data:

- null values (TOML has no null)
- non-table documents (scalars or arrays at the root)
- anchors / aliases and tags other than `!!timestamp`
- non-scalar mapping keys

## CSV / TSV

A table, not a document: the header row names the columns, and every data
record becomes a mapping. Cells are typed against the YAML core schema, so
`42` is an integer and `true` a boolean — while the cell's **source text**
is what travels, so a CSV → YAML → CSV round trip reproduces the original
bytes:

```python title="CSV input"
import pyrs_yaml

table = pyrs_yaml.load_csv("name,port\nweb-1,8080\n")
# [{'name': 'web-1', 'port': 8080}]

yaml_text = pyrs_yaml.from_csv("name,port\nweb-1,8080\n")
# - name: web-1\n  port: 8080\n
```

Behavior notes:

- the delimiter is `,` by default and `\t` for TSV; `;` and `|` are accepted
  too (`delimiter=` on the Python entry points, `--input csv|tsv` on `pyq`);
- quoting follows RFC 4180 — a quoted field may hold the delimiter, a line
  break, and `""` for one quote;
- a leading UTF-8 BOM is stripped, and CRLF and LF are both accepted;
- an empty cell is null; a cell with a leading zero keeps its spelling
  (`007` stays `007`) and resolves to an integer;
- the engine's own round trip is byte-for-byte; through the YAML *text* hop
  three things normalize — CRLF becomes LF, a BOM is stripped and not written
  back, and an empty cell and a cell spelled `null` are the same document,
  because YAML has one null spelling;
- quoting is syntax here, not a type assertion: a cell written `"true"` is inferred as a
  boolean, so quoting is not preserved through the text hop. Use `--no-infer` (or
  `infer=False`) when every cell must stay a string; the same applies to a string that
  looks like a number or a date;
- a row whose cell count differs from the header's, and a header that names
  the same column twice, are **refused with a position**, never padded or
  silently repaired;
- TSV is the same engine with a tab delimiter, not a second format.

Writing goes back the same way, and refuses what CSV cannot hold — a nested
mapping inside a cell has no CSV spelling:

```python title="YAML to CSV"
import pyrs_yaml

csv_text = pyrs_yaml.to_csv("- name: web-1\n  port: 8080\n")
# name,port\nweb-1,8080\n
```

## INI

Read-only by design — INI has no official grammar, so pyrs-yaml ingests it
via the standard library parser and leaves writing to your tools of choice:

```python title="INI input"
import pyrs_yaml

config = pyrs_yaml.load_ini("[server]\nHost = 127.0.0.1\nPort = 8080\n")
# {'server': {'Host': '127.0.0.1', 'Port': '8080'}}
```

Behavior notes:

- key case is preserved (`Host`, not `host`);
- sections, `;`/`#` comments and multiline values follow
  `configparser.RawConfigParser` semantics in strict mode —
  duplicate keys or missing section headers raise `ValueError`;
- all values are strings; convert types yourself or route the data
  through YAML first:

```python title="INI to YAML"
import pyrs_yaml

yaml_text = pyrs_yaml.safe_dump(pyrs_yaml.load_ini("[s]\nport = 8080\n"))
# s:\n  port: '8080'\n
```

## See Also

- [Custom Schemas](custom-schema.md) — the `json` scalar-resolution schema
- [Command-Line Interface](cli.md) — `to-json` / `from-json` / `to-csv` / `from-csv` subcommands
- [Round-Trip Preservation](round-trip.md) — what makes YAML the editing hub
