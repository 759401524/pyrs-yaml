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
| v0.18.0 | 2026-10-17 | [CHANGELOG.md §[0.18.0]](CHANGELOG.md#v0180--2026-10-17) |
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

No scope is open for v0.19.0. A milestone scope is written at a milestone review; when it ships,
its entries move to `CHANGELOG.md` and its table to the ledger. Only an undecided choice with a
live trigger belongs here.

### Open decision

| Item | State | Trigger to revisit |
|:-----|:------|:-------------------|
| `rust-i18n` as the error-message backend | Deferred 2026-10-16, price on record | A wheel-size or import-time budget is written down. The catalog alone compiles and links 46 crates (113 in the wheel graph, 67 without it); replacing it rewrites 421 `t!(..)` call sites and four locale-parity gates. The measurements are in the review note below. |

### Closed, recorded in the ledger

The rulings that used to sit here shipped: the writer keeps its silence on keys its reader
collapses `(di)`; the stub reconciliation and the alias-key marker are `(bo)`, `(cy)` and `(br)`;
the schema-container words, the non-finite-float refusal and line-slot ownership are `(bm)`,
`(bk)` and `(db)`. Details live in [`docs/dev/quality-ledger.md`](docs/dev/quality-ledger.md);
re-deriving them here would make this file a second ledger.

### How the next scope is written

The revisit rule in the Research & Exploration section below governs: every milestone review
re-evaluates each open item and promotes it to a `### vX.Y.Z` scope here, defers it with a reason,
or closes it. An item may not stay in this section once it ships.

---

## Review Notes 2026-10-16 (the weight of the message catalog)

| Item | Decision | Rationale |
|:-----|:---------|:----------|
| `rust-i18n` as the error-message backend | **Defer, with the price on the record** | Measured on this tree over `cargo metadata`'s resolved graph, following normal edges only and treating a proc-macro crate's subtree as build-time: the wheel reaches **113** crates, and cutting the single `rust-i18n` edge leaves **67** - so **46** crates are compiled and linked for the catalog alone (a second `granit-parser` 1.3.0, `toml`, `serde`/`serde_json`/`serde-saphyr`, `globwalk`/`globset`/`ignore`, `encoding_rs`, `crossbeam-*`), and **58** more sit on the build side. `rust-i18n` 4.2.4 offers two optional features (`load-path`, `log-miss-tr`) and neither removes any of that, because the cone is non-optional. No byte share is claimed for the 3,845,632-byte `.pyd`: symbol attribution needs `cargo-bloat`, which is not a project tool, so this row prices the graph and stops there. |
| Replacing it | **Not scoped** | The surface is 421 `t!(..)` call sites over four catalog files (`en`, `ja-JP`, `ko-KR`, `zh-CN`). The minimum substitute is a compile-time key table with the same `%{key}` placeholders plus the env/`locale` detection already written here - and the locale-parity gates would be rewritten against the new shape, since today they read YAML. That is work of the same order as the note-slot line (dj)-(dq), bought against a benefit that still has no number in bytes or instructions. Revisit when a wheel-size or import-time budget is written down first. |
| The earlier ruling on the second `granit-parser` | **Standing, and not overturned here** | That finding was about the *build* graph; this one walks the *link* graph and finds `granit-parser 1.3.0` reached by a normal edge through `rust-i18n-support`. The two agree only if the crate is dead-stripped before the binary is written, which is exactly the claim a graph walk cannot make. Closing this row needs a symbol-level measurement, not another dependency tree. |
| The open question "replace `granit-parser` with a 100% YAML 1.2 compliant parser" | **Closed, not deferred** | The revisit rule was overdue on it: the item's own note says "Revisit v0.14.0" and four releases have shipped since, and no item may stay unchecked past two reviews. Its premise is gone rather than unproven — the recorded suite rate is 99.75% (405/406) with the single miss `ZYU8` a deliberate strictness rejection, and `docs/dev/quality-ledger.md` already states the rate is saturated, so more work on this axis buys no compliance. The gap that kept the option alive, eager-parse at 0.42x of `yaml-rs`, is measured in the same record to be the structural cost of strict compliance plus comment and span events — the fidelity this project sells, not slack an in-house scanner gets to drop. Reopening needs a measured premise: a prototype that keeps span and comment events and beats granit on the committed Ir scenarios. Until that probe is paid for, a 3-6 month rewrite is a bet, not a roadmap row. |

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

## Where the engineering record lives

This file is a plan. What was found, measured, refuted and gated lives in three development
documents, none of which is published site content (they sit outside `docs/en`, so `check_i18n.py`
asks no translations of them):

| document | holds |
|:---------|:------|
| [`docs/dev/quality-ledger.md`](docs/dev/quality-ledger.md) | the entry-by-entry record from `(h)` onward, the note-survival invariant, and the milestone scopes after they shipped |
| [`docs/dev/boundaries.md`](docs/dev/boundaries.md) | deliberate engine boundaries, and the fuzz findings by family |
| [`docs/dev/perf.md`](docs/dev/perf.md) | the cross-library ranking snapshot and the Ir gate history |

`CHANGELOG.md` cites them per entry as `(details: quality-ledger (xx))`, and its released versions
are folded so the page reads as a list of releases rather than a wall of prose.

## Research & Exploration

Open questions for future roadmaps; none is committed to a version.

> **Revisit rule** (from Review Notes 2026-08-02): every milestone review must re-evaluate all
> open items below — promote, defer with reason, or close. No item stays open for more than two
> consecutive milestone reviews.

No item is open. The closed ones, each with its record:

- **Custom YAML 1.2 parser** — closed 2026-10-16: suite compliance is 99.75% (405/406, the miss a
  deliberate strictness rejection) and the eager-parse gap is the measured cost of strictness plus
  comment and span events. The `9C9N` and flow-indent guards remain the standing answer for any
  parser swap; probes are `tests/test_yaml_suite.py`.
- **Free-threaded CPython, numpy and `--no-default-features`** — shipped: the `gil_used = false`
  matrix, cp314t wheels, and the flag removed once rust-numpy 0.24+ carried free-threaded support.
- **YAML Schema language** — shipped in v0.14.0; `type: map` / `type: seq` arrived in v0.18.0.
- **`yaml-edit` competitor analysis** — closed 2026-08-11: passive tracking has no value.
- **Community plugins** — shipped in v0.14.0.

**Committed and shipped**: v0.11.0 through v0.18.0. Each scope's table ends
[`docs/dev/quality-ledger.md`](docs/dev/quality-ledger.md), and the release entries are in
`CHANGELOG.md`.
