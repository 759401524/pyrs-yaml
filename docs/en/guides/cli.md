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

Every subcommand that reads YAML takes `--duplicate-keys` — `fmt`, `get`, `set`, `delete`, `rename`, `sort-keys`, `move`, `validate`, `to-json`, `to-toml`, `to-jsonc` and `to-json5`. It is the reader's answer when one mapping holds two keys that YAML counts as one: `as_is` (the default), `error`, `last_wins` or `first_wins`. A value outside the four is refused on stderr, naming them, with exit code `1`; the refusal to read a duplicate under `as_is` or `error` is `YamlDuplicateKeyError`.

```bash
$ pyrs-yaml get twins.yaml '$.a' --duplicate-keys last_wins    # the later pair wins
2

$ pyrs-yaml get twins.yaml '$.a' --duplicate-keys first_wins   # the earlier pair wins
1

$ pyrs-yaml get twins.yaml '$.a' --duplicate-keys newest       # not one of the four
error: unknown duplicate_keys "newest"; expected as_is, error, last_wins or first_wins
```

`twins.yaml` there holds the plain duplicate `a: 1` / `a: 2`, and reading it under `as_is` or `error` is the refusal named above. `last_wins` is what the library's `allow_duplicate_keys=True` means; see [Duplicate Keys](../features.md#duplicate-keys) for what `as_is` answers per identity class.

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

"Exactly one node per path" is one *pair*, and a document can hold two pairs spelling the same key — told apart only by the tag or anchor the reader keeps. `delete` is the command that asks what to do about it: `--on-duplicate` takes `one` (the default, the pair the path resolves to), `all` (every pair the key could answer with, so the name is gone from the output too) or `error` (refuse while more than one remains).

```bash
$ cat dup.yaml
a: 1
!!str a: 2
b: 3

$ pyrs-yaml delete dup.yaml '$.a' --on-duplicate all
b: 3

$ pyrs-yaml delete dup.yaml '$.a' --on-duplicate error
error: cannot delete '$.a': edit failed: duplicate-key-target

$ pyrs-yaml delete dup.yaml '$.a' --on-duplicate both
error: cannot delete '$.a': edit failed: unknown on_duplicate "both"; expected one, all or error
```

`--on-duplicate error` is a no-op on a document with one pair per key — the refusal fires only while a duplicate actually remains. `set`, `rename`, `sort-keys` and `move` keep the `one` behaviour and take no such flag.

Notes:

- `set` adds the final key of a path even without `--create-missing` when its parent exists; the flag is only needed for missing *intermediate* keys.
- `sort-keys` orders the keys of the mapping at `path` (default root); it is not recursive.
- `move`'s destination must already exist and its value is replaced by the moved subtree; wildcards are rejected on both ends.

### Validation (`validate`) { #cmd-validate }

`validate` checks a document against a JSON Schema file. `--schema` is an alias of `--schema-file`; with neither flag the command checks syntax only:

```bash
pyrs-yaml validate app.yaml --schema-file schema.json
pyrs-yaml validate app.yaml --schema schema.json        # the same parameter
```

Evaluation is delegated to Python's `jsonschema`, so the file is a standard JSON Schema document:

```json
{
  "type": "object",
  "properties": {"port": {"type": "integer"}},
  "required": ["port"]
}
```

The command is silent on success and exits `0`; on failure the reference engine's message is printed to stderr and the exit code is `1` — handy in CI. Measured: a passing document writes nothing to either stream and exits `0`, a document the schema refuses prints `error: 'port' is a required property` on stderr and exits `1`, and a document that does not parse prints `error: YAML parse error: mapping values are not allowed in this context at char 4 line 1 column 5` for the same exit code.

Two schema inputs answer outside that sentence, because evaluation is delegated to Python's `jsonschema` and nothing this project publishes installs it — the base dependency is `typing-extensions` alone and the extras are `cli`, `pydantic` and `settings`:

- a schema file that is not JSON — the YAML spelling a reader reaches for — is stopped by `json.loads` before the evaluator is consulted, so the command prints a traceback whose last line is `json.decoder.JSONDecodeError: Expecting value: line 1 column 1 (char 0)` and exits `1` (measured through the console script: stderr holds the traceback, stdout stays empty).
- a JSON schema that does reach the evaluator on an install without `jsonschema` ends the same way, as a `ModuleNotFoundError` traceback. That one was measured in-process with `sys.modules["jsonschema"] = None` — the reproduction the test suite uses — because the evaluator is installed in this development venv; the command's `except YamlValidateError` arm catches neither class, which is why the line is a traceback rather than `error: …`.

Both are registered boundaries in `.ci/known-residuals.json` (`api:validate-schema-text-is-json-only` and `api:validate-evaluator-is-not-installed-by-anything-we-ship`), not defects the CLI fixes; see [Exceptions — YamlValidateError](../api/exceptions.md#yamlvalidateerror).

See [Custom Schemas](custom-schema.md) for scalar-resolution schemas and the `Document.validate()` API.

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
pyrs-yaml move stream.yaml "$.src" "$.nested" -A          # move where both paths resolve
pyrs-yaml validate stream.yaml --schema-file s.yaml -A    # failures report "document N"
```

Supported by `fmt`, `get`, `set`, `delete`, `rename`, `sort-keys`, `move`, `validate`, and `to-json`. Outputs are joined with standard `---` separators; edit commands apply where the path resolves and fail only when no document matches.

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
