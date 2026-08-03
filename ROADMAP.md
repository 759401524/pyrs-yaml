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
| v0.11.3 | 2026-08-03 | [CHANGELOG.md §\[0.11.3\]](CHANGELOG.md#0113---streaming-write--process-hardening-target-q3-2026) |
| v0.11.2 | 2026-08-03 | [CHANGELOG.md §\[0.11.2\]](CHANGELOG.md#0112---2026-08-03) |
| v0.11.0 | 2026-08-02 | [CHANGELOG.md §\[0.11.0\]](CHANGELOG.md#0110---2026-08-02) |
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

## v0.11.3 — "Streaming Write + Process Hardening" (target: Q3 2026)

> Complete the big-file story v0.11.2 opened (read is constant-memory, write still isn't) and close the two process debts flagged in the 2026-08-02 closure that caused v0.10.0-class release failures.

| # | Item | Layer | Priority | Notes |
|:--|:-----|:------|:--------:|:------|
| 1 | **Streaming write** — `YAML.dump_stream(file_obj, iterable)` / `dump_file(path, ...)`: serializer emits events chunk-by-chunk to a Python file object; constant memory on 100MB+ output | Rust + Python | 🔴 | ✅ Commits `061ebfd`/`11bdb80`/`7e6e821` |
| 2 | **Line-offsets cache** — carry `compute_line_offsets(source)` (src/parser/yaml/comment.rs:14) through the 5 edit primitives so an edit burst costs O(N+edit) not O(N×edit) | Rust | 🟡 | ✅ Commit `ef53ddc` |
| 3 | **publish.yml pre-release validation** — CI job on PRs touching the publish workflow (or `workflow_dispatch` dry-run) running `maturin build --release --generate-stubs` in a linux container, catching the v0.10.0-class stub failure before Release | CI | 🔴 | ✅ Commit `cb3c6fc` |
| 4 | **Changelog mirror sync check** — prek hook or CI job asserting root `CHANGELOG.md` `[Unreleased]` == `docs/{en,ja,ko,zh}` changelog mirrors | CI/Process | 🟡 | ✅ Commit `cb3c6fc` |
| 5 | **`with` context manager** for document scoping | Python | 🟡 | ✅ Commit `2bfc483` |
| 6 | **Compliance score reporting** — public `compliance_report()` surfacing the yaml-test-suite pass rate (tests gate at 75%) | Python | 🟡 | ✅ Commit `6599ee7` |

**Design decisions (2026-08-03)**: mmap-backed file streaming (read + edit without loading) stays deferred (abi3 portability blocker). Community plugins / YAML Schema language stay in Research. Line-offsets cache is an architectural optimization, not a fix (CodSpeed same-runner 3-branch showed no real edit regression).

**Changelog mapping**: Entries under `[0.11.3]` in CHANGELOG.md.

---

## v0.12.0 — "Compliance Improvement" (target: Q3 2026)

> Raise the YAML Test Suite pass rate from 75% to 90%+ by fixing 60+ parser-edge cases via **post-processing** (pre-process input + post-process AST) around saphyr-parser.

| # | Item | Layer | Fix approach | Priority | Notes |
|:--|:-----|:------|:------------|:--------:|:------|
| 1 | **Escape sequence expansion** — support unknown escape chars, trailing content after double-quoted scalars | Rust (post-processing) | Pre-process input | 🟡 | Stage 1; ~2d |
| 2 | **Comment and whitespace handling** — comment intercepting multiline text, comment separation from tokens | Rust (post-processing) | Pre-process input | 🟡 | Stage 1; ~2d |
| 3 | **Indentation edge cases** — invalid indentation, wrongly indented line, block collection indentation | Rust (post-processing) | Pre-process input | 🟡 | Stage 2; ~3d |
| 4 | **Block mapping key detection** — did not find expected key, simple key `:` ambiguity | Rust (post-processing + saphyr) | Pre-process + saphyr patch | 🔴 | Stage 2-3; ~5d; simple key ambiguity may need saphyr changes |
| 5 | **Flow context disambiguation** — mapping values not allowed in flow context, flow sequence `,`/`]` | Rust (post-processing) | Pre-process flow context | 🟡 | Stage 2-3; ~3d |
| 6 | **Document boundary fixes** — document start/end marker, directive handling | Rust (post-processing) | Pre-process input | 🟡 | Stage 1; ~1d |
| 7 | **Duplicate key edge cases** — null/undefined key handling | Rust (post-processing) | Post-process AST | 🟡 | Stage 1; ~1d |

**Design constraint**: saphyr-parser upstream may not be actively maintained; fixes requiring parser changes will need a maintained fork.

**Changelog mapping**: Entries under `[0.12.0]` in CHANGELOG.md.

---

**Deferred (not committed, revisit at each milestone review)**: `yaml-edit` competitor feature tracking; community plugins / YAML Schema language; `--no-default-features` numpy-free free-threaded wheel. Tracked in Research & Exploration below with a revisit rule (see Review Notes 2026-08-02).

---

## Research & Exploration

Tracked as open questions for future roadmap inclusion; not committed to any version.

> **Revisit rule** (from Review Notes 2026-08-02): every milestone review must re-evaluate all unchecked items below — promote, defer with reason, or close. No item stays unchecked for more than two consecutive milestone reviews.

- [x] **Free-threaded CPython support** — `Py_GIL_DISABLED` + full `gil_used = false` build matrix ✅ Delivered in v0.10.0 (cp314t wheels on PyPI)
- [ ] **Custom YAML 1.2 parser** — evaluate replacing saphyr-parser with a 100% YAML 1.2 compliant Rust parser. YAML 1.2 spec is ~80 pages with formal grammar; reference implementation libyaml (C) ~15K lines. Estimated effort: 3-6 months for production quality. Alternative: fork saphyr-parser and incrementally fix to 100%.
- [ ] **YAML Schema language** — dedicated schema definition format beyond JSON Schema
- [ ] **`yaml-edit` competitor analysis** — track their feature expansion; respond with differentiator strategy
- [ ] **Community plugins** — allow third-party Python modules to register custom node types
- [ ] **`--no-default-features` build** — exclude `numpy` from wheel for free-threaded Python (see CHANGELOG v0.6.0)

**Committed (moved from this list, see Planned)**: v0.11.0 (surgical serialization), v0.11.2 (streaming parse, with v0.11.1); v0.11.3 (streaming write, with scoping, compliance reporting, line-offsets cache, publish pre-validation); v0.12.0 (compliance improvement).
