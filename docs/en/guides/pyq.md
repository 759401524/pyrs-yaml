---
title: pyq - the Rust CLI
description: jq/yq-style YAML, JSON, TOML and INI processing in Rust with pyrs-yaml-core - no Python required.
tags:
  - docs
status: new
---

`pyq` is the native Rust command-line tool: `pyrs-yaml-core` directly behind
a jq/yq-style interface, with no Python at runtime. It complements the
Python-based [pyrs-yaml CLI](cli.md); both share the same core, exit-code
and error-message semantics.

## Install

```bash
cargo install --path crates/pyrs-yaml-cli   # from a checkout
# or build in-tree:
cargo build -p pyrs-yaml-cli --release      # -> target/release/pyq
```

## Query (jq-style)

```bash
# JSONPath-lite: dot keys, [n], [-n] (python-style), ['key'], [*]
# a missing leading dot is fine, and `.` selects the whole document
$ pyq get '.servers[-1].host' inventory.yaml
web-3

# wildcards expand to ALL matches, streamed like jq: one YAML document
# each (or one JSON value per line with --json)
$ pyq get '.servers[*].port' --json inventory.yaml
8080
8081

# stdin when the file is `-` or omitted; --raw for bare scalars
$ cat services.yaml | pyq get --raw .db.pool.size
20

# JSON output (order-preserving)
$ pyq get '.servers' --json services.yaml
[ { "host": "web-1", "port": 8080 }, ... ]
```

## Filter verbs (jq-style post-processing)

Structured flags, not an expression language - applied to the match
stream in this fixed order regardless of flag order on the command line:
`select -> sort -> unique -> slice`, then `join`.

```bash
pyq get '.servers[*]' --select 'port >= 1000' services.yaml
pyq get '.servers[*]' --sort-by host --desc services.yaml
pyq get '.tags[*]' --unique --skip 2 --take 5 blob.yaml
pyq get '.hosts[*]' --join ',' --raw inventory.yaml   # one bare line
```

| Flag | jq equivalent | Notes |
|------|---------------|-------|
| `--select 'PATH OP LITERAL'` | `select(.PATH OP LITERAL)` | OP `== != > >= < <=`; literal is YAML; missing path or mixed kinds compare false (no jq total order) |
| `--sort-by PATH` / `--desc` | `sort_by(.PATH)` | stable; missing key sorts last |
| `--unique` | `unique` | dedup after sorting, like jq |
| `--first` / `--last` | `.[0]` / `.[-1]` | mutually exclusive |
| `--skip N` / `--take N` | `.[N:][…]` | slice the stream |
| `--join SEP` | `join(SEP)` | all-scalar streams only |

## Edit (yq-style)

```bash
# values are YAML expressions (JSON works: YAML is a superset)
pyq set '.db.pool.size' 50 services.yaml          # print edited doc
pyq set -i '.db.pool.size' 50 services.yaml       # rewrite the file
pyq set --create-missing '.a.b.c' 1 empty.yaml    # grow mappings
pyq delete '.legacy_field' -i config.yaml
pyq sort-keys '$' -i config.yaml                  # sort one mapping level
```

All edits run through the shared splice engine: untouched lines - comments,
blank lines, odd spacing - keep their exact bytes whenever the document's
layout is splice-eligible.

Edited documents round-trip through the same serializer as `fmt`:
comments, anchors and key order survive; the inserted value keeps its
own source style (`[1, two]` stays flow, `"true"` stays a quoted string).

## Convert

```bash
pyq fmt k8s.yaml                 # comment-preserving normalization
pyq fmt --explicit-start cfg.yaml
pyq to-json config.yaml          # YAML -> JSON (key order kept)
pyq to-toml compose.yaml
pyq from-toml Cargo.toml         # TOML -> YAML
pyq from-json package.json       # JSON -> YAML
pyq from-ini settings.ini        # INI -> YAML (values are strings)
pyq validate k8s.yaml --schema rules.yaml   # parse + schema-language rules
pyq frontmatter README.md --body-out body.md # split Markdown front matter
```

Input format resolves by file extension (`.json`, `.toml`, `.ini`);
override with `--input yaml|json|toml|ini`. Since YAML is a JSON
superset, JSON content also parses on the YAML path unchanged.

## Exit codes

- `0` success;
- `1` with `pyq: <message>` on stderr for missing paths, parse failures
  or TOML-inexpressible shapes (null values, non-table roots) - the same
  stable messages the Python API raises.

## Shell completion

```bash
pyq completion bash > /etc/bash_completion.d/pyq   # bash
pyq completion zsh  > "${functions[@]:0:1}/_pyq"   # zsh
pyq completion fish | source                        # fish
pyq completion powershell > pyq.ps1                 # PowerShell
```

## Scope

| Capability | pyq | pyrs-yaml CLI (Python) |
|------------|-----|------------------------|
| Query / set / delete / format / convert | ✅ | ✅ |
| Verb post-processing (`select`/`sort`/`unique`/...) | ✅ | — |
| Layout-pinned edits (splice engine) | ✅ | ✅ |
| sort-keys at a path | ✅ | ✅ |
| Rename / move / append / insert / frontmatter / `validate` | ✅ | ✅ |
| Multi-document edits (`-A` with set/delete) | planned | ✅ |

Both CLIs edit through the same core splice engine with layout pinning
(untouched lines, including standalone comments, never drift). The
Python CLI remains the full-feature surface (`-A` multi-document,
`validate`, rename/move/frontmatter); `pyq` targets fast,
dependency-free scripting of single documents.

## See Also

- [Command-Line Interface](cli.md) — the Python-based `pyrs-yaml` command
- [TOML, JSON & INI Formats](formats.md) — the library-side conversion API
- [In-Place Editing](editing.md) — the round-trip model `pyq` edits through
