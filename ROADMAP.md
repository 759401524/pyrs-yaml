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
| 4 | **Block mapping key detection** — did not find expected key, simple key `:` ambiguity | Rust (post-processing + granit) | Pre-process + granit patch | 🔴 | ✅ Closed 2026-08-04 — audit found no fixable case |
| 5 | **Flow context disambiguation** — mapping values not allowed in flow context, flow sequence `,`/`]` | Rust (post-processing) | Pre-process flow context | 🟡 | ✅ Closed 2026-08-04 — audit found no fixable case |

**Phase 0 strictness audit (decision gate) — result: EMPTY fix list → items close (2026-08-04)**: these items have no in-suite target (all suite tests already pass). A 70-probe corpus (~20/bucket: indentation, block-mapping keys, flow context) was compared against a PyYAML oracle via `tests/test_strictness_audit.py`. The parser matched the oracle on **64/70** probes (26 reject-match, 38 accept-match). The 6 divergences are all **deliberate** and documented in the test:

- **5 accepted-by-us but rejected-by-PyYAML** — each is a YAML 1.2 spec or yaml-test-suite requirement where PyYAML is the outlier, not a laxness bug: empty mapping keys (`2JQS`, `CFD4`, `FRK4`, `UKK6` — suite requires accepting `: a`, `[ : empty key ]`), local tags (`C4HZ` — PyYAML fails only at constructor stage, not parse), implicit document after `...` (YAML 1.2 `l-yaml-stream` grammar).
- **1 rejected-by-us but accepted-by-PyYAML** (`{a: 1, a: 2}`) — deliberate duplicate-key strictness; no suite test requires accepting duplicate non-empty keys.

Per the plan's risk note ("the audit records oracle disagreements but does not change our parser to match PyYAML quirks"), none of these were changed. Fixing the 5 would **regress** suite compliance below 405/406; fixing the 1 is already deliberate strictness. **No fixes shipped** — items 3/4/5 close with the audit corpus pinned as a regression test. Do not invent fixes to justify the original ~11d estimate.

**Design constraint**: granit-parser upstream may not be actively maintained; item 4 may require a maintained fork. Unchanged — item 4 needed no fork because the audit surfaced no fixable case.

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

## v0.11.7 — "CI signal hygiene" (target: Q3 2026)

> Replace the deliberately-failing `stub-build-check` CI job with static assertions that pass when the repo is correct (green CI), fail only on regression. Track `rust-numpy` free-threaded support status for re-enabling ndarray on cp314t.

| # | Item | Layer | Status |
|:--|:-----|:------|:------|
| 1 | **stub-build-check → release-guard** — replace the always-red container build with static assertions: `grep` guards `publish.yml` against `--generate-stubs`, `git ls-files` asserts `.pyi` is tracked, `test -f` checks `py.typed` | CI | ✅ |
| 2 | **Numpy free-threaded tracking** — ROADMAP.md documents `rust-numpy` free-threaded support (PyO3/rust-numpy#476) as a tracked dependency | Docs | ✅ |

**Changelog mapping**: Entries under `[0.11.7]` in CHANGELOG.md.

---

## v0.12.0 — "Competitive Response" (target: Q3 2026)

> Respond to `yaml-edit` competitor features with a fast, round-trip-preserving editing story. D3 ships the create-missing path write; D4 adds Rust-backed AST traversal.

| # | Item | Layer | Status |
|:--|:-----|:------|:------|
| D3 | **`set(create_missing=True)`** — create missing intermediate mapping keys along an edit path (`doc.set("$.a.b.c", 2)`) | Rust + Python | ✅ 2026-08-04 |
| D4 | **`doc.walk()` / `doc.scalars()`** — Rust-backed depth-first traversal yielding `Node` objects, matching `Node.walk()` semantics without per-node `to_dict()` resolution | Rust + Python | ✅ 2026-08-04 |

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

**Deferred (not committed, revisit at each milestone review)**: Custom YAML 1.2 parser (evaluation deferred — granit migration just shipped, let it settle); numpy free-threaded re-enable (blocked on upstream rust-numpy). Tracked in Research & Exploration below with a revisit rule (see Review Notes 2026-08-11).

---

## Known Engine Boundaries (2026-09-29)

Deliberate hub-model limits in the TOML spoke, pinned by characterization tests in `tests/test_toml.py` (classes `TestSectionHeaderCommentBoundary`) so they cannot silently drift. Both stem from the shared YAML text hub (`from_toml` → YAML → `to_toml`), where YAML is the interchange format. **Values always round-trip losslessly**; only certain TOML-only *stylistic* forms are not reproducible.

- **Table-header inline comment (`[sec] # note`).** The shared YAML engine does not capture a comment on a container's key line (verified independently: pure YAML `sec: # note` also drops it on parse), so the note is not re-emitted by `to_toml`. Standalone/leading comments above a header and trailing comments on a leaf `key = value` line *are* preserved. Root-fixing needs a change to the locked granit comment-capture model — high blast radius across all YAML comment output and the 99.75 % compliance guarantee — so it is explicitly declined rather than silently shipped.
- **Binary integer source (`0b1010`).** Canonicalised to decimal on the round trip because YAML Core has no `0b` spelling (a faithful `0b` in YAML would re-resolve to a string, corrupting the value). Hex/octal source *is* preserved (YAML Core resolves them back to the same integer). Recorded in `toml/parser.rs` (`parse_prefixed_body_via_dispatch`) as a deliberate choice, not a defect.

### Fuzz findings (weekly-scheduled `fuzz.yml`, engine surfaces, 2026-10-02)

`fuzz.yml` now fuzzes all four engine surfaces on a weekly cron (plus `workflow_dispatch` and a `fuzz/**` path trigger), uploading minimized crash artifacts on failure so the finding pipeline is closed: crash -> regression test -> seed -> fix. Eleven crashes have been surfaced and fixed to date:

- The JSON comment char-boundary panic (#213).
- Unterminated-quote anchor names (#215).
- Double-decoded double-quoted scalars (#216).
- The 12-byte `&&&&:<LF>#&&&:&` anchor restructure (#218 — the raw anchor scanner took `:`+EOL as name material, re-harvested phantom anchors from comment text and overlapping `&` runs, and shifted every later id-name pairing; the emitted `&&&&: v` then re-parsed as anchor `&&&` plus a value indicator). The full parse -> to_yaml -> re-parse loop is now pinned at pipeline level (#224).
- The nested self-referential merge stack-overflow (#226 — `resolve_mapping_merges` re-walked freshly prepended anchor clones with the path cycle-guard already popped, so a `&b` body re-using `*b` expanded a fresh clone every round and the descent overflowed the native stack; the tail walk now recurses only into the mapping's own children, plus a `MAX_MERGE_DEPTH` budget).
- The trailing-colon anchor-name emit drift (#227 — `write_anchor_tag` emitted `&name` bare, so a name ending in `:` merged with the appended space into a value indicator and lost one character per round; unsafe names are now emitted as quoted `&"name"` anchors).
- The quoted anchor name swallowing a line break (#228 — `scan_anchor_name`'s quoted branch read the name across a carriage return to a closing quote on a later line, producing `X-\r:&`; the growth per round was only caught at pipeline level. granit ends an anchor token at CR/LF, so a closing quote past a line break no longer qualifies).

Four of those were the same subsystem — the raw anchor scanner's name grammar drifting from granit's tokenizer — and were root-fixed (not patched per shape) in #230: `scan_anchor_name` now mirrors granit's `scan_anchor` exactly (maximal `is_anchor_char` run, no invented quoted-anchor or value-indicator-colon branch, anchor tokens skipped atomically), subsuming #215/#218/#227/#228 by construction. Block scalars followed the same method: read the authoritative reader first, then close the writer against its exact read map — #231 (Clip trailing blanks re-emitted as Keep, header comments on the header line) and this cycle's fold-aware newline writer (runs and leading blanks measured against granit's folded read; inner and leading runs now close by construction, subsuming crashes cfb3fa83/c18cb1fd/490c4beb/6288e5be). The next open finding is a distinct fidelity gap, not an emission drift: `write_scalar_for_key` drops a mapping key's tag/anchor and an empty plain key re-reads through granit's `~` normalisation (crash-86a9ae7b, `!g:\t\t:`); the fix is key-level metadata preservation, one root cause per PR. The schedule keeps surfacing residual drift before it ships.

The scheduled loop is proven end-to-end: each `workflow_dispatch`/cron run fuzzes the four surfaces and, on a crash, uploads the minimized artifact for triage (findings #226/#227 were surfaced and fixed this way). The engine is never declared "clean" — the point of the schedule is that it keeps surfacing new edge cases to pin, one root cause per PR.

---

## Leaderboard & Performance Status (2026-09-30)

Continuous ranking gates run in normal CI (`tests/test_leaderboard.py`,
`tests/test_toml_leaderboard.py`, `tests/test_json_leaderboard.py`); absolute
per-op timings live in CodSpeed (`test_benchmark_*`). Timing gates are deliberately
**in-process self-relative floors** (a native path vs the AST / round-trip path it
replaces), never cross-library ranking asserts — those proved flaky on shared CI
runners (macOS especially) and are tracked in CodSpeed instead.

- **YAML**: top-3 in class enforced against PyYAML / ruamel / ryaml / yaml_rs — pyrs #1-#2 serialize, #2-#3 parse.
- **TOML**: the native single-pass `load_toml` is gated against the in-process AST route it bypasses (`parse(from_toml(doc)).to_dict()`, ~2.2×/1.9× on medium/large), and the native writer `doc.to_toml()` against the `to_yaml()` round-trip it replaces (~4.3× `tomli_w` / ~65× `tomlkit` in CodSpeed) — effectively #1 among installed TOML libraries on both parse and serialize. The former "faster than `tomllib` by 2×" blocking assert was **retired as unsound**: the ratio is a platform property, not a code property (~4.4× on Windows, ~1.8× on the macOS runner), so it flipped red on `macos-latest` with the kernel unchanged. Cross-library ranking is owned solely by CodSpeed (`test_pyrs_load_toml` vs `test_tomllib_load` / `test_tomlkit_parse`).
- **JSON (largely optimized; #1 is a C-serializer ceiling)**: `load_jsonc` uses a direct parse→Python fast path (single pass, no intermediate AST) covering objects/arrays, i64 integers, floats, the simple string escapes, booleans/null and escape-free strings — measured **faster than stdlib `json.loads`**, ranking #2 behind `orjson` (fastest-in-class C serializer). Serialize iterated too: a native single-pass writer replacing the old `to_dict()` + `json.dumps` double conversion (~10×), direct key write, and bulk-copy string emission. Open: literal #1 over `orjson` needs an orjson-class from-scratch buffer/number formatter (architectural, parity risk). Output-buffer capacity preallocation was measured (~11% on a 150 KB doc) but **rejected** — the size-estimate walk cancels the saving and a fixed reserve risks small-document regressions.
- **JSONC / JSON5**: benchmarked (`test_benchmark_api.py`) for zero-regression tracking; competitive ranking against the (few) installed JSONC/JSON5 libraries needs optional deps — deferred pending a dependency decision.

---

## Research & Exploration

Tracked as open questions for future roadmap inclusion; not committed to any version.

> **Revisit rule** (from Review Notes 2026-08-02): every milestone review must re-evaluate all unchecked items below — promote, defer with reason, or close. No item stays unchecked for more than two consecutive milestone reviews.

- [x] **Free-threaded CPython support** — `Py_GIL_DISABLED` + full `gil_used = false` build matrix ✅ Delivered in v0.10.0 (cp314t wheels on PyPI)
- [ ] **Custom YAML 1.2 parser** — evaluate replacing granit-parser with a 100% YAML 1.2 compliant Rust parser. YAML 1.2 spec is ~80 pages with formal grammar; reference implementation libyaml (C) ~15K lines. Estimated effort: 3-6 months for production quality. Alternative: fork granit-parser and incrementally fix to 100%. ⏸️ Deferred 2026-08-11 — granit migration just shipped in v0.13.0; let it settle. Revisit v0.14.0.
    - **2026-09-26 in-tree experiment (rejected → keep 1.1.0)**: bumped granit-parser 1.1.0 → 1.3.0 (API-compatible, `cargo check` clean, nextest 205/205, pytest 1226 green) — but yaml-test-suite compliance **regressed 405/406 → 404/406**: 1.3.0 wrongly *accepts* `9C9N` (wrongly-indented flow sequence), violating the strictness posture pinned by the v0.11.5 audit. Any future parser swap/fork/uplift MUST re-run `tests/test_yaml_suite.py` + the `9C9N` probe as a hard gate.
    - **2026-09-26 follow-up (fork-uplift path closed too)**: benchmarked granit 1.3.0 on the codspeed Rust bench channel — `parse_large` median regressed 26.8 → 29.7 µs with pathological mean tails (up to 391 µs); only `parse_medium` slightly improved. So 1.3 offers no perf upside to justify carrying the 9C9N fix in a fork. The residual eager-parse gap vs `yaml-rs` (0.42×) is now *measured* to be the structural cost of strict compliance + comment/span events (yaml_rs itself wrongly accepts `9C9N` and wrongly rejects `2JQS`). Remaining levers: an in-house scanner (3-6 months) or an upstream issue for the 9C9N regression (probe repro available).
    - **2026-10-01 resolution (1.3.0 shipped + in-tree guard)**: granit-parser was nonetheless bumped to 1.3.0 (PR #169) and the predicted `9C9N` regression materialized — the suite's ≥95% threshold gate drowned `405/406 → 404/406` and it passed green CI. Fixed in-tree: the AST receiver now rejects an under-indented multi-line flow continuation (`parser::mod` flow-indent guard), restoring `405/406`. `9C9N` is pinned as a **per-case hard gate** on literal input (`tests/test_yaml_suite.py::test_9c9n_wrongly_indented_flow_is_rejected`, no `skipif`) plus a Rust unit test, so no future parser bump can hide this class behind the threshold. The parser-replacement question stays open (in-house scanner / upstream `9C9N` issue remain the only levers).
- [x] **Numpy free-threaded re-enable** — when `rust-numpy` (PyO3/rust-numpy#476) lands solid free-threaded support, remove `--no-default-features` from cp314t build lines and let the runtime probe (`py.import("numpy").is_ok()`) auto-detect. Tracked in v0.11.7. ✅ Closed 2026-08-15 — rust-numpy v0.24.0+ supports free-threaded Python; v0.29.0 (already pinned) inherits support. `--no-default-features` removed from cp314t build lines in `publish.yml`/`ci.yml`.
- [x] **YAML Schema language** — dedicated schema definition format beyond JSON Schema ✅ Promoted to v0.14.0 planning 2026-08-11
- [x] **`yaml-edit` competitor analysis** — track their feature expansion; respond with differentiator strategy ✅ Closed 2026-08-11 — passive tracking provides no value
- [x] **Community plugins** — allow third-party Python modules to register custom node types ✅ Promoted to v0.14.0 planning 2026-08-11
- [x] **`--no-default-features` build** — exclude `numpy` from wheel for free-threaded Python ✅ Committed in v0.11.6

**Committed (moved from this list, see Planned)**: v0.11.0 (surgical serialization), v0.11.2 (streaming parse, with v0.11.1); v0.11.3 (streaming write, with scoping, compliance reporting, line-offsets cache, publish pre-validation); v0.11.5 (parser robustness — audit closed empty, docs-only release, with strictness regression corpus `tests/test_strictness_audit.py`); v0.11.6 (numpy-free free-threaded wheel); v0.11.7 (CI signal hygiene + numpy tracking); YAML Schema language (promoted 2026-08-11, v0.14.0 planning); Community plugins (promoted 2026-08-11, v0.14.0 planning).
