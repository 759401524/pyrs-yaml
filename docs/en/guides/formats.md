---
title: TOML, JSON & INI Formats
description: Exchange TOML, JSON and INI with pyrs-yaml while keeping YAML as the single editable representation.
tags:
  - docs
status: new
---

Read and write TOML, JSON and INI — with YAML as the hub.

## The Hub Model

pyrs-yaml speaks several config formats, but **YAML is the only editable
representation**: round-trip editing (comments, anchors, in-place splices)
is YAML-exclusive by design. The exchange formats convert in and out:

```text
load_toml / load_ini / JSON text ──▶ values & YAML ──▶ parse() / edit() / dump
to_toml  ◀── YAML text ◀──────────────────┘
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
    built-in `!timestamp` plugin:

    ```python
    pyrs_yaml.load_toml("when = 2026-01-02T03:04:05Z\n")
    # {'when': datetime.datetime(2026, 1, 2, 3, 4, 5, tzinfo=datetime.timezone.utc)}
    ```

### What `to_toml` rejects

TOML cannot express some YAML shapes; these raise `ValueError` with stable
messages instead of silently losing data:

- null values (TOML has no null)
- non-table documents (scalars or arrays at the root)
- anchors / aliases and tags other than `!timestamp`
- non-scalar mapping keys

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
- [Command-Line Interface](cli.md) — `to-json` / `from-json` subcommands
- [Round-Trip Preservation](round-trip.md) — what makes YAML the editing hub
