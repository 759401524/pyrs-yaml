# Roadmap

> See [CHANGELOG.md](CHANGELOG.md) for detailed per-version change logs (Keep a Changelog format).
> Roadmap tracks planned capabilities; CHANGELOG tracks shipped changes.

All versions follow [Semantic Versioning](https://semver.org/) (major.minor.patch). Pre-1.0: MINOR adds features, PATCH fixes bugs.

**API stability**: v0.x releases do not guarantee backward compatibility. Stable semver guarantee starts at v1.0.

---

## Architecture: Rust Core, Python Expression

```text
Python layer (flexible, ecosystem-friendly)          Rust layer (fast, safe, deterministic)
┌────────────────────────────────┐                   ┌──────────────────────────────────┐
│  YAML() instance API           │                   │  YAMLConfig (Rust struct)        │
│  YAML(typ, schema, depth)      │── PyO3 boundary ─▶│  parse / serialize engines       │
│  Node high-level API           │                   │  CustomNode AST structures       │
│  Node.find / .filter / .walk   │                   │  max_depth guard                 │
│  .set_value / .to_yaml()       │                   │  MergedView (read-only)          │
│  Tag registry (@decorator)     │                   │  Tag handler registry            │
│  Pydantic integration          │                   │  YAML_SCHEMA constants           │
│  Error formatting (Python)     │                   │  Serializer hot-path             │
│  Benchmark orchestration       │                   │  YAMLTestSuite compliance        │
└────────────────────────────────┘                   └──────────────────────────────────┘
```

| Layer | Responsibility | Strength |
|:------|:---------------|:---------|
| **Rust core** | Parse, serialize, AST data, safety guards | Zero-copy, memory-safe, deterministic GC |
| **Python layer** | API design, ecosystem integration, user interaction | Dynamic typing, decorators, introspection, Python toolchain |

**Data ownership**: `Node` is a borrowed reference into `YamlDocument`'s AST. `Node` is invalid when `YamlDocument` is garbage-collected. `Node` must not outlive its parent document.

---

## Released

| Version | Date | Changelog |
|:--------|:-----|:----------|
| v0.10.0 | 2026-08-01 | [CHANGELOG.md §\[0.10.0\]](CHANGELOG.md#0100---2026-08-01) |
| v0.9.0 | 2026-08-01 | [CHANGELOG.md §\[0.9.0\]](CHANGELOG.md#090---2026-08-01) |
| v0.8.0 | 2026-07-30 | [CHANGELOG.md §\[0.8.0\]](CHANGELOG.md#080---2026-07-30) |
| v0.7.1 | 2026-07-30 | [CHANGELOG.md §\[0.7.1\]](CHANGELOG.md#071---2026-07-30) |
| v0.7.0 | 2026-07-29 | [CHANGELOG.md §\[0.7.0\]](CHANGELOG.md#070---2026-07-29) |
| v0.6.0 | 2026-07-27 | [CHANGELOG.md §\[0.6.0\]](CHANGELOG.md#060---2026-07-27) |
| v0.5.0 | 2026-07-27 | [CHANGELOG.md §\[0.5.0\]](CHANGELOG.md#050---2026-07-27) |
| v0.4.0 | 2026-07-27 | [CHANGELOG.md §\[0.4.0\]](CHANGELOG.md#040---2026-07-27) |
| v0.3.0 | 2026-07-27 | [CHANGELOG.md §\[0.3.0\]](CHANGELOG.md#030---2026-07-27) |
| v0.2.0 | 2026-07-26 | — |
| v0.1.0 | 2026-07-25 | [CHANGELOG.md §\[0.1.0\]](CHANGELOG.md#010---2026-07-25) |

---

## Planned

### v0.7.0 — "Be the Fastest" (target: Q2/Q3 2026)

> Speed is pyyaml-rs's primary differentiator. Every release must prove it.

| # | Item | Layer | Priority | Notes |
|:--|:-----|:------|:--------:|:------|
| 1 | **Benchmark baseline** — measure current parse/serialize/round-trip speed vs PyYAML, ruamel.yaml, ryaml, yaml-edit | Python | 🔴 | Critical-path blocker; no perf claim valid without it |
| 2 | **Benchmark harness** — cargo-criterion (Rust) + Python `timeit` matrix; CI-integrated | Both | 🔴 | Establishes reproducible measurement infra |
| 3 | **Serializer hot-path optimization** — profile `Serializer::serialize_node_internal`; target block-style 50× over PyYAML | Rust | 🔴 | Depends on #1 for baseline |
| 4 | **`max_depth` protection (parser)** — recursion guard in `AstReceiver`; emit `YamlMaxDepthError`; default 1000 | Rust | 🟡 | |
| 5 | **`max_depth` protection (serializer)** — parallel guard in `Serializer::serialize_node_internal` | Rust | 🟡 | Currently missing; parser-only is insufficient |
| 6 | **Error messages with context lines** — Rust provides line/col/message; Python formats source snippets + caret marker | Python | 🟡 | Ensure max_depth errors also carry line info |
| 7 | **Alias preservation during serialization** — emit `*name` references, not expanded subtrees | Rust | 🟡 | Round-trip fidelity |
| 8 | **YAML Test Suite regression guard** — CI runs full test suite on every commit; report compliance % | Both | 🟡 | |

**Changelog mapping**: All items → `[0.7.0] - YYYY-MM-DD` section. Categories: `Added` (1–7), `Changed` (8).

---

### v0.8.0 — "Unlock the AST" (target: Q3/Q4 2026)

> Custom AST is a capability no other Python YAML library has. These features make it programmable.

| # | Item | Layer | Priority | Notes |
|:--|:-----|:------|:--------:|:------|
| 1 | **`YAML()` instance API** — `YAML(typ="rt"\|"safe"\|"full", schema="core"\|"yaml1.1", depth=1000)` with reusable config | Both | 🔴 | Core promise; blocks #2–#5 |
| 2 | **Python Node API** — `node.find(path)`, `node.filter(pred)`, `node.walk()`, `node.set_value()`, `node.to_yaml()` | Python | 🔴 | AST remains "locked" without this |
| 3 | **Simplified query language for `find()`** — JSONPath-like subset (e.g. `$.servers[0].port`) | Python | 🔴 | Defines the `path` grammar for #2 |
| 4 | **`node.is_valid()` check** — returns `False` if parent `YamlDocument` has been GC'd | Python | 🔴 | Prevents use-after-free / segfault |
| 5 | **`merged()` read-only view** — `MergedView` resolves anchors/`<<` without mutating the original AST | Rust | 🟡 | |
| 6 | **Document metadata** — `doc.version`, `doc.tags`, `doc.directives` from `doc_infos` | Rust | 🟡 | |
| 7 | **Backward compatibility** — `parse()`/`safe_load()` become syntactic sugar for `YAML().parse()`/`.safe_load()` | Both | 🟡 | |
| 8 | **Lifecycle warnings** — emit `RuntimeWarning` when a GC'd `YamlDocument` is referenced via stale `Node` | Python | 🟡 | Mitigates silent segfault risk |

**Node lifecycle contract**: `Node` borrows from `YamlDocument`. Deleting or garbage-collecting `YamlDocument` invalidates all `Node` references. Attempting to use a stale `Node` raises `YamlDocumentError("parent document has been released")`.

**Changelog mapping**: Entries under `[0.8.0]` in CHANGELOG.md.

**Scope risk**: Critical path (#1 → #2 → #3) may exceed Q3/Q4 if started fresh. Consider deferring 🟡 items to v0.8.1.

---

### v0.9.0 — "Ecosystem Ready" (target: Q4 2026)

> Integrate into the Python ecosystem without reinventing wheels.

| # | Item | Layer | Priority | Notes |
|:--|:-----|:------|:--------:|:------|
| 1 | **Tag handler registry** — `@pyyaml_rs.register_tag("!include")` decorator + `pyyaml_rs.register_tag("!ts", handler)` imperative form | Both | 🔴 | Spec: decorator wraps callable `(node) → Python object` |
| 2 | **Tag handler chaining** — first matching registration wins; explicit priority arg for conflicts | Both | 🟡 | Extensibility pattern |
| 3 | **Pydantic integration** — `yaml.parse_as(MyModel, yaml_str)` → `MyModel.model_validate(yaml.parse(yaml_str))` | Python | 🔴 | Type signature: `parse_as(model: type[T], src: str) → T` |
| 4 | **`allow_duplicate_keys`** — config flag, default `False`; when `True`, later key wins (YAML 1.2 §3.2.1.3 behavior) | Rust | 🟡 | Spec ref clarifies ambiguity |
| 5 | **`SerializeOptions` expansion** — `width` (line wrapping, default 80), `indent(mapping, sequence, offset)` | Both | 🟡 | Needs implementation spec for wrapping algorithm |

**Tag handler error propagation**: Python tag callback raises → Rust catches the Python exception → wraps in `YamlTagError("tag !include handler failed: <original traceback>")` → surfaces to Python caller with full stack trace.

**Changelog mapping**: Entries under `[0.9.0]` in CHANGELOG.md.

---

### v0.10.0 — "Edit In Place" (released 2026-08-01)

> In-place editing on a fidelity-preserving AST — the `yaml-edit` differentiator, without sacrificing round-trip.

| # | Item | Layer | Priority | Notes |
|:--|:-----|:------|:--------:|:------|
| 1 | **Rust edit primitives** — `set_path` / `insert_path` / `append_path` / `delete_path` / `rename_path` on `CustomNode` with metadata preservation (`src/py/editing.rs`) | Rust | 🔴 | ✅ Delivered |
| 2 | **`_*_path` PyO3 methods** — GIL-released (`py.detach`), atomic (revision bumped only on success), lazy `source_dirty` re-sync (`src/py/mod.rs`) | Rust | 🔴 | ✅ Delivered |
| 3 | **Python path API** — `doc.set/insert/append/delete/rename(path, ...)` + root sugar `doc["k"] = v` / `del doc["k"]`; `YamlPathError` for wildcard/`..` in edit paths (`python/pyrs_yaml/editing.py`) | Python | 🔴 | ✅ Delivered |
| 4 | **Node edit methods** — `Node.set_value/insert/append/delete/rename` with revision-based staleness detection (`YamlDocumentError` + `RuntimeWarning`) (`python/pyrs_yaml/node.py`) | Python | 🔴 | ✅ Delivered |
| 5 | **Alias-aware editing** — setting an alias's own path replaces it in place; editing *through* an alias raises `YamlEditError` | Both | 🟡 | ✅ Delivered |
| 6 | **Docs** — `docs/{en,zh,ja,ko}/guides/editing.md` + API/features/changelog updates | Docs | 🟡 | ✅ Delivered |
| 7 | **Edit benchmarks** — 6 divan benches (set/insert/delete, small→large) in `benches/yaml_bench.rs` | Rust | 🟡 | ✅ Delivered |

**Changelog mapping**: Entries under `[0.10.0]` in CHANGELOG.md.

**Remaining reserve (not committed)**: YAML 1.2 spec compliance score reporting; Python `with` context manager for document scoping; `yaml-edit` competitor feature tracking.

---

### v0.11.0 — "Surgical Serialization" (target: Q4 2026)

> Edit large documents without paying O(doc) re-serialization — completes the yaml-edit differentiator (v0.10.0 editing + this release's byte-level fidelity). Scope decided via brainstorming 2026-08-02.

| # | Item | Layer | Priority | Status | Notes |
|:--|:-----|:------|:--------:|:------:|:------|
| 1 | **Source span tracking** — `source_range: Option<Range<usize>>` on every `CustomNode` variant, populated at parse time | Rust | 🔴 | ✅ `00ecc92` | ⭐ AST structural change (approved 2026-08-02); touches all constructors/serializer/tests — 118 Rust + 658 Python tests as guard |
| 2 | **Dirty-region splice** — edit primitives (set/insert/append/delete/rename + root sugar) mark dirty nodes; `flush_source` regenerates the smallest enclosing block regions, byte-copies all untouched text | Rust | 🔴 | ✅ `7b0cf2f` | Fallback to full serialize: flow-style containers, doc-wide structural changes, layout options (`sort_keys`/`width`/`explicit_*`/indent) differing from source layout |
| 3 | **Fidelity property tests** — byte equality of untouched regions after every edit op | Rust | 🔴 | ✅ `8f6122d` | The splice-correctness guarantee |
| 4 | **Edit-flush benchmarks** — divan: single-key edit on a synthetic 10MB block doc → `source()`/`to_yaml()` time ∝ region | Rust | 🟡 | ✅ `3456f40` | Target ≥100× vs v0.10.0 full re-serialize; extend edit_* benches |
| 5 | **Docs** — guides/perf updates in en/zh/ja/ko | Docs | 🟡 | ✅ `82e73d7` | |

**Design decisions (2026-08-02)**: per-scalar exact byte splice rejected — comments/anchors/tags attached to nodes + insert/delete span shifts make it fragile; recorded as a future optimization. Subtree-memoized full serialization rejected — no benefit for linear text assembly.

**Changelog mapping**: Entries under `[Unreleased]` in CHANGELOG.md.

---

### v0.11.1 — "Streaming Parse" (target: Q1 2027)

> Constant-memory traversal of 100MB+ YAML files. Scope decided via brainstorming 2026-08-02.

| # | Item | Layer | Priority | Notes |
|:--|:-----|:------|:--------:|:------|
| 1 | **Lazy event iterator** — `YAML.load_stream(file_obj)` / `YAML.load_stream_file(path)` returning `YamlStream` iterator holding a saphyr `Parser` fed in ~64KB chunks from the Python file object | Rust | 🔴 | GIL released per chunk (`py.detach`); reuse `stream_event_to_py_dict` |
| 2 | **Early termination** — consumer stops ⇒ stop reading further chunks (existing `should_continue` semantics) | Rust | 🔴 | |
| 3 | **Memory-bound test** — 100MB synthetic file: peak RSS < ~64MB + constant | Rust | 🔴 | vs current ~input size + full AST + full event vec |
| 4 | **Event parity property test** — stream output equals `parse_stream` on the same input, event by event | Rust | 🟡 | |
| 5 | **Docs** — en/zh/ja/ko | Docs | 🟡 | |

**Design decisions (2026-08-02)**: callback-push `feed(&[u8])` model rejected for v1 (borrow-checker pain across feed calls; stdin/network later). mmap-based I/O deferred (abi3 portability). Existing string-based `parse_stream(yaml: str, on_event=...)` stays for in-memory compat.

**Changelog mapping**: Entries under `[Unreleased]` in CHANGELOG.md.

---

**Deferred (not committed, revisit at each milestone review)**: `with` context manager for document scoping; YAML 1.2 spec compliance score reporting; `yaml-edit` competitor feature tracking; community plugins / YAML Schema language; `--no-default-features` numpy-free free-threaded wheel. Tracked in Research & Exploration below with a revisit rule (see Review Notes 2026-08-02).

---

## Research & Exploration

Tracked as open questions for future roadmap inclusion; not committed to any version.

> **Revisit rule** (from Review Notes 2026-08-02): every milestone review must re-evaluate all unchecked items below — promote, defer with reason, or close. No item stays unchecked for more than two consecutive milestone reviews.

- [x] **Free-threaded CPython support** — `Py_GIL_DISABLED` + full `gil_used = false` build matrix ✅ Delivered in v0.10.0 (cp314t wheels on PyPI)
- [ ] **YAML Schema language** — dedicated schema definition format beyond JSON Schema
- [ ] **`yaml-edit` competitor analysis** — track their feature expansion; respond with differentiator strategy
- [ ] **Community plugins** — allow third-party Python modules to register custom node types
- [ ] **`--no-default-features` build** — exclude `numpy` from wheel for free-threaded Python (see CHANGELOG v0.6.0)

**Committed to v0.11.x (moved from this list, see Planned)**: Streaming large documents → v0.11.1; Incremental serialization → v0.11.0.

---

## Appendix: Review Notes (2026-07-29)

> Generated through self-reflection → critical/counter-critique → self-iteration → gap-filling analysis.

### Key Findings Integrated Above

| Finding | Resolution |
|:--------|:-----------|
| v0.7.0 perf target ("50×") lacked baseline | Added #1 Benchmark baseline + #2 Harness as 🔴 blockers |
| Serializer missing `max_depth` guard | Added v0.7.0 #5 explicitly |
| Alias expansion broke round-trip fidelity | Added v0.7.0 #7 |
| v0.8.0 Node API had no query grammar | Added v0.8.0 #3 (JSONPath-like subset) |
| Stale `Node` → segfault risk | Added v0.8.0 #4 (`is_valid()`) + #8 (lifecycle warnings) |
| Tag registry decorator syntax undefined | Added spec note to v0.9.0 #1 + chaining (#2) |
| Pydantic integration lacked type signature | Added signature to v0.9.0 #3 |
| Duplicate keys handling ambiguous | Added YAML 1.2 spec reference to v0.9.0 #4 |

### Timeline Feasibility Note

Releases v0.1.0–v0.6.0 were compressed into a few days (July 2025 – July 2026), indicating rapid iteration but shallow review buffers. The v0.8.0 critical path (Instance API → Node API → Query Language) is the highest-risk segment; if not started by mid-Q3 2026, consider splitting into v0.8.0 (API surface) and v0.8.1 (query language + lifecycle safety).

---

## Appendix: Review Notes (2026-08-02)

> Generated through the four-phase loop: 洞察反馈 (insight feedback) → 溯源析理 (root-cause tracing) → 内化重构 (internalize & refactor) → 螺旋闭环 (spiral closure). Source signals: v0.10.0 release (published 2026-08-01), post-release verification, and the v0.11.x brainstorming session.

### Phase 1 — 洞察反馈: Signals

| # | Insight | Observed at |
|:--|:--------|:------------|
| 1 | publish.yml is validated only at tag push — the `--generate-stubs` failure surfaced at Release, never on PRs/pushes | v0.10.0 publish run failed on linux/musllinux |
| 2 | `docs/en/changelog.md` (canonical English mirror) missed in both v0.9.0 and v0.10.0 releases; deployed docs changelog was stale at v0.6.0 | Post-release site verification |
| 3 | CodSpeed flagged `serialize_medium` −10.92% — actually a runner CPU change (EPYC 7763 → EPYC 9V74 → Xeon 8573C), not code | v0.10.0 vs v0.9.0 comparison |
| 4 | Research & Exploration items have no revisit mechanism — `with` scoping, compliance reporting, competitor tracking, plugins stayed unchecked across releases | ROADMAP review |
| 5 | `flush_source` re-serializes the whole document after every edit (O(doc)) — the v0.10.0 edit API has no byte-level AST↔source mapping | Code read (src/py/mod.rs:467) |
| 6 | `parse_stream` is pseudo-streaming — whole input string + whole `Vec<StreamEvent>` in memory (src/py/mod.rs:1104) | Code read |

### Phase 2 — 溯源析理: Root Causes

| # | Root cause | Layer |
|:--|:-----------|:------|
| 1 | Validation timing lag: publish recipe never runs outside the tag-triggered workflow, so recipe changes are unverifiable until Release | Process |
| 2 | Release checklist was an implicit convention, not an enforced gate; dual-maintained changelog mirrors (root + docs/en) lacked a sync check; no post-release site verification step | Process |
| 3 | Simulation-mode CodSpeed is sensitive to physical CPU modeling; runner pool is non-deterministic; no hardware-fingerprint gate before declaring regressions | Tooling |
| 4 | Planning ≠ commitment: Research items carry no owner, no review cadence, no closure rule | Planning |
| 5 | v0.10.0 preserved fidelity at the comment/anchor layer but never recorded the AST↔source-text byte mapping — the architectural debt the edit API left behind | Architecture |
| 6 | parse_stream was designed as an event abstraction over `str`, not an I/O abstraction — the input model (whole-string) caps memory behavior | Architecture |

### Phase 3 — 内化重构: Resolutions Landed

| # | Resolution | Artifact |
|:--|:-----------|:---------|
| 1 | v0.11.0 #1: source span tracking on `CustomNode` — the missing AST↔source byte mapping (⭐ structural change, approved) | ROADMAP v0.11.0 |
| 2 | v0.11.0 #2–#4: dirty-region splice + fidelity property tests + O(region) edit-flush benchmarks (target ≥100×) | ROADMAP v0.11.0 |
| 3 | v0.11.1 #1–#4: lazy event iterator over chunked reader + early termination + memory-bound + event-parity tests | ROADMAP v0.11.1 |
| 4 | Rejected approaches recorded as future options (per-scalar splice, callback-push feed, mmap) so the decision space isn't lost | ROADMAP v0.11.0/v0.11.1 design decisions |
| 5 | Release checklist now mandates `docs/{en,ja,ko,zh}/changelog.md` + post-release docs-site verification (v0.9.0–v0.10.0 gap) | AGENTS.md Release Process |
| 6 | CodSpeed guidance: hardware-difference attribution before treating Simulation flags as regressions | AGENTS.md (post-release verification) |
| 7 | Research & Exploration gains a revisit rule — every milestone review promotes/defer/close; no item unchecked for >2 consecutive reviews | ROADMAP Research & Exploration |

### Phase 4 — 螺旋闭环: Closure Verification

- **Every insight has a landed resolution**: #1–#6 in Phase 1 map 1:1 to Phase 3 #1–#6; #4 additionally closed the planning gap via the revisit rule.
- **Design consistency**: span tracking (v0.11.0 #1) is the sole AST structural change and was explicitly approved 2026-08-02; v0.11.0 → v0.11.1 order confirmed (editing continuity before I/O expansion).
- **No silent carries**: deferred items (`with` scoping, compliance reporting, competitor tracking, plugins, numpy-free wheel) are explicitly listed in Planned's "Deferred" block AND tracked in Research with the revisit rule.
- **Loop state**: closed. Next spiral turn = feature-level spec + writing-plans when v0.11.0 implementation starts; each feature gets its own design doc under `docs/superpowers/specs/`.
- **Process debts still open (tracked, not blocking)**: publish.yml pre-release validation (workflow_dispatch dry-run) and CodSpeed hardware-fingerprint gating are candidate process improvements — intentionally not committed to any version; revisit at next milestone review.
