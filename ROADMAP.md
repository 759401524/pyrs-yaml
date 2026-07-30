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
| v0.6.0 | 2026-07-29 | [CHANGELOG.md §\[0.6.0\]](CHANGELOG.md#060--2026-07-29) |
| v0.5.0 | 2026-07-27 | [CHANGELOG.md §\[0.5.0\]](CHANGELOG.md#050--2026-07-27) |
| v0.4.0 | 2026-07-27 | [CHANGELOG.md §\[0.4.0\]](CHANGELOG.md#040--2026-07-27) |
| v0.3.0 | 2026-07-27 | [CHANGELOG.md §\[0.3.0\]](CHANGELOG.md#030--2026-07-27) |
| v0.2.0 | 2026-07-26 | [CHANGELOG.md §\[0.2.0\]](CHANGELOG.md#020--2026-07-26) |
| v0.1.0 | 2026-07-25 | [CHANGELOG.md §\[0.1.0\]](CHANGELOG.md#010--2026-07-25) |

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

### v0.10.0 — Reserve (pending scope)

- Overflow from v0.9.0 if scope expands
- Potential areas:
  - `yaml-edit`-style in-place editing
  - YAML 1.2 spec compliance score reporting
  - Python `with` context manager for document scoping

---

## Research & Exploration

Tracked as open questions for future roadmap inclusion; not committed to any version.

- [ ] **Free-threaded CPython support** — `Py_GIL_DISABLED` + full `gil_used = false` build matrix (started in v0.6.0, CI job exists)
- [ ] **Streaming large documents** — event-based/iterator API for files > 100 MB
- [ ] **YAML Schema language** — dedicated schema definition format beyond JSON Schema
- [ ] **Incremental serialization** — serialize only modified subtrees in `YamlDocument`
- [ ] **`yaml-edit` competitor analysis** — track their feature expansion; respond with differentiator strategy
- [ ] **Community plugins** — allow third-party Python modules to register custom node types
- [ ] **`--no-default-features` build** — exclude `numpy` from wheel for free-threaded Python (see CHANGELOG v0.6.0)

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
