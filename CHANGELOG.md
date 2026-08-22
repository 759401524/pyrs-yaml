# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

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
  subcommand.- **Optional third-party type plugins** — `!duration` (`pendulum.Duration`),
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

### Docs

- **Corrected stale references across all locale docs (en/zh/ja/ko)** —
  `saphyr-parser` → `granit-parser`, YAML compliance 98.1% → 99.75%
  (405/406 suite cases), ABI3 support 3.9–3.13 → 3.8–3.15 (py3.9+ → py3.8+),
  and benchmark tables updated to current CodSpeed CI numbers (parse 21–43×,
  serialize 55–177× faster than PyYAML). Rust-side benchmark sections migrated
  from Criterion to divan (`benches/yaml_bench.rs` →
  `crates/pyrs-yaml/benches/yaml_bench.rs`).

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
