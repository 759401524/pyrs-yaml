# Roadmap

> See [CHANGELOG.md](CHANGELOG.md) for detailed per-version change logs (Keep a Changelog format).
> Roadmap tracks planned/planned capabilities; CHANGELOG tracks shipped changes.

All versions follow [Semantic Versioning](https://semver.org/) (major.minor.patch). Pre-1.0: MINOR adds features, PATCH fixes bugs.

---

## Architecture: Rust core, Python expression

```text
Python layer (flexible, ecosystem-friendly)          Rust layer (fast, safe, deterministic)
┌────────────────────────────┐                       ┌──────────────────────────────┐
│  YAML() instance API       │                       │  YAMLConfig (Rust struct)    │
│  YAML(typ, schema, depth)  │──── PyO3 boundary ───▶│  parse / serialize engines   │
│  Node high-level API       │                       │  CustomNode AST structures   │
│  Node.find / .filter / .walk│                      │  max_depth guard             │
│  .set_value / .to_yaml()   │                       │  MergedView (read-only)      │
│  Tag registry (@decorator) │                       │  Tag handler registry        │
│  Pydantic integration      │                       │  YAML_SCHEMA constants       │
│  Error formatting (Python) │                       │  Serializer hot-path         │
│  Benchmark orchestration   │                       │  YAMLTestSuite compliance    │
└────────────────────────────┘                       └──────────────────────────────┘
```

**Data ownership**: `Node` is a borrowed reference into `YamlDocument`'s AST. `Node` is invalid when `YamlDocument` is garbage-collected. `Node` must not outlive its parent document.

| Layer | Responsibility | Strength |
|:------|:---------------|:---------|
| **Rust core** | Parse, serialize, AST data, safety guards | Zero-copy, memory-safe, deterministic GC |
| **Python layer** | API design, ecosystem integration, user interaction | Dynamic typing, decorators, introspection, Python toolchain |

**API stability**: v0.x releases do not guarantee backward compatibility. Stable semver guarantee starts at v1.0.

---

## Released

### v0.6.0 — 2026-07-29 — see [CHANGELOG.md §\[0.6.0\]](CHANGELOG.md#060--2026-07-27)

### v0.5.0 — 2026-07-27 — see [CHANGELOG.md §\[0.5.0\]](CHANGELOG.md#050--2026-07-27)

### v0.4.0 — 2026-07-27 — see [CHANGELOG.md §\[0.4.0\]](CHANGELOG.md#040--2026-07-27)

### v0.3.0 — 2025-07-27 — see [CHANGELOG.md §\[0.3.0\]](CHANGELOG.md#030--2025-07-27)

### v0.2.0 — 2025-07-26 — see [CHANGELOG.md §\[0.2.0\]](CHANGELOG.md#020--2025-07-26)

### v0.1.0 — 2025-07-25 — see [CHANGELOG.md §\[0.1.0\]](CHANGELOG.md#010--2025-07-25)

---

## Planned

### v0.7.0 — "Be the fastest" (target: Q2/Q3 2026)

> Speed is pyyaml-rs's primary differentiator. Every release must prove it.

| # | Item | Layer | Priority |
|:--|:-----|:------|:--------:|
| 1 | **Benchmark baseline** — measure current parse/serialize/round-trip speed vs PyYAML, ruamel.yaml, ryaml, yaml-edit | Python | 🔴 |
| 2 | **Serializer hot-path optimization** — profile `Serializer::serialize_node_internal`; target block-style 50× over PyYAML | Rust | 🔴 |
| 3 | **`max_depth` protection** — recursion guard in `AstReceiver` + `Serializer`; emit `YamlMaxDepthError`; default 1000 | Rust | 🟡 |
| 4 | **Error messages with context lines** — Rust provides line/col/message; Python formats source snippets + caret marker | Python | 🟡 |
| 5 | **YAML Test Suite regression guard** — CI runs full test suite on every commit; report compliance % | Both | 🟡 |

**Changelog mapping**: All items become entries under the `[0.7.0] - YYYY-MM-DD` section in CHANGELOG.md with categories `Added` (1–4) and `Changed` (5).

---

### v0.8.0 — "Unlock the AST" (target: Q3/Q4 2026)

> Custom AST is a capability no other Python YAML library has. These features make it programmable.

| # | Item | Layer | Priority |
|:--|:-----|:------|:--------:|
| 1 | **`YAML()` instance API** — `YAML(typ="rt"\|safe\|full, schema="core"\|yaml1.1, depth=1000)` with reusable config | Both | 🔴 |
| 2 | **Python Node API** — `node.find(path)`, `node.filter(pred)`, `node.walk()`, `node.set_value()`, `node.to_yaml()` | Python | 🔴 |
| 3 | **`merged()` read-only view** — `MergedView` resolves anchors/`<<` without mutating the original AST | Rust | 🟡 |
| 4 | **Document metadata** — `doc.version`, `doc.tags`, `doc.directives` from `doc_infos` | Rust | 🟡 |
| 5 | **Backward compatibility** — `parse()`/`safe_load()` become syntactic sugar for `YAML().parse()`/`.safe_load()` | Both | 🟡 |

**Node lifecycle contract**: `Node` borrows from `YamlDocument`. Deleting or garbage-collecting `YamlDocument` invalidates all `Node` references. Attempting to use a stale `Node` raises `YamlDocumentError("parent document has been released")`.

**Changelog mapping**: Entries under `[0.8.0]` in CHANGELOG.md.

---

### v0.9.0 — "Ecosystem ready" (target: Q4 2026)

> Integrate into the Python ecosystem without reinventing wheels.

| # | Item | Layer | Priority |
|:--|:-----|:------|:--------:|
| 1 | **Tag handler registry** — `@pyyaml_rs.register_tag("!include")` and `pyyaml_rs.register_tag("!ts", handler)` | Both | 🔴 |
| 2 | **Pydantic integration** — `yaml.parse_as(MyModel, yaml_str)` → `MyModel.model_validate(yaml.parse(yaml_str))` | Python | 🔴 |
| 3 | **`allow_duplicate_keys`** — config flag, default `False`; when `True`, later key wins (YAML 1.2 spec behavior) | Rust | 🟡 |
| 4 | **`SerializeOptions` expansion** — `width` (line wrapping), `indent(mapping, sequence, offset)` | Both | 🟡 |

**Tag handler error propagation**: Python tag callback raises → Rust catches the Python exception → wraps in `YamlTagError("tag !include handler failed: <original traceback>")` → surfaces to Python caller with full stack trace.

**Changelog mapping**: Entries under `[0.9.0]` in CHANGELOG.md.

---

### v0.10.0 — Reserve (pending scope)

- Overflow from v0.9.0 if scope expands
- Potential areas: `yaml-edit`-style in-place editing, YAML 1.2 spec compliance score reporting, Python `with` context manager for document scoping

---

## Research & Exploration

Tracked as open questions for future roadmap inclusion; not committed to any version.

- [ ] **Free-threaded CPython support** — `Py_GIL_DISABLED` + full `gil_used = false` build matrix (started in `[0.6.0]`, CI job exists)
- [ ] **Streaming large documents** — event-based/iterator API for files > 100 MB
- [ ] **YAML Schema language** — dedicated schema definition format beyond JSON Schema
- [ ] **Incremental serialization** — serialize only modified subtrees in `YamlDocument`
- [ ] **`yaml-edit` competitor analysis** — track their feature expansion; respond with differentiator strategy
- [ ] **Community plugins** — allow third-party Python modules to register custom node types
- [ ] **`--no-default-features` build** — exclude `numpy` from wheel for free-threaded Python (see CHANGELOG `[0.6.0]`)
