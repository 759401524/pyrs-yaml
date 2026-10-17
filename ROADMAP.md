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

Only work that has not been released belongs here. A milestone scope is written at a milestone
review; when it ships, its table moves to the engineering record and its entries go to
`CHANGELOG.md`, which is why the v0.11.3 - v0.12.0 scoping that used to sit in this section now
ends the ledger instead, and only the release index above remembers the versions.

### Closed: two rulings, both of them found by measuring

The one item that opened this section closed with a ruling rather than a fold: the writer keeps its
silence about a mapping whose keys its own reader resolves as one entry. Both options that would
have made it loud cost a pass over every key of every mapping, and that price was measured, not
estimated - +31…33% on the five `serialize_*` scenarios on the branch that tried it
(`docs/dev/perf.md`) - against a floor that admits 0.50%. The guarantee is where the cost is not:
the edit layer, which touches each key once, resolves a key by the identity the reader uses, so `~`,
`null` and `NULL` cannot both reach the text (`tests/test_edit.py::TestKeyIdentityAcrossSpellings`),
and what a hand-built `NodeMap` still produces is pinned as the shape it is
(`serializer::the_write_path_keeps_both_lines_of_a_pair_its_reader_resolves_as_one`). The generator
has refused to build the shape since the entry that came out of the property tier recorded it
(ledger (br)), and the ruling itself is (di).

The stub item that used to sit in this section shipped in this
cycle, and measuring it moved the question it asked: `python/pyrs_yaml/pyrs_yaml.pyi` was not the
first thing missing, because `parse` was re-exported through a wrapper annotated `-> Any` and no
checker reached `YamlDocument` at all; the class gap was twelve attached members, not thirteen, and
`Node` / `MergedView` were never in it, being Python modules a `py.typed` package already types. The
answer is the declared reconciliation - the route reads the members off the compiled module's
classes and writes them into the artifact - where (bo) recorded how the gap surfaced and (cy)
records what it took to close it. The alias-key item that used to sit here — a mapping key written
as an alias was emitted as `null`, dropping the entry the document had made — shipped in this cycle,
and the row that described it here was wrong on a point of fact: the compact `*x: v` is not readable
by this engine at all (`simple key expected ':'`), so the writer carries the `?` marker in both
container styles rather than choosing between two readable spellings. ruamel.yaml was the part of
that sentence worth keeping care about: its scanner raises from 0.19, while 0.18.16 — what `uv.lock`
resolves for the Python 3.8 legs — reads the compact form as an empty-tuple key, and the blanket
claim reddened those legs before both versions were measured. Its record is the changelog entry and
`pbt::tests::prop_alias_key_survives_the_round_trip` — the first property in this tier that can
build the shape, because the AST strategy never puts an alias in key position and `try_roundtrip`
filters a mismatching tree away instead of failing it, which is how the loss went unreported while
every tier above it was green.

The second of the two that used to sit in this table has since shipped: an inline dict schema
carried only the sections the emitter knew about, so a `validate` section vanished silently and a
pattern containing both quote characters serialised into text the parser rejected. The dict now goes
through `from_dict`, the library's own round-trip writer, which is also the general lesson - inside
a round-trip library, converting data to text is the writer's job, never a hand-built one (ledger
(bp)).

The line-end-slot question the removed row asked is answered, and the answer is the one the flow
path already spends: a line break. Measured against the reader first, as that row insisted - a
comment line written directly above a key that is not the mapping's first belongs to *that key*
(`a: 1` + `# K` + `b: 2  # V` re-reads with `K` on `b`), so only a first pair hands its key's note
up to the container. The value keeps the trailing slot it needs and the key's note goes above the
pair when the slot is taken, which is why no pinned comment-on-a-pair emission changed shape: only
the collision case does. Over 4000 generated trees, notes that never reached the emitted text fell
from 907 to 257 and the block-key class of them from 707 to 34 (ledger (db)).

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
- [x] **Custom YAML 1.2 parser** — evaluate replacing granit-parser with a 100% YAML 1.2 compliant
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
    - **Closed 2026-10-16 under the revisit rule** — see the Review Notes row above: the
      compliance premise is satisfied (99.75%, 405/406, the miss by design), and the perf
      premise is explained as the cost of the fidelity this project sells. The `9C9N` and
      flow-indent guards stay as the standing answer for parser swaps of any kind.
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
