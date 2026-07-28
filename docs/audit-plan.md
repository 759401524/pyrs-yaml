# pyyaml-rs Comprehensive Audit Plan

**Date:** 2026-07-28
**Scope:** Full codebase audit — Rust source, Python bindings, tests, and documentation
**Goal:** Identify and catalog duplicate functionality, redundant data flows, inefficiencies, and deprecated components

---

## 1. Duplicate Functionality

### 1.1 `collect_anchors` — Two functions, same name, different purposes

- **`src/lib.rs:410`** — `collect_anchors(node, &mut HashMap<String, &CustomNode>)` — collects anchor name → node reference for alias dereference
- **`src/parser/yaml/merge.rs:35`** — `collect_anchors(node, &mut HashMap<String, IndexMap<CustomNode, CustomNode>>)` — collects anchor name → mapping pairs for merge key resolution

**Action:** Rename the merge.rs variant to `collect_anchor_mappings` to eliminate name collision and clarify intent.

### 1.2 `safe_dump` / `safe_dumps` — Trivial alias with zero added value

```rust
// src/lib.rs:1088-1100
fn safe_dump(py: Python, data: Py<PyAny>) -> PyResult<String> { ... }
fn safe_dumps(py: Python, data: Py<PyAny>) -> PyResult<String> {
    safe_dump(py, data)  // ← pure delegation
}
```

**Action:** Remove `safe_dumps` and deprecate it in the Python `__init__.py` exports, or merge into `safe_dump` with a deprecation warning.

### 1.3 `find_inline_comment` — Duplicated in two receivers

- `src/parser/mod.rs:234-262` (AstReceiver)
- `src/parser/stream.rs:136-158` (StreamReceiver)

Identical logic, identical variable names, identical early-return/restore pattern.

**Action:** Extract a shared helper `find_inline_comment(raw_comments: &[RawComment], comment_idx: &mut usize, line: usize, after_col: usize) -> Option<Comment>`.

### 1.4 `find_standalone_before_line` — Same duplication

- `src/parser/mod.rs:265-281`
- `src/parser/stream.rs:160-176`

**Action:** Extract shared helper as above.

### 1.5 `next_anchor_name_from_raw` — Same duplication

- `src/parser/mod.rs:284-292`
- `src/parser/stream.rs:178-186`

**Action:** Extract shared helper.

### 1.6 Line offset computation — Duplicated in two constructors

`AstReceiver::new()` and `StreamReceiver::new()` both compute `line_offsets` with identical logic. `stream.rs` has a standalone `compute_line_offsets()` function, but `mod.rs` duplicates the inline loop.

**Action:** Use `compute_line_offsets()` from `stream.rs` in `mod.rs` as well (or move it to a shared module).

### 1.7 `convert_tag` — Two slightly different versions

- `src/parser/mod.rs:142-167` — takes `&saphyr_parser::Tag`, returns `Tag`
- `src/parser/stream.rs:409-420` — takes `Option<&saphyr_parser::Tag>`, returns `Option<Tag>`

**Action:** Make `convert_tag` in `mod.rs` take `Option<&Tag>` and return `Option<Tag>`, then use it uniformly.

### 1.8 Integration test `EventSink` pattern — Duplicated in two files

- `src/integration/saphyr.rs:3-11`
- `src/integration/yaml_suite.rs:5-13`

**Action:** Extract a shared `EventSink` test helper in `integration/mod.rs`.

---

## 2. Redundant Data Storage & Processing

### 2.1 `parse_all_docs` clones full YAML source for every document

`src/lib.rs:807-814`:

```rust
.map(|ast| YamlDocument {
    ast,
    schema: schema_enum,
    source: Some(yaml.to_string()),  // ← clones the ENTIRE input for each doc
})
```

**Impact:** For a 10-document YAML file, the full source string is duplicated 10 times.

**Action:** Store a shared `Arc<str>` or `Rc<str>` for the source, or store only the document index range rather than the full text per document.

### 2.2 `resolve_merges` parameter in `parse_stream` is dead code

`src/parser/stream.rs:442`: `let _ = resolve_merges;` — explicitly ignored.

**Action:** Remove the `resolve_merges` parameter from `parse_stream()`, `StreamReceiver`, and the Python-facing `parse_stream` function signature.

### 2.3 `node_to_pyobject` creates empty collections per call

`src/lib.rs:456-460`:

```rust
fn node_to_pyobject(node: &CustomNode, py: Python, schema: YamlSchema) -> PyResult<Py<PyAny>> {
    let anchors = HashMap::new();  // ← allocated but never used
    let mut visited = HashSet::new();  // ← allocated but never used
    node_to_pyobject_inner(node, py, &anchors, &mut visited, schema)
}
```

**Action:** Split `node_to_pyobject_inner` into two paths — one that takes anchors/visited and one that doesn't — or use `None` for the empty collections.

---

## 3. Inefficient / Unnecessary Processes

### 3.1 `stream_event_to_py_dict` — ~130 lines of repetitive dict construction

`src/lib.rs:860-987` — Each event type manually constructs a `PyDict` with identical patterns (`set_item("type", ...)?`, `set_item("value", ...)?`, etc.). The comment-handling branches are especially verbose.

**Action:** Create a helper macro or builder pattern to reduce duplication. At minimum, extract the common dict-creation and type-setting pattern.

### 3.2 `ndarray_to_node` dispatch macro — 18 near-identical arms

`src/lib.rs:557-571` — The `dispatch_dtype!` macro is applied 18 times. 8 integer types all use `v.to_string()`. Float types differ only in NaN handling. Complex types differ only in format string.

**Action:** Group integer types into a single macro invocation or use a macro that takes the type and conversion expression as parameters more aggressively.

### 3.3 `extract_anchors` — Cognitive complexity 67

`src/parser/yaml/comment.rs:92-160` — The function does quote-tracking, anchor-name scanning, and boundary detection all in a single pass with deep nesting.

**Action:** Decompose into: (1) a quote-aware character iterator, (2) an anchor name scanner, (3) a boundary detector.

### 3.4 `resolve_mapping_merges` — Cognitive complexity 46

`src/parser/yaml/merge.rs:58-116` — Mixes merge key detection, anchor lookup, pair merging, and order preservation in one function.

**Action:** Extract: `collect_merge_data()`, `apply_merge_pairs()`, `prepend_merged_pairs()`.

### 3.5 `unescape_double_quoted` — Cognitive complexity 37

`src/parser/yaml/scalar.rs:19-85` — Deep match chain with escape handling, Unicode escapes, and line continuation.

**Action:** Extract `unescape_unicode_escape()` and `unescape_line_continuation()` helpers.

### 3.6 `format_yaml_type` test helper duplicated in `types.rs`

`src/parser/yaml/types.rs:40-63` — A test-only `format_yaml_type` function is defined inside `#[cfg(test)] impl YamlType`. This is fine in isolation, but the formatting logic mirrors the serializer's scalar writing and could drift out of sync.

---

## 4. Outdated / Deprecated Components

### 4.1 `YamlSchema::Yaml11` naming inconsistency

The variant `Yaml11` doesn't match YAML 1.1 spec naming conventions. Other variants use `Core`, `Json`, `Failsafe` — short, canonical names. `Yaml11` should be `Yaml1_1` for consistency and clarity.

### 4.2 `resolve_yaml_type` legacy single-argument API

`src/parser/yaml/types.rs:32-34`:

```rust
pub fn resolve_yaml_type(value: &str) -> YamlType {
    crate::parser::yaml::schema::resolve_yaml_type(value, YamlSchema::Core)
}
```

This legacy function hardcodes Core schema and predates the schema-parameterized version. It's only used internally but creates ambiguity about which version to call.

**Action:** Deprecate and remove; redirect callers to `resolve_yaml_type(value, schema)`.

### 4.3 `YamlOrg2002` alias in `parse_schema`

`src/lib.rs:398`: `"yaml.org,2002" | "yamlorg2002"` → `YamlSchema::Core`. The mixed-case alias `YamlOrg2002` is non-standard and confusing.

**Action:** Remove the `YamlOrg2002` alias or standardize to `yaml-org-2002`.

### 4.4 `jsonschema` Python dependency not optional

`pyproject.toml` lists `jsonschema>=4.25.1` as a hard dependency, but `validate()` is not a core feature. If `jsonschema` is unavailable, the import fails at the Python level with an unhelpful error.

**Action:** Make `jsonschema` an optional dependency with a lazy import in `validate()`, or document the requirement clearly.

### 4.5 `read_markdown_str` takes unused `_py: Python` parameter

The function doesn't interact with Python objects and doesn't need the GIL, but PyO3 requires the parameter for `#[pyfunction]`. This is a known PyO3 limitation but worth noting.

### 4.6 `_is_last` parameter in `serialize_node_internal` is unused

`src/serializer.rs:119` — The `_is_last` parameter is never read. It was likely intended for formatting decisions (e.g., trailing commas) but was never implemented.

---

## Execution Plan

| Priority | Category | Item | Effort | Impact |
|----------|----------|------|--------|--------|
| P1 | Duplicate | 1.1–1.8 (extract shared helpers) | Medium | High — reduces maintenance burden |
| P1 | Redundant | 2.1 (source cloning in parse_all_docs) | Medium | High — memory efficiency |
| P1 | Dead code | 2.2 (resolve_merges in stream) | Low | Medium — removes confusion |
| P2 | Inefficient | 3.1 (stream_event_to_py_dict refactor) | Medium | Medium — code clarity |
| P2 | Inefficient | 3.3–3.5 (decompose high-CC functions) | Medium | Medium — maintainability |
| P3 | Deprecated | 4.1–4.6 (naming, aliases, deps) | Low–Medium | Low–Medium — correctness |

---

## Verification Plan

1. Run `cargo clippy -- -D warnings` to ensure no new warnings after refactoring
2. Run `cargo test` to verify all existing tests still pass
3. Run `uv run pytest tests/ -v` to verify Python-side tests pass
4. Run `cargo fmt` to ensure formatting compliance
5. Verify round-trip preservation tests still pass after any serialization changes
