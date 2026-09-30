---
title: Changelog
description: All notable changes to pyrs-yaml, formatted per Keep a Changelog and Semantic Versioning.
tags:
  - docs
status: new
---

## Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

### [Unreleased]

#### Added

- **JSONC block-comment hot-sample bench** — objective §测试覆盖 5
  names "block-comment" as a required hot sample; previously only
  inline `//` comments were measured. New fixture drives
  `test_load_jsonc_block_comments` with 50 pairs + header/footer
  blocks, catching scanner regressions on the `/* ... */` path via
  CodSpeed.
- **PyYAML + ruamel.yaml cross-library parity for YAML** — objective
  §测试覆盖 3 names both as oracles; previously only benchmarks and a
  printout. `tests/test_yaml_crosslib.py` closes the correctness gap:
  20 canonical documents × 5 parity surfaces + 2 documented divergences
  (dup-key strictness, YAML 1.1 legacy booleans schema-scope) = 122
  tests. Optional deps skipif clean.
- **tomlkit cross-library parity for `load_toml`** — objective §测试
  覆盖 3 names tomlkit as an oracle; previously tomlkit only appeared
  in benchmarks. `tests/test_toml_crosslib.py` adds 24 tests covering
  11 canonical constructs with three-way agreement (pyrs/tomlkit/
  tomllib), pins the `>i64` spec-strict divergence, and asserts the
  `-2^63` boundary. Optional dep, skipif clean.
- **orjson as strict-JSON oracle for `load_json`** — objective
  §测试覆盖 3 required correctness parity against orjson; previously orjson
  only appeared in benchmarks. 16 canonical documents assert byte-for-byte
  agreement, and 12 non-strict forms (comments, trailing commas, single
  quotes, `NaN`/`Infinity`/`-Infinity`, hex, leading zero, `+.5`, `5.`) are
  rejected by both orjson and `load_json`. orjson rejects the bare literals
  that stdlib `json.loads` accepts under `allow_nan=True`, so it is the
  stronger RFC 8259 oracle. Optional dep, `skipif` clean.
- **CLI ↔ Binding parity gate (`tests/test_cli_binding_parity.py`)** — the
  Pillar 1 invariant is now an executable contract: CLI's registered
  commands are checked against a fixed 18-name inventory, every `to-X` /
  `from-X` verb requires its matching `YamlDocument.to_X` / `from_X` /
  `load_X` sibling, the `load_*` family symmetry (json/jsonc/json5/toml)
  is asserted, and editing/validate/compliance verbs map to live Python
  API. A drift on either surface now fails CI.
- **`load_json` property tests + CodSpeed benches** — Hypothesis
  (`test_load_json_matches_stdlib_json` + `test_load_json_matches_load_jsonc_on_strict_domain`)
  pins the strict loader against `json.loads` for every generated canonical
  document and asserts both loaders agree on the strict domain; a widening of
  the fast path or a fallback drift now surfaces as a property failure.
  Three CodSpeed benches (`test_load_json_large` / `_floats` / `_escapes`)
  mirror the load_jsonc samples so the strict binding layer is tracked.
- **`load_json` (strict) — completes the `load_*` family parity** — the binding
  exposed `load_jsonc` / `load_json5` / `load_toml` but the strict RFC 8259
  counterpart was missing. `pyrs_yaml.load_json(s)` now matches `json.loads` on
  canonical documents and rejects the JSONC/JSON5 extensions (`//`, `/* */`,
  trailing commas, single quotes, bare `Infinity`/`NaN`, `0x…`) with a typed
  `YamlParseError`. The fast path shares `json_fast::try_load` with
  `load_jsonc` (which bails on every non-canonical byte — zero widening risk);
  declined constructs route through the STRICT `from_json` AST parser. Closes
  the last gap under the CLI ↔ Binding parity entry: every CLI format now has
  its `load_*` sibling, Pillar 1 complete. Re-exported and in `__all__`;
  `.pyi` regenerated via `maturin generate-stubs`.
- **Dialect writer fixed-point properties** — `fmt_pbt.rs` promised a writer
  fixed point in its header but never implemented one. Four proptests now hold
  it for JSON/JSONC/JSON5/TOML (the sole input filter drops hand-built ASTs whose
  distinct keys spell the same JSON name — outside RFC 8259's object domain).
  The gate immediately surfaced three real comment-fidelity defects (Fixed).
- **Hot-spot benchmark corpus** — seven CodSpeed wall-time benches target the
  historically fragile serialization paths: YAML block-scalar documents (all six
  header spellings `|`, `|-`, `|+`, `>`, `>-`, `>+`) and comment-dense documents,
  TOML multi-line strings / radix integers / underscore separators / exponents /
  datetimes, and JSON5 exotic number forms (hex, `+.1`, `5.`, `Infinity`, `NaN`,
  single quotes, trailing commas). Fixtures live in `tests/data/yaml_samples.py`,
  benches in `tests/test_benchmark_api.py`. Building this corpus is what surfaced
  the nested block-scalar indentation bug fixed below.
- **Text-level re-parse gate (`prop_output_always_parses`)** — the Rust proptest
  suite now asserts that every generated AST serializes to text the parser
  accepts again. The AST-vs-AST round-trip property silently skipped shapes
  whose serialized text could not re-parse (`try_roundtrip` returns `None`),
  leaving an entire defect class invisible; the new gate caught six real
  serializer bugs on its first runs (see the Fixed entries below), each now
  additionally pinned by targeted Rust unit tests and a Python regression class
  (`TestNestedBlockScalarIndent` in `tests/test_roundtrip_bugs.py`).
- **TOML datetime offset range** — a numeric UTC offset is now range-checked
  (hours 00..=23, minutes 00..=59); `+12:60` / `+24:00` are rejected instead of
  accepted. Shape was checked but values never were (toml-test
  `invalid/datetime/offset-overflow-minute`; illegal in both 1.0 and 1.1).
- **TOML table-redefinition strictness** — a table created implicitly by a dotted
  key is now closed: a later `[header]` may not re-open it (`[fruit]` + `apple.color`
  then `[fruit.apple]`; `[t1]` + `t2.t3.v` then `[t1.t2]`), and a table cannot be
  redefined as an array (`[tbl]` then `[[tbl]]`). Invalid under BOTH 1.0 and 1.1
  (toml-test `invalid/table` duplicate-key/redefine); legit super-tables and sibling
  dotted keys still parse (mis-acceptance 30 -> 21, valid decode-match unchanged).
- **toml-test conformance harness** — `tests/test_toml_test_suite.py` runs the
  official [toml-test](https://github.com/toml-lang/toml-test) corpus the same way
  `test_yaml_suite.py` runs the YAML suite: untracked local artifact, `skipif` on
  absence, measured-floor gates, and a type-tag adapter for decode comparison.
- **TOML temporal types decode correctly** — date-only and time-only values now
  carry distinct `!date`/`!time` tags (date-times keep `!timestamp`) so they use
  `date`/`time.fromisoformat` instead of crashing. toml-test surfaced a bare time
  (`07:32:00`), a seconds-less time (`13:37`), and a lowercase-delimiter datetime
  (`1987-07-05t17:45:00z`) each raising `ValueError` on valid TOML; `!time` now
  pads omitted seconds, `!timestamp` canonicalizes lowercase `t`/`z`, and both
  normalize a fractional-second field of any precision (TOML `.6`; pre-3.11
  `fromisoformat` wants 3 or 6 digits) to microseconds.
- **TOML control-character strictness** — raw C0 codes (NUL, FF, DLE, US, ...) and
  DEL (U+007F) are now rejected inside basic, literal and multi-line strings (only
  tab, plus newlines in multi-line forms, stay legal). toml-test's `invalid/control`
  corpus surfaced 13 such mis-accepted documents; comment bodies now reject the
  same control codes (5 more `comment-*` documents) and a bare carriage return
  (0x0D outside a CRLF pair) is rejected by a central entry scan (4 more `*-cr`
  documents). The conformance harness now reads corpus bytes verbatim — its earlier
  `read_text()` applied universal-newline translation, rewriting bare CR to LF and
  masking the whole class.
- **TOML number-literal strictness** — leading-zero decimals (`01`, `-01`), a sign
  on radix-prefixed integers (`+0x1F`, `-0b101` — `signed-int` is decimal-only),
  and trailing/double underscores (`1_`, `1__0`) are now rejected. toml-test
  `invalid/integer` + `invalid/float` surfaced 23 mis-acceptances (total 71 -> 48);
  the prior "radix integers can be signed" behavior was a spec violation.
- **TOML inline-table key-collision strictness** — an inline table rejects a dotted
  key that equals, extends, or is shadowed by a defined path (`{ a = 1, a.b = 2 }`,
  `{ a.b = 1, a.b.c = 2 }`); sibling paths (`{ a.b = 1, a.c = 2 }`) stay legal.
  toml-test `invalid/inline-table` duplicate-key/overwrite surfaced these (total 48 -> 39).
- **TOML non-ASCII string crash fixed** — the basic and multi-line basic string parsers
  advanced byte-wise and could slice mid-character on multi-byte input (U+00A0, …),
  panicking; both now consume whole characters. Found via the toml-test corpus; completes
  #153 for the single-line/JSON paths.
- **Format fuzz + robustness fixes** — new `proptest` property tests fuzz the
  TOML/JSON/JSONC/JSON5 parsers and writers (no-panic + re-parseability). They
  found and fix a mid-character slice panic in the TOML and JSON string parsers
  and a JSONC/JSON5 inline-`//`-comment bug that swallowed the following `,`/`}`.
- **YAML merge/alias property fuzz** — the `proptest` suite now generates well-formed
  anchor/alias/merge-key docs (single/sequence aliases, inline-map merges, and the
  scalar/null sources the #166 fix rejects), including a self-referential anchor and
  repeated alias refs. `arb_custom_node()` emits `meta.anchor` but never an `Alias`,
  so the alias/merge paths (the #163/#166 structure class) were previously unfuzzed
  in-process. `prop_merge_alias_never_panics` asserts parse + merge resolution never
  panic or overflow and that any parsed tree re-serializes and re-parses cleanly.
- **CLI format parity** — the CLI gains `to-toml`/`from-toml`, `to-jsonc`/`from-jsonc`
  and `to-json5`/`from-json5`, mirroring `to-json`/`from-json`, so every binding
  format is reachable from the command line.
- **JSON string-escape fast path (perf)** — `load_jsonc` decodes the eight simple
  two-byte escapes inline instead of bailing the whole doc to the AST path; escape
  JSON stays on the fast path (~15x faster than the AST route). Matches
  `json.loads`; `\u`/invalid escapes still route through the AST path.
- **JSON float fast path (perf)** — `load_jsonc` parses canonical floats
  (decimals/exponents) directly into Python objects instead of bailing the whole
  doc to the AST path; values match `json.loads` exactly (correctly-rounded
  parse). A new benchmark exercises the branch for regression tracking.
- **Faster JSON string serialization (perf)** — strings with no escapable byte
  are bulk-copied in one `push_str` instead of per-char UTF-8 re-encoding;
  string-heavy `to_json` ~35% faster (41 -> 27 ns/item), byte-identical output.
- **`YamlDocument.to_toml()`** — documents emit TOML straight from the AST,
  mirroring `to_json`/`to_jsonc`/`to_json5`, instead of the `to_toml(doc.to_yaml())`
  round-trip. Byte-identical output; the writer is ~4.3x faster than `tomli_w`.
- **`to_json` native serializer (perf)** — `YamlDocument.to_json` now uses the
  native engine instead of `to_dict()` + `json.dumps`. Byte-identical for ASCII,
  ~10x faster (1200-item ~1450µs → ~120µs, beating `json.dumps`). Non-ASCII now
  raw UTF-8 (like `to_jsonc`/`to_json5`) instead of `\uXXXX`; still valid JSON.
- **JSON object keys written directly (perf)** — the writer emits mapping keys
  straight into the buffer (no per-key `String` alloc); compact `to_json` ~2x
  faster (~120µs → ~60µs), byte-identical, now #2 in the field.
- **JSON load fast path** — `load_jsonc` parses canonical strict JSON straight
  into Python objects, skipping the `CustomNode` AST (~5-6x faster, now ahead of
  stdlib `json.loads`). Non-canonical input (floats, escapes, comments, huge
  ints, trailing commas) falls back to the general path; values unchanged.
- **TOML multi-line string fidelity** — a TOML multi-line string is projected
  as a `ScalarStyle::Literal` YAML block (surviving the text hub) and re-emitted
  by `to_toml` as a `"""` block instead of an escaped single line. Values
  round-trip byte-for-byte and the output is idempotent; a single-line string
  stays single-line. No AST struct change (reuses `Literal`).
- **TOML document-level comment fidelity** — `to_toml` now emits the root
  mapping's leading comment, so a document-opening standalone `# note` survives
  a TOML → hub → TOML round trip instead of being dropped (the TOML counterpart
  of the JSON writer's `emit_root_leading`). Native TOML parses and comment-free
  documents are unaffected.
- **JSON5 Unicode identifier keys** — unquoted object keys accept the full
  Unicode `ID_Start` / `ID_Continue` set (not ASCII only), so `from_json5` /
  `load_json5` parse `{ é: 1, 名: 2, हिन्दी: 3 }`. Built on the
  `unicode-ident` tables (rustc’s own lexer crate) for exact per-script
  conformance including combining marks. Gated on JSON5, so strict
  `from_json` / `from_jsonc` still require quoting. Lone UTF-16 surrogates stay
  rejected. Adds a dependency (`unicode-ident`).
- **JSON5 Unicode structural whitespace** — `from_json5` / `load_json5` accept
  the whitespace JSON5 adds to RFC 8259's four: VT, FF, NBSP, every Unicode `Zs`
  separator, the LS/PS line terminators and ZWNBSP (U+FEFF). Built on `std`
  `char::is_whitespace` (minus NEL U+0085) plus U+FEFF — no new dependency.
  Strict `from_json` / `from_jsonc` still reject all of them.
- **JSON5 line continuation and `\'` escape** — double-quoted JSON5 strings
  accept a backslash-newline line continuation (removing both) and an escaped
  single quote (`\'`); strict JSON / JSONC still reject both. Mirrors the
  single-quoted handling from #125, completing JSON5 string fidelity.
- **JSON5 string escapes `\v` and `\0`** — `from_json5` accepts
  vertical tab (`\v`) and NUL (`\0`) in double- and single-quoted
  strings; strict JSON / JSONC still reject them. Completes JSON5
  grammar with #120 (numbers) and #124 (value semantics).
- **JSON5 value semantics on load** — `load_json5` resolves the
  JSON5-only number forms (`0x1F`→31, `+7`→7, `5.`→5.0, `Infinity`/`NaN`)
  to real numbers via a new `Schema::Json5`, leaving strict JSON/JSONC
  loaders unchanged and `to_json5_text` still emitting the source
  spelling.
- **JSON5/JSONC reachable through the public API** —
  `pyrs_yaml.from_json5` / `load_json5`, and `YamlDocument.to_jsonc()` /
  `to_json5()` (native-engine emit, so comments and JSON5 styles
  survive). Also fixes a reachability gap: `from_jsonc` / `load_jsonc`
  were never re-exported from the `pyrs_yaml` package, so
  `pyrs_yaml.from_jsonc(...)` raised `AttributeError`; they now appear
  in `__all__`. `to_jsonc`/`to_json5` gained `emit_root_leading` so a
  document-level standalone comment is preserved. New JSON-family
  benchmark coverage added to `test_benchmark_api.py`.
- **JSON5 writer (`to_json5_text` / `to_json5_text_pretty`)** —
  contract-B step 2. Serializes the AST back to JSON5, restoring
  single-quoted strings and the `0x…`/`.5`/`+7`/`Infinity`/`NaN` numeric
  forms the parser preserves, plus `//` comments. Keys are always quoted
  (lossless). Shares an internal `Mode` (Json/Jsonc/Json5); strict and
  JSONC output unchanged.
- **JSON5 numeric forms on the parser** — `from_json5` (new
  `allow_json5_numbers` axis) accepts hex (`0xDECAF`), leading/trailing
  dot (`.5`, `5.`), leading `+` (`+7`), leading zero (`07`), and bare
  `Infinity` / `NaN` / `-Infinity`, each keeping its exact source text
  for a future JSON5 writer. STRICT / JSONC stay gated off and reject
  them as before. `from_jsonc`'s stale "comments are stripped" doc was
  corrected.
- **TOML inline-table interior comment fidelity** — PR #119 captures
  interior `# ...` comments in inline tables (own-line above a member
  → leading, same-line after its value → trailing) and threads them
  through the IR so they round-trip instead of vanishing. Undecorated
  inline tables keep the compact one-line form; a decorated one nested
  in an array goes multi-line. Also repaired a latent #114 bug where
  `skip_all_blank` mistook a comment's own terminator newline for a
  blank line.
- **YAML receiver writes standalone comments into
  `decor.leading_comment`** — PR #117b moves the last engine (the
  granit-parser receiver) onto the slot convention PR #114 / #115
  established. Standalone notes on scalars / mappings / sequences
  land in `NodeMeta::decor.leading_comment` rather than the older
  `comment(standalone = true)`. Hand-built fixtures stay equal thanks
  to #117's normalisation, and `CustomNode::remove_comment` now
  clears **both** slots so Python `Node.remove_comment()` still
  behaves correctly on YAML-parsed docs.
- **Cross-slot standalone normalisation + Python
  `Node.leading_comment`** — `NodeMeta::eq` / `Hash` treat the
  standalone note as one concept across the newer `leading_comment`
  slot (TOML / JSON) and the older `comment(standalone = true)` slot
  (YAML receiver + hand-built fixtures). Setters / removers act
  atomically on both, and the YAML serializer reads the normalised
  view so `to_yaml(toml_ast)` keeps its leading notes. Python
  `Node.leading_comment` getter / setter / remover mirror
  `Node.comment` and now surface TOML / JSONC standalone notes.
- **TOML 1.1.0 grammar** — `from_toml` parses to the v1.1.0 spec
  (2025-12-18). Four additions: **(A1)** multi-line inline tables +
  trailing commas; **(A2)** `\xHH` basic-string byte escape;
  **(A3)** `\e` = U+001B; **(A4)** optional seconds in time and
  date-time. `TomlDialect::V1_0` / `from_toml_v1_0` remain as strict
  1.0.0 escape hatch. A parser-side off-by-one in the space-separated
  date-time detection was fixed alongside.
- **JSON dual-slot comment fidelity** — the JSONC parser writes
  standalone (`// ...` on its own line) notes onto the new
  `leading_comment` slot from #114, while inline (`// trailing`) stays
  on `comment`. Object members and array elements can now carry both
  notes on the same node — impossible under #112's single-slot model.
  The writer prefers `leading_comment` and falls back to `comment`
  with `standalone = true` for hand-built fixtures.
- **TOML blank-line + dual-slot comment fidelity** — `NodeMeta` grows
  `leading_comment: Option<Comment>` and `blank_before: bool` (both
  excluded from structural `Hash` / `PartialEq`), letting a section
  header or AOT element carry both the standalone note above and the
  inline trailing note after `]` without either displacing the other.
  `to_toml` reproduces blank-line groupings; the very first pair never
  emits a leading blank line. Hand-built / YAML-origin nodes still
  rendering via the writer's fallback read.
- **JSON5 dialect** — `pyrs_yaml_core::json::from_json5(text)` and
  `from_json_with_options(text, JsonParseOptions)` accept the full
  JSON5 axis set: trailing commas, single-quoted strings, unquoted
  identifier keys, and line/block comments. Each axis is togglable via
  `JsonParseOptions`; `STRICT`, `JSONC`, and `JSON5` constants ship as
  presets.
- **JSONC/JSON5 bindings and CLI** — `pyrs_yaml.from_jsonc(str)`
  returns YAML text; `pyrs_yaml.load_jsonc(str)` returns a Python dict
  / list. `pyq from-json` grows `--jsonc` and `--json5` flags so
  `tsconfig.json` / `settings.json` files feed straight into the verb
  pipeline.
- **JSONC comment preservation** — `from_jsonc` now attaches captured
  `//` line and `/* */` block comments onto the AST's
  `NodeMeta::comment` (standalone on the key node, inline on the value
  node), mirroring the TOML model from PR #109. The matching pair
  `to_jsonc_text(node)` / `to_jsonc_text_pretty(node, indent)` emits
  them back at their original positions; block comments collapse to
  `//` on output. Strict writers `to_json_text` / `to_json_text_pretty`
  remain byte-identical, so consumers opt into preservation selectively.
- **pyq multi-document edits** — `-A/--all-docs` now covers every edit
  command (set/delete/rename/move/append/insert/sort-keys) plus
  `to-json -A` (JSON array, Python parity). Each document splices
  against its own segment of the stream (`MultiDocEditor` +
  `DirtyUnit::shifted`): untouched documents and all `---` separators
  keep their exact bytes, a plan miss skips its document (Python try/
  skip semantics; all-miss still exits 1), and one layout-dirty document
  falls back alone instead of de-pinning its neighbours.
- **JSONC parsing** — `pyrs_yaml_core::json::from_jsonc(text)` and
  `from_json_with_options(text, JsonParseOptions)` accept `// line` and
  `/* block */` comments at any whitespace position (the dialect used by
  TypeScript's `tsconfig.json` and VS Code's `settings.json`). Comments
  are stripped, not preserved. Trailing commas and other JSON5-only
  forms remain rejected so the accepted language stays a strict superset
  of RFC 8259. `from_json` is unchanged (strict mode by default).
- **TOML comment fidelity** — the parser now captures both standalone
  (`# ...` on its own line above a pair or section header) and inline
  (`key = value # ...` / `[name] # ...`) comments and attaches them onto
  the shared AST through `NodeMeta::comment`. `to_toml(from_toml(src))`
  re-emits those comments at their original positions so `pyq edit` and
  `YamlDocument.set()` no longer strip annotation notes from a TOML
  round trip. Whitespace fidelity (blank-line separators between pairs)
  stays on the writer's default layout per the design doc.
- **TOML numeric source fidelity** — `to_toml(from_toml(src))` preserves
  hex (`0xDEADBEEF`) and octal (`0o755`) integer spellings and exponent
  form floats (`1e10`, `-3.14e-2`) verbatim. Underscore separators,
  explicit `+` signs, negative radix (`-0x1F`) and binary (`0b101`)
  canonicalise to decimal because YAML Core schema cannot re-read them,
  keeping the shared AST interoperable with the YAML pipeline. Comment
  fidelity and JSONC arrive in follow-up PRs per the design doc.
- **pyq feature completion** — the CLI reaches parity with the Python
  CLI's surface: `rename`/`move`/`append`/`insert` splice edits, `validate`
  (parse check, or against schema-language rules via `--schema rules.yaml`),
  `frontmatter` (`--body-out` splits the body), and `-A/--all-docs` on
  `get`/`fmt`/`to-json` for multi-document streams. Fixing the wiring
  surfaced a core engine bug: `move_path` returned only the destination
  INSERT unit, so the spliced text kept a duplicate of the moved subtree
  (invisible whenever documents fell back to re-serialization); it now
  returns both units and the bindings apply them through their batch
  splice path.
- **`pyq` filter verbs** — structured jq-style post-processing on the
  match stream: `--select 'PATH OP LITERAL'`, `--sort-by PATH` /
  `--desc`, `--unique`, `--first` / `--last`, `--skip N` / `--take N`,
  and `--join SEP`, applied in the fixed pipeline order
  `select -> sort -> unique -> slice` then `join` on `get` and the
  `from-*` converters. Deliberately flags, not an expression language:
  the predicate is one micro-grammar parse (micro ~40 lines), mixed
  comparison kinds are `false` (documented divergence from jq's total
  order), and startup stays instant.
- **`pyq completion`** — print a shell completion script for bash, zsh,
  fish or PowerShell (`pyq completion bash > ...`), powered by
  `clap_complete` (approved addition to the CLI crate's dependencies;
  it stays inside the `pyrs-yaml-cli` binary and does not touch the
  Python distribution).
- **`pyq sort-keys`** — sort the keys of the mapping at any path (`$` for
  the root), in place or to stdout, through the same core plan/splice
  engine as `set`/`delete` - closing the parity gap with the Python
  CLI's `sort-keys` command.
- **Command-line interface** — a new `pyrs-yaml` command (opt-in via
  `pip install "pyrs-yaml[cli]"`, requires Python 3.10+) exposing the library
  from the terminal: `fmt` (round-trip reformat preserving comments/anchors/
  order), `get` (JSONPath queries with `--format yaml|json|text`), `set` /
  `delete` / `rename` (path-based edits with `--inplace`, `--string`,
  `--create-missing`), `validate` (CI-friendly exit codes), and `to-json` / `from-json` conversions. All
  commands read stdin via `-` and default to stdout output. Implemented in
  pure Python (`python/pyrs_yaml/cli/`) on top of
  [Cyclopts](https://github.com/BrianPugh/cyclopts) as an optional extra, so
  the base install keeps zero extra dependencies and Python 3.8 support.
- **CLI expansion** — `sort-keys` (sort mapping keys at a path), `move`
  (relocate a subtree to an existing destination), `frontmatter` (extract
  Markdown front matter as YAML, optional body split), and `compliance`
  (YAML Test Suite report with `--json`) commands; `-A/--all-docs`
  multi-document mode on `fmt`/`get`/`set`/`delete`/`rename`/`sort-keys`/
  `validate`/`to-json`; and mutually exclusive `validate --schema <name>`
  vs `--schema-file <path>`. The undocumented `python -m
  pyrs_yaml.compliance` entry point was removed in favor of the
  subcommand.
- **`YamlStream` is now importable** — `from pyrs_yaml import YamlStream`
  works as documented in the API reference and type stubs; the class was
  previously returned by `YAML().load_stream*()` but never exported from the
  native module.
- **CLI `move --all-docs`** — `move` now accepts `-A/--all-docs`, applying the
  subtree move to every document where both paths resolve (same semantics as
  `set`/`delete`/`rename`), so the multi-document flag covers all edit
  commands.
- **Docs ↔ API consistency guard** — `tests/test_docs_api.py` scans every
  locale doc page for `pyrs_yaml.…` chains, `import pyrs_yaml…`, and
  `from pyrs_yaml … import …` claims, and fails if any referenced symbol does
  not exist at runtime (~965 claims checked).
- **Optional third-party type plugins** — `!duration` (`pendulum.Duration`),
  `!arrow` (`arrow.Arrow`), and `!ulid` (`ulid.ULID`) auto-register when the
  corresponding library is installed (`_register_third_party` in
  `python/pyrs_yaml/plugins/_builtin.py`). Each uses a distinct tag so existing
  `!timestamp` / `!date` / `!uuid` handlers are unaffected; a plain stdlib
  `timedelta` is never matched by `!duration`.
- **pydantic-settings YAML source** — `PyrsYamlConfigSettingsSource`
  (`python/pyrs_yaml/settings.py`) is a drop-in replacement for
  `pydantic_settings.YamlConfigSettingsSource` that parses with pyrs-yaml
  (YAML 1.2 core schema) instead of PyYAML. It is exported lazily so
  `import pyrs_yaml` never requires pydantic-settings; install with
  `pip install "pyrs-yaml[settings]"` (Python 3.10+). `dump_pydantic` and
  `parse_as` now use the same lazy module-level `__getattr__` export pattern.
- **`pyq` — native Rust CLI crate** — `crates/pyrs-yaml-cli` (workspace
  member, clap-based) puts `pyrs-yaml-core` behind a jq/yq-style command
  line with no Python at runtime: `fmt` (comment-preserving round-trip),
  `get <path>` (JSONPath-lite with `--json`/`--raw`), `set <path> <value>`
  and `delete <path>` (yq-style edits with `--create-missing` and
  `-i/--inplace`, round-trip-preserving output), `to-json` (order
  preserved), `to-toml`, `from-json` / `from-toml` / `from-ini`; input
  format by extension with `--input` override, stdin via `-`, non-zero
  exit with the core's stable error text.
- **TOML and INI exchange formats** — hub-and-spoke multi-format support
  with YAML as the single editable representation: `from_toml`/`to_toml`
  convert TOML text ⇄ YAML text (Rust `toml_edit`), `load_toml` reads TOML
  directly into Python values (datetimes via the built-in `!timestamp`
  plugin; TOML strings never re-resolve), and `load_ini` reads INI via
  stdlib configparser (strict, read-only). TOML output rejects
  inexpressible shapes with stable errors; round-trip editing stays
  YAML-only by design.

#### Changed

- **granit-parser 1.1 → 1.3** — bumped the YAML event parser from 1.1.0 to
  1.3.0. The upgrade is a semver-compatible minor bump inside the 1.x line:
  1.2.0 added optional `Options` fields for exotic-document limits, 1.2.1
  tightened a few parse results to match the YAML spec, and 1.3.0 added two
  defaulted `Input` methods (`fetch_block_scalar_line` and
  `take_quoted_scalar_ascii_chunk`) that let the scanner step over block and
  quoted scalar bytes faster. The project consumes the parser through
  `Parser::new_from_str` and implements only `EventReceiver` /
  `SpannedEventReceiver`, never `Input`, so no source changes were required —
  the new trait methods resolve to their default implementations. Full suite
  green: `cargo nextest run --all` (359), `pytest` (1436 + 43 numpy), pure-Rust
  `--no-default-features` build, and the YAML test-suite compliance gates
  unchanged.
- **Native JSON & TOML cores** — the `serde_json` and `toml_edit`
  dependencies are gone. `pyrs-yaml-core` ships an RFC 8259 JSON engine
  (byte-level scanner, verbatim number spelling so `from_json → to_json` is
  byte-stable and precision-preserving, typed line/column errors, strict
  rejections for trailing commas, leading zeros, lone surrogates, unescaped
  control characters, and multi-root documents) and a TOML 1.0 engine
  covering the full grammar — bare/quoted/dotted keys, basic/literal/
  multiline strings, decimal / hexadecimal / octal / binary integers with
  underscore separators, floats including `inf`/`nan`/exponent forms, and
  offset/local date, time, and date-time — every rejection surfacing as
  `ParseError::Syntax` with 0-indexed `line`/`col` in the granit-parser
  house style. Public surface unchanged; round-trip tests and
  `tests/test_toml.py` green with the native path.
- **Internal duplication cleanup** — benchmark fixtures composed from shared
  blocks, PyO3 path-edit methods delegate to the existing
  `apply_metadata_edit` helper, and repeated file-read/error-mapping and
  line-offset boilerplate collapsed into shared functions. No public behavior
  change; the duplicate-code rate measured by jscpd drops from 5.25% to 3.45%.
- **`YamlDocument.validate()` caches compiled validators** — the first
  successful validation against a schema (JSON text *or* dict) caches a
  compiled `jsonschema` validator; later calls skip schema parsing, meta-schema
  checking, and validator construction. Dict schemas are keyed by object
  identity with a deep-copy snapshot guard: in-place mutation is detected by
  `==` and transparently recompiles. The cached path raises
  `exceptions.best_match(validator.iter_errors(instance))`, identical to
  `jsonschema.validate()` semantics. WSL wall-time: `document_validate` −98%.
- **Structural dedupe of parser/serializer kernels** — mapping and sequence
  rendering share one `write_container_node` skeleton (output byte-identical,
  `serialize_*` medians −5~11%); single- and multi-document parse entry points
  share one `load_ast` error contract; the schema resolution chains share
  `bool_word`/`numeric_tail` and YAML 1.1 no longer re-checks the core's
  null/bool words per scalar; anchor registration (`register_anchor`) and the
  standalone-vs-inline comment taxonomy (`is_standalone_placement`) are
  single-sourced across the AST and stream receivers. Repo duplicate rate
  3.38% → 2.60%.

#### Fixed

- **Dialect writers/parsers lost or misplaced document-level comments** — a
  file-leading `// note` was misclassified as inline and stolen by the first
  JSONC/JSON5 object member (vanishing on empty containers) instead of landing
  on the root the writer annotates; the JSON-family **and TOML** writers now
  trim comment bodies at emit to match the parser's trimmed storage (an
  untrimmed comment used to oscillate whitespace across passes); and a
  comment-only TOML document keeps its leftover note on the empty root instead
  of serializing to `""` (a hand-built non-empty root carrying both a root and
  a first-element note is outside TOML's representable domain and filtered from
  the property). Leading comments across all five formats now reach a
  byte-stable fixed point (three pinned Rust tests).
- **Nested block scalar bodies kept their parent's indentation** — a literal or
  folded scalar under a nested key emitted its body lines one fixed indent step
  from column zero instead of one step below the `b: |` header line, so every
  nested block-scalar shape re-parsed as an error or a wrong value. The writer
  now threads a `block_base` (the parent line's column) through every emission
  site; round-trip is text-exact for all seven nesting shapes. Found by the
  TOML hot-spot bench.
- **The serializer emits only re-parseable YAML** — five spelling defects caught
  by the new text-level gate: double anchor/tag headers when a mapping value
  moved to its own line pre-emitted a header the child also wrote; block
  scalars inside flow collections (or key position) now demote to
  double-quoted; flow containers starting their own line and the complex-key
  value marker `:` lost their parent indentation; complex keys with a
  standalone comment/tag serialized ambiguous text (the note now moves above
  `?` and the body gets its own deeper lines); and plain scalars with edge
  whitespace or embedded flow indicators (`,[]`) inside flow collections are
  now quoted instead of corrupting the token. Tagged empty containers fold
  their header onto the `{}`/`[]` line; compact dash items refuse to inline
  commented values. Pinned by nine Rust tests and Python regressions.
- **TOML rejected the legal minimum i64 integer** — `from_toml`/`load_toml` failed
  on `-9223372036854775808` (`i64::MIN`): the signed path parsed the unsigned
  magnitude first and overflowed before negation ran. The sign now parses with
  the digits (`i64::from_str` accumulates negatively), signed floats keep their
  exponent spelling, and the old negate pass is gone. Found via the new
  Python-side Hypothesis dialect fuzz (`tests/test_property_dialects.py`, with
  stdlib `json`/`tomllib`/`pyjson5` oracles and type-strict equality; it also
  pins the two known AST-ambiguous spellings — bare JSON5 `Infinity`/`NaN`
  literals and >i64 digit strings). Rust regression:
  `toml::parser::tests::i64_lower_bound_negative_integer_is_accepted`.
- **Wrongly-indented flow sequence continuation is rejected again** — upgrading
  the YAML parser to granit-parser 1.3 (see *Changed*) silently began *accepting*
  a multi-line flow collection whose continuation line is indented no further
  than its enclosing block key (yaml-test-suite `9C9N`: `flow: [a,` then `b,` at
  column 0), regressing strictness `405/406 → 404/406` — invisible to the suite's
  ≥95% threshold gate, so it passed green CI. An in-tree post-parse guard in the
  AST receiver now tracks the enclosing block indentation and rejects an
  under-indented flow continuation, restoring `405/406`. The guard uses only
  spans the parser already computes, so correctly indented multi-line flows are
  untouched. `9C9N` is now pinned as a per-case hard gate (literal input, no
  `skipif`) in `tests/test_yaml_suite.py`, plus a Rust unit test
  (`parser::tests::flow_continuation_under_indented_is_rejected`).
- **Self-referential merge keys no longer overflow the native stack** — a `<<`
  whose expansion points back at its own anchor (`a: &a` containing
  `b: {<<: *a}`) expanded forever inside `resolve_merge_keys`, exhausting the
  native stack and killing the whole interpreter process (Windows exit
  `0xC00000FD`, a segmentation fault). The cycle guard was applied only while
  *collecting* merged pairs, never while *walking* the expansion, so recursive
  re-entry was never caught. The anchor guard is now path-scoped: an anchor's
  name stays on the recursion path while its expansion is walked, and a merge
  that resolves back to an ancestor already on that path terminates as an empty
  expansion instead of recursing. An acyclic AST cannot carry PyYAML's cyclic
  dict, so a self-merge now bottoms out at `{}` rather than crashing. Four
  related merge-semantics defects were fixed in the same pass: a null / scalar /
  sequence merge source no longer survives as a literal `<<` key, an inline
  mapping used directly as a merge value (`<<: {x: 1}`) now merges, and a
  non-alias element in a merge sequence (`<<: [*a, {y: 2}]`) keeps its inline
  map. Covered by six Rust and nine Python regression tests (`merge::tests`,
  `tests/test_gaps.py::TestSelfReferentialMerge166`).
  Reported by [@bourumir-wyngs](https://github.com/bourumir-wyngs) in #166.
- **NumPy serialization no longer reads Python memory without the GIL** — the
  ndarray writer borrowed the array's data buffer via `unsafe { as_slice() }`
  and then iterated that borrowed slice *inside* `py.detach`, i.e. after
  releasing the GIL. `&[T]` is `Send` no matter its provenance, so the borrow
  checker could not catch it, but the memory is owned by Python and another
  thread could resize or write the array concurrently — an unsound data race /
  UB that only surfaces under concurrency. The buffer is now snapshotted into
  Rust-owned memory while the GIL is still held (`slice.to_vec()`), and only
  the scalar→node conversion runs off-thread. This was the *only* `unsafe`
  buffer borrow in the binding layer; every other `py.detach` site was audited
  and touches Rust-owned state only (AST, source text, `BufWriter<File>`).
  Covered by `tests/test_numpy.py::TestNumpyConcurrency`.
  Reported by [@bourumir-wyngs](https://github.com/bourumir-wyngs) in #165.
- **Repeated alias references no longer resolve to `None`** — `to_dict()`
  expanded aliases behind a *global* visited-anchor set that was never
  cleared, so only the first reference to any anchor produced a value and
  every later one silently degraded to `None`:

    ```yaml
    a: &x 1
    b: *x      # 1
    c: *x      # was None, now 1
    ```

    The blast radius was wider than "the second reference": two sibling
    references inside one container poisoned each other too (`{a: &x {p: 1},
    b: {q: *x}, c: {q: *x}}` gave `b` a value and `c` a `None`). The guard is
    now scoped to the current recursion path — pushed for the duration of one
    expansion, popped afterwards — so repeated and sibling references each get
    their own fully built value while genuine cycles still terminate. `<<`
    merge resolution and the AST itself were audited and are unaffected. Six
    new PyYAML-parity cases in `tests/test_direct_load.py` pin the agreement,
    and two tests that had pinned the buggy output as expected behaviour were
    rewritten. Reported by [@bourumir-wyngs](https://github.com/bourumir-wyngs)
    in #163.
- **Document header comments no longer vanish on nested first values** —
  the parser kept one shared comment slot for all in-progress containers,
  so a nested container start clobbered a standalone header note before
  it reached the AST (dropped at parse time; invisible to `to_dict`,
  fatal for `dump`). Slot handling is now a per-container stack.
- **Splice edits no longer duplicate leading comments** — when a
  regenerated region text carried the pair/item's own standalone note,
  the replaced range skipped the old note line and both survived; the
  plan now widens the range over note lines (`pyq set`/`delete` and the
  bindings' splice path share the fix).
- **`!timestamp` accepts a trailing `Z` on every supported Python** —
  `datetime.fromisoformat` only recognizes the UTC-`Z` suffix from 3.11;
  the plugin now normalizes `...Z` to `+00:00` first, fixing
  `Invalid isoformat string` on 3.8–3.10 for YAML `!timestamp` scalars
  and TOML datetimes via `load_toml` / `from_toml`.

#### Performance

- **Direct event→Python load materialization** — `safe_load`, `safe_loads`
  and `YAML().safe_load*` build Python objects in one pass over the granit
  event stream instead of constructing the full AST and walking it again in
  `convert.rs`; schema resolution, raw-text mapping keys and duplicate-key
  errors stay identical. Anchored/tagged/merge/multi-document shapes fall
  back via zero-cost pre-vetoes. WSL wall-time: scalar-heavy `safe_load`
  −21~25%, family overall −13~18%, fallbacks unchanged.
- **Anchor extraction byte gate** — `extract_anchors` returns empty after a
  single `&` byte-containment check; anchor-free documents skip the
  per-character quote state machine. Rust-side `parse_*` benches improve
  11–18% at the median; the scan itself drops 1.5µs → 38ns.
- **Interned stream-event dict keys** — fixed per-event keys from
  `parse_stream` / `load_stream` reuse interned strings (`pyo3::intern!`).
  WSL wall-time: `parse_stream` −34%, `parse_stream_multidoc` −39%,
  `load_stream` −22%.
- **Decomposition micro-benchmarks** — new `granit_events_*` benches isolate
  the pure granit event-pipeline cost from AST construction (bench-only).
- **Multi-document parse without per-document clones** — `on_document_end`
  moves the completed document into the collection instead of deep-cloning it.
  WSL wall-time: `parse_all_docs` −9.7%, `safe_loads` (multi-doc) −9.5%,
  `YAML().safe_loads` −6.7%.
- **Streaming write reuses one buffer across documents** — new
  `direct_dump_into` writes each document into a reused `String`, and
  `dump_iterable` skips `normalize_doc`'s re-copy in the normal case.
  WSL wall-time: `dump_stream_multi_doc` −27.2%, `dump_stream` −4.4%.
- **Scalar fast paths in the AST builder** — `unescape_double_quoted` returns
  early without backslashes; `detect_chomping` pulls lines lazily instead of
  collecting the whole document per block scalar. WSL wall-time: `to_dict`
  family −4~9%, scalar-type loads −3~4%, no regressions.

#### Docs

- **Corrected the numpy guide's 0-D scalar section (all locales)** — the
  snippet claimed 0-D arrays "reshape to a single-element list"
  (`assert data == [42]`); shipped behavior (pinned by `tests/test_numpy.py`)
  serializes bare scalars (`assert data == 42`). The en page gained a warning
  admonition for the 0-D `bool` → `1.0` rust-numpy quirk.

### [v0.15.0] — 2026-08-19

#### Added

- **Node metadata setters/getters** — `Node.comment` / `Node.anchor` / `Node.tag` read properties and `set_comment` / `set_anchor` / `set_tag` (plus `remove_*` variants), backed by new path-based edit operations. Editing an alias or a missing path raises; standalone comments on inline scalar values and sequence items are now serialized on their own indented lines (fixes pre-existing broken round-trip for `child:\n  # c\n  val` and `- a\n# c\n- b`).
- **Verbatim tags** — `set_tag("!<tag:yaml.org,2002:str>")` now produces a verbatim tag (empty handle), and verbatim tags parsed from source survive round-trip: `Tag`'s `Display` emits `!<...>` wrapping for empty-handle tags, `parse_tag` recognizes the `!<...>` form, and stream events serialize tags through `Display`.
- **Schema file IO and listing** — `load_schema(name, path)` reads a schema definition from a file and registers it; `list_schemas()` returns all registered schema names (built-in `failsafe`/`json`/`core`/`yaml1.1` plus custom).
- **Node style/format setters/getters** — `Node.scalar_style` / `Node.flow_style` / `Node.chomping` read properties and `set_scalar_style` / `set_flow_style` / `set_chomping` methods. ScalarStyle/Chomping now derive `Copy`. Non-scalar nodes return `None` / are no-op; aliases and missing paths raise.
- **Schema structural validation** — a `validate` section in a schema definition adds structural checks (path-qualified scalar types, `sequence_of`/`mapping_of` containers, `required`); `validate_against_schema(data, schema_yaml)` raises `YamlValidateError` listing every failure.
- **`Node.copy()`** — deep-copies a subtree as a standalone Python value (dict/list/scalar), detached from the document, for pasting via `set_value()`.
- **Deep editing API** — `doc.set_many({path: value})` sets multiple paths (with wildcard `[*]` and deep-scan `..` support) in a single splice burst; `doc.sort_keys()` orders mapping keys in place; `Node.move(new_path)` relocates a subtree; `Node.path` / `Node.find_first()` / `Node.value_eq()` add path access, first-wildcard lookup, and value comparison.
- **Property-based testing for 0.14+ features** — Rust proptests for `validate_node`, schema parsing, style-settings round-trip; Python hypothesis tests for `set_many` wildcards, metadata-edit preservation, `sort_keys` idempotency. `hypothesis` moved to the `test` group so CI runs property tests.
- **Serializer fix** — standalone comments on empty flow containers (`key: {}` / `key: []`) no longer produce invalid YAML; demoted to inline.

#### Changed

- **NumPy re-enabled on free-threaded (cp314t) wheels** — the `--no-default-features` flag is removed from the cp314t build lines; rust-numpy 0.29 supports free-threaded Python, so `numpy.ndarray` serialization is now available on free-threaded wheels when NumPy is installed (auto-detected at runtime).

#### Docs

- **Corrected stale references across all locale docs (en/zh/ja/ko)** — `saphyr-parser` → `granit-parser`, YAML compliance 98.1% → 99.75% (405/406 suite cases), ABI3 support 3.9–3.13 → 3.8–3.15 (py3.9+ → py3.8+), and benchmark tables updated to current CodSpeed CI numbers (parse 21–43×, serialize 55–177× faster than PyYAML). Rust-side benchmark sections migrated from Criterion to divan (`benches/yaml_bench.rs` → `crates/pyrs-yaml/benches/yaml_bench.rs`).

### [v0.14.1] — 2026-08-15

#### Fixed

- **Single-quoted scalars with backslash + control/noncharacter** — such values now use double-quoting; single quotes cannot escape control chars/noncharacters.
- **Noncharacters and BOM quoted** — `needs_quotes` / `needs_double_quoted` now require quoting for U+FFFE/U+FFFF/plane-end noncharacters and U+FEFF (BOM).
- **Double-quoted escape width** — code points above U+FFFF now emit the 8-digit `\Uxxxxxxxx` form (the 4-digit `\u` form is BMP-only).
- **Folded plain-scalar continuation indent** — continuation indent is derived from the value's start column so nested sequence/mapping items stay indented past the parent block indent.
- **Multi-byte wrap boundary** — `wrap_plain_scalar` floors the wrap slice to a char boundary instead of panicking on 4-byte UTF-8 straddling.
- **`hypothesis` in publish test requirements** — `.ci/requirements-test.txt` pins `hypothesis>=6.113.0` so the publish workflow can run the property suite.

#### Added

- **`scripts/fuzz_panics.py`** — high-volume local Hypothesis fuzz harness with a hostile strategy across dump/parse/edit/idempotency.

### [v0.14.0] — 2026-08-14

#### Added

- **YAML Schema Language** — define custom schemas with `rules` mapping
  regex patterns to YAML types; register via `register_schema()`.
- **Inline dict schema** — `schema` parameter accepts `dict` in `YAML()`,
  `parse()`, `safe_load()`, etc.
- **Community Plugins** — `CustomType` base class with
  `from_yaml`/`to_yaml`/`can_parse`/`validate`; register via `register_type()`.
- **Built-in plugins** — `!timestamp` (datetime) and `!set` registered by default.

#### Changed

- **Schema resolution is pluggable** — `SchemaResolver` trait + `Schema`
  enum with global `SchemaRegistry`. Built-in schemas retain zero-cost dispatch.
- **`node_to_pyobject` and `direct_dump` check registered `CustomType`s** —
  tagged scalars convert via `from_yaml()`; objects serialize via `to_yaml()`.
- **`get()` is literal-key only** — `YamlDocument.get()` no longer guesses
  JSONPath for keys containing `.` or `[`; every key is treated as a
  top-level mapping key, consistent with `__getitem__`/`__setitem__`.
  Path access stays available via `find()`/`node()`.

#### Fixed

- **Quoted scalars always load as strings** — implicit type resolution now
  applies only to plain scalars (YAML 1.2): `safe_load('"true"')` returns the
  string `"true"`, not `True`. The serializer keeps negative numbers
  round-tripping through the document (`to_yaml`) path.
- **Lone-quote keys round-trip** — mapping keys that are a single `'` or `"`
  are emitted as quoted scalars instead of unparseable YAML.
- **Empty collections emit `{}`/`[]`** — dumping empty mappings/sequences no
  longer yields an empty document that re-parses as `None`.

### [v0.13.0] — 2026-08-10

#### Changed

- **Rust MSRV raised to 1.96 and edition bumped to 2024** - both crates now
  declare `rust-version = "1.96"` and `edition = "2024"`; CI pins the
  `build`/`test-freethreaded` jobs to Rust 1.96 for deterministic wheel builds
  and adds an `msrv-check` job running `cargo check`/`cargo test` at the MSRV
  to prevent silent MSRV drift (the `rust-lint` job stays on `stable`).
  The floor is set above PyO3 0.29's own baseline (rustc 1.83) for std API
  headroom (e.g. `assert_matches!`, stabilized 1.96) with no code migration
  needed. `TAG_REGISTRY` (tag handler storage) refactored to
  `std::sync::LazyLock`, dropping the `Mutex<Option<...>>` indirection.

#### Performance

- **`safe_dump` / `from_dict` / `dump_file` / `dump_iterable`: direct writer**
  — Python→YAML serialization without intermediate `CustomNode` AST.
  Single-pass `direct_dump` replaces the old two-pass `pyobject_to_node` +
  `to_yaml`. 7x faster on `safe_dump` (28ns→4ns), 6x faster on `from_dict`
  (35ns→6ns). (#60)
- **`safe_load` / `safe_loads` / `to_dict`: fast-path skip anchor tracking**
  — when input has no `&` characters, skip `collect_anchors` + anchor
  resolution and use the simpler `node_to_pyobject_simple` path. (#59)
- **`resolve_core_type`: first-byte dispatch whitelist** — non-numeric/
  non-boolean first bytes return `Str` immediately, avoiding schema
  resolution overhead for the common case. (#59)
- **granit-parser migration** — saphyr-parser replaced with granit-parser
  1.0.1 for native `Event::Comment` emission, eliminating the full-text
  `scan_yaml()` pre-scan. parse_small -18%, parse_large -21%,
  roundtrip_large -18%.

#### Fixed

- **`float_to_yaml_string` round-trip fix** — appends `.0` when Rust
  Display drops the decimal (`42` → `42.0`) so floats round-trip as
  floats instead of becoming ints.
- **Reverted `count_nodes` pre-allocation** — the full AST traversal cost
  more than the reallocations it avoided (serialize_10mb was ~14% slower);
  buffer growth is left to the Vec.

#### Added

- **`max_depth` on stream & frontmatter APIs** — `parse_stream(yaml, on_event, max_depth)`,
  `read_markdown(path, schema, max_depth)`, `read_markdown_str(content, schema, max_depth)`
  accept `max_depth` (default 1000). Stream parsing now enforces the nesting-depth limit
  via core `parse_stream_with_options` (previously stream events had no depth limit).
- **Pydantic integration** — `dump_pydantic()` serializes a Pydantic model
  to YAML string via `model_dump(mode='json')` + `safe_dump`; `parse_as()`
  parses YAML string into a Pydantic model instance. Both use lazy imports,
  no hard dependency on pydantic. (#61)

#### Internal

- **Split `py/mod.rs`** — monolithic 1786-line module broken into
  `document.rs` (YamlDocument), `yaml_instance.rs` (YAML class),
  `functions.rs` (module-level functions), `stream_iterator.rs`,
  `walk_helpers.rs`. `mod.rs` reduced to 128 lines. (#61)
- **`needs_quotes()` guard + `double_quoted_scalar()` constructor** —
  strings like `'true'` / `'42'` / `'null'` now emit as double-quoted
  scalars under the core schema instead of being misread on re-parse
  (`pyobject_to_node` + `json_value_to_node`).
- **CodSpeed benchmarks unified on `codspeed-divan-compat`** —
  `exclude-allocations` removes allocator noise; cross-library benchmarks
  consolidated into `tests/test_benchmark_crosslib.py` with shared
  `tests/data/yaml_samples.py` fixtures and streaming coverage.

### [v0.12.1] — 2026-08-06

#### Added

- **`set(create_missing=True)`** - missing intermediate mapping keys along
  the edit path are created as nested mappings (e.g. setting `a.b.c` on
  `a: 1` creates `b` and `c`); index segments that miss are still an error,
  and a scalar intermediate along the path still raises.
- **`doc.walk()` / `doc.scalars()`** - Rust-backed depth-first AST traversal
  yielding `Node` objects, avoiding per-node `to_dict()` resolution.
  `walk()` returns all nodes; `scalars()` returns only scalar/null nodes.
- **Rust core module tests** - 39 new tests covering `editing::navigate`
  (key_eq, navigate, navigate_mut, normalize_index, mapping_key_index),
  `editing::region` (line helpers, node_is_flow, extend_delete_over_comments,
  nav_err), `editing::dirty` (DirtyKind/DirtyUnit constructors), and
  `editing::metadata` (with_metadata_from, needs_quoting).
- **Python doc.walk() edge case tests** - 9 new tests for empty doc, null
  values, deeply nested, flow collections, mixed types.

#### Changed

- **Monorepo workspace** - source code split into `crates/pyrs-yaml-core/`
  (pure Rust, no PyO3) and `crates/pyrs-yaml/` (PyO3 bindings). Root
  `Cargo.toml` is now a workspace. Old `src/` directory and `build.rs`
  removed.
- **pyproject.toml** - added `tool.maturin.manifest-path` pointing to
  `crates/pyrs-yaml/Cargo.toml`.
- **Parse hot paths** - single-pass comment/anchor extraction, lazy
  duplicate-key detection, `shift_insert` merge prepending, and skipped
  `DocumentEnd` deep-clone for single-document parses cut large-document
  parse cost ~19% (CodSpeed: parse[large] +13.9%, parse[medium] +16.6%,
  roundtrip[large] +12.2%).
- **`Arc<str>` scalar storage** - `CustomNode::Scalar` and comment/event
  text share allocations via `Arc<str>`; AST nodes shrink 8 bytes and
  clones become refcount bumps instead of deep copies.

#### Fixed

- **`set(create_missing=True)` nested chain build** - the created mapping
  chain no longer duplicates the first segment as a nested key level.
- **`set(create_missing=True)` eligibility** - freshly created keys are now
  eligible for the value write (the eligibility check no longer runs after
  the synthetic pair is inserted).
- **Standalone comments before simple mapping keys** - round-trip
  previously dropped standalone comments attached to simple-key nodes;
  now preserved (two regression tests).

### [0.11.7] - 2026-08-04

#### Changed

- **stub-build-check replaced with release-guard** - the always-red container
  build (`validate.yml`) that deliberately failed to reproduce the v0.10.0
  `--generate-stubs` failure mode is replaced with three static assertions
  that **pass** when the repo is correct: `grep` guards `publish.yml` against
  `--generate-stubs`, `git ls-files` asserts the committed `.pyi` is tracked,
  and `test -f` checks `py.typed` exists. The job now gives green CI on
  correct state, red only on regression.

#### Added

- **Numpy free-threaded tracking** - ROADMAP.md now tracks `rust-numpy` free-
  threaded support status (PyO3/rust-numpy#476) as a dependency for re-enabling
  ndarray serialization on cp314t wheels when the Rust binding matures.

### [0.11.6] - 2026-08-04

#### Changed

- **Free-threaded (cp314t) wheels are now numpy-free** - built with
  `--no-default-features`, so rust-numpy is excluded entirely (smaller
  binary, no runtime probe). `safe_dump` on a `numpy.ndarray` raises
  `YamlTypeError` on free-threaded builds; GIL builds (Python 3.8-3.15)
  keep full ndarray serialization.

#### Added

- **Free-threaded CI validation** - `test-freethreaded` job now builds
  and tests with `--no-default-features`, matching the shipped
  free-threaded wheel configuration.
- **Install docs** - `docs/{en,zh,ja,ko}` note that free-threaded
  wheels are numpy-free (ndarray serialization unavailable on cp314t).

### [0.11.5] - 2026-08-04

#### Changed

- **Parser robustness items 3/4/5 closed via Phase 0 strictness audit** — the 70-probe corpus (indentation, block-mapping keys, flow context) compared against a PyYAML oracle showed **no fixable accepted-but-invalid case** (64/70 match; the 6 divergences are deliberate YAML 1.2 / yaml-test-suite requirements where PyYAML is the outlier, and one deliberate duplicate-key strictness). Compliance stays at **99.75% (405/406)**. Full write-up in `ROADMAP.md` §v0.11.5 and `tests/test_strictness_audit.py`.

#### Added

- `tests/test_strictness_audit.py` — 70-probe strictness regression corpus pinning current rejection/acceptance behavior (both directions), so future parser changes cannot silently regress strictness or over-reject.

### [0.11.4] - 2026-08-04

#### Fixed

- Duplicate null/empty mapping keys no longer error (`: a\n: b`, `~: a\n~: b`) — matches yaml-test-suite 2JQS; real duplicate keys still raise `YamlDuplicateKeyError`
- Compliance harness: correctly-rejected invalid YAML now counts as pass (was lowering the rate despite compliant behavior)
- Compliance harness: `convert_special_chars` tab decoding via regex — any run of `—`/`‖` + `»` is one tab, fixing tab-encoded suite cases

#### Changed

- YAML Test Suite pass rate gate raised from >75% to **≥95%**; current rate **99.75%** (405/406)
- Known deviation documented: `ZYU8` (`%YAML 1.1 1.2`) is rejected by design (invalid per YAML 1.2 grammar, matches PyYAML/libyaml)

### [0.11.3] - 2026-08-03

#### Added

- Streaming write: `YAML.dump_stream(file_obj, iterable)` / `YAML.dump_file(path, iterable)` with document-level constant memory, auto `---` separators, and `explicit_start`/`explicit_end` flags
- `YamlDocument` `with` context manager: snapshot/rollback transaction scoping
- `compliance_report()`: public YAML Test Suite pass-rate reporting (version-consistent)

#### Changed

- Edit-burst line-offset cache: internal O(N+edit) carry-through in the splice layer (public API unchanged)
- `compute_compliance` moved from tests to `pyrs_yaml.compliance`; version no longer hardcoded

#### Fixed

- Changelog mirror drift guard: prek hook + CI job assert root/mirror `[Unreleased]` sync
- Publish stub pre-validation: CI reproduces v0.10.0-class `--generate-stubs` container failures before Release

### [0.11.2] - 2026-08-03

#### Added

- `YAML.load_stream(file_obj)` / `YAML.load_stream_file(path)`: lazy event iterators with O(anchors + chunk) memory

#### Performance

- **Parse no longer computes splice eligibility** — the O(document) layout check now runs lazily on the first edit via `YamlDocument.splice_checked`, restoring the v0.11.0 regression: parse_comments -59%, parse_anchors -42%, parse/roundtrip/edit -10~35% all back to v0.10.0 levels
- **Linear-cursor layout check** — replaces per-node binary search over precomputed line offsets (monotonic source-order traversal)

#### Changed

- `parse_with_options` returns `CustomNode` (was `(CustomNode, bool)`); splice eligibility is now internal to `YamlDocument` and computed on demand

### [0.11.0] - 2026-08-02

#### Added

- **Surgical Serialization** — byte-level source span tracking on every AST node; segment-based splice — edits regenerate only the touched region, untouched text is byte-copied
- proptest fidelity property tests (new dev-dependency)
- 10MB edit-flush benchmarks (divan)

#### Changed

- `flush_source` now splices segments; falls back to full serialization for flow-style regions, non-default layout documents, merged keys, CRLF/BOM documents, and after materialization (single-burst model)
- Splice edits preserve `---`/`...`/directive marker lines as untouched bytes (full serialization previously dropped them — deliberate behavior difference)

### [0.10.0] - 2026-08-01

#### Added

- **In-place editing** — edit parsed documents without losing formatting metadata:
    - Path API: `doc.set(path, value)`, `doc.insert(path, index, value)`, `doc.append(path, value)`, `doc.delete(path)`, `doc.rename(path, new_key)` with JSONPath-style paths (`$.a.b[0]`); root sugar via `doc["key"] = value` and `del doc["key"]`
    - Node API: `doc.node()` / `doc.find(path)` return `Node` objects with `set_value` / `append` / `insert` / `delete` / `rename`, plus tree traversal (`parent`, `children`, `walk`, `filter`)
    - Full metadata preservation — replaced scalars keep comment/anchor/tag/quoting; renamed keys keep position and comments; mapping order preserved on delete
    - Atomic edits — failed operations leave the document (and its revision) untouched
    - Lazy source re-sync — `source()` / `to_yaml()` / `reparse()` re-serialize only after a successful edit
    - Stale-node detection — `Node` access after a document edit raises `YamlDocumentError` (with `RuntimeWarning`)
    - New exceptions: `YamlEditError`, `YamlPathError` (i18n across en/zh-CN/ja-JP/ko-KR)
    - Alias-aware editing — setting an alias's own path replaces it in place; editing through an alias raises `YamlEditError`
- **Edit benchmarks** — 6 new divan benchmarks in `benches/yaml_bench.rs` (set/insert/delete on small–large documents)

#### Changed

- `YamlDocument.source()` now returns `str` and lazily re-serializes after in-place edits

### [0.9.0] - 2026-08-01

#### Added

- **Python 3.13, 3.14 and 3.15 support** — PyO3 `abi3-py38` wheel covers Python 3.8-3.15 (GIL build); `abi3t` + `abi3t-py315` provide free-threaded stable ABI
- **Free-threaded CPython (no-GIL) support** — `#[pymodule(gil_used = false)]` declares module as thread-safe for free-threaded Python; `Py_GIL_DISABLED` cfg flag gates numpy (rust-numpy has no free-threaded support yet — numpy feature must be disabled for free-threaded builds via `--no-default-features`)
- **CI free-threaded job** — new `test-freethreaded` workflow job validates compilation and tests against Python 3.14t
- **`pyo3-build-config` build dependency** — enables `#[cfg(Py_GIL_DISABLED)]`, `#[cfg(Py_3_15)]` etc. compiler flags via `build.rs`
- **`numpy` made optional** — feature-gated behind `numpy` feature (default enabled); excluded automatically under `Py_GIL_DISABLED`
- **`allow_duplicate_keys`** — `YAML(allow_duplicate_keys=True)`, `parse(..., allow_duplicate_keys=True)`, `parse_file`, `safe_load`, `safe_loads`, `parse_all_docs` all accept the flag; duplicate mapping keys raise `YamlDuplicateKeyError` by default, `last value wins` when allowed
- **`SerializeOptions` expansion** — `doc.to_yaml_with_options()` gains `width` (line wrapping, 0 = off), `indent_mapping`, `indent_sequence`, `indent_offset` alongside existing `indent_size`/`explicit_start`/`explicit_end`/`sort_keys`/`max_depth` (`src/py/mod.rs:432`)
- **Tag handler registry** — `register_tag("!custom")` decorator and imperative forms + `clear_tag_handlers()`; scalar nodes carrying a registered tag are transformed through the handler (`src/py/tag_registry.rs`)
- **Tag handler chaining with priority** — multiple handlers per tag run in ascending `priority` order; `YamlTagSkip` lets a handler pass through to the next, fallback keeps the original value
- **Pydantic integration** — `parse_as(Model, yaml, **yaml_kwargs)` parses YAML and validates against a Pydantic v2 model; raises `ImportError` with guidance when pydantic is absent (`python/pyrs_yaml/pydantic.py`)
- **`.pyi` type stubs** — auto-generated by maturin and committed so `register_tag`, `parse_as`, `to_yaml_with_options` and the new exceptions are visible to type checkers

#### Changed

- CI Python matrix expanded: 3.8-3.14 across ubuntu, windows, macos
- Stable ABI: `abi3-py39` → `abi3-py38` (wider Python 3.8+ support), added `abi3t` + `abi3t-py315` (free-threaded stable ABI)
- `pyproject.toml` classifiers updated with 3.13, 3.14, 3.15 entries
- **CI optimization: redundant Rust compilation eliminated** — a single `rust-lint` job runs `cargo clippy` + `cargo test` once; the build job produces one abi3 wheel per OS which test jobs install instead of running `maturin develop`, removing Rust compilation from 21 matrix jobs (~86% fewer compiles); `Swatinem/rust-cache` added to all jobs
- **pydantic test dependency** — `pydantic>=2.10.6` added to `[dependency-groups] test` and `.ci/requirements-test.txt` (SSOT via `uv sync` in ci.yml)

#### Fixed

- **Windows DLL loading** — removed `#[cfg(test)]` block from `src/py/tag_registry.rs` which broke `import pyrs_yaml` on Windows (`250b8d0`)
- **Python 3.8 compatibility** — `from __future__ import annotations` in `pydantic.py` (`63d2495`)
- **CI pydantic skip** — `pytest.importorskip("pydantic")` so tests pass when pydantic is not installed (`7be011d`)
- **CI glob expansion on Windows** — `shell: bash` for `pip install dist/*.whl` (PowerShell does not expand `*`) (`2f7778d`)
- **Non-string tag handler returns now raise `YamlTagError`** — a handler returning a non-`str` value (previously silently ignored, keeping the original scalar) now errors with `Tag handler '!x' must return a string` (`src/py/mod.rs:resolve_tags`)
- **`to_yaml_with_options` indent wiring** — `indent_mapping`/`indent_sequence`/`indent_offset` are now honored by the serializer (previously dead fields); each defaults to `indent_size`/0 when omitted (`src/serializer.rs`)
- **`width` no longer hangs for tiny values** — `width < continuation indent` falls back to emitting the remainder unwrapped instead of looping forever (`src/serializer.rs:write_plain_scalar`)
- **`remove_tag(name)`** — new function to unregister a tag handler; complements `register_tag`/`clear_tag_handlers` (`src/py/tag_registry.rs`)
- **`duplicate-key` errors are i18n'd** — `YamlDuplicateKeyError` messages now flow through `format_i18n_error` across all 4 locales (`src/i18n/locales/*.yml`)

### [0.8.0] - 2026-07-30

#### Added

- **`YAML()` instance API** — `YAML(typ="rt"|"safe"|"full", schema="core"|"yaml1.1", max_depth=1000)` with reusable configuration; `.parse()`, `.safe_load()`, `.safe_loads()`, `.parse_file()`, `.parse_all_docs()` methods
- **Python `Node` API** — `Node` class with `find()`, `filter()`, `walk()`, `to_yaml()`, `parent`, `children`, `root_type`, `value` for AST navigation; JSONPath-like query language (`$.key.sub`, `$.arr[0]`, `$..deep`)
- **`doc.version` metadata** — `YamlDocument.version()` returns the YAML spec version (default "1.2")
- **`MergedView`** — `doc.merged()` returns a read-only dict-like view with merge keys resolved
- **Lifecycle warnings** — `Node.release()` to explicitly invalidate a node; stale access emits `RuntimeWarning` + `YamlDocumentError`

#### Changed

- `parse()` / `safe_load()` now delegate to `YAML().parse()` / `.safe_load()` as syntactic sugar
- `YamlDocument` now stores `version` field for document metadata

### [0.7.1] - 2026-07-30

#### Added

- **ryaml benchmark comparison** — `tests/test_benchmark.py` now benchmarks against `ryaml` (Rust YAML library) alongside PyYAML and ruamel.yaml; `benchmark_compare.py` rewritten as a feature comparison report (`tests/test_benchmark.py:25-28`, `.github/workflows/ci.yml:219`)
- **CI compliance threshold raised** — YAML Test Suite compliance gate increased from 70% to 75% in `test_compliance_report()`; valid parse rate gate at 95% (`tests/test_yaml_suite.py:251`)
- **CI dependency consolidation** — added `.ci/requirements-test.txt` and `.ci/requirements-test-lite.txt` for unified test dependency management across publish workflow and local dev
- **Benchmark modernization** — migrated from `pytest-benchmark` to `pytest-codspeed` for faster C-extension-based statistical benchmarking; all CI jobs now use `-r .ci/requirements-test.txt`
- **Rust benchmarks migrated to Divan** — replaced `codspeed-criterion-compat` with `codspeed-divan-compat` v5.0.1; 16 benchmarks rewritten from Criterion groups to `#[divan::bench]` attributes (`Cargo.toml`, `benches/yaml_bench.rs`)

#### Changed

- CI benchmark job installs `ryaml` for cross-library comparison
- `benchmark_compare.py` now delegates timing to `pytest-benchmark` and serves as a feature comparison/reporting tool

### [0.7.0] - 2026-07-29

#### Added

- **Serializer `max_depth` guard** — `serialize_node_internal` now tracks recursion depth and raises `YamlMaxDepthError` when exceeding the limit (default 1000), matching the parser's protection (`src/serializer.rs:135-145`)
- **Serializer hot-path optimization** — 5 optimizations targeting block-style serialization for ~4.9% roundtrip speedup:
    - Inlined `write_anchor_tag` and `write_inline_comment` None checks (eliminates method calls for ~99% of nodes)
    - `write_indent` hot/cold path split (direct index for cached levels ≤64)
    - `write_plain_scalar` fast path for short ASCII alphanumeric strings (≤8 chars)
    - `write_scalar_for_key` direct dispatch for Plain scalars (avoids dispatch chain)
- **pytest-benchmark migration** — Python benchmarks migrated from raw `time.perf_counter()` to `pytest-benchmark` for statistical rigor, structured JSON output, and CI integration (`tests/test_benchmark.py` + updated `tests/test_performance.py`)

#### Changed

- `pytest-benchmark` replaces raw `timeit` in Python benchmarks
- CI benchmark job now runs `pytest --benchmark-json` instead of standalone script

#### Removed

- `write_inline_comment` method — inlined at all call sites
- `Comment` import from serializer — no longer needed

### [0.6.0] - 2026-07-27

#### Added

- **Async serialization** — `safe_dumps_async`, `safe_dump_async`, `safe_loads_async`, `safe_load_async` via `asyncio.run_in_executor` (`python/pyrs_yaml/async_dump.py`)
- **JSON Schema validation** — `YamlValidateError` exception + `YamlDocument.validate(schema)` method (accepts `str` or `dict`); delegates to Python `jsonschema` module
- **`YamlDocument.to_json()`** — serialize document to JSON string (uses Python `json.dumps`)
- **Incremental re-parse** — `YamlDocument` now stores source text (`doc.source()`); `doc.reparse(resolve_merges=True, schema="core")` re-parses in-place
- **29 new tests** across `test_async.py` (8), `test_validate.py` (14), `test_reparse.py` (7)

#### Changed

- `YamlValidateError` registered as new custom exception (inherits `ValueError`)
- `rust_i18n::i18n!` macro path updated to `"src/i18n/locales"`
- `validate_translations()` test paths updated to match new locale directory

#### Removed

- Deleted redundant `src/i18n/en.ftl`, `src/i18n/zh-CN.ftl` (never referenced by rust-i18n)
- Moved `locales/*.yml` → `src/i18n/locales/` (co-located with i18n module)

#### Dependency Changes

- Runtime dependency: `jsonschema>=4.25.1`
- Dev dependency: `pytest-asyncio>=0.23` (moved from runtime, no longer pinned)

### [0.5.0] - 2026-07-27

#### Fixed

- **`Serializer::write_node`** — `.unwrap()` on `values.iter().next().unwrap()` in `block_mapping`/`block_sequence` replaced with safe indexed access to eliminate potential panic on edge-case ASTs
- **`YAML_SCHEMA` constant** — typo `yamorg2002` corrected to `yamlorg2002` (matches YAML 1.2 spec URL)
- **Development documentation** — `AGENTS.md` updated with mandatory `uv run` prefix for Python commands and direct `cargo` for Rust commands

### [0.4.0] - 2026-07-27

#### Added

- **132 new gap-filling tests** — comprehensive coverage for previously untested APIs
- **i18n function tests** — `set_language`, `get_language`, `list_languages`, `detect_language`, `negotiate_language`
- **`parse_all_docs`** dedicated test suite — single doc, multiple docs, empty, comments
- **`parse_file` success case tests** — basic parsing, comments preservation, file-not-found error
- **`to_yaml_with_options` tests** — `explicit_start`, `explicit_end`, `indent_size`, `sort_keys` order preservation
- **`to_dict()` method tests** — scalar root, nested, list, bool, null, anchor resolution, empty mapping/sequence
- **YamlDocument dunder method tests** — `__repr__`, `__str__`, `__contains__`, `__len__`, `__iter__`, `__getitem__`, `root_type()`
- **Bytes input tests** — `parse(b"key: value")`, UTF-8 bytes, invalid UTF-8 error
- **Unicode & special character tests** — CJK, emoji, roundtrip, CRLF line endings, duplicate keys
- **`safe_load`/`safe_loads` feature coverage** — anchors, merge keys, block scalars, flow collections, special floats, type resolution
- **`from_dict` edge cases** — special characters in keys, nested lists, None values, empty dict/list
- **`from_json` round-trip** — nested structures, arrays, invalid JSON error
- **`dump_file` tests** — success path, invalid path error
- **YAML Test Suite individual case tests** — octal, hex, scientific notation, NaN, infinity, merge keys, explicit/implicit keys, bool/null variants, block scalar strip (`|-`), flow collections
- **`resolve_merges` parameter tests** — preserving `<<` when disabled, resolving by default
- **Flow collections roundtrip** — root-level and nested flow mapping/sequence
- **Anchor on non-scalar nodes** — mapping anchors (`&defaults`) and sequence anchors (`&items`)
- **Sequence indexing tests** — positive index, out-of-range error
- **Merge key integration** — roundtrip with resolved and unresolved merge keys
- **Tag preservation** — `!!seq` and `!!map` tag test coverage
- **Comment preservation** — inline and standalone comment tests on complex structures

#### Changed

- Fixed version sync: `python/pyrs_yaml/__init__.py` `__version__` updated from 0.2.0 to 0.4.0 to match Cargo.toml/pyproject.toml
- Removed stale 0.2.0 wheel artifacts from `dist/`

### [0.3.0] - 2026-07-27

#### Added

- **NumPy ndarray serialization** — `safe_dump()` / `safe_dumps()` / `from_dict()` / `dump_file()` now support `numpy.ndarray` of all dimensions (0-D through N-D)
    - Supported dtypes: `int8/16/32/64`, `uint8/16/32/64`, `float32/64`, `complex64/128`, `bool`
    - Multi-dimensional arrays serialize as nested YAML lists with correct indentation
    - Complex numbers serialize as `(re+imj)` string format
    - `0-D` scalar arrays reshape to 1-D and serialize as a single-item list
    - `PyUntypedArray` + `PyArrayDyn` via `numpy` Rust crate for zero-copy dtype dispatch
    - GIL released during slice iteration for maximum performance
- **`quoted_scalar()`** — new `CustomNode::quoted_scalar()` constructor for values requiring single-quoted YAML style
- **Type resolution for quoted scalars** — `resolve_yaml_type` now applied to `SingleQuoted`/`DoubleQuoted` scalars for correct round-trip of quoted negative numbers
- **Comprehensive NumPy test suite** — 42 tests covering all dtypes, dimensions (0-D through 4-D), negative numbers, infinity, NaN, empty arrays, and edge cases
- Flow collections (`{}`/`[]`) round-trip support with `flow_style` field on Mapping/Sequence AST nodes
- `parse()` accepts both `str` and `bytes` input
- `parse()` supports `resolve_merges` parameter to opt out of merge key expansion
- `parse_all_docs()` for multi-document parsing via saphyr events
- `to_yaml_with_options()` with `indent_size`, `explicit_start`, `explicit_end`, `sort_keys` parameters
- `get()` supports default value parameter
- `dump_file()` for writing YAML to files
- Criterion benchmarks in `benches/yaml_bench.rs` (parse/serialize/roundtrip)
- GitHub Actions CI with matrix testing (3 OS x 4 Python versions)
- Anchor name parsing expanded to full YAML 1.2 spec (dots, colons, hashes, quoted anchors)
- `__version__` attribute, `py.typed` PEP 561 marker

#### Fixed

- **Negative number round-trip** — YAML 1.2 block sequences cannot contain plain scalars starting with `-`; negative numbers are now quoted during serialization and correctly parsed back as integers/floats
- **N-D array support** — replaced `PyArray1<T>` with `PyArrayDyn<T>` to support arrays of any dimension, not just 1-D
- **Correct nesting depth** — multi-dimensional arrays now produce exactly N levels of nesting (shape[1..] handles inner dimensions, root dimension wrapped by `plain_sequence`)
- Alias resolution in `to_dict()` and `safe_load()` — aliases now resolve to referenced values instead of `None`
- `safe_loads()` no longer uses naive `split("---")` — uses saphyr's document events
- Mapping/Sequence tags no longer discarded during parsing
- `format_scalar_for_key()` now handles Literal/Folded block scalar styles

#### Changed

- Added `numpy` crate (v0.29) as a dependency for ndarray type dispatch
- Upgraded PyO3 from 0.21 to 0.29
- Replaced 15+ boilerplate `CustomNode` constructions with `plain_scalar()`/`plain_mapping()`/`plain_sequence()`/`plain_null()` constructors
- Serializer extracted `write_anchor_tag()` and `write_inline_comment()` helpers
- Parser extracted `detect_flow_style()` helper
- Removed dead code: `ParseOptions`, `find_inline_comment`, `find_standalone_comment_before`, `format_yaml_type` (test-only)
- Consolidated 6 duplicate test files, moved 9 diagnostic scripts to `scripts/`
- Improved error messages with key/index/type context

### [0.1.0] - 2026-07-25

#### Added

- Initial release with YAML 1.2 compliance via saphyr-parser
- Custom AST with full metadata (comments, anchors, tags, chomping, scalar styles)
- Round-trip preservation of comments, anchors, tags, and formatting
- PyYAML-compatible API (`safe_load`/`safe_dump`)
- `from_dict`/`from_json` conversion functions
- `read_markdown`/`read_markdown_str` for YAML frontmatter extraction
- Block scalars (`|`/`>`) with chomping indicators (`|-`/`|+`/`>-`/`>+`)
- Escape sequences (`\n`, `\t`, `\uXXXX`, `\xXX`)
- YAML 1.2 type resolution (null, bool, int, float, infinity, NaN)
- Merge key resolution (`<<: *alias`)
- Complex keys (sequence/mapping as key)
