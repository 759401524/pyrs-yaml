---
title: Command-Line Interface
description: Use the pyrs-yaml CLI to format, query, edit, validate, and convert YAML files from the terminal with round-trip fidelity.
tags:
  - docs
status: new
---

## Command-Line Interface

pyrs-yaml ships an optional command-line tool, `pyrs-yaml`, that exposes the library's core capabilities — round-trip formatting, JSONPath queries, in-place editing, schema validation, and format conversion — directly in your terminal.

!!! note "Requirements"
    The CLI requires the optional `cli` extra and **Python >= 3.10**. The library itself keeps supporting older interpreters.

### Installation

=== "pip"

    ```bash
    pip install "pyrs-yaml[cli]"
    ```

=== "uv"

    ```bash
    uv add --optional cli pyrs-yaml
    ```

Verify the installation:

```bash
pyrs-yaml --version
```

### Command Overview

| Command | Purpose |
|---------|---------|
| [`fmt`](#cmd-fmt) | Reformat YAML preserving comments, anchors, and order |
| [`get`](#cmd-get) | Query values by JSONPath expression |
| [`set`](#cmd-edit) | Set a value at a path |
| [`delete`](#cmd-edit) | Remove a node at a path |
| [`rename`](#cmd-edit) | Rename a mapping key |
| [`validate`](#cmd-validate) | Validate YAML against a schema |
| [`to-json`](#cmd-convert) | Convert YAML to JSON |
| [`from-json`](#cmd-convert) | Convert JSON to YAML |

Every command reads from **stdin** when the file argument is `-` or omitted, and writes to **stdout** unless `-o/--output` or `-i/--inplace` says otherwise.

### Formatting (`fmt`) { #cmd-fmt }

`fmt` re-serializes a document through the round-trip AST — comments, anchors, key order, and styles survive:

```bash
$ echo "a:    1 # keep me" | pyrs-yaml fmt -
a: 1  # keep me
```

Useful options:

```bash
pyrs-yaml fmt config.yaml --indent 4        # 4-space indentation
pyrs-yaml fmt config.yaml --inplace         # rewrite the file in place (-i)
pyrs-yaml fmt config.yaml -o formatted.yaml # write to another file
```

### Querying (`get`) { #cmd-get }

`get` evaluates a [JSONPath](editing.md#path-syntax)-style expression and prints each match:

```bash
$ pyrs-yaml get deploy.yaml '$.servers[0].host'
db.example.com

$ pyrs-yaml get deploy.yaml '$..name' --format text   # deep scan
web
db

$ pyrs-yaml get deploy.yaml '$.servers[*]'            # subtrees as YAML (default)
```

Output formats via `--format/-f`: `yaml` (default), `json`, or `text` (raw scalar values).

### Editing (`set`, `delete`, `rename`) { #cmd-edit }

Editing commands target exactly one node per path (wildcards are rejected):

```bash
# VALUE is parsed as YAML — numbers, bools, and nested structures just work
pyrs-yaml set config.yaml "$.retries" 5
pyrs-yaml set config.yaml "$.tags" '[a, b]'
pyrs-yaml set config.yaml "$.token" '12345' --string          # force string
pyrs-yaml set config.yaml "$.a.b.c" new --create-missing      # create parents

pyrs-yaml delete config.yaml "$.legacy_key"
pyrs-yaml rename config.yaml "$.old_name" new_name

pyrs-yaml set config.yaml "$.port" 8080 --inplace             # edit file in place
```

Edits preserve surrounding metadata — a comment above or beside the edited node stays put.

### Validation (`validate`) { #cmd-validate }

`validate` checks a document against a schema definition (file path) or a registered schema name:

```bash
pyrs-yaml validate app.yaml --schema schema.yaml
pyrs-yaml validate app.yaml --schema my_schema        # registered via register_schema()
```

The command is silent on success and exits `0`; on failure every violation is printed to stderr and the exit code is `1` — handy in CI:

```yaml
# schema.yaml
name: app
extends: core
validate:
  - path: $.port
    type: int
    required: true
```

See [Custom Schemas](custom-schema.md) for the full schema language.

### Conversion (`to-json`, `from-json`) { #cmd-convert }

Both directions compose naturally in pipelines:

```bash
$ pyrs-yaml to-json config.yaml
{
  "b": {
    "c": 2
  }
}

$ echo '{"name": "x"}' | pyrs-yaml from-json -
name: x
```

### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Runtime error — unreadable input, parse failure, no match, validation failure |
| `2` | Usage error — unknown command or option |

!!! tip "Scripting"
    Because errors go to stderr and data to stdout, `pyrs-yaml` composes cleanly: `pyrs-yaml get deploy.yaml '$..host' | sort -u`.
