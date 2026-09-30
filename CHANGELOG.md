# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **TOML datetime offset range** — a numeric UTC offset is now range-checked
  (hours 00..=23, minutes 00..=59); `+12:60` / `+24:00` are rejected instead of
  accepted. The offset was shape-checked but its values never validated
  (toml-test `invalid/datetime/offset-overflow-minute`; valid in neither 1.0 nor 1.1).
- **TOML table-redefinition strictness** — a table created implicitly by a dotted
  key is now closed: a later `[header]` may not re-open it (`[fruit]` +
  `apple.color` then `[fruit.apple]`; `[t1]` + `t2.t3.v` then `[t1.t2]`), and a
  table can no longer be redefined as an array of tables (`[tbl]` then `[[tbl]]`).
  These are invalid under BOTH TOML 1.0 and 1.1 (toml-test `invalid/table`
  `duplicate-key-*`/`redefine-*`); legitimate implicit super-tables and sibling
  dotted keys still parse (mis-acceptance total 30 -> 21, valid decode-match
  unchanged).
- **toml-test conformance harness** — `tests/test_toml_test_suite.py` runs the
  official language-agnostic [toml-test](https://github.com/toml-lang/toml-test)
  corpus the same way `test_yaml_suite.py` runs the YAML suite: an untracked
  local artifact (`Reference/toml-test`), every test `skipif` its absence, and
  honest measured-floor gates. A type-tag adapter maps `load_toml` output to
  toml-test's `{"type": ..., "value": ...}` form for decode comparison and
  invalid-rejection checks.
- **TOML temporal types decode correctly** — date-only (`1979-05-27`) and
  time-only (`07:32:00`) values now carry distinct `!date` / `!time` tags (local
  and offset date-times keep `!timestamp`), so they route to `date.fromisoformat`
  / `time.fromisoformat` instead of crashing in `datetime.fromisoformat`. Surfaced
  by toml-test: a bare local time (`07:32:00`), a seconds-less time (`13:37`), and
  a lowercase-delimiter offset datetime (`1987-07-05t17:45:00z`) each raised a raw
  `ValueError` on *valid* TOML. The `!time` plugin now pads omitted seconds for
  the 3.8 floor, `!timestamp` canonicalizes lowercase `t`/`z`, and both normalize
  a fractional-second field of any precision (TOML allows `.6`; `fromisoformat`
  pre-3.11 accepts only 3- or 6-digit fractions) to microseconds.
- **TOML control-character strictness** — raw C0 control codes (NUL, FF, DLE,
  US, ...) and DEL (U+007F) are now rejected inside basic, literal, and
  multi-line strings (only tab and, in multi-line forms, newlines are legal),
  where the single-line basic path had guarded C0 but every other form accepted
  them silently. toml-test's `invalid/control` corpus surfaced 13 such
  mis-accepted documents. Comment bodies now reject the same control codes
  (`# ...\u0000...` etc., 5 further `invalid/control/comment-*` documents), and a bare
  carriage return (a `0x0D` outside a CRLF pair, in any context) is now rejected by
  a central scan at the parse entry (the last 4 `invalid/control/*-cr` documents).
  The toml-test conformance harness also switched to reading corpus bytes verbatim
  — its earlier `read_text()` silently applied universal-newline translation, which
  had rewritten bare CR to LF before the parser ever saw it, masking that whole
  bug class.
- **TOML number-literal strictness** — decimal integers with leading zeros
  (`01`, `007`, `-01`, `01.5`), a sign on a radix-prefixed integer (`+0x1F`,
  `-0b101`, `+0o644` — `signed-int` only wraps a decimal integer), and
  trailing/double underscore separators (`1_`, `1__0`, `0x1_`) are now rejected
  rather than silently accepted. toml-test's `invalid/integer` and `invalid/float`
  corpora surfaced 23 such mis-acceptances (mis-acceptance total 71 -> 48). The
  earlier "radix integers can carry a sign" behavior was a spec violation and is
  now a rejection assertion.
- **TOML inline-table key-collision strictness** — an inline table now rejects a
  dotted key that equals, extends, or is shadowed by an already-defined path
  (`{ a = 1, a.b = 2 }`, `{ a.b = 1, a = 2 }`, `{ a.b = 1, a.b.c = 2 }`,
  `{ a = { b = 1 }, a.c = 2 }`), while sibling dotted paths (`{ a.b = 1, a.c = 2 }`)
  stay legal. toml-test's `invalid/inline-table` `duplicate-key-*` and `overwrite-*`
  groups surfaced these (mis-acceptance total 48 -> 39).
- **TOML non-ASCII string crash fixed** — the basic and multi-line basic string
  parsers advanced byte-wise and could slice a `&str` mid-character on multi-byte
  content (e.g. U+00A0, U+0251), panicking. Surfaced by the toml-test conformance
  corpus; both loops now consume whole characters. (Completes what #153 fixed for
  the single-line and JSON string paths.)
- **Format fuzz + robustness fixes** — new `proptest` property tests fuzz the
  TOML / JSON / JSONC / JSON5 parsers and writers with arbitrary input and assert
  no-panic and re-parseability. They surfaced and fix three real defects: a
  mid-character slice **panic** in the TOML and JSON basic-string parsers (on
  certain lossy-decoded inputs), and a JSONC/JSON5 writer bug where an inline
  `//` comment was not newline-terminated, so it swallowed the following `,` or
  `}` and produced output that could not re-parse.
- **CLI format parity** — the `pyrs-yaml` CLI gains `to-toml` / `from-toml`,
  `to-jsonc` / `from-jsonc`, and `to-json5` / `from-json5`, mirroring the existing
  `to-json` / `from-json`. Every format the Python binding handles is now
  reachable from the command line (the binding's document methods preserve
  comments, number source and multiline styles through the YAML hub).
- **JSON string-escape fast path (perf)** — `load_jsonc` now decodes the eight
  simple two-byte JSON escapes (quote, backslash, slash, backspace, form-feed,
  newline, carriage return, tab) inline instead of bailing the whole document to
  the AST path, so escape-bearing JSON (common in configs/logs) stays on the fast
  path — measured ~15x faster than the AST route on a log-style payload. Values
  match `json.loads` exactly; a `\u` escape or any invalid escape still routes
  through the AST path unchanged. A new benchmark exercises the branch.
- **JSON float fast path (perf)** — `load_jsonc` now parses canonical floats
  (decimals and exponents) straight into Python objects instead of bailing the
  whole document to the AST path. Float-bearing JSON — ubiquitous in configs and
  metrics — now uses the fast path; values match `json.loads` exactly (a
  correctly-rounded parse identical to CPython's `float`), and a new benchmark
  exercises the branch so it is tracked for regressions.
- **Faster JSON string serialization (perf)** — the writer now bulk-copies a
  string in a single `push_str` when no byte needs escaping, instead of
  re-encoding UTF-8 char-by-char. String-heavy `to_json` payloads serialize
  ~35% faster (measured 41 -> 27 ns/item); output is byte-identical (round-trip
  and serialization-snapshot tests unchanged).
- **`YamlDocument.to_toml()`** — documents now emit TOML directly from the AST,
  mirroring `to_json` / `to_jsonc` / `to_json5`. Previously getting TOML from a
  parsed document required `to_toml(doc.to_yaml())` — a serialize-to-YAML-then-
  re-parse round-trip; the new method calls the native writer once. Output is
  byte-identical to the round-trip and reloads to the same data. The writer
  itself measures ~4.3x faster than `tomli_w` (locked by a new serialize gate).
- **`to_json` native serializer (perf)** — `YamlDocument.to_json` now uses the
  native engine instead of `to_dict()` + Python `json.dumps` (a double
  conversion). Byte-identical for ASCII and ~10x faster (a 1200-item document
  drops ~1450µs → ~120µs, now beating `json.dumps`). Behavior change: non-ASCII
  is emitted as raw UTF-8 (matching `to_jsonc` / `to_json5`) rather than
  `\uXXXX` escapes; output stays valid JSON that `json.loads` parses the same.
- **JSON object keys written directly (perf)** — the native JSON/JSONC/JSON5
  writer emits mapping keys straight into the output buffer instead of
  allocating a `String` per key via `key_text`. `to_json` on a 1200-item
  document is ~2x faster still (compact ~120µs → ~60µs), byte-identical output;
  JSON serialize now measures #2 against the installed field (behind only
  orjson).
- **JSON load fast path** — `load_jsonc` now parses canonical strict JSON
  (objects/arrays, `i64` integers, booleans, `null`, escape-free strings)
  straight into Python objects, skipping the intermediate `CustomNode` AST.
  Measured ~5-6x faster than the AST round-trip and now ahead of the stdlib
  `json.loads`. Any non-canonical input (floats, escape sequences, comments,
  out-of-range integers, trailing commas, invalid tokens) falls back to the
  general AST path, so values and error reporting are unchanged.
- **TOML multi-line string fidelity** — a TOML multi-line string (`"""…"""`
  or `'''…'''`) now keeps its shape through the hub: the parser marks it,
  it is projected as a `ScalarStyle::Literal` YAML block scalar (so the
  multi-line form survives the text round-trip), and `to_toml` re-emits it as
  a `"""` block instead of collapsing it to an escaped single line. Values
  round-trip byte-for-byte (quotes and backslashes re-escaped), the emission is
  idempotent, and a single-line `"a\nb"` correctly stays single-line. Uses the
  existing `ScalarStyle::Literal` representation — no AST struct change.
- **TOML document-level comment fidelity** — `to_toml` now emits the root
  mapping's leading comment, so a document-opening standalone `# note`
  survives a TOML → hub → TOML round trip instead of being dropped. This is
  the TOML counterpart of the JSON writer's `emit_root_leading` (#114 / #122).
  The pair and section emitters already handled their own slots; only the
  root-level note was unreachable. Native TOML parses (which place the first
  note on the first key) are unaffected, and a comment-free document emits no
  stray line.
- **JSON5 Unicode identifier keys** — unquoted object keys now accept the full
  Unicode `ID_Start` / `ID_Continue` set rather than ASCII only, so
  `from_json5` / `load_json5` parse `{ é: 1, 名: 2, हिन्दी: 3 }`.
  Implemented with the `unicode-ident` tables (the crate rustc’s own lexer
  uses) for exact per-script conformance, including combining marks
  mid-identifier. Gated on the JSON5 flag, so strict `from_json` and
  `from_jsonc` still require such keys to be quoted. Lone UTF-16 surrogates
  in `\uXXXX` escapes remain rejected (a Rust `String` cannot represent them
  losslessly). Adds a new dependency (`unicode-ident`).
- **JSON5 Unicode structural whitespace** — `from_json5` / `load_json5` now
  treat the whitespace JSON5 adds to RFC 8259's four (tab / space / LF / CR)
  as inter-token separators: vertical tab, form feed, NBSP (U+00A0), every
  Unicode `Zs` space separator, the LS/PS line terminators (U+2028 / U+2029)
  and ZWNBSP (U+FEFF). Implemented with `std`'s `char::is_whitespace` (minus
  NEL U+0085, which JSON5 does not classify as whitespace) plus an explicit
  U+FEFF — no new dependency. Gated on JSON5 mode, so strict `from_json` and
  `from_jsonc` still reject every one of them, byte-for-byte unchanged.
- **JSON5 line continuation and `\'` escape** — double-quoted JSON5 strings
  now accept the two escape forms JSON5 adds beyond the JSON set: a backslash
  immediately before a line terminator (line continuation, which removes both),
  and an escaped single quote (`\'` → `'`). Gated on JSON5 mode, so strict
  `from_json` and `from_jsonc` still reject both exactly as before. Mirrors the
  single-quoted-string handling from #125, completing JSON5 string fidelity.
- **JSON5 string escapes `\v` and `\0`** — `from_json5` now accepts the
  two string escapes JSON5 adds to the JSON set: `\v` (vertical tab
  U+000B) and `\0` (NUL U+0000), in both double- and single-quoted
  strings. Gated on JSON5 mode, so strict `from_json` and `from_jsonc`
  still reject them exactly as before. Completes the JSON5 grammar
  support alongside #120 (numeric forms) and #124 (value semantics).
- **JSON5 value semantics on load** — `load_json5` now resolves the
  JSON5-only numeric forms to real Python numbers instead of leaving
  them as strings: hexadecimal integers (`0x1F` → `31`), a leading `+`
  (`+7` → `7`), a trailing decimal point (`5.` → `5.0`), and
  `Infinity` / `NaN` → `float('inf')` / `float('nan')`. Implemented as
  a new `Schema::Json5` value resolver layered on the JSON one (shared
  forms delegate, so they can't drift), leaving the strict JSON and
  JSONC loaders byte-for-byte unchanged. Serialization fidelity is
  unaffected: `to_json5_text` still round-trips the original source
  spelling (`0x1F` stays `0x1F`) — load value and emit style are now
  independently correct.
- **JSON5/JSONC reachable through the public API** — `pyrs_yaml.from_json5`
  and `pyrs_yaml.load_json5` (the JSON5 counterparts of `from_jsonc` /
  `load_jsonc`), plus `YamlDocument.to_jsonc()` and `YamlDocument.to_json5()`
  emit methods that route through the native engine (so comments and
  JSON5 styles survive) rather than `json.dumps`. Also fixes a
  long-standing reachability gap: `from_jsonc` / `load_jsonc` were
  exported by the extension module but never re-exported from the
  `pyrs_yaml` package `__init__`, so `pyrs_yaml.from_jsonc(...)` raised
  `AttributeError` — they now appear alongside `from_json` and in
  `__all__`. `to_jsonc` / `to_json5` gained `emit_root_leading`: a
  document-level standalone comment (which lands on the root
  container's `leading_comment` slot) is now emitted before the
  outermost `{`/`[` instead of being dropped. New benchmark coverage for
  the whole JSON family (`from_jsonc` / `from_json5` / `load_json5` /
  `to_jsonc` / `to_json5`) joins `test_benchmark_api.py`.
- **JSON5 writer (`to_json5_text` / `to_json5_text_pretty`)** —
  contract-B step 2, complementing #120's parse side. Serializes the
  shared AST back to JSON5 text, restoring the JSON5-only spellings the
  parser preserves on the AST: single-quoted strings (any scalar tagged
  `ScalarStyle::SingleQuoted`, which only the JSON5 parser produces) and
  the numeric forms `0x…` / `.5` / `5.` / `+7` / `Infinity` / `NaN` as
  bare tokens. Comments (line and block, both normalized to `//`) are
  emitted exactly as in JSONC, using the `leading_comment` / `comment`
  slots from #114/#117. Object keys are always quoted — JSON5 permits
  bare identifier keys but quoting is lossless and keeps one code path.
  `to_json_text` / `to_jsonc_text` are unchanged; the writer now shares
  a single `Mode` (Json / Jsonc / Json5) internally instead of a boolean
  comment flag.
- **JSON5 numeric forms on the parser** — `from_json5` (and
  `from_json_with_options` with the new `allow_json5_numbers` axis) now
  accept the number spellings that are legal in JSON5 but invalid in RFC
  8259: hexadecimal integers (`0xDECAF`, `0XFF`, `-0x1F`), a leading
  decimal point (`.5`), a trailing decimal point (`5.`), an explicit
  leading `+` (`+7`, `+.25`), a leading zero (`07`), and the bare
  `Infinity` / `NaN` literals (including signed `-Infinity`). Each form
  keeps its exact source text on the plain scalar (the same
  source-spelling strategy as TOML #108) so a follow-up JSON5 writer can
  reproduce it verbatim. `STRICT` and `JSONC` are gated off this axis,
  so they reject every one of these exactly as before — the RFC 8259
  contract is unchanged. `from_jsonc`'s stale "comments are stripped"
  doc comment was corrected: since #112/#115 comments are preserved on
  the AST and round-trip through `to_jsonc_text`.
- **TOML inline-table interior comment fidelity** — PR #119 plumbs
  interior `# ...` comments through the inline-table IR so they survive
  the round trip instead of being silently dropped. A `# ...` on its
  own line above a member is captured as that member's leading note
  (onto its key's `leading_comment`); a `# ...` on the member's own
  line after its value is captured as trailing (onto the value's
  `comment`). Undecorated inline tables keep the compact single-line
  `{ a = 1, b = 2 }` form; a decorated inline table nested in an array
  promotes to the multi-line inline form (a line comment cannot live
  inside a one-line `{}`), while a decorated top-level inline table is
  promoted to a `[section]` (long-standing behaviour since #107) that
  now carries the comments. Fixing this also surfaced and repaired a
  latent #114 bug: `skip_all_blank` counted a standalone comment's own
  terminating newline as a blank line, so re-parsing emitted spurious
  blank lines before comment-led pairs.
- **YAML receiver writes standalone comments into `decor.leading_comment`**
  — PR #117b migrates the last remaining engine (the granit-parser
  receiver in `parser/mod.rs`) onto the slot convention PR #114
  introduced for TOML and PR #115 for JSONC. Standalone `# ...` notes
  on scalars, mappings and sequences now land on
  `NodeMeta::decor.leading_comment` instead of the older
  `comment(standalone = true)` shape. Every existing hand-built
  fixture continues to compare equal thanks to #117's AST-layer
  normalisation, and `CustomNode::remove_comment` now clears **both**
  slots atomically so `Node.remove_comment()` still does the right
  thing on YAML-parsed docs (widened semantics).
- **Cross-slot standalone normalisation + Python
  `Node.leading_comment` API** — `NodeMeta::eq` / `Hash` now treat
  "the standalone note" as one concept regardless of whether it lives
  in the newer `decor.leading_comment` slot (introduced by PR #114 and
  used by the TOML / JSON engines) or the older `comment` slot with
  `standalone = true` (still written by the YAML receiver and every
  hand-built pre-#114 fixture). Parsed YAML documents compare equal to
  hand-built fixtures and vice versa, so a future YAML write-side
  migration is transparent. `CustomNode::set_leading_comment` /
  `remove_leading_comment` write / clear **both** slots atomically so
  the two conventions never disagree, and `set_leading_comment`
  preserves an inline trailing note still living in `comment`. The
  YAML serializer reads the normalised view, so
  `to_yaml(toml_ast)` / `to_yaml(json_ast)` keep their leading notes
  instead of dropping them. On the Python side, `Node.leading_comment`
  exposes a getter / setter / remover mirroring `Node.comment`, and
  TOML- or JSONC-parsed documents surface their standalone notes to
  Python callers for the first time.
- **TOML 1.1.0 grammar** — `from_toml` now parses to the [TOML v1.1.0
  spec](https://toml.io/en/v1.1.0) released 2025-12-18. Four
  concrete additions over 1.0.0: **(A1)** inline tables may span
  multiple lines and end with a trailing comma (`{\n a = 1,\n b = 2,
  \n}`); **(A2)** `\xHH` basic-string escape for codepoints
  0x00..=0xFF; **(A3)** `\e` escape for U+001B (ESC); **(A4)**
  seconds are optional in time and date-time values, so `t = 14:15`
  and `dt = 2010-02-03 14:15` parse and round-trip byte-stable.
  `TomlDialect::V1_0` and `from_toml_v1_0` remain for consumers
  pinning the strict 1.0.0 grammar; every 1.0.0 document parses
  identically under both dialects. The parser-side off-by-one in
  `space_time_sep` detection (which meant `T`-less date-times were
  never recognised in 1.0 mode either) was fixed alongside.
- **JSON dual-slot comment fidelity** — the JSONC parser now writes
  standalone (`// ...` on its own line above a pair or element) notes
  onto the `leading_comment` slot PR #114 introduced, while inline
  (`... value // trailing`) notes stay on `comment`. Object members and
  array elements can therefore carry BOTH simultaneously — a shape the
  single-slot #112 model could not express. `to_jsonc_text_pretty` reads
  the new slot first and falls back to `comment` with `standalone =
  true` so hand-built fixtures keep rendering. Behaviour is otherwise
  unchanged for strict JSON (`to_json_text` still emits no `//`).
- **TOML blank-line and dual-slot comment fidelity** — `NodeMeta` grows
  `leading_comment: Option<Comment>` and `blank_before: bool` (both
  excluded from structural `Hash` / `PartialEq`), so a section header or
  array-of-tables element can carry BOTH the standalone note above it
  AND the inline trailing note after `]` without either displacing the
  other. `to_toml` reproduces the blank-line groupings the source had
  (`a = 1\n\nb = 2` round-trips byte-stable), while a design-review rule
  suppresses blank lines before the very first pair of a document.
  Hand-built / YAML-origin nodes that still put a standalone note into
  `comment` render identically thanks to the writer's fallback read.
- **JSON5 dialect** — `pyrs_yaml_core::json::from_json5(text)` and the
  general `from_json_with_options(text, JsonParseOptions)` accept the
  full JSON5 axis set: trailing commas in arrays and objects,
  single-quoted strings, unquoted identifier keys (`A-Z a-z _ $`), and
  line/block comments. Each axis is individually togglable on
  `JsonParseOptions { allow_trailing_commas, allow_single_quoted,
  allow_unquoted_keys, allow_comments }`; `STRICT`, `JSONC`, and `JSON5`
  constants ship as the common presets.
- **JSONC/JSON5 bindings and CLI** — `pyrs_yaml.from_jsonc(str)` returns
  YAML text (mirrors `from_json`), `pyrs_yaml.load_jsonc(str)` returns
  a Python dict / list directly (mirrors `load_toml`). The `pyq`
  command gains `--jsonc` and `--json5` flags on `from-json` so
  `tsconfig.json` / `settings.json` files feed straight into the
  existing verb pipeline.
- **JSONC comment preservation** — `from_jsonc` now attaches captured
  `//` line and `/* */` block comments onto the AST's `NodeMeta::comment`
  (standalone on the key node, inline on the value node), mirroring the
  TOML model from PR #109. The matching pair
  `to_jsonc_text(node)` / `to_jsonc_text_pretty(node, indent)` emits
  them back at their original positions; block comments collapse to
  `//` on output (the AST stores only the body text). Strict RFC 8259
  writers `to_json_text` / `to_json_text_pretty` remain byte-identical:
  they ignore comments even when the AST carries them, so consumers can
  opt into preservation selectively.
- **pyq multi-document edits** — `-A/--all-docs` now covers every edit
  command (set/delete/rename/move/append/insert/sort-keys) plus
  `to-json -A` (JSON array, Python parity). Each document splices
  against its own segment of the stream (`MultiDocEditor` +
  `DirtyUnit::shifted`): untouched documents and all `---` separators
  keep their exact bytes, a plan miss skips its document (Python try/
  skip semantics; all-miss still exits 1), and one layout-dirty document
  falls back alone instead of de-pinning its neighbours.
- **JSONC parsing** — `pyrs_yaml_core::json::from_jsonc(text)` and the
  more general `from_json_with_options(text, JsonParseOptions)` accept
  `// line` and `/* block */` comments at any position whitespace is
  legal (the dialect popularised by TypeScript's `tsconfig.json` and
  VS Code's `settings.json`). Comments are stripped, not preserved on
  re-serialisation; trailing commas and other JSON5-only forms are still
  rejected so the accepted language remains a strict superset of
  RFC 8259 JSON. `from_json` continues to enforce strict RFC 8259 by
  default; no existing API changes.
- **TOML comment fidelity** — the parser now captures both standalone
  (`# ...` on its own line above a pair or section header) and inline
  (`key = value # ...` / `[name] # ...`) comments and attaches them onto
  the shared AST through `NodeMeta::comment` (standalone on the key
  node, inline on the value node). `to_toml(from_toml(src))` re-emits
  those comments at their original positions, so `pyq edit` and
  `YamlDocument.set()` no longer strip annotation notes from a TOML
  round trip. Whitespace fidelity (blank-line separators between
  pairs) stays on the writer's default layout per the design doc.
- **TOML numeric source fidelity** — `to_toml(from_toml(src))` now preserves
  the source spelling of hex (`0xDEADBEEF`) and octal (`0o755`) integers and
  exponent-form floats (`1e10`, `-3.14e-2`, `6.02E23`). Radix prefixes are
  emitted verbatim so the round-trip is byte-stable; underscore separators
  and explicit `+` signs are canonicalised to plain decimal because YAML
  Core schema does not accept them (which keeps the shared AST
  interoperable with the `load_yaml`/`to_dict` pipeline). Negative radix
  forms (`-0x1F`) also canonicalise for the same reason. Binary (`0b101`)
  canonicalises to decimal because YAML Core has no binary rule. Comment
  fidelity and JSONC support land in follow-up PRs (design doc at
  `docs/superpowers/specs/toml-json-fidelity-design.md`).
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
  locale doc page (`docs/{en,zh,ja,ko}`) for `pyrs_yaml.…` attribute chains,
  `import pyrs_yaml…`, and `from pyrs_yaml … import …` claims, and fails if
  any referenced symbol does not exist at runtime (~965 claims checked).
  Guards against the class of drift that let the missing `YamlStream` export
  stay undocumented-by-tests.
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
  member, clap-based, installed via `cargo install --path crates/pyrs-yaml-cli`)
  puts `pyrs-yaml-core` directly behind a jq/yq-style command line, with no
  Python at runtime: `fmt` (comment/order-preserving round-trip), `get <path>`
  (JSONPath-lite: dot keys, `[n]`/`[-n]`, bracket keys, `*`; `--json`/`--raw`
  output), `set <path> <value>` and `delete <path>` (yq-style edits with
  `--create-missing` and `-i/--inplace` file rewrite; edited documents
  re-serialize through the round-trip serializer, keeping comments and
  the value's own style), `to-json` (order-preserving), `to-toml`, and the import commands
  `from-json` / `from-toml` / `from-ini`. Input formats resolve by extension
  (`--input` overrides; YAML content is always a JSON superset); stdin via
  `-` or omission; non-zero exit with the core's stable error text on
  failures. 12 integration/unit tests pin parity with the Python API.
- **TOML and INI exchange formats** — hub-and-spoke multi-format support
  with YAML as the single editable representation:
    - `from_toml(toml: str) -> str` and `to_toml(yaml: str, schema=...) -> str`
    convert TOML text ⇄ YAML text (Rust `toml_edit` 0.25);
    - `load_toml(toml: str) -> dict` materializes TOML directly into Python
    values, with datetimes routed through the built-in `!timestamp` plugin;
    TOML strings never re-resolve (a `"true"` value stays a string);
    - `load_ini(text: str) -> dict` reads INI via stdlib configparser (strict,
    case-preserving; read-only by design — INI has no official grammar).
  TOML output rejects inexpressible shapes (null values, non-table roots,
  aliases, non-scalar keys) with stable error messages. Round-trip editing
  (comments, anchors, splices) remains YAML-only by design.

### Changed

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
- **Native JSON & TOML cores** — the `serde_json` and `toml_edit` dependencies
  are gone. `pyrs-yaml-core` ships an RFC 8259 JSON engine (byte-level
  scanner, verbatim number spelling so `from_json → to_json` is byte-stable
  and precision-preserving for large integers/doubles, typed line/column
  errors, strict rejections for trailing commas, leading zeros, lone
  surrogates, unescaped control characters, and multi-root documents) and a
  TOML 1.0 engine covering the full grammar — bare/quoted/dotted keys,
  basic/literal/multiline strings with every escape form, decimal /
  hexadecimal / octal / binary integers with underscore separators, floats
  including `inf` / `nan` / exponent forms, and offset / local date, time,
  and date-time — every rejection surfacing as `ParseError::Syntax` with
  0-indexed `line`/`col` in the granit-parser house style. CLI, bindings
  and pyproject surface unchanged; round-trip tests and `tests/test_toml.py`
  green with the native path.
- **Internal duplication cleanup** — benchmark fixtures composed from shared
  blocks, PyO3 path-edit methods delegate to the existing
  `apply_metadata_edit` helper, and repeated file-read/error-mapping and
  line-offset boilerplate collapsed into shared functions. No public behavior
  change; the duplicate-code rate measured by jscpd drops from 5.25% to 3.45%.
- **`YamlDocument.validate()` caches compiled validators** — the first
  successful validation against a schema (JSON text *or* dict) caches a
  compiled `jsonschema` validator; later calls skip schema parsing,
  meta-schema checking, and validator construction. Dict schemas are keyed by
  object identity with a deep-copy snapshot guard: an in-place mutation is
  detected by `==` on the next use and transparently recompiles. The cached
  path raises `exceptions.best_match(validator.iter_errors(instance))`,
  identical to `jsonschema.validate()` semantics. WSL wall-time:
  `document_validate` −98% (660µs → 13µs).
- **Structural dedupe of parser/serializer kernels** — mapping and sequence
  rendering share one `write_container_node` skeleton (output byte-identical,
  `serialize_*` medians −5~11%); single- and multi-document parse entry
  points share one `load_ast` error contract; the schema resolution chains
  share `bool_word`/`numeric_tail` and YAML 1.1 no longer re-checks the
  core's null/bool words per scalar; anchor registration (`register_anchor`)
  and the standalone-vs-inline comment taxonomy (`is_standalone_placement`)
  are single-sourced across the AST and stream receivers. Repo duplicate
  rate 3.38% → 2.60%.

### Fixed

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
  the built-in plugin now normalizes `...Z` to `+00:00` first, fixing
  `ValueError: Invalid isoformat string` on 3.8–3.10 for YAML
  `!timestamp` scalars and TOML datetimes arriving through `load_toml` /
  `from_toml`.

### Performance

- **Direct event→Python load materialization** — `safe_load`, `safe_loads`
  and `YAML().safe_load*` now build Python objects in ONE pass over the
  granit event stream (collect + recursive descent) instead of constructing
  the full `CustomNode` AST only to walk it again in `convert.rs`. Plain
  scalars resolve through the same schema chain, mapping keys keep their
  raw-text semantics, and duplicate-key errors are reported identically;
  anchored/tagged/merge/multi-document shapes stay on the AST pipeline via
  zero-cost pre-vetoes (a `&` byte scan, a separator line-scan, and a
  payload-level `<<` check). WSL wall-time: `safe_load_scalar_types`
  −21~25%, `safe_load` family −13~18%, fallbacks unchanged. The event-span
  anchor-replay variant was implemented, measured (+48% on anchored docs),
  and reverted; the alias-parity tests in `tests/test_direct_load.py` pin
  the anchored-shape behavior spec either way.
- **Anchor extraction byte gate** — `extract_anchors` returns empty after a
  single `&` byte-containment check; documents without anchors (the common
  case) skip the per-character quote state machine entirely. Rust-side
  `parse_*` divan benches improve 11–18% at the median (e.g. `parse_large`
  31.2µs → 26.3µs; the extraction scan itself drops 1.5µs → 38ns).
- **Interned stream-event dict keys** — the fixed keys emitted per event by
  `parse_stream` / `load_stream` (`line`, `column`, `type`, `value`, `style`,
  `anchor`, `tag`) now reuse interned string objects via `pyo3::intern!`,
  eliminating one Python string allocation per key insert. WSL wall-time:
  `parse_stream` −34%, `parse_stream_multidoc` −39%, `load_stream` −22%.
- **Decomposition micro-benchmarks** — new `granit_events_*` benches isolate
  the pure granit event-pipeline cost from AST construction (bench-only).
- **Multi-document parse without per-document clones** — `on_document_end`
  moves the completed document into the collection instead of deep-cloning it
  (the next document rebuilds the result; the clone was pure overhead).
  WSL wall-time: `parse_all_docs` −9.7%, `safe_loads` (multi-doc) −9.5%,
  `YAML().safe_loads` −6.7%.
- **Streaming write reuses one buffer across documents** — new
  `direct_dump_into` writes each document into a reused `String`, and
  `dump_iterable` skips `normalize_doc`'s re-copy when the text already ends
  with exactly one newline (the normal case). WSL wall-time:
  `dump_stream_multi_doc` −27.2%, `dump_stream` −4.4%.
- **Scalar fast paths in the AST builder** — `unescape_double_quoted` returns
  early for quoted strings without backslashes (no per-char state machine) and
  `detect_chomping` pulls lines lazily instead of collecting every line of the
  document per block scalar. WSL wall-time: `to_dict` family −4~9%,
  `safe_load_scalar_types[strings/numbers]` −3~4%, no regressions.
- **Memoized block indentation** — the serializer's indent cache is now also
  used by block-scalar header writing, and `test_benchmark_api.py` gained
  `validate` and `load_file` benchmarks.

### Docs

- **Corrected the numpy guide's 0-D scalar section (all locales)** — the
  ``0-D Scalar Arrays`` snippet claimed 0-D arrays "reshape to a single-element
  list" (`assert data == [42]`); the shipped behavior (pinned by
  `tests/test_numpy.py`) serializes them as bare scalars (`assert data == 42`).
  Text corrected in `docs/{en,zh,ja,ko}/guides/numpy.md`, and the en page gained
  a warning admonition documenting the 0-D `bool` → `1.0` rust-numpy quirk.
- **Corrected stale references across all locale docs (en/zh/ja/ko)** —
  `saphyr-parser` → `granit-parser`, YAML compliance 98.1% → 99.75%
  (405/406 suite cases), ABI3 support 3.9–3.13 → 3.8–3.15 (py3.9+ → py3.8+),
  and benchmark tables updated to current CodSpeed CI numbers (parse 21–43×,
  serialize 55–177× faster than PyYAML). Rust-side benchmark sections migrated
  from Criterion to divan (`benches/yaml_bench.rs` →
  `crates/pyrs-yaml/benches/yaml_bench.rs`).

## [v0.15.0] — 2026-08-19

### Added

- **Node metadata setters/getters** — `Node.comment` / `Node.anchor` /
  `Node.tag` read properties and `set_comment` / `set_anchor` / `set_tag`
  (plus `remove_*` variants) on `python/pyrs_yaml/node.py`, backed by new
  path-based edit operations in `py/editing/mod.rs` and `#[pymethods]`
  (`_set_comment_path`, `_set_tag_path`, `_set_anchor_path`, `_remove_*_path`,
  `_get_comment`, `_get_anchor`, `_get_tag`). Editing an alias or a missing
  path raises; standalone comments on inline scalar values and sequence items
  are now serialized on their own indented lines (fixes pre-existing broken
  round-trip for `child:\n  # c\n  val` and `- a\n# c\n- b`).
- **Node style/format setters/getters** — `Node.scalar_style` /
  `Node.flow_style` / `Node.chomping` read properties and
  `set_scalar_style` / `set_flow_style` / `set_chomping` methods, backed by
  `#[pymethods]` (`_set_scalar_style_path`, `_set_flow_style_path`,
  `_set_chomping_path`, `_get_scalar_style`, `_get_flow_style`,
  `_get_chomping`). ScalarStyle/Chomping now derive `Copy`. Non-scalar nodes
  return `None` / are no-op; aliases and missing paths raise.
- **Verbatim tags** — `set_tag("!<tag:yaml.org,2002:str>")` now produces a
  verbatim tag (empty handle), and verbatim tags parsed from source survive
  round-trip: `Tag`'s `Display` emits `!<...>` wrapping for empty-handle tags,
  `parse_tag` recognizes the `!<...>` form, and stream events serialize tags
  through `Display`.
- **Schema file IO and listing** — `load_schema(name, path)` reads a schema
  definition from a file and registers it; `list_schemas()` returns all
  registered schema names (built-in `failsafe`/`json`/`core`/`yaml1.1` plus
  custom). Exposes the existing `registry::names()` and wraps
  `register_schema` with file I/O.
- **Schema structural validation** — a `validate` section in a schema
  definition adds structural checks (path-qualified scalar types,
  `sequence_of`/`mapping_of` containers, `required` presence); the new
  `validate_against_schema(data, schema_yaml)` raises `YamlValidateError`
  listing every failure.
- **`Node.copy()`** — deep-copies a subtree as a standalone Python value
  (dict/list/scalar), detached from the document, for pasting via
  `set_value()`.
- **Deep editing API** — `doc.set_many({"$.path": value})` sets multiple
  paths (with wildcard `[*]` and deep-scan `..` support) in a single splice
  burst; `doc.sort_keys()` orders mapping keys in place; `Node.move(new_path)`
  relocates a subtree atomically; `Node.path` / `Node.find_first()` /
  `Node.value_eq()` add path access, first-wildcard lookup, and value
  comparison. Rust primitives: `sort_keys_path` / `move_path` /
  `set_many_path` + `apply_batch_edit`.
- **Property-based testing for 0.14+ features** — Rust proptests for
  `validate_node` (no panic, error paths exist), schema parsing, and
  style-settings round-trip; Python hypothesis tests for `set_many`
  wildcard equivalence, metadata-edit value preservation, and `sort_keys`
  idempotency. `hypothesis` moved to the `test` dependency group so CI
  (`uv sync --group test`) actually runs property tests.
- **Serializer fix** — standalone comments on **empty** flow containers
  (`key: {}` / `key: []`) no longer serialize to invalid YAML; they are
  demoted to inline comments (`key: {}  # note`).

### Changed

- **NumPy re-enabled on free-threaded (cp314t) wheels** — the
  `--no-default-features` flag is removed from the cp314t build lines in
  `publish.yml` and `ci.yml`; rust-numpy 0.29 (already pinned) supports
  free-threaded Python since v0.24.0, so `numpy.ndarray` serialization is now
  available on free-threaded wheels when NumPy is installed (auto-detected at
  runtime; inert when absent). Closes the deferred Research & Exploration item
  "Numpy free-threaded re-enable".
- **granit-parser upgraded to 1.1.0** — the YAML 1.2 parser dependency moves
  from the `1.0` to `1.1` semver line. Backward-compatible maintenance
  release: adds `Options`/`emit_comments` configuration, new
  `new_from_*_with_options` parser constructors, fuzz-testing hardening, and a
  performance fix for validating large plain/block scalars (ASCII fast path).
  No API changes required in `pyrs-yaml-core`.

## [v0.14.1] — 2026-08-15

### Fixed

- **Single-quoted scalars with backslash + control/noncharacter** — a quoted
  value containing a backslash was routed to single-quoting, but single quotes
  cannot escape control characters or Unicode noncharacters, so the emitted
  YAML was unparseable. Such values now use double-quoting (`direct_dump` and
  the shared `write_plain_scalar` single-quote branch).
- **Noncharacters and BOM quoted** — `needs_quotes` / `needs_double_quoted`
  now treat Unicode noncharacters (U+FFFE/U+FFFF and the plane-end twins) and
  U+FEFF (BOM) as requiring quoting: granit drops a plain U+FEFF as a
  document-start BOM and rejects raw noncharacters even inside quoted scalars.
- **Double-quoted escape width** — `write_double_quoted_scalar` escapes
  noncharacters and control chars; for code points above U+FFFF it now emits
  the 8-digit `\Uxxxxxxxx` form (the 4-digit `\u` form is only valid for the
  BMP).
- **Folded plain-scalar continuation indent** — `wrap_plain_scalar`
  continuation indent is no longer a fixed 2 spaces; it is derived from the
  value's start column on the current line, so folded plain scalars inside
  nested sequence/mapping items stay indented past the parent block indent
  (granit otherwise reports "simple key expected ':'").
- **Multi-byte wrap boundary** — `wrap_plain_scalar` now floors the wrap slice
  to a char boundary instead of panicking when a 4-byte UTF-8 character
  straddles the wrap column.
- **`hypothesis` in publish test requirements** — `.ci/requirements-test.txt`
  now pins `hypothesis>=6.113.0` so the publish workflow (which does not
  install the `dev` dependency group) can run the property test suite.

### Added

- **`scripts/fuzz_panics.py`** — high-volume local Hypothesis fuzz harness that
  bypasses pytest `@settings` caps with a hostile strategy (control chars,
  NBSP, backslashes, long multibyte runs) across dump/parse/edit/idempotency.

## [v0.14.0] — 2026-08-14

### Added

- **YAML Schema Language** — define custom schemas as YAML files with a
  `rules` list mapping regex patterns to YAML types (`null`/`bool`/`int`/
  `float`/`str`), plus an optional `extends` base schema. Registered via
  `register_schema(name, schema_yaml)` and used as `YAML(schema=name)`.
- **Inline dict schema** — the `schema` parameter of `YAML()`, `parse()`,
  `parse_file()`, `parse_all_docs()`, `safe_load()`, and `safe_loads()`
  accepts an inline dict, serialized and registered automatically.
- **Community Plugins** — `CustomType` base class with
  `can_parse`/`from_yaml`/`to_yaml`/`validate` methods; register via
  `register_type()` (imperative or decorator). Custom types handle tagged
  scalars on load and Python objects on dump.
- **Built-in plugins** — `!timestamp` (maps to `datetime`) and `!set`
  registered by default in `pyrs_yaml/plugins/`.

### Changed

- **Schema resolution is pluggable** — `YamlSchema` enum refactored into a
  `SchemaResolver` trait + `Schema` enum with a global `SchemaRegistry`
  pre-loaded with the four built-in schemas (`failsafe`, `json`, `core`,
  `yaml1.1`). Custom schemas register via the registry; built-in Core keeps
  its zero-cost `match` dispatch.
- **`node_to_pyobject` and `direct_dump` check registered `CustomType`s** —
  tagged scalars convert via `from_yaml()` on load; matching Python objects
  serialize via `to_yaml()` on dump.
- **`get()` is literal-key only** — `YamlDocument.get()` no longer guesses
  JSONPath for keys containing `.` or `[`; every key is treated as a
  top-level mapping key, consistent with `__getitem__`/`__setitem__`.
  Path access stays available via `find()`/`node()`.

### Fixed

- **Quoted scalars always load as strings** — implicit type resolution now
  applies only to plain scalars (YAML 1.2): `safe_load('"true"')` returns the
  string `"true"`, not `True`. The serializer keeps negative numbers
  round-tripping through the document (`to_yaml`) path.
- **Lone-quote keys round-trip** — mapping keys that are a single `'` or `"`
  are emitted as quoted scalars instead of unparseable YAML.
- **Empty collections emit `{}`/`[]`** — dumping empty mappings/sequences no
  longer yields an empty document that re-parses as `None`.

## [v0.13.0] — 2026-08-10

### Changed

- **Rust MSRV raised to 1.96 and edition bumped to 2024** - both crates now
  declare `rust-version = "1.96"` and `edition = "2024"`; CI pins the
  `build`/`test-freethreaded` jobs to Rust 1.96 for deterministic wheel builds
  and adds an `msrv-check` job running `cargo check`/`cargo test` at the MSRV
  to prevent silent MSRV drift (the `rust-lint` job stays on `stable`).
  The floor is set above PyO3 0.29's own baseline (rustc 1.83) for std API
  headroom (e.g. `assert_matches!`, stabilized 1.96) with no code migration
  needed. `TAG_REGISTRY` (tag handler storage) refactored to
  `std::sync::LazyLock`, dropping the `Mutex<Option<...>>` indirection.

### Performance

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

### Fixed

- **`float_to_yaml_string` round-trip fix** — appends `.0` when Rust
  Display drops the decimal (`42` → `42.0`) so floats round-trip as
  floats instead of becoming ints.
- **Reverted `count_nodes` pre-allocation** — the full AST traversal cost
  more than the reallocations it avoided (serialize_10mb was ~14% slower);
  buffer growth is left to the Vec.

### Added

- **`max_depth` on stream & frontmatter APIs** — `parse_stream(yaml, on_event, max_depth)`,
  `read_markdown(path, schema, max_depth)`, `read_markdown_str(content, schema, max_depth)`
  accept `max_depth` (default 1000). Stream parsing now enforces the nesting-depth limit
  via core `parse_stream_with_options` (previously stream events had no depth limit).
- **Pydantic integration** — `dump_pydantic()` serializes a Pydantic model
  to YAML string via `model_dump(mode='json')` + `safe_dump`; `parse_as()`
  parses YAML string into a Pydantic model instance. Both use lazy imports,
  no hard dependency on pydantic. (#61)

### Internal

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

## [v0.12.1] — 2026-08-06

### Added

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

### Changed

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

### Fixed

- **`set(create_missing=True)` nested chain build** - the created mapping
  chain no longer duplicates the first segment as a nested key level.
- **`set(create_missing=True)` eligibility** - freshly created keys are now
  eligible for the value write (the eligibility check no longer runs after
  the synthetic pair is inserted).
- **Standalone comments before simple mapping keys** - round-trip
  previously dropped standalone comments attached to simple-key nodes;
  now preserved (two regression tests).

## [0.11.7] - 2026-08-04

### Changed

- **stub-build-check replaced with release-guard** - the always-red container
  build (`validate.yml`) that deliberately failed to reproduce the v0.10.0
  `--generate-stubs` failure mode is replaced with three static assertions
  that **pass** when the repo is correct: `grep` guards `publish.yml` against
  `--generate-stubs`, `git ls-files` asserts the committed `.pyi` is tracked,
  and `test -f` checks `py.typed` exists. The job now gives green CI on
  correct state, red only on regression.

### Added

- **Numpy free-threaded tracking** - ROADMAP.md now tracks `rust-numpy` free-
  threaded support status (PyO3/rust-numpy#476) as a dependency for re-enabling
  ndarray serialization on cp314t wheels when the Rust binding matures.

## [0.11.6] - 2026-08-04

### Changed

- **Free-threaded (cp314t) wheels are now numpy-free** - built with
  `--no-default-features`, so rust-numpy is excluded entirely (smaller
  binary, no runtime probe). `safe_dump` on a `numpy.ndarray` raises
  `YamlTypeError` on free-threaded builds; GIL builds (Python 3.8-3.15)
  keep full ndarray serialization.

### Added

- **Free-threaded CI validation** - `test-freethreaded` job now builds
  and tests with `--no-default-features`, matching the shipped
  free-threaded wheel configuration.
- **Install docs** - `docs/{en,zh,ja,ko}` note that free-threaded
  wheels are numpy-free (ndarray serialization unavailable on cp314t).

## [0.11.5] - 2026-08-04

### Changed

- **Parser robustness items 3/4/5 closed via Phase 0 strictness audit** — the 70-probe corpus (indentation, block-mapping keys, flow context) compared against a PyYAML oracle showed **no fixable accepted-but-invalid case** (64/70 match; the 6 divergences are deliberate YAML 1.2 / yaml-test-suite requirements where PyYAML is the outlier, and one deliberate duplicate-key strictness). Compliance stays at **99.75% (405/406)**. Full write-up in `ROADMAP.md` §v0.11.5 and `tests/test_strictness_audit.py`.

### Added

- `tests/test_strictness_audit.py` — 70-probe strictness regression corpus pinning current rejection/acceptance behavior (both directions), so future parser changes cannot silently regress strictness or over-reject.

## [0.11.4] - 2026-08-04

### Fixed

- Duplicate null/empty mapping keys no longer error (`: a\n: b`, `~: a\n~: b`) — matches yaml-test-suite 2JQS; real duplicate keys still raise `YamlDuplicateKeyError`
- Compliance harness: correctly-rejected invalid YAML now counts as pass (was lowering the rate despite compliant behavior)
- Compliance harness: `convert_special_chars` tab decoding via regex — any run of `—`/`‖` + `»` is one tab, fixing tab-encoded suite cases

### Changed

- YAML Test Suite pass rate gate raised from >75% to **≥95%**; current rate **99.75%** (405/406)
- Known deviation documented: `ZYU8` (`%YAML 1.1 1.2`) is rejected by design (invalid per YAML 1.2 grammar, matches PyYAML/libyaml)

## [0.11.3] - 2026-08-03

### Added

- Streaming write: `YAML.dump_stream(file_obj, iterable)` / `YAML.dump_file(path, iterable)` with document-level constant memory, auto `---` separators, and `explicit_start`/`explicit_end` flags
- `YamlDocument` `with` context manager: snapshot/rollback transaction scoping
- `compliance_report()`: public YAML Test Suite pass-rate reporting (version-consistent)

### Changed

- Edit-burst line-offset cache: internal O(N+edit) carry-through in the splice layer (public API unchanged)
- `compute_compliance` moved from tests to `pyrs_yaml.compliance`; version no longer hardcoded

### Fixed

- Changelog mirror drift guard: prek hook + CI job assert root/mirror `[Unreleased]` sync
- Publish stub pre-validation: CI reproduces v0.10.0-class `--generate-stubs` container failures before Release

## [0.11.2] - 2026-08-03

### Added

- `YAML.load_stream(file_obj)` / `YAML.load_stream_file(path)`: lazy event iterators with O(anchors + chunk) memory

### Performance

- **Parse no longer computes splice eligibility** — the O(document) layout check now runs lazily on the first edit via `YamlDocument.splice_checked`, restoring the v0.11.0 regression: parse_comments -59%, parse_anchors -42%, parse/roundtrip/edit -10~35% all back to v0.10.0 levels
- **Linear-cursor layout check** — replaces per-node binary search over precomputed line offsets (monotonic source-order traversal)

### Changed

- `parse_with_options` returns `CustomNode` (was `(CustomNode, bool)`); splice eligibility is now internal to `YamlDocument` and computed on demand

## [0.11.0] - 2026-08-02

### Added

- **Surgical Serialization** — byte-level source span tracking on every AST node; segment-based splice — edits regenerate only the touched region, untouched text is byte-copied
- proptest fidelity property tests (new dev-dependency)
- 10MB edit-flush benchmarks (divan)

### Changed

- `flush_source` now splices segments; falls back to full serialization for flow-style regions, non-default layout documents, merged keys, CRLF/BOM documents, and after materialization (single-burst model)
- Splice edits preserve `---`/`...`/directive marker lines as untouched bytes (full serialization previously dropped them — deliberate behavior difference)

## [0.10.0] - 2026-08-01

### Added

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

### Changed

- `YamlDocument.source()` now returns `str` and lazily re-serializes after in-place edits

## [0.9.0] - 2026-08-01

### Added

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

### Changed

- CI Python matrix expanded: 3.8-3.14 across ubuntu, windows, macos
- Stable ABI: `abi3-py39` → `abi3-py38` (wider Python 3.8+ support), added `abi3t` + `abi3t-py315` (free-threaded stable ABI)
- `pyproject.toml` classifiers updated with 3.13, 3.14, 3.15 entries
- **CI optimization: redundant Rust compilation eliminated** — a single `rust-lint` job runs `cargo clippy` + `cargo test` once; the build job produces one abi3 wheel per OS which test jobs install instead of running `maturin develop`, removing Rust compilation from 21 matrix jobs (~86% fewer compiles); `Swatinem/rust-cache` added to all jobs
- **pydantic test dependency** — `pydantic>=2.10.6` added to `[dependency-groups] test` and `.ci/requirements-test.txt` (SSOT via `uv sync` in ci.yml)

### Fixed

- **Windows DLL loading** — removed `#[cfg(test)]` block from `src/py/tag_registry.rs` which broke `import pyrs_yaml` on Windows (`250b8d0`)
- **Python 3.8 compatibility** — `from __future__ import annotations` in `pydantic.py` (`63d2495`)
- **CI pydantic skip** — `pytest.importorskip("pydantic")` so tests pass when pydantic is not installed (`7be011d`)
- **CI glob expansion on Windows** — `shell: bash` for `pip install dist/*.whl` (PowerShell does not expand `*`) (`2f7778d`)
- **Non-string tag handler returns now raise `YamlTagError`** — a handler returning a non-`str` value (previously silently ignored, keeping the original scalar) now errors with `Tag handler '!x' must return a string` (`src/py/mod.rs:resolve_tags`)
- **`to_yaml_with_options` indent wiring** — `indent_mapping`/`indent_sequence`/`indent_offset` are now honored by the serializer (previously dead fields); each defaults to `indent_size`/0 when omitted (`src/serializer.rs`)
- **`width` no longer hangs for tiny values** — `width < continuation indent` falls back to emitting the remainder unwrapped instead of looping forever (`src/serializer.rs:write_plain_scalar`)
- **`remove_tag(name)`** — new function to unregister a tag handler; complements `register_tag`/`clear_tag_handlers` (`src/py/tag_registry.rs`)
- **`duplicate-key` errors are i18n'd** — `YamlDuplicateKeyError` messages now flow through `format_i18n_error` across all 4 locales (`src/i18n/locales/*.yml`)

## [0.8.0] - 2026-07-30

### Added

- **`YAML()` instance API** — `YAML(typ="rt"|"safe"|"full", schema="core"|"yaml1.1", max_depth=1000)` with reusable configuration; `.parse()`, `.safe_load()`, `.safe_loads()`, `.parse_file()`, `.parse_all_docs()` methods
- **Python `Node` API** — `Node` class with `find()`, `filter()`, `walk()`, `to_yaml()`, `parent`, `children`, `root_type`, `value` for AST navigation; JSONPath-like query language (`$.key.sub`, `$.arr[0]`, `$..deep`)
- **`doc.version` metadata** — `YamlDocument.version()` returns the YAML spec version (default "1.2")
- **`MergedView`** — `doc.merged()` returns a read-only dict-like view with merge keys resolved
- **Lifecycle warnings** — `Node.release()` to explicitly invalidate a node; stale access emits `RuntimeWarning` + `YamlDocumentError`

### Changed

- `parse()` / `safe_load()` now delegate to `YAML().parse()` / `.safe_load()` as syntactic sugar
- `YamlDocument` now stores `version` field for document metadata

## [0.7.1] - 2026-07-30

### Added

- **ryaml benchmark comparison** — `tests/test_benchmark.py` now benchmarks against `ryaml` (Rust YAML library) alongside PyYAML and ruamel.yaml; `benchmark_compare.py` rewritten as a feature comparison report (`tests/test_benchmark.py:25-28`, `.github/workflows/ci.yml:219`)
- **CI compliance threshold raised** — YAML Test Suite compliance gate increased from 70% to 75% in `test_compliance_report()`; valid parse rate gate at 95% (`tests/test_yaml_suite.py:251`)
- **CI dependency consolidation** — added `.ci/requirements-test.txt` and `.ci/requirements-test-lite.txt` for unified test dependency management across publish workflow and local dev
- **Benchmark modernization** — migrated from `pytest-benchmark` to `pytest-codspeed` for faster C-extension-based statistical benchmarking; all CI jobs now use `-r .ci/requirements-test.txt`
- **Rust benchmarks migrated to Divan** — replaced `codspeed-criterion-compat` with `codspeed-divan-compat` v5.0.1; 16 benchmarks rewritten from Criterion groups to `#[divan::bench]` attributes (`Cargo.toml`, `benches/yaml_bench.rs`)

### Changed

- CI benchmark job installs `ryaml` for cross-library comparison
- `benchmark_compare.py` now delegates timing to `pytest-benchmark` and serves as a feature comparison/reporting tool

## [0.7.0] - 2026-07-29

### Added

- **Serializer `max_depth` guard** — `serialize_node_internal` now tracks recursion depth and raises `YamlMaxDepthError` when exceeding the limit (default 1000), matching the parser's protection (`src/serializer.rs:135-145`)
- **Serializer hot-path optimization** — 5 optimizations targeting block-style serialization for ~4.9% roundtrip speedup:
    - Inlined `write_anchor_tag` and `write_inline_comment` None checks (eliminates method calls for ~99% of nodes)
    - `write_indent` hot/cold path split (direct index for cached levels ≤64)
    - `write_plain_scalar` fast path for short ASCII alphanumeric strings (≤8 chars)
    - `write_scalar_for_key` direct dispatch for Plain scalars (avoids dispatch chain)
- **pytest-benchmark migration** — Python benchmarks migrated from raw `time.perf_counter()` to `pytest-benchmark` for statistical rigor, structured JSON output, and CI integration (`tests/test_benchmark.py` + updated `tests/test_performance.py`)

### Changed

- `pytest-benchmark` replaces raw `timeit` in Python benchmarks
- CI benchmark job now runs `pytest --benchmark-json` instead of standalone script

### Removed

- `write_inline_comment` method — inlined at all call sites
- `Comment` import from serializer — no longer needed

## [0.6.0] - 2026-07-27

### Added

- **Async serialization** — `safe_dumps_async`, `safe_dump_async`, `safe_loads_async`, `safe_load_async` via `asyncio.run_in_executor` (`python/pyrs_yaml/async_dump.py`)
- **JSON Schema validation** — `YamlValidateError` exception + `YamlDocument.validate(schema)` method (accepts `str` or `dict`); delegates to Python `jsonschema` module
- **`YamlDocument.to_json()`** — serialize document to JSON string (uses Python `json.dumps`)
- **Incremental re-parse** — `YamlDocument` now stores source text (`doc.source()`); `doc.reparse(resolve_merges=True, schema="core")` re-parses in-place
- **29 new tests** across `test_async.py` (8), `test_validate.py` (14), `test_reparse.py` (7)

### Changed

- `YamlValidateError` registered as new custom exception (inherits `ValueError`)
- `rust_i18n::i18n!` macro path updated to `"src/i18n/locales"`
- `validate_translations()` test paths updated to match new locale directory

### Removed

- Deleted redundant `src/i18n/en.ftl`, `src/i18n/zh-CN.ftl` (never referenced by rust-i18n)
- Moved `locales/*.yml` → `src/i18n/locales/` (co-located with i18n module)

### Dependency Changes

- Runtime dependency: `jsonschema>=4.25.1`
- Dev dependency: `pytest-asyncio>=0.23` (moved from runtime, no longer pinned)

## [0.5.0] - 2026-07-27

### Fixed

- **`Serializer::write_node`** — `.unwrap()` on `values.iter().next().unwrap()` in `block_mapping`/`block_sequence` replaced with safe indexed access to eliminate potential panic on edge-case ASTs
- **`YAML_SCHEMA` constant** — typo `yamorg2002` corrected to `yamlorg2002` (matches YAML 1.2 spec URL)
- **Development documentation** — `AGENTS.md` updated with mandatory `uv run` prefix for Python commands and direct `cargo` for Rust commands

## [0.4.0] - 2026-07-27

### Added

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

### Changed

- Fixed version sync: `python/pyrs_yaml/__init__.py` `__version__` updated from 0.2.0 to 0.4.0 to match Cargo.toml/pyproject.toml
- Removed stale 0.2.0 wheel artifacts from `dist/`

## [0.3.0] - 2026-07-27

### Added

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

### Fixed

- **Negative number round-trip** — YAML 1.2 block sequences cannot contain plain scalars starting with `-`; negative numbers are now quoted during serialization and correctly parsed back as integers/floats
- **N-D array support** — replaced `PyArray1<T>` with `PyArrayDyn<T>` to support arrays of any dimension, not just 1-D
- **Correct nesting depth** — multi-dimensional arrays now produce exactly N levels of nesting (shape[1..] handles inner dimensions, root dimension wrapped by `plain_sequence`)
- Alias resolution in `to_dict()` and `safe_load()` — aliases now resolve to referenced values instead of `None`
- `safe_loads()` no longer uses naive `split("---")` — uses saphyr's document events
- Mapping/Sequence tags no longer discarded during parsing
- `format_scalar_for_key()` now handles Literal/Folded block scalar styles

### Changed

- Added `numpy` crate (v0.29) as a dependency for ndarray type dispatch
- Upgraded PyO3 from 0.21 to 0.29
- Replaced 15+ boilerplate `CustomNode` constructions with `plain_scalar()`/`plain_mapping()`/`plain_sequence()`/`plain_null()` constructors
- Serializer extracted `write_anchor_tag()` and `write_inline_comment()` helpers
- Parser extracted `detect_flow_style()` helper
- Removed dead code: `ParseOptions`, `find_inline_comment`, `find_standalone_comment_before`, `format_yaml_type` (test-only)
- Consolidated 6 duplicate test files, moved 9 diagnostic scripts to `scripts/`
- Improved error messages with key/index/type context

## [0.1.0] - 2026-07-25

### Added

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
