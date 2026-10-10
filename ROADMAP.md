# Roadmap

> See [CHANGELOG.md](CHANGELOG.md) for detailed per-version change logs (Keep a Changelog format).
> Roadmap tracks planned capabilities; CHANGELOG tracks shipped changes.

All versions follow [Semantic Versioning](https://semver.org/) (major.minor.patch). Pre-1.0: MINOR
adds features, PATCH fixes bugs.

**API stability**: v0.x releases do not guarantee backward compatibility. Stable semver guarantee
starts at v1.0.

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

**Data ownership**: `Node` is a borrowed reference into `YamlDocument`'s AST. `Node` is invalid when
`YamlDocument` is garbage-collected. `Node` must not outlive its parent document.

---

## Released

| Version | Date | Changelog |
|---------|------|-----------|
| v0.17.0 | 2026-10-01 | [CHANGELOG.md §[0.17.0]](CHANGELOG.md#0170---2026-10-01) |
| v0.16.0 | 2026-10-01 | [CHANGELOG.md §[0.16.0]](CHANGELOG.md#0160---2026-10-01) |
| v0.15.0 | 2026-08-19 | [CHANGELOG.md §[0.15.0]](CHANGELOG.md#0150---2026-08-19) |
| v0.14.1 | 2026-08-15 | [CHANGELOG.md §[0.14.1]](CHANGELOG.md#0141---2026-08-15) |
| v0.14.0 | 2026-08-14 | [CHANGELOG.md §[0.14.0]](CHANGELOG.md#0140---2026-08-14) |
| v0.13.0 | 2026-08-10 | [CHANGELOG.md §[0.13.0]](CHANGELOG.md#0130---2026-08-10) |
| v0.12.1 | 2026-08-06 | [CHANGELOG.md §[0.12.1]](CHANGELOG.md#0121---2026-08-06) |
| v0.11.7 | 2026-08-04 | [CHANGELOG.md §[0.11.7]](CHANGELOG.md#0117---2026-08-04) |
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

## Planned

Only work that has not been released belongs here. A milestone scope is written at a milestone
review; when it ships, its table moves to the engineering record and its entries go to
`CHANGELOG.md`, which is why the v0.11.3 - v0.12.0 scoping that used to sit in this section now
ends the ledger instead, and only the release index above remembers the versions.

### Open: one ruling the measurement found and no change has made yet

| # | Item | Layer | Priority |
|:--|:-----|:------|:--------:|
| 1 | **Which artefact does the shipped `.pyi` describe?** - found while verifying published signatures against the running package. The stub is generated from the extension, so whatever `python/pyrs_yaml/__init__.py` adds afterwards is invisible to it: `Node` (41 members) and `MergedView` (6) are not declared at all, and 13 editing/query members of `YamlDocument` (`set`, `set_many`, `insert`, `append`, `delete`, `rename`, `sort_keys`, `walk`, `scalars`, `find`, `node`, `merged`) are missing. An editor sees one class and the runtime offers another, which no amount of doc checking can repair. The options are a Python-typed public layer the generator can reach, moving those members behind the extension boundary, or a declared reconciliation in `check_stub_drift.py` - and each answers what "the stub" means, so it is a design decision rather than a patch | Rust (`pyrs-yaml` binding) + Python package | 🔴 |

Both were found by measuring rather than by review, and both are written up with their evidence in
the engineering record (ledger (bo)).

The second has since shipped: an inline dict schema carried only the sections the emitter knew
about, so a `validate` section vanished silently and a pattern containing both quote characters
serialised into text the parser rejected. The dict now goes through `from_dict`, the library's own
round-trip writer, which is also the general lesson - inside a round-trip library, converting data
to text is the writer's job, never a hand-built one (ledger (bp)).

The rulings that used to sit here are shipped. Whether `mapping_of` / `sequence_of` should assert
their container: they do, and the language gained the words to say so (`type: map` / `type: seq`,
and `map` / `seq` as member types). Scope decides the strength - a rule naming a path asserts that
node, a pathless rule selects the nodes it can describe, and members are asserted either way. Found
in the same function as the char-boundary crash that closed the `unwrap` audit (ledger (bj)); the
seven passing shapes that made the ruling, the greedy `[*]` matcher it exposed, the
two-checks-in-one-rule silent drop and the alias boundary it deliberately leaves are in the record
(ledger (bm)).

What the strict JSON writers should do with a non-finite float is shipped too: they refuse it, with
the reason key `json-cannot-represent-non-finite`, because every substitute silently changes the
value (ledger (bk)). `to_json5` remains the dialect that carries the value. What is *not* offered is
a lenient `null`, because nothing has asked for a documented way to lose the value on purpose - if
that demand appears, it arrives as an option with its own name rather than as a default.

An item arrives in this section when measurement finds the engine has made a choice nobody decided,
and leaves the moment a change ships the decision. That is why it stays short rather than growing
into a second ledger: a ruling without an open item is a ruling someone has to re-derive.

### How the next scope is written

The revisit rule in the Research & Exploration section below governs: every milestone review
re-evaluates each open item and promotes it to a `### vX.Y.Z` scope here, defers it with a reason,
or closes it. An item may not stay in this section once it ships.

---

## Review Notes 2026-08-15 (v0.14.1 milestone)

| Item | Decision | Rationale |
|:-----|:---------|:----------|
| Numpy free-threaded re-enable | **Promote → [Unreleased]** | Upstream blocker resolved: rust-numpy v0.24.0+ supports free-threaded Python; v0.29.0 (already pinned) inherits support. Remove `--no-default-features` from cp314t builds; runtime probe auto-detects. |
| Custom YAML 1.2 parser | **Defer** | granit-parser stable at 99.75% compliance (405/406); 7 workarounds in place but no urgent regression. Revisit next milestone. |

---

## Review Notes 2026-08-11 (v0.13.0 milestone)

Milestone review of all Research & Exploration items per the revisit rule.

| Item | Decision | Rationale |
|:-----|:---------|:----------|
| Custom YAML 1.2 parser | **Defer** | saphyr→granit-parser migration shipped in v0.13.0; let it stabilize before evaluating replacement. Revisit v0.14.0. |
| Numpy free-threaded re-enable | **Defer** | Blocked on upstream `rust-numpy` (PyO3/rust-numpy#476). Revisit when upstream lands. |
| YAML Schema language | **Promote → v0.14.0** | Independent design deliverable; scope as a design spec for v0.14.0. |
| `yaml-edit` competitor analysis | **Close** | Passive tracking provides no value; if yaml-edit ships a significant feature, the community will surface it. |
| Community plugins | **Promote → v0.14.0** | Related to YAML Schema language; third-party registry extension. Scope as design + prototype for v0.14.0. |

---

**Deferred (not committed, revisit at each milestone review)**: Custom YAML 1.2 parser (evaluation
deferred — granit migration just shipped, let it settle); numpy free-threaded re-enable (blocked on
upstream rust-numpy). Tracked in Research & Exploration below with a revisit rule (see Review Notes
2026-08-11).

---

## Where the engineering record lives

This file is a plan. What was found, measured, refuted and gated lives in three development
documents, none of which is published site content (they sit outside `docs/en`, so `check_i18n.py`
asks no translations of them):

| document | holds |
|:---------|:------|
| [`docs/dev/quality-ledger.md`](docs/dev/quality-ledger.md) | the entry-by-entry record `(h)`-`(bg)`, the note-survival invariant, and the milestone scopes after they shipped |
| [`docs/dev/boundaries.md`](docs/dev/boundaries.md) | deliberate engine boundaries, and the fuzz findings by family |
| [`docs/dev/perf.md`](docs/dev/perf.md) | the cross-library ranking snapshot and the Ir gate history |

`CHANGELOG.md` cites them per entry as `(details: quality-ledger (xx))`, and its released versions
are folded so the page reads as a list of releases rather than a wall of prose.

## Research & Exploration

Tracked as open questions for future roadmap inclusion; not committed to any version.

> **Revisit rule** (from Review Notes 2026-08-02): every milestone review must re-evaluate all
> unchecked items below — promote, defer with reason, or close. No item stays unchecked for more
> than two consecutive milestone reviews.

- [x] **Free-threaded CPython support** — `Py_GIL_DISABLED` + full `gil_used = false` build matrix
  ✅ Delivered in v0.10.0 (cp314t wheels on PyPI)
- [ ] **Custom YAML 1.2 parser** — evaluate replacing granit-parser with a 100% YAML 1.2 compliant
  Rust parser. YAML 1.2 spec is ~80 pages with formal grammar; reference implementation libyaml (C)
  ~15K lines. Estimated effort: 3-6 months for production quality. Alternative: fork granit-parser
  and incrementally fix to 100%. ⏸️ Deferred 2026-08-11 — granit migration just shipped in v0.13.0;
  let it settle. Revisit v0.14.0.
    - **2026-09-26 in-tree experiment (rejected → keep 1.1.0)**: bumped granit-parser 1.1.0 → 1.3.0
      (API-compatible, `cargo check` clean, nextest 205/205, pytest 1226 green) — but
      yaml-test-suite compliance **regressed 405/406 → 404/406**: 1.3.0 wrongly *accepts* `9C9N`
      (wrongly-indented flow sequence), violating the strictness posture pinned by the v0.11.5
      audit. Any future parser swap/fork/uplift MUST re-run `tests/test_yaml_suite.py` + the `9C9N`
      probe as a hard gate.
    - **2026-09-26 follow-up (fork-uplift path closed too)**: benchmarked granit 1.3.0 on the
      codspeed Rust bench channel — `parse_large` median regressed 26.8 → 29.7 µs with pathological
      mean tails (up to 391 µs); only `parse_medium` slightly improved. So 1.3 offers no perf upside
      to justify carrying the 9C9N fix in a fork. The residual eager-parse gap vs `yaml-rs` (0.42×)
      is now *measured* to be the structural cost of strict compliance + comment/span events
      (yaml_rs itself wrongly accepts `9C9N` and wrongly rejects `2JQS`). Remaining levers: an
      in-house scanner (3-6 months) or an upstream issue for the 9C9N regression (probe repro
      available).
    - **2026-10-01 resolution (1.3.0 shipped + in-tree guard)**: granit-parser was nonetheless
      bumped to 1.3.0 (PR #169) and the predicted `9C9N` regression materialized — the suite's ≥95%
      threshold gate drowned `405/406 → 404/406` and it passed green CI. Fixed in-tree: the AST
      receiver now rejects an under-indented multi-line flow continuation (`parser::mod` flow-indent
      guard), restoring `405/406`. `9C9N` is pinned as a **per-case hard gate** on literal input
      (`tests/test_yaml_suite.py::test_9c9n_wrongly_indented_flow_is_rejected`, no `skipif`) plus a
      Rust unit test, so no future parser bump can hide this class behind the threshold. The
      parser-replacement question stays open (in-house scanner / upstream `9C9N` issue remain the
      only levers).
- [x] **Numpy free-threaded re-enable** — when `rust-numpy` (PyO3/rust-numpy#476) lands solid
  free-threaded support, remove `--no-default-features` from cp314t build lines and let the runtime
  probe (`py.import("numpy").is_ok()`) auto-detect. Its v0.11.7 scope is now in
  [`docs/dev/quality-ledger.md`](docs/dev/quality-ledger.md). ✅ Closed 2026-08-15 —
  rust-numpy v0.24.0+ supports free-threaded Python; v0.29.0 (already pinned) inherits support.
  `--no-default-features` removed from cp314t build lines in `publish.yml`/`ci.yml`.
- [x] **YAML Schema language** — dedicated schema definition format beyond JSON Schema ✅ Promoted
  to v0.14.0 planning 2026-08-11
- [x] **`yaml-edit` competitor analysis** — track their feature expansion; respond with
  differentiator strategy ✅ Closed 2026-08-11 — passive tracking provides no value
- [x] **Community plugins** — allow third-party Python modules to register custom node types ✅
  Promoted to v0.14.0 planning 2026-08-11
- [x] **`--no-default-features` build** — exclude `numpy` from wheel for free-threaded Python ✅
  Committed in v0.11.6

**Committed (moved from this list; each scope's table now ends
[`docs/dev/quality-ledger.md`](docs/dev/quality-ledger.md), and the release entries are in
`CHANGELOG.md`)**: v0.11.0 (surgical serialization), v0.11.2
(streaming parse, with v0.11.1); v0.11.3 (streaming write, with scoping, compliance reporting,
line-offsets cache, publish pre-validation); v0.11.5 (parser robustness — audit closed empty,
docs-only release, with strictness regression corpus `tests/test_strictness_audit.py`); v0.11.6
(numpy-free free-threaded wheel); v0.11.7 (CI signal hygiene + numpy tracking); YAML Schema language
(promoted 2026-08-11, v0.14.0 planning); Community plugins (promoted 2026-08-11, v0.14.0 planning).
