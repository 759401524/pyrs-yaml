# Performance status

The cross-library ranking snapshot, and the history of the instruction-count gate that replaced
wall-clock judgement as the enforcement layer. Ranking belongs to CodSpeed; the zero-regression
verdict belongs to the committed Ir baseline, because a cross-process timing table on a shared
runner cannot settle a sub-10% question (measured, and recorded as the reason the divan tables
stopped being load-bearing).

Not site content: `zensical.toml` builds `docs/en`, and `check_i18n.py` mirrors only pages under
it, so this file has no `zh`/`ja`/`ko` twin and is not in the published navigation. It is an
engineering record, in the same category as `QUALITY_MATRIX.md` and `ROADMAP.md` at the root.

Related documents: [quality ledger](quality-ledger.md), [engine boundaries](boundaries.md).

## Ir gate history

Provenance is read from `.ci/ir-baseline.json` by the script that built this page, so it cannot
rot independently of the file the gate actually uses:

```json
{
  "generated_by": {
    "environment": "github-runner ubuntu24, image 20261004.327.1",
    "note": "Written by `scripts/ir_gate.py --update`; no value in this file is transcribed. The reasoning behind the tolerance, and the measurements that sized it, are in QUALITY_MATRIX.md section 2.",
    "repeats": 3,
    "rustc": "1.97.1",
    "tool": "callgrind"
  },
  "tolerance_hint": 0.005,
  "toolchain": "rustc 1.97.1 (8bab26f4f 2026-07-14)"
}
```

The entries that grew it, in order:

- **(ah)** the defence measured itself, and the measurement became a gate
- **(am)** the gate's headroom was measured for the first time, and it is thinner than the 2% line
  assumed
- **(ao)** the gate cannot reach the Python binding at all, registered rather than papered over
- **(ap)** that registered hole closed with the artefact its own exit criterion named
- **(aq)** two runner jobs measured the same code 1.44% apart, so the baseline states who owns it
- **(as)** a change was adjudicated on a channel that could not see it, then re-adjudicated on one
  that can
- **(ax)** two of the five formats had no scenario in either direction; `ir-bridge-absent` now finds
  that
- **(ay)** the in-process timing floors' verdict rule, refuted by its own CI log

## Leaderboard & Performance Status (2026-09-30)

**The fuzz tier's `slow-unit-*` findings were measured rather than believed, and they split into a
false alarm and one real hot-path signal.** libFuzzer writes a `slow-unit` file when an execution
crosses its threshold, and a `-runs=1` replay reported **451 ms** for an 18-byte YAML document —
which reads like a quadratic bug in the parser. Re-running the same binary over 200 executions of
the same input gives **34 ms total (0.17 ms/exec)**, and a 5-byte `a: 1` control costs 29 ms total
(0.145 ms/exec): the 451 ms was cold-process cost — instrumentation counter allocation, first-touch
page faults, allocator warm-up — paid once per `cargo fuzz run` and attributed by libFuzzer to the
only execution it performed. **So a `slow-unit` produced by a `-runs=0`/`-runs=1` replay is not
evidence of anything, and the CI replay mode is exactly that.** The method note is the deliverable:
any latency claim here needs ≥200 executions and an input-sized control in the same process, or it
measures the loader.

With that protocol, three of the four files are unremarkable, and one is not. Per-exec: `parse_yaml`
18 B → 1.5 ms, `parse_toml` 67 B → 2.4 ms, `parse_json` 1276 B → **4.94 ms** (258 bytes/ms),
`parse_json` 69 B → **1.57 ms** (44 bytes/ms).

**The first diagnosis of that outlier was wrong and is retired.** It looked like non-ASCII handling
— the 69-byte input is a `'` plus a long run of multi-byte code points, and the offset machinery
this ledger already knows is skipped for pure ASCII made a tidy suspect. A direct A/B killed it: the
same 69 bytes as pure ASCII costs **7 ms / 200 execs**, and with the multi-byte code point
substituted in it costs **7 ms / 200 execs** — identical. The rejection path is not the cost centre
either (both of those are rejected inputs). The table is in `.cache/iso_json.txt`; the hypothesis it
refutes is the one this file proposed an hour earlier, recorded so the next reader does not re-spend
the turn.

**What the measurements do show is a per-element constant large enough to be the whole story.**
Scaling `parse_json` at 100 execs per point (`/tmp/iso2`, script `.cache/scaling_json.sh`): a
numeric array of 50/100/200/400/800 elements costs 2.04 / 4.05 / 6.91 / 11.94 / 23.78 ms per exec,
and an array of 50/100/200/400/800 small objects costs 8.23 / 14.34 / 25.61 / 79.15 / 104.54 ms.
Growth is **linear**, so there is no quadratic algorithm here — but the slope is roughly **40 µs per
scalar** and **130–200 µs per small object**, against a measured harness floor of ~0.145 ms for a
whole 5-byte document. That is three to four orders of magnitude above what a tokenizer should cost
per token, and it means the parse hot path is dominated by something done *per node* rather than by
scanning bytes: candidate mechanisms to attribute before touching are the per-node offset/line
machinery, the `Arc<str>` + `NodeDecor` + `IndexMap` construction with SipHash on whole-node keys,
and any per-scalar registry or schema-resolution lookup. **No change was made on this evidence** — a
constant of that size is profiler work (`perf record` / a `divan` bench over `arr400` on the release
build the benchmarks use), and the arbiter is the CodSpeed gate, not a libFuzzer wall clock, which
is instrumented and inflates exactly this kind of measurement. `arr400`/`keys400` are the inputs to
start from; the artifacts stay under `fuzz/artifacts/*/slow-unit-*`.

Continuous ranking gates run in normal CI (`tests/test_leaderboard.py`,
`tests/test_toml_leaderboard.py`, `tests/test_json_leaderboard.py`); absolute per-op timings live in
CodSpeed (`test_benchmark_*`). Timing gates are deliberately **in-process self-relative floors** (a
native path vs the AST / round-trip path it replaces), never cross-library ranking asserts — those
proved flaky on shared CI runners (macOS especially) and are tracked in CodSpeed instead.

- **YAML**: top-3 in class enforced against PyYAML / ruamel / ryaml / yaml_rs — pyrs #1-#2
  serialize, #2-#3 parse.
- **TOML**: the native single-pass `load_toml` is gated against the in-process AST route it bypasses
  (`parse(from_toml(doc)).to_dict()`, ~2.2×/1.9× on medium/large), and the native writer
  `doc.to_toml()` against the `to_yaml()` round-trip it replaces (~4.3× `tomli_w` / ~65× `tomlkit`
  in CodSpeed) — effectively #1 among installed TOML libraries on both parse and serialize. The
  former "faster than `tomllib` by 2×" blocking assert was **retired as unsound**: the ratio is a
  platform property, not a code property (~4.4× on Windows, ~1.8× on the macOS runner), so it
  flipped red on `macos-latest` with the kernel unchanged. Cross-library ranking is owned solely by
  CodSpeed (`test_pyrs_load_toml` vs `test_tomllib_load` / `test_tomlkit_parse`).
- **JSON (largely optimized; #1 is a C-serializer ceiling)**: `load_jsonc` uses a direct
  parse→Python fast path (single pass, no intermediate AST) covering objects/arrays, i64 integers,
  floats, the simple string escapes, booleans/null and escape-free strings — measured **faster than
  stdlib `json.loads`**, ranking #2 behind `orjson` (fastest-in-class C serializer). Serialize
  iterated too: a native single-pass writer replacing the old `to_dict()` + `json.dumps` double
  conversion (~10×), direct key write, and bulk-copy string emission. Open: literal #1 over `orjson`
  needs an orjson-class from-scratch buffer/number formatter (architectural, parity risk).
  Output-buffer capacity preallocation was measured (~11% on a 150 KB doc) but **rejected** — the
  size-estimate walk cancels the saving and a fixed reserve risks small-document regressions.
- **JSONC / JSON5**: benchmarked (`test_benchmark_api.py`) for zero-regression tracking; competitive
  ranking against the (few) installed JSONC/JSON5 libraries needs optional deps — deferred pending a
  dependency decision.
- **Regression arbitration moved to instruction counts (2026-10-06)** — CodSpeed cannot adjudicate
  single-digit regressions. While landing the YAML note-fidelity work, three consecutive pushes each
  doing *strictly less* serialization work were scored −7.7%, −10.5% and −9.8% on the identical
  benchmark set (CodSpeed's own report flags "Different runtime environments detected"). The same
  change measured **+6.18% instructions** in `callgrind` on one machine, and after removing
  redundant per-node comment-view lookups it measured **+2.57%** — a 58% recovery the gate never
  displayed. So `scripts/ir_gate.py` + `.ci/ir-baseline.json` now run in `codspeed.yml`: same-binary
  subtraction (a setup-only pass is subtracted out, because callgrind counts the whole process),
  fixed iteration counts (an auto-tuned sample count would make the total machine-dependent — the
  exact property being removed), 2% tolerance (calibrated: the same code measured +1.45% apart
  between a WSL image and a GitHub runner, because instruction totals count the loader and malloc
  too), ~±0.001% repeat precision on one machine, ~6 s of measurement. CodSpeed keeps the
  cross-library ranking role; the Ir gate owns "did this commit make the hot path do more work?" Two
  things fell out of wiring it up: the bench documents no longer exist in two copies
  (`pyrs_yaml_core::bench_inputs` is now the single source for both harnesses, so they cannot
  drift), and the per-node constant in this pillar's open ledger got its first decomposition —
  `serialize_small` costs ~2,848 Ir per 22-byte document while `parse_small` costs ~24,876, so the
  big fish is **parse**, not serialize. Open: #267's five `serialize_*` benchmarks are still flagged
  red by CodSpeed; the Ir gate — built with the repo's real release profile, unlike the +2.57%
  figure above which came from an `lto = false` probe — measures the residual cost at +3.2%…+5.8% on
  the serialize scenarios while `parse_anchors` gains 11%, and #267 now carries the re-baselined
  numbers.

---
