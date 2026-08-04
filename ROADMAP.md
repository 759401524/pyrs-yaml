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
|---------|------|-----------|
| v0.11.6 | 2026-08-04 | [CHANGELOG.md §[0.11.6]](CHANGELOG.md#0116---2026-08-04) |
| v0.11.5 | 2026-08-04 | [CHANGELOG.md §\[0.11.5\]](CHANGELOG.md#0115---2026-08-04) |
| v0.11.4 | 2026-08-04 | [CHANGELOG.md §\[0.11.4\]](CHANGELOG.md#0114---2026-08-04) |
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

## v0.11.5 — "Parser Robustness" (target: Q3 2026)

> Reframed from the original v0.12.0 "Compliance Improvement" items 3/4/5. The YAML Test Suite pass rate is saturated at **99.75%** (405/406 — only `ZYU8` fails, rejected by design), so these items no longer move the compliance metric. They harden rejection of invalid YAML edge cases beyond the suite, each bound to a strictness-audit probe corpus.

| # | Item | Layer | Fix approach | Priority | Status |
|:--|:-----|:------|:------------|:--------:|:------|
| 3 | **Indentation edge cases** — invalid indentation, wrongly indented line, block collection indentation | Rust (post-processing) | Pre-process input | 🟡 | ✅ Closed 2026-08-04 — audit found no fixable case |
| 4 | **Block mapping key detection** — did not find expected key, simple key `:` ambiguity | Rust (post-processing + saphyr) | Pre-process + saphyr patch | 🔴 | ✅ Closed 2026-08-04 — audit found no fixable case |
| 5 | **Flow context disambiguation** — mapping values not allowed in flow context, flow sequence `,`/`]` | Rust (post-processing) | Pre-process flow context | 🟡 | ✅ Closed 2026-08-04 — audit found no fixable case |

**Phase 0 strictness audit (decision gate) — result: EMPTY fix list → items close (2026-08-04)**: these items have no in-suite target (all suite tests already pass). A 70-probe corpus (~20/bucket: indentation, block-mapping keys, flow context) was compared against a PyYAML oracle via `tests/test_strictness_audit.py`. The parser matched the oracle on **64/70** probes (26 reject-match, 38 accept-match). The 6 divergences are all **deliberate** and documented in the test:

- **5 accepted-by-us but rejected-by-PyYAML** — each is a YAML 1.2 spec or yaml-test-suite requirement where PyYAML is the outlier, not a laxness bug: empty mapping keys (`2JQS`, `CFD4`, `FRK4`, `UKK6` — suite requires accepting `: a`, `[ : empty key ]`), local tags (`C4HZ` — PyYAML fails only at constructor stage, not parse), implicit document after `...` (YAML 1.2 `l-yaml-stream` grammar).
- **1 rejected-by-us but accepted-by-PyYAML** (`{a: 1, a: 2}`) — deliberate duplicate-key strictness; no suite test requires accepting duplicate non-empty keys.

Per the plan's risk note ("the audit records oracle disagreements but does not change our parser to match PyYAML quirks"), none of these were changed. Fixing the 5 would **regress** suite compliance below 405/406; fixing the 1 is already deliberate strictness. **No fixes shipped** — items 3/4/5 close with the audit corpus pinned as a regression test. Do not invent fixes to justify the original ~11d estimate.

**Design constraint**: saphyr-parser upstream may not be actively maintained; item 4 may require a maintained fork. Unchanged — item 4 needed no fork because the audit surfaced no fixable case.

**Changelog mapping**: Entries under `[0.11.5]` in CHANGELOG.md.

---

## v0.11.6 — "numpy-free free-threaded wheel" (target: Q3 2026)

> Ship `cp314t` (free-threaded) wheels built with `--no-default-features` so rust-numpy is excluded entirely. Current free-threaded wheels compile the numpy feature (default) but runtime-probe it (`src/py/python_types.rs:61`) since free-threaded environments typically lack numpy; the change strips the dead linkage (smaller binary, no numpy capsule code, no probe needed). GIL wheels keep numpy enabled.

| # | Item | Layer | Status |
|:--|:-----|:------|:------|
| 1 | **`--no-default-features` wheel** — add the flag to the free-threaded wheel build steps in `publish.yml` (windows + macos `-i python3.14t`) | CI | ✅ Commit `9ad41f3` |
| 2 | **Free-threaded CI validation** — `test-freethreaded` job builds with `--no-default-features` | CI | ✅ Commit `9ad41f3` |
| 3 | **Install docs note** — `docs/{en,zh,ja,ko}`: free-threaded wheels are numpy-free (ndarray serialization unavailable on cp314t) | Docs | ✅ Commit `9ad41f3` |

**Changelog mapping**: Entries under `[0.11.6]` in CHANGELOG.md.

---

## v0.12.0 — (open slot, target: TBD)

> Scope was previously "Compliance Improvement"; that work shipped in v0.11.4 (Stage 1 → 99.75%) and the numpy-free wheel moved to v0.11.6. **No scope committed** — determined at the next milestone review per the Deferred revisit rule.

---

**Deferred (not committed, revisit at each milestone review)**: `yaml-edit` competitor feature tracking; community plugins / YAML Schema language. Tracked in Research & Exploration below with a revisit rule (see Review Notes 2026-08-02).

---

## Research & Exploration

Tracked as open questions for future roadmap inclusion; not committed to any version.

> **Revisit rule** (from Review Notes 2026-08-02): every milestone review must re-evaluate all unchecked items below — promote, defer with reason, or close. No item stays unchecked for more than two consecutive milestone reviews.

- [x] **Free-threaded CPython support** — `Py_GIL_DISABLED` + full `gil_used = false` build matrix ✅ Delivered in v0.10.0 (cp314t wheels on PyPI)
- [ ] **Custom YAML 1.2 parser** — evaluate replacing saphyr-parser with a 100% YAML 1.2 compliant Rust parser. YAML 1.2 spec is ~80 pages with formal grammar; reference implementation libyaml (C) ~15K lines. Estimated effort: 3-6 months for production quality. Alternative: fork saphyr-parser and incrementally fix to 100%.
- [ ] **YAML Schema language** — dedicated schema definition format beyond JSON Schema
- [ ] **`yaml-edit` competitor analysis** — track their feature expansion; respond with differentiator strategy
- [ ] **Community plugins** — allow third-party Python modules to register custom node types
- [x] **`--no-default-features` build** — exclude `numpy` from wheel for free-threaded Python ✅ Committed in v0.11.6

**Committed (moved from this list, see Planned)**: v0.11.0 (surgical serialization), v0.11.2 (streaming parse, with v0.11.1); v0.11.3 (streaming write, with scoping, compliance reporting, line-offsets cache, publish pre-validation); v0.11.5 (parser robustness — audit closed empty, docs-only release, with strictness regression corpus `tests/test_strictness_audit.py`); v0.11.6 (numpy-free free-threaded wheel).
