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
| [`sort-keys`](#cmd-edit) | Sort mapping keys at a path |
| [`move`](#cmd-edit) | Move a subtree to another existing path |
| [`frontmatter`](#cmd-frontmatter) | Extract Markdown front matter as YAML |
| [`validate`](#cmd-validate) | Validate YAML against a schema |
| [`to-json`](#cmd-convert) | Convert YAML to JSON |
| [`from-json`](#cmd-convert) | Convert JSON to YAML |
| [`compliance`](#cmd-compliance) | Report YAML Test Suite compliance |

Every command reads from **stdin** when the file argument is `-` or omitted, and writes to **stdout** unless `-o/--output` or `-i/--inplace` says otherwise. Stream-shaped input is handled with [`-A/--all-docs`](#multi-doc).

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

pyrs-yaml sort-keys config.yaml                               # sort root mapping keys
pyrs-yaml sort-keys config.yaml "$.meta"                      # sort one nested mapping
pyrs-yaml move deploy.yaml "$.staging" "$.environments.dev"   # relocate a subtree
```

Edits preserve surrounding metadata — a comment above or beside the edited node stays put.

Notes:

- `set` adds the final key of a path even without `--create-missing` when its parent exists; the flag is only needed for missing *intermediate* keys.
- `sort-keys` orders the keys of the mapping at `path` (default root); it is not recursive.
- `move`'s destination must already exist and its value is replaced by the moved subtree; wildcards are rejected on both ends.

### Validation (`validate`) { #cmd-validate }

`validate` checks a document against a schema definition file or a registered schema name — the two options are mutually exclusive:

```bash
pyrs-yaml validate app.yaml --schema-file schema.yaml
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

### Multi-document streams { #multi-doc }

Add `-A/--all-docs` to treat input as a stream of `---`-separated documents instead of just the first one:

```bash
pyrs-yaml fmt stream.yaml -A                              # reformat every document
pyrs-yaml get stream.yaml '$..name' --format text -A      # query across documents
pyrs-yaml to-json stream.yaml -A                          # JSON array of documents
pyrs-yaml set stream.yaml "$.retries" 5 -A                # edit every document
pyrs-yaml validate stream.yaml --schema-file s.yaml -A    # failures report "document N"
```

Supported by `fmt`, `get`, `set`, `delete`, `rename`, `sort-keys`, `validate`, and `to-json`. Outputs are joined with standard `---` separators; edit commands apply where the path resolves and fail only when no document matches.

### Markdown front matter (`frontmatter`) { #cmd-frontmatter }

```bash
$ pyrs-yaml frontmatter post.md
title: Hello

$ pyrs-yaml frontmatter post.md --body-out body.md   # also split out the body
```

Exits `1` when the page has no front matter. See [Markdown Frontmatter](frontmatter.md) for the library API.

### YAML Test Suite compliance (`compliance`) { #cmd-compliance }

```bash
pyrs-yaml compliance [--json] [SUITE_DIR]
```

Runs the parser against the [yaml-test-suite](https://github.com/yaml/yaml-test-suite) corpus (default checkout location: `./Reference/yaml-test-suite`) and prints pass/fail statistics per suite section — useful when evaluating pyrs-yaml against other YAML implementations.

### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Runtime error — unreadable input, parse failure, no match, validation failure |
| `2` | Usage error — unknown command or option |

!!! tip "Scripting"
    Because errors go to stderr and data to stdout, `pyrs-yaml` composes cleanly: `pyrs-yaml get deploy.yaml '$..host' | sort -u`.
