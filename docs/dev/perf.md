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
  displayed. So `scripts/ir_gate.py` + `.ci/ir-baseline.json` now run in `ci.yml` as the `ir` leg of
  the merge verdict: same-binary
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

## The per-node constant, attributed on the Ir channel (2026-10-13)

The 2026-09-30 lead named three candidate mechanisms and claimed none of them. `callgrind` over the
gate's own harness, at the gate's own 2,000 iterations, finds one of them is the largest single
function in every reader except the YAML one:

| scenario | largest self cost | its share | `Hasher::write` share |
|:--|:--|--:|--:|
| `from_json_medium` | `core::hash::sip::Hasher::write` | 13.61% | 13.61% |
| `from_json5_medium` | `core::hash::sip::Hasher::write` | 13.45% | 13.45% |
| `from_toml_medium` | `core::hash::sip::Hasher::write` | 13.12% | 13.12% |
| `parse_medium` | `granit_parser::parser::Parser::peek_token` | 11.39% | 6.93% (third) |

In `from_json_medium` the whole key-cost family is `Hasher::write` 13.61% + `<CustomNode as
Hash>::hash` 1.45% + `NodeMeta::hash_custom_node_identity` 1.08% + `reserve_rehash` 1.25% = 17.4%
**hashing**, and `__memcmp_avx2_movbe` is a further 6.96% - key *comparison*, what the map does
after the hash. Allocation traffic in the same run (`_int_malloc`, `_int_free`, `malloc`, `free`,
`malloc_consolidate`, `unlink_chunk`, `finish_grow`, `do_reserve_and_handle`) totals 25.3%. So both
halves of that guess are real - the node-key hash and the per-node allocation - and the third is
not: the offset/line machinery does not appear in the top twenty at all.

What this is **not** is a measured win. The shares come from a rebuilt binary (`strip=false` for
symbols) whose whole-process totals sit -0.05%…-2.9% from the committed baseline for that reason
and for the image difference `(aq)` and `(by)` already record, so they are quoted as shares only;
and this run did not subtract the setup pass, so no per-iteration count is stated. The table
licenses one hypothesis - a cheaper node-key hash removes work from every reader at once - and the
arbiter stays `scripts/ir_gate.py` on the enforcing image, with the invariant tests that already pin
key equality and hash/`Eq` consistency carrying the correctness half.

Reproducing it one harness at a time: `scripts/ir_gate.py` also builds the binding harness, and
that binary links CPython. On this box's WSL the shared object exists but sits off the linker's
default path, so `LIBRARY_PATH=/usr/lib/python3.12/config-3.12-x86_64-linux-gnu` is what lets the
full gate run there — with `RUSTUP_TOOLCHAIN=1.97.1`, or the reading describes a different build.
The channels stay separate whatever a host can link: a `serialize_*` or `parse_*` number is the
engine harness's question, and it is answered with no interpreter in the process by
`CARGO_PROFILE_RELEASE_STRIP=false cargo bench -p pyrs-yaml-core
--bench ir_gate --features ir-gate --no-run` (the profile's `strip = true` is what leaves
`callgrind_annotate` reporting `???:0x…`), then
`valgrind --tool=callgrind <that binary> from_json_medium`.

## The string path was not allocating twice (2026-10-13)

The table above puts 25.3% of the read path in the allocator, so the obvious next cut was
`Parser::string`: it builds a `String` byte-run by byte-run and the node builder then makes the
`Arc<str>` the AST holds - which reads like two allocations per string, on a fixture whose 26
`malloc` calls per parse are almost exactly its 26 `string` calls. The change was written and
tested: `string()` returns `Cow<'a, str>`, the escape-free path borrows the source, and
`quoted_or_plain` takes the `Cow` so the buffer is made once, by whoever owns the node.

It made the path slower. `from_json_medium` 120,164,607 → 120,858,397 counted instructions,
**+0.58%**, with the two scenarios the change cannot enter repeating as controls in the same pair
of runs (`from_toml_medium` -0.002%, `parse_medium` -0.0002%), so the +0.58% is the change and not
the machine. The 57 JSON tests passed either way, which is what makes the number the only evidence
worth having.

The reason is a detail worth keeping: a `String` filled by one `push_str` of its exact length has
`capacity == len`, and `Arc<str>::from(String)` steals such a buffer instead of copying it. There
was never a second allocation to remove; borrowing the source replaces a steal with an allocation
*plus* a copy of bytes the parser already held. Reverted, nothing shipped.

What that leaves: the allocator's share on this path is the tree's own footprint, not slack in the
string decoding. The remaining candidates are the containers (each object and array grows its
`IndexMap` / `Vec` buffer) and the per-node shape of the AST itself - and the second is the
architecture the repository has already ruled out rewriting for fit-of-purpose reasons (see
`AGENTS.md`, the no-refactor rulings on the AST and the binding boundary), so any attempt on it has
to arrive as its own design decision rather than as an optimisation smuggled in beside one.

## Every resolver trimmed text that had nothing to trim (2026-10-13)

The same profile that found `Hasher::write` put 1.64% of `from_json_medium` in
`<str>::trim_matches::<is_yaml_blank>`: 1,976,000 counted instructions over 52,000 calls, **38 per
call** - and 52,000 is one call per scalar the JSON reader touches, key or value, almost all of
which neither start nor end in separation. `resolve_core_type` asks "is this text a number or a
word?" and its first act was to strip edge separation, so `name` paid a two-ended predicate walk to
discover that nothing was stripped. `resolve_yaml11_type` then trimmed again before delegating to
the core chain, so a YAML 1.1 scalar paid it twice.

The rule that makes the walk free to skip: every character `is_yaml_blank` accepts is ASCII
(`SP TAB CR LF NUL`), and UTF-8 keeps an ASCII byte out of a multi-byte sequence. So `value` starts
or ends blank exactly when its **first or last byte** is one of those five bytes, and two table
reads answer a question that previously cost 38 instructions. When either end is blank the trim is
the original `trim_matches(is_yaml_blank)`; the guard only declines work it can prove is not there.
The class table is built at compile time *by calling the predicate* over the 128 ASCII positions,
not by restating the set - `(bi)` in the ledger is the record of a rule written twice and
disagreeing.

Measured in one pair of runs of the same harness binary (`callgrind`, whole-process `I refs`,
local build, 2 000 iterations each):

| scenario | before | after | |
|:--|--:|--:|--:|
| `from_json_medium` | 120,167,551 | 119,281,047 | **−0.74%** |
| `from_toml_medium` | 200,129,565 | 198,295,602 | −0.92% |
| `to_json_medium` | 22,244,247 | 21,870,305 | −1.68% |
| `serialize_medium` | 47,556,611 | 47,386,528 | −0.36% |
| `parse_medium` | 343,640,135 | 343,645,084 | +0.0014% |

The last row is the control the project's A/B discipline asks for: the YAML text reader does not ask
this question per scalar, and it repeated inside the same-binary spread (the two `from_json_medium`
runs before the change differed by 0.005%). The two write scenarios moved because the writers
resolve plain scalars through the same crate - which is the point of a shared schema layer, and the
reason a reader-side cut lands on both sides of the round trip.

The mechanism, not just the total: after the change `<str>::trim_matches` is gone from the profile
and `trim_yaml_blank` costs 1,092,000 (0.92%), so 884,000 of the 888,571 instructions the scenario
lost are accounted for by the function that was removed. One refinement was tried and refused:
`#[inline]` on the guard gave 119,277,876 against 119,281,047, −0.003%, inside the noise floor -
LLVM had already folded what mattered, so the attribute was an unmeasured hint and it was dropped.

Three tests carry the correctness half: `the_blank_table_covers_exactly_the_predicate` (all 256 byte
values, so the ASCII-domain assumption is checked rather than asserted),
`trim_yaml_blank_agrees_with_str_trim_for_every_code_point_at_either_end` (every code point placed
at both ends of `…42` / `42…`, built in stack buffers so the sweep does not test the allocator), and
`trim_yaml_blank_removes_only_yaml_separation` for the shapes the resolvers care about - `"\0"`,
`"  a  b  "`, NBSP and NEL at either end, an emoji tail. Mutants confirm they bite: reading the
wrong end byte reddens the sweep with `first: Some((0, "42\0"))`, and widening the table's bound to
`byte <= 0x80` is caught by the domain test.

The arbiter agreed, and it is the number that ships: the `Instruction-count baseline` job measured
all twenty-one scenarios against this head on the enforcing image - `from_json_medium` 120,751,446 →
119,876,976 (-0.72%), `from_toml_medium` -0.89%, `from_jsonc_medium` -0.66%,
`from_json5_medium` -0.47%, `to_json_medium` -1.73%, `to_jsonc_medium` -1.59%,
`to_json5_medium` -1.56%, `to_toml_medium` -0.95%, the `serialize_*` writes -0.21…-0.51%,
`to_python_*` -0.04…-0.36% - and every `parse_*` scenario at +0.00%. Local and CI differ by
hundredths of a percent, which is the build/image residual the `-0.05%…-2.9%` note above already
warns about; what travelled between the two channels is the shape, and the shape is the claim.

## The containers were not the allocator's problem either (2026-10-13)

Two hypotheses about the same 25.3% have now been measured and put down. The first was a double
allocation per string, refuted by an A/B in "The string path was not allocating twice" above. The
second was the sentence that same section ended on: "the remaining candidates are the containers
(each object and array grows its `IndexMap` / `Vec` buffer)". The containers are priced below, and
they are not where the money is.

Method, and what it can and cannot see. `callgrind` over the gate's own harness gives self cost per
function and a call count per edge; `valgrind --tool=dhat` gives blocks, bytes and lifetimes per
allocation point. Two things learned on the way are worth recording because they will bite the next
attempt: dhat's compact JSON (`dhatFileVersion: 2`) carries file/line indices but **no function
names** for its point paths, so it is usable for totals and lifetimes only; and attributing
allocator cost to its *callers* through callgrind edges is unreliable on this binary, because
several symbol blocks are written nameless (`fn=(820)`) and the largest edge in the graph lands in
an unnamed caller. What follows therefore quotes self costs and call counts, not inclusive caller
totals.

`from_json_medium`, one run of the current tree, 119,278,980 counted instructions, 2,000 iterations:

| cost centre | self instructions | share | per parse |
|:--|--:|--:|--:|
| libc allocator (`_int_malloc` 10.49M, `_int_free` 9.15M, `malloc` 6.66M, `free` 4.26M, `malloc_consolidate` 2.34M, `unlink_chunk` 1.45M, `_int_free_merge_chunk` 0.97M, `arena.c:free` 0.46M, `_int_free_maybe_consolidate` 0.36M) | 36,124,074 | 30.3% | 18,062 |
| `RawVecInner::finish_grow` | 3,056,000 | 2.56% | 1,528 |
| `RawVecInner::reserve::do_reserve_and_handle` | 1,508,000 | 1.26% | 754 |
| `IndexMap` `reserve_rehash` | 1,924,152 | 1.61% | 962 |
| tree teardown (`drop_glue<CustomNode>` ×2 + `drop_glue<NodeMeta>` + `Arc<str>::drop_slow`) | 5,460,000 | 4.58% | 2,730 |
| `__memcpy_avx_unaligned_erms` | 9,403,098 | 7.88% | 4,702 |

The Rust-side growth machinery - the thing a "reserve the containers better" change would shrink -
is 6,488,152 instructions, **5.4%** of the scenario, and it runs *two* `realloc`s and *two*
rehashes per parse: 4,003 and 4,000 calls over 2,000 iterations. Growth is amortised doubling on a
handful of buffers, not per-node churn, so there is no per-node win available there. `IndexMap`
growth was the specific guess and it is 962 instructions per parse.

dhat sizes the whole thing from the other side: **156,015 blocks / 29,298,570 bytes over 2,000
parses = 78 allocations and 14,649 bytes per parse**, against **48 blocks / 12,416 bytes live at the
peak**, and 2 blocks at the end. So the fixture's tree is 48 heap objects, the transient part of the
run is about 30 more, and the allocator's 30.3% is what glibc charges to make and unmake roughly
that many blocks 2,000 times. It is not slack in the parser: it is the price of the node model, and
the model is the thing `AGENTS.md` rules out rewriting for fit-of-purpose reasons. Reducing it means
interning scalars, an arena, or an inline small-string node - each a design decision about the AST,
each needing its own argument, none of them an optimisation that can be slipped in beside one.

One number in that table is worth carrying forward as the cheapest remaining target that *is* inside
the current model: teardown is 4.58% of the run and it walks the tree twice over (`drop_glue` self
cost 2.54% + 1.44% for `NodeMeta`) before the frees it triggers even start. Dropping an AST is
therefore comparable in cost to building one, and the walk - not `free` - is the part a change could
make cheaper without touching what a node holds. Not attempted here: the claim would need the same
paired-run evidence this file keeps asking for, and the scenario that would show it
(`from_json_medium` with its tree dropped at the end of each iteration) is the gate's own harness,
so it is measurable in one more session rather than in one more guess. The next section is that
session.

## A node was 208 bytes and 128 of them were metadata (2026-10-14)

That target had a shape, and the shape was measured rather than guessed. `std::mem::size_of` on the
tree as it stood:

| type | bytes |
|:--|--:|
| `CustomNode` | 208 |
| `NodeMeta` | 128 |
| `Tag` | 48 |
| `NodeMap<CustomNode, CustomNode>` | 72 |
| `NodeDecor` | 32 |
| `Comment` | 24 |

61% of a node was the metadata a strict JSON document never fills, and each of `NodeMeta`'s five
`Option` slots was a branch of its own when a tree is dropped. The two costs the section above
prices then follow the size: the copies (`memcpy`, 7.88%, 4,702 instructions per parse) move
208-byte nodes, and the teardown walk (4.58%, 2,730 per parse) asks five questions per node before a
single `free` starts.

The first change is not the layout, it is the door. Every read and write of those five slots now
goes through `NodeMeta`'s own accessors - `comment()` / `set_comment()` / `take_comment()` and the
same trio for the anchor, the tag and the source range, `decor_mut()` for the boxed leading-note
bundle, `is_identity_clean()` where the serializer used a pattern to ask whether a node carries no
comment, anchor or tag - and the struct literals that named every field became `with_comment()`,
`with_tag()`, `with_anchor()`, `with_source_range()` and `scanned()`. Nothing outside `pyrs-ast`
reaches a metadata field now.

The door was going to cost nothing, and that was the wrong control. The claim was a hash: the
`ir_gate` harness binary built from the accessor tree and the one built from its parent came out the
*same bytes* (sha256 `c504386fe4b52efc`), and identical machine code cannot move an instruction
count. Both builds used the host's rustc 1.99.0. The gate pins **rustc 1.97.1** - a different
codegen - and PR #382's CI run said `serialize_small +0.61%`. Measured paired on 1.97.1 in WSL the
accessor-only pass reads `serialize_small +5.49%`, concentrated in `write_plain_scalar` (+81
counted instructions per iteration) and `write_scalar_for_key`, i.e. in the two calls the pattern
match became a function call. So byte-identity on a compiler the gate does not use is evidence about
nothing; the only control that settles a "costs nothing" claim is the gate's own toolchain.

What ships is therefore the door and the layout as one measured change, because together they are
net negative on 1.97.1. `NodeMeta { comment, decor: Box<_>, source_range, fidelity: Box<Fidelity> }`
with `Fidelity { anchor, tag }` takes the struct from 128 to **64** bytes and `CustomNode` from 208
to **144** in the default `std` build - the feature split swaps the node map's hasher (`RandomState`
for `std`, the crate's own fixed-seed `FxHasher` otherwise), and on that path the node lands at
**128**; the `no_std` CI job found that by reddening on a tripwire that pinned 144 alone. The test
`node_layout_is_part_of_the_hot_path_contract` pins all four numbers, so the table above cannot rot
silently. (Pinning four numbers did not last: the section at the end of this file unifies the
hasher, and the node is 128 on both sides now.) The paired run (2,000 iterations minus
`--setup-only`, `m197.bin` vs the retuned binary):

| scenario | paired delta on rustc 1.97.1 |
|:--|--:|
| `from_json_medium` | −4.83% |
| `from_jsonc_medium` | −4.02% |
| `serialize_anchors` | −3.95% |
| `from_json5_medium` | −3.50% |
| `parse_inline_merge` | −3.53% |
| `parse_anchors` | −3.45% |
| `parse_medium` | −1.51% |
| `from_toml_medium` | +0.74% |
| `serialize_medium` | +0.17% |
| `serialize_small` | −0.07% |

The rest sit inside ±0.2%. What decided the grouping is the first attempt's number: boxing the
`source_range` too - which the YAML adapter records for *every* node it reads - gave
`parse_medium +5.94%`, `parse_block +4.38%` and `serialize_medium +3.16%` in exchange for
`from_json_medium −7.41%`. A field written per node must stay flat; only the anchor and the tag,
which only an explicitly anchored or tagged source ever produces, go behind the pointer. Comments
and the leading-note bundle stay where the fidelity formats can reach them without an allocation.

Two cheaper variants are deliberately not on the list: shrinking `Tag` (48 bytes) or swapping
`anchor: Option<String>` for `Option<Arc<str>>`. Each saves bytes and neither shortens the drop
walk, so they would be measured against a node shape this change has now moved anyway.

---

## What folding on write costs (2026-10-14)

Issue #336 describes a real fidelity hole: a mapping can hold two entries whose keys the reader
resolves alike (`!E null` twice, `null:` beside `'null':`, `k:` beside `'k':`, two empty `{}` keys),
because the node map compares whole nodes while the reader identifies a key by what it resolves to.
Emission then produced either text the strict reader refuses (`duplicate key: null`) or text that
re-reads a line shorter than the tree, and `to_yaml` returns a `String`, so it cannot say so. The
ruling was to fold on write - last entry wins, the dropped line's notes re-home - and this section
is the price of doing it that way.

Implemented on branch `yaml-writer-key-fold-336` (987d98b6), together with three tests: five rows of
the issue's table re-emitted and strictly re-read; the anchor boundary asserted as a test; and note
ordering pinned to document order. All three fail when the fold is disabled. Measured paired on the
compiler the gate pins (rustc 1.97.1, `GLIBC_TUNABLES` as `.ci/ir-baseline.json` records it, loop =
full run minus `--setup-only`), with the *same tree* and the fold switched off as the base:

| scenario | fold on write |
|:--|--:|
| `serialize_anchors` | +33.655% |
| `serialize_medium` | +32.585% |
| `serialize_block` | +31.801% |
| `serialize_small` | +31.481% |
| `serialize_block_scalars` | +12.529% |
| `parse_small` | +0.000% |

The first attempt read +40-55%. callgrind attributed the excess to what the check allocated rather
than to what it computed: one `Vec` of key references and one `HashMap` per mapping, with SipHash
over borrowed tag strings. Rewriting detection to reuse a single fingerprint map across the document
(Fx-style multiply-xor over the class, identity re-verified only on a hash hit, `Other` keys skipped
because the node map already compares those as nodes) removed the allocations and brought
`serialize_small` from +40.6% to +31.5%.

What is left is structural, not incidental. In these fixtures a key line costs about 55 counted
instructions to write, so any per-key duplicate check is a large *relative* tax: +31% of
`serialize_small` is roughly 14 instructions per key. No implementation of "look at every key before
writing the mapping" is going to land inside the gate's 0.50% tolerance, which makes this a
deliberate-upward-movement decision (`accept_drift` on the baseline-refresh workflow) rather than an
optimisation to grind on.

It is also not the place the defect comes from. A tree that came from the reader cannot hold the
pair - the reader folds it or refuses it - so the writer would re-derive a fact it already knows.
The shape enters through the editing API, where two path segments that spell differently and resolve
alike (`~` and `null`) both insert into the same mapping. Folding there is cheap - one scan per
edit, not per key of every document ever written - but it is not enough on its own: `set_path`
returns an *insert* dirty unit, so patching the source adds the new line and leaves the old one, and
the document text would still hold the duplicate. The work #336 needs is therefore in the edit layer
plus a Replace/Remove unit in the dirty plan, not in the serializer.

That is why this branch is shelved rather than merged with a re-baselined floor: the tax is
permanent, the guarantee it adds is available at a place that costs nothing, and the gate's
zero-regression premise is what makes the next measurement trustworthy. The identity model the fold
needed (`CustomNode::key_identity`, `KeyIdentity`) is merged and independent of the decision -
PR #383 - so the edit-layer fold will not have to re-derive it.

---

## The node map was hashing its keys with SipHash (2026-10-14)

The profile the file carried forward was two changes old, so it was re-measured on the merged
layout before choosing a target. Same method as the section above it: `callgrind` over the gate's
own harness, `GLIBC_TUNABLES=glibc.cpu.hwcaps=-AVX512F,-AVX2,-AVX,-SSE4_2,-POPCOUNT`, loop cost =
full run minus `--setup-only`, rustc 1.97.1. Shares of the loop, three repeats, spread below
0.01%:

| cost centre | `from_json_medium` | `parse_medium` |
|:--|--:|--:|
| libc allocator (`_int_free`, `_int_malloc`, `malloc`, `free`) | 29.10% | 9.46% |
| hash (`Hasher::write`, all of it SipHash) | 13.11% | 8.64% |
| copy (`__memcpy_avx_unaligned_erms`) | 6.67% | 8.05% |
| teardown (`drop_glue<CustomNode>`, `Arc<str>::drop_slow`) | 4.73% | 2.51% |

Two numbers moved since the old table: `memcpy` is 7.88% to 6.67% because the node it copies got
smaller, and the allocator's 30.3% is still the largest single share. The one nobody had priced is
the hash row: `<core::hash::sip::Hasher<Sip13Rounds> as Hasher>::write` self-costs 10,214,000
instructions, **8.98% of the JSON read path**, and the caller edge is a single line -

```text
10,214,000 ( 8.98%)  < ???:<pyrs_ast::ast::CustomNode as core::hash::Hash>::hash::<std::hash::random::DefaultHasher> (180,000x)
```

180,000 calls over 2,000 iterations is 90 key hashes per parse, which is the fixture's mapping keys
and nothing else: `NodeMap` under the `std` feature was `IndexMap<K, V, std::hash::RandomState>`,
and `RandomState` is SipHash-1-3. The `no_std` side already used the crate's own fixed-seed FxHash.
So the default build paid a cryptographic-per-message hash to place a key in a short-lived table,
and the split meant the two feature sets were not even the same node (144 bytes against 128).

Unifying both sides on `FxHasher` is the whole change. Paired on 1.97.1, three repeats, same
session, alternating binaries:

| scenario | paired delta |
|:--|--:|
| `from_json_medium` | -12.75% |
| `parse_medium` | -4.95% |
| `to_json_medium` | +0.11% |

The repeats of one pair agreed to the instruction (`from_json_medium` fx: 98,994,200 three times),
and -12.75% is roughly what the 8.98% attribution plus the smaller node predicts: the write path
hashes nothing, so `to_json_medium` is a copy-and-allocator scenario and moved by its noise.

What it cost to land, because the number is not zero: every place that spelled a node-keyed map as
`IndexMap<CustomNode, CustomNode>` was silently asking for `RandomState`, so ~40 sites across
`pyrs-yaml-core`, `pyrs-yaml-cli` and the bindings had to name `NodeMap` instead - which is the
same fix the `no_std` build already needed and never got, since nothing but `pyrs-ast`,
`pyrs-event`, `pyrs-schema`, `pyrs-json` and `pyrs-toml` is built for the bare-metal target. A
custom hasher also drops indexmap's `From<[(K, V); N]>` impl, so two test sites became
`.into_iter().collect()`.

Nothing in the API can tell a SipHash table from an FxHash one - both look up the same keys - so the
choice is pinned by a test that hashes a key twice, once through the map's own hasher and once
through `FxHasher`, and compares the digests:
`the_node_map_hashes_with_the_crates_own_hasher_whatever_the_features_say`. Mutation-checked, as the
file asks: putting `std::hash::RandomState` back on the std side reddens it on that side and passes
on the other, which is exactly the asymmetry this change removed.

The trade-off is HashDoS, and it is now the whole build's, not embedded's: a document written to
collide can degrade a node map to linear probing. The reasoning that accepted it for `no_std`
extends - the keys come from a file the caller chose, the tables die with the tree, and
`IndexMap` keeps insertion order so a collision costs probes, not changed output - and it is
recorded on `NodeMap` rather than left implicit.

Cheapest remaining targets in the same profile, and the first of them is now closed by measurement
rather than by work: this crate's `FxHasher::write` folds one byte per round, where rustc-hash reads
eight at a time, and 90 keys per parse looked like the place that would show. It does not. Paired on
1.97.1 against the same tree with only that loop changed to word-at-a-time (3 repeats,
`from_json_medium` 98,994,200 → 98,854,200): `from_json_medium` −0.14%, `parse_medium` −0.04%,
`to_json_medium` −0.00%. The keys here are short - a handful of bytes each - so the tail loop runs
whatever the shape, and what is left of the hash row is the 180,000 digest calls themselves, not the
byte loop inside them. A change that cannot move a scenario past the gate's 0.50% tolerance is not a
win available at this address, so the word-at-a-time variant was measured and dropped. What remains
in the profile is `memcpy` at 6.67% and 8.05% and the allocator's 29.1%, both of which are the node
model the file has already ruled out rewriting.

---

## Two readers still decoded one character at a time (2026-10-14)

The section above closed with "what remains is `memcpy` and the allocator", which was true of the
node model and premature about the file: re-attributing the readers on 1.97.1 after the hasher
landed (`full - --setup-only`, per-function `callgrind_annotate`) collapsed the hash row of
`from_json_medium` from 13.11% to 3.62% and left the allocator at 33.36%, but it also showed two
scanners that had never been given the source-run treatment. The JSON reader's double-quoted
`string()` bulk-scans to the next byte that needs a decision and copies the run; its JSON5
single-quoted twin, and the TOML reader's single-line basic-string scanner, were still decoding one
code point per iteration - `chars().next()`, then `push(ch)`, then advance by `len_utf8()` - so each
character cost a UTF-8 decode, a capacity check and a push. The two rows were 6.30% of
`from_json5_medium` (6,916,000 counted instructions) and 5.10% of `from_toml_medium`
(`parse_basic_string_content`, 8,648,000), which on that fixture's eight basic strings is about 540
per value of nine characters: roughly sixty instructions to move a byte that was already in the
buffer.

Both now bulk-copy. The stop set is the bytes that need a decision and nothing else: `"` `\` and the
two line terminators, plus `forbidden_control` for TOML's basic string. It is sound for the same
reason the existing bulk scan in `string()` is: every byte in the set is ASCII, and UTF-8 keeps an
ASCII byte out of a multi-byte sequence, so a run that stops only on those begins and ends on a char
boundary - which is what the per-character loop had been guaranteeing by decoding. JSON5's dialect
matters for the boundary in the other direction: a control character is *legal* inside a
single-quoted string, so it is not in that scanner's stop set and the run copies through it.

Both now bulk-copy. The stop set is the bytes that need a decision and nothing else: `"` `\` and the
two line terminators, plus `forbidden_control` for TOML's basic string. It is sound for the same
reason the existing bulk scan in `string()` is: every byte in the set is ASCII, and UTF-8 keeps an
ASCII byte out of a multi-byte sequence, so a run that stops only on those begins and ends on a char
boundary - which is what the per-character loop had been guaranteeing by decoding. JSON5's dialect
matters for the boundary in the other direction: a control character is *legal* inside a
single-quoted string, so it is not in that scanner's stop set and the run copies through it.

Paired on 1.97.1, same session, alternating binaries, two repeats (which agreed to the instruction):

| scenario | paired delta | why it is in this table |
|:--|--:|:--|
| `from_toml_medium` | -5.55% | the basic-string scanner |
| `from_json5_medium` | -5.76% | the single-quoted twin |
| `from_json_medium` | +0.00% | control: the double-quoted path was untouched |
| `to_toml_medium` | -0.00% | control: the same crate's writer was untouched |

A change of this shape is worth exactly one test per stop-set decision, and the two added are
mutation-checked: dropping `forbidden_control` from TOML's stop set reddens
`a_multibyte_run_is_copied_whole_and_the_bytes_that_ended_it_still_do` with `"a = "é\u{1}" parsed
instead of rejecting`, and adding `b < 0x20` to JSON5's stop set reddens
`json5_single_quoted_runs_stop_only_where_the_decoding_loop_stopped` at the arm that now says the
scan stops on exactly these cases. Both mutations were reverted; neither test is decoration.

The multiline basic-string twin is deliberately not in this change. Its run cannot be long: TOML 1.0
line-ending normalisation and the backslash-continuation rule mean the scan has to stop at every
line terminator anyway, and no scenario measures a multiline document, so there is no number to
price it against.

What the same profile says about the rest of the TOML reader, kept because it is the next question:
`from_toml_medium` counts 169,449,833 instructions against `from_json_medium`'s 98,994,186 for the
*same* document (both are `MEDIUM_YAML` rendered for that dialect), and its allocator row is 37.83%
against JSON's 33.36%. The reason is structural and nameable - TOML parses into a `CowTable` /
`TomlTable` model with `String` keys and per-pair annotations, then projects it into the hub AST in
`cow_table_to_node`, so every key is allocated, hashed and dropped twice. That intermediate exists
because TOML semantics ask for it (out-of-order `[a.b]` headers, implicit tables, duplicate-table
rejection), so removing it is a design decision with a conformance surface, not an optimisation to
smuggle in beside one - which is what this file says about the hub AST itself.

One measurement trap, since it briefly sent me to fix a hash cost that was not there: bucketing a
`callgrind_annotate` listing by function-name regex put TOML's hash row at 12.46%. The regex matched
`sip|FxHash|Hasher`, and `Hasher` appears in `<indexmap::map::IndexMap<String, TomlTable,
core::hash::BuildHasherDefault<..>>>::insert*` - the *type parameter* of the map's own methods, not
digest work. TOML's real digest row is 3.33%, the same `FxHasher` the hub uses. A bucket is a
question about names, so the only reading that counts is the list of names inside it: print the
members, not the total. The probe that did this round's attribution was one-off scratch under
`target/` when this was written; it is `scripts/ir_gate.py --profile` now, described at the end of
this file.

---

## Two per-token calls that had nothing to do (2026-10-14)

The same attribution that found the scanners put `Parser::ws` at 6.63% of `from_json_medium` and
10.08% of `from_json5_medium`, on a fixture that contains **no whitespace at all**. That is not a
scanning cost: `from_json_medium` spends 3,315 counted instructions there for ~90 calls, which is
thirty-odd instructions of call and match-chain per question whose answer is "the next byte is not
separation". The comment-attachment row was the same shape - `attach_pending` self-cost 3.23% on a
document with no comments, because every key and every value meets it.

Both now answer "nothing to do" without doing it. `ws` is an inlined guard over one small predicate,
`at_separation`, and the loop moved to `ws_extended` unchanged; `attach_pending` returns on an empty
list. The guard's byte sets are a *sound superset* of the loop's, in the direction that matters: if
the loop would consume something, the guard says so. Over-calling is harmless - a high byte that
turns out not to be JSON5 separation costs a call, never a position - and the direction is pinned by
`the_whitespace_guard_never_declines_what_the_loop_would`, which asks both the same question for all
128 ASCII bytes, five multi-byte separators plus one non-separator, three comment shapes and three
dialects, and requires the same resulting position and pending-note count. Mutation-checked: making
the guard answer `false` for `/` fails it with `left: (0, 0) right: (4, 1)` on `"//x\n1"` under
JSONC, which is the under-call that would silently drop a comment. `attach_pending` needs no test of
its own - skipping work for an empty list is correct whatever fills the list, and the JSONC comment
tests already pin what a non-empty one does.

Measured as three binaries from three commits of the same branch, so each guard carries its own
number (paired, two repeats agreeing to the instruction):

| scenario | previous change → +`ws` guard | +`attach_pending` guard |
|:--|--:|--:|
| `from_json_medium` | -4.33% | -1.56% |
| `from_jsonc_medium` | -3.52% | -1.25% |
| `from_json5_medium` | -4.49% | -1.40% |
| `to_json_medium` | +0.00% | — |

The writer is the control: it calls neither function. The `ws` row is larger than the ~2.5% the
per-call arithmetic predicts; the likeliest reason is that inlining lets the callers keep `pos` live
across the test instead of spilling it around a call, but that is an explanation this file has not
measured, so it is offered as one and the number stands without it. The `attach_pending` row
recovers about half of its 3.23% attribution, the rest being the `mem::take` at the two call sites,
which stays because a comment slot has to be claimed before the value's separation is scanned.

What that leaves in the same profile, in size order: the allocator (33.36% of `from_json_medium`,
which this file has already shown to be the tree's own footprint rather than slack), the per-string
`quoted_or_plain` decision (8.24%, and its own earlier entry reduced it from a full resolver to a
table lookup), and `Parser::string` itself (9.17%). None of the three is a call that does nothing.

---

## The TOML writer formatted every line and allocated every piece (2026-10-14)

Attributing the writers, after the reader work, found the same shape of cost in `pyrs-toml`'s
emitter and a much larger one. `to_toml_medium` counted 53,218,850 instructions against
`to_json_medium`'s 21,283,040 for the *same* document - the writer was 2.5x the writer it was
supposed to resemble. Its rows: `scalar_key` 10.03%, `String as fmt::Write::write_str` 7.64%,
`core::fmt::write` 5.54%, `emit_pair` 6.10%, `quote_basic` 4.98%, `value_str` 3.81%, allocator
28.65%. The formatter rows are one cause and the allocator rows another: every emitted line went
through `writeln!`, and every key and every value was a `String` of its own before being copied into
the document - an array or an inline table allocated a `Vec` of those and joined them, then
formatted that, then copied it again.

Both were measured separately, on three binaries from three commits of one branch:

| step | scenario | paired delta |
|:--|:--|--:|
| `writeln!` -> `push_str` in the document emitter | `to_toml_medium` | -19.00% |
| keys and values written into the buffer | `to_toml_medium` | -33.27% |
| both together | `to_toml_medium` | **-45.95%** |
| control (reader, untouched path) | `from_toml_medium` | -0.15% |
| control (YAML writer) | `serialize_medium` | +0.01% |
| control (JSON writer) | `to_json_medium` | +0.00% |

53,218,850 -> 43,104,898 -> ~28,761,000 counted instructions, repeats agreeing to the instruction
(the last figure is the paired ratio applied to the measured one - `pair.py` prints the base and the
delta, not both absolutes). The `quote_basic`/`quote_multiline` String-returning helpers became
`push_quoted`/`push_multiline` appending to the caller's buffer, `quote_key` became `push_key`,
`value_str` became `write_value`, and the sequence and inline-table paths stopped materialising a
`Vec<String>` to join - the one remaining owned key is a section header's name, which is genuinely
held across the loop that reads it. `quote_key` is gone; the other two live on as `quoted` and
`multiline` inside the test module, because an assertion wants a value back and shipped code does
not.

Byte-identity is the claim that matters for a writer, and the tests cannot make it: they assert what
the output should be, not that it is unchanged. So the differential ran both trees over every TOML
document in the repository - 777 valid `toml-test` files plus the fuzz corpus and seeds,
deduplicated by content hash to 712 inputs - routed `TOML -> hub AST -> TOML` so the bytes compared
are the writer's own, with a second pass through the reader to catch text that only moves on
re-emission.

- 242 inputs produce text on both trees; every one of the 242 SHA-256 digests matches.
- Every rejection raises the same error class with the same message prefix on both trees.
- 9 inputs are not fixed points - identically the same 9 on both trees, so a pre-existing property
  of those shapes rather than something this introduced.
- Zero differing records out of 712.

The check that makes the comparison mean anything: the "before" tree was built from the parent
commit and grepped for the new helpers (0 occurrences) before its output was trusted, because a
differential between two identical trees passes for exactly the same reason it would pass if nothing
had changed.

What the same profile leaves, named with its number: the output buffer's growth is 10.7% of
`to_json_medium` (`realloc` + `_int_realloc` + `finish_grow`), and this file has already priced that
answer twice - capacity preallocation measured ~11% on a 150 KB document and was rejected because
the size-estimate walk cancels the saving and a fixed reserve regresses small documents.
`to_toml_medium` is now 28.8M against JSON's 21.3M, and the gap that remains is `fmt_toml_float` and
the integer fallback, which format through `core::fmt` because they mean it. The larger number is
still the reader: `from_toml_medium` counts 160,049,672 against `from_json_medium`'s 93,234,186,
which is the intermediate table model this file names above and has not touched.

---

## The YAML writer asked the same question twice per node, in two different ways (2026-10-14)

Two rows of the same profile owned the cost of writing one YAML document. `write_plain_scalar` and
`write_scalar_for_key` were 9.21% and 9.29% of `serialize_medium`, with ~10% more in the
`is_contained_in` / `try_fold` rows under them, and `write_indent` was 8.10%.

The first belonged to one function: `needs_double_quoted`, which answered "can this token go out
plain?" with four `contains`, two `chars().any` and sixteen `starts_with` - twenty-two passes over
the same text, most looking at characters the first pass had already looked at. It is now one byte
pass. A byte below 0x80 settles the whole ASCII question at once - the C0 controls and DEL are what
`char::is_control` means for seven bits, and `:` and `#` are the two embedded characters that end or
comment a plain token - and the leading test reads one byte instead of walking sixteen characters.
A byte at or above 0x80 answers nothing by itself; it records that the character-level half has to
run, because NBSP, the BOM, a C1 control and a YAML non-character can only arrive from there. UTF-8
never hides one of those bytes inside another character, which is the same reason `pyrs-schema`'s
`needs_quotes` was given this shape a day earlier: the writer's copy had not caught up.

The second was a cache. `write_indent` kept a `Vec<String>` indexed by width - a length test per
line, a `resize` on the first line at a new depth, a `" ".repeat` allocating one string per distinct
width, then a borrow of the entry to copy. The job is a slice of a run of spaces, so it is a
constant 64-column table sliced to width, with `repeat_n` for the indentation deeper than that which
no measured document reaches.

The claim is equivalence, not "faster and the tests passed", so the six-scan form is kept in the
test module as a reference and asked the same question as the fast path over every code point below
U+10000 in five positions - alone, leading, trailing, interior, beside a space - plus a stride
through the astral planes and the non-characters explicitly.
`needs_double_quoted_agrees_with_the_scan_for_every_code_point` caught the mistake in the first
version on its own: the tail read `!saw_high_byte || …`, which quoted every plain ASCII string, and
the reference disagreed on `"a b"` before any other test ran. Mutating `b == b':'` out of the byte
test reddens it and `prop_output_always_parses` together, so both the exactness and the round-trip
property are load-bearing. The indent table needs no test of its own: `" ".repeat(width)` and
`&SPACES[..width]` are the same bytes by construction, and every serialization test in the suite
reads them back.

Paired on rustc 1.97.1, three binaries from three commits of one branch, two repeats agreeing to the
instruction, so each step carries its own number:

| scenario | byte pass | + indent table | total |
|:--|--:|--:|--:|
| `serialize_anchors` | -28.01% | | |
| `serialize_medium` | -18.09% | -8.20% | **-24.81%** |
| `serialize_block` | | -10.82% | |
| `serialize_block_scalars` | -13.05% | | |
| `serialize_small` | | -29.81% | |
| `parse_medium` (control) | +0.00% | | |

The anchor row moves most under the byte pass because it holds the most scalars per output byte;
`serialize_small` moves most under the indent table because a small document is nearly all
indentation-per-line relative to its scalars. The control is the reader of the same fixture, which
calls neither function.

What is left in the same profile: `serialize_node_internal` at 13.77% is the dispatch itself, which
is the node model this file has already ruled out rewriting, and `memcmp` at 6.63% is the writer's
own `remaining`-width bookkeeping for line folding - both named so the next reader does not
rediscover them as low fruit.

---

## The attribution probe is a gate flag now (2026-10-14)

Every round above began from the same throwaway: build the release benchmark unstripped, run it
under callgrind twice (once with `--setup-only`), subtract, rank the functions. A third session
rebuilding that by hand is an argument for registering it, so it is
`scripts/ir_gate.py --profile SCENARIO` now — `--top N` widens the table, and a scenario name this
tree does not build is refused with the list it does build.

Measured on the tree that has all four rounds above in it:

```text
=== serialize_medium: loop 32,648,888 instr (full 33,131,280 - setup 482,392)
  18.32%     5,980,000  <pyrs_yaml_core::serializer::Serializer>::serialize_node_internal'2
  13.38%     4,368,000  pyrs_yaml_core::serializer::write_plain_scalar
  12.36%     4,034,000  <pyrs_yaml_core::serializer::Serializer>::write_scalar_for_key
  10.73%     3,504,000  <pyrs_yaml_core::serializer::Serializer>::write_mapping_pair'2
   5.83%     1,902,000  <pyrs_yaml_core::serializer::Serializer>::write_indent
```

(Names shortened here; the tool prints them with the object file in front, which is `???:` for
code the binary carries and `./malloc/…` for libc.)

Three things about reading it, since each is a way the scratch version got someone wrong:

- **`callgrind_annotate` parses the event file, and the tool parses that.** A hand-written record
  reader has to reimplement callgrind's compressed positions and call-cost attribution. The first
  cut of this one summed 8,854,254 instructions for a run whose own `summary:` line said
  4,506,831, and put nearly all of them under `?`. `tests/test_ir_profile.py` holds the row parser
  down to three cases: one row is one function, two rows for one name are one function, and no row
  is ever grouped with another by a pattern over its name.
- **Rows are printed as named, so one function can occupy two.** `write_mapping_pair` appears at
  10.73% and again at 3.36%, the second under callgrind's `'2` disambiguation of two symbols with
  the same name. They are not merged: merging by stripping a suffix is the `Hasher`-regex mistake
  in a different font — a question about names dressed as a measurement. Add them by hand if the
  question is about the function, and see what the row list says before doing it.
- **Symbols are not free to forget.** The gate's own binary is stripped, and a profile of a
  stripped binary reads `0x000000000006cc10'2` where it should read `write_plain_scalar` — a table
  of addresses, not an attribution. `--profile` sets `CARGO_PROFILE_RELEASE_STRIP=false` for itself,
  which changes the artifact's bytes and none of the measured code, so the numbers the baseline
  compares are still the same instructions.

Nothing here gates anything. `--profile` prints and exits 0; the pass/fail path is untouched, and
the environment the gate needs is the environment the profile needs (valgrind, `callgrind_annotate`,
and on this box WSL with a `PYO3_PYTHON` plus a link path to that interpreter's `lib`, because the
binding harness is built for the `to_python_*` scenarios whether or not anyone profiles one — a
profile that refuses to build for a linker flag is the gate's own failure, not the flag's).

## The floors were moved to the image that enforces them (2026-10-16)

`AGENTS.md` states the number that matters for this gate is not the tolerance but the distance
between a committed floor and what the tree actually costs. The 2026-10-14 refresh had found six
writer scenarios sitting 22–46% below their floors and four readers 4.7–11.2% below, which means the
gate had been waving through a regression the size of the whole TOML writer win. So the refresh
workflow was dispatched on `main` at `d1436aff` (run 38857499407) and its artifact committed
unedited: the toolchain came from the baseline (`rustc 1.97.1`, not the runner's `@stable`),
`--repeats 3`, callgrind, `generated_by.environment = github-runner ubuntu24, image 20261012.334.1`.

What moved, in counted instructions against the previous floors:

| Direction | Scenarios | Movement |
| --- | --- | --- |
| Floors tightened (code is faster than the floor claimed) | `serialize_anchors`, `serialize_medium`, `serialize_small`, `serialize_block` | −0.89% … −1.50% |
| Readers moved down | `from_json_medium`, `from_json5_medium`, `from_jsonc_medium`, `from_toml_medium` | −0.17% … −0.61% |
| Essentially flat | `serialize_block_scalars`, `parse_inline_merge`, `parse_medium`, `parse_anchors`, `parse_block`, `to_python_medium` | ±0.05% |
| Floors loosened | `to_python_anchors`, `parse_small`, `to_jsonc_medium`, `to_json_medium`, `to_python_small`, `to_json5_medium`, `stream_medium`, `to_toml_medium` | +0.12% … +0.49% |

The gate refused nothing — every movement is inside the 0.50% line, so no `accept_drift` name was
needed and none was passed. The four writer floors are now where the code is, which is the point of
the channel: a regression on those paths costs a red leg instead of eating 20% of slack.

**What a second run on the same image says.** The question left open above is whether an upward move
is the code or the runner, so the refresh was dispatched a second time on `main` (run 38858719775,
artifact `ir-baseline-34`, same pinned `rustc 1.97.1`, same image id `20261012.334.1`) and compared
scenario by scenario with the committed file. Every movement reproduced: `to_toml_medium` measured
+0.494% again and `stream_medium` +0.478% again — identical to three decimals, not merely close —
and the largest disagreement anywhere was 0.09 percentage points, on `to_python_small`, the
CPython-linked scenario whose term has since been named and pinned: the interpreter drew a fresh
hash seed for every measured process (`docs/dev/quality-ledger.md` (gj)). The sentence this replaces
pointed at an `AGENTS.md` claim about noise that no governance file contains.

So those two floors were not loosened by run-to-run noise; they sit where this image repeatably puts
them, and no code on those paths has moved since the 2026-10-14 numbers. Two consequences, stated as
properties of the gate rather than as comfort. Run-to-run noise on the enforcing image is measured
at ≤0.09pp, five times inside the 0.50% line, so a regression the size of the tolerance is still
caught. And what is left unexplained is the *image step* — `20261004.327.1` → `20261012.334.1` — on
the TOML writer and the event stream, which the `AGENTS.md` residual does not cover because that
figure was measured between hosts and this is between runner images: the same order of size, on the
two paths that moved. That is the honest amount being accepted here, recorded rather than rounded
away.

## The number a local run moves is the compiler, not the host (2026-10-16)

`AGENTS.md` closed the previous section with a residual it had only measured on two scenarios, and
said so. The whole table is now measured, and the answer is not the one the sentence predicted: the
term that moves a local reading is the toolchain, and it is an order of magnitude larger than the
tolerance.

One tree (`main` at `9dc5d5fb`, after the property tier merged), one host (WSL2,
`Linux 6.18.54.1-microsoft-standard-WSL2`), `--repeats 3`, the `MEASUREMENT_ENV` ISA pin unchanged —
only `RUSTUP_TOOLCHAIN` differs. Residual is the largest of three samples against the floor the
baseline commits, which was generated on the enforcing image by `rustc 1.97.1`.

| Compiler | Past the 0.50% line | Worst upward move | Largest downward move | Spread of the nineteen non-binding scenarios |
| --- | --- | --- | --- | --- |
| `rustc 1.97.1` (the baseline's pin) | 0 of 22 | `to_python_medium` +0.1639% | `to_python_anchors` −0.2335% | −0.0029% … +0.0012% |
| `rustc 1.99.0` (this box's default stable) | 5 of 22 | `serialize_medium` +9.0772% | — | `serialize_block_scalars` −7.7094% |

The pinned reading is the cross-host residual the earlier sentence was reaching for: a WSL machine
reproduces the enforcing image's floors to within 0.17%, and every scenario that does not link
CPython agrees to three decimal places — 0.001% at worst, against a tolerance of 0.50%. The three
binding scenarios are the only ones that move measurably, which is what `AGENTS.md` already says of
them.

The unpinned reading is a different phenomenon, and its shape is the evidence. Five scenarios are
past the line — `serialize_medium` +9.08%, `serialize_block` +8.86%, `serialize_small` +8.03%,
`serialize_anchors` +7.95%, `to_toml_medium` +3.55% — all YAML writers, and the sixth writer
scenario in the same family, `serialize_block_scalars`, moves *down* 7.71%. Host noise does not pick
a side like that; a code-generation change does. The binary digests say the same thing in the gate's
own vocabulary: `4fdb240b2e078d7c` on 1.99.0 against `2c4f20059f8d530c` on 1.97.1, so by
`binary_pairing`'s rule the movement is code, and these numbers pair with no other run's.

Read the other way, the pinned run is the stronger evidence, and it is worth stating precisely
because it looks like a problem. Its digests are `2c4f20059f8d530c` and `6718eaba88f2d3bf`, while
the baseline records `a7e3c34f86b81ae6` and `2972cd2647dedaad` — a WSL build is not the runner's
build, so by the same `binary_pairing` rule the gate would call these bytes different code. And yet
nineteen of the twenty-two scenarios reproduce the image's numbers to within ±0.003%, the worst
non-binding move being `parse_block` at −0.0029%. What makes a counted-instruction measurement
comparable, then, is the pinned ISA and the pinned compiler, not the exact build — which is why the
same digests disagreeing is not a reason to distrust the residual, and why the three scenarios that
*were* noisy (`to_python_*`) were noisy for a reason no governance file named until (gj): they link
CPython, and the interpreter chose a fresh hash seed per measured process. That term is pinned now,
and their within-run spread measures 0.0004-0.0009%.

Three consequences, all acted on rather than noted.

- **CI cannot be reddened by this.** `.github/workflows/ci.yml` reads
  `.ci/ir-baseline.json['toolchain']` and installs that compiler before it measures, and the refresh
  job does the same unless told otherwise (the defect `tests/test_ir_baseline_workflow.py` pins).
  The drift lives only on a machine whose default stable has moved past the pin.
- **A local run now says so.** `scripts/ir_gate.py` prints a toolchain note when the recorded banner
  and the current one disagree — naming both versions, the +9.1%/−7.7% pair above, and the
  `RUSTUP_TOOLCHAIN=<pin>` fix — the note also reaches `--report-only`, which used to print maxima
  with neither the warning nor the binary pairing, and the comparing path's `FAIL toolchain changed`
  reuses the same sentence instead of a second one. `tests/test_ir_gate_provenance.py` pins all of
  it, including that a same-version rebuild is *not* reported as a version change.
- **What shipped was not what was measured.** `publish.yml` built every wheel through
  `maturin-action`, whose `rust-toolchain` input defaults to current `stable`, so released artifacts
  came from a compiler no measurement described while the floors were drawn on the pinned one. The
  seven wheel builds, the `pyq` asset job and `ci.yml`'s release-wheel step now carry the recorded
  version — the last one so that the input is exercised on three OSes per pull request rather than
  for the first time on a tag push — and `tests/test_publish_toolchain.py` fails if pin and baseline
  disagree (drifting one step was measured to redden exactly that test, and the `sdist` step stays
  unpinned because an sdist compiles nothing). This is not a speed win: summed over all 22
  scenarios, `rustc 1.99.0` reads +0.198% against `1.97.1` — +6.35% across the `serialize_*` family,
  −0.004% everywhere else. The reason to pin is that a floor has to describe the artifact users
  install, so that a codegen move arrives as a decision with a number attached instead of shipping.

What this retires: the environment note used to tell a reader that a WSL number had once been +1.45%
off the runner's on `serialize_block_scalars`. Measured here, that scenario is 7.71% *below* the
floor on the newer compiler and 0.00005% below it on the pinned one, so the figure was never a host
term, and a note that quotes a number nobody can reproduce teaches the reader to distrust the wrong
variable. The sentence now points at this section instead of carrying a number of its own.

Not measured: no macOS or ARM reading — the table compares two compilers on one Linux userland, and
the glibc term is pinned by `MEASUREMENT_ENV`, not by this experiment. And the +0.1639% on
`to_python_medium` is one machine's largest of three; the CPython-linked scenarios were the wide
ones until (gj) named and pinned the reason — the interpreter's per-process hash seed — which this
file used to attribute to a sentence `AGENTS.md` does not in fact contain, and 0.060% of run-to-run
spread on `to_python_anchors` in the same sitting is in the same range. What "the host term is
pinned" does *not* cover is recorded in (gk): re-measuring this tree on the same pinned compiler
still leaves three scanner scenarios far off the image's floors, so a host reading is comparable to
the floors on the channel the seed touched, and not on those three.

## The floor a win leaves behind (2026-10-17)

`AGENTS.md` names the number that matters: the distance between a committed floor and what the tree
actually costs, not the pass/fail of one run. PR #465 moved that distance on purpose and left the
file behind. Identifying a table by walking to it rather than by the text its segments join to
dropped one allocation and one linear scan per header, and the TOML read path fell from 159,859,652
counted instructions to 150,441,866 on this box. The floor stayed at the old number, which is the
failure mode the 2026-10-14 refresh was written against: a gate holding six percent of borrowed room
waves through a regression as large as the win that created it.

Run 38993563233 (`Ir baseline refresh`, run #35) measured the whole tree where the gate runs: image
`20261012.334.1`, `rustc 1.97.1` read from the file rather than from `@stable`, callgrind,
`--repeats 3`, binary digests `a2c048c502d87d0b` and `817e763c4a90d81c`. No scenario needed
`accept_drift` — the guard refuses moves *up* past 0.5% and the largest one here is +0.048%.

| scenario | floor | now | move |
|:---------|------:|------:|-----:|
| `from_toml_medium` | 159,859,652 | 150,444,810 | **−5.889%** |
| `to_python_anchors` | 327,814,199 | 327,624,155 | −0.058% |
| `serialize_small` | 4,111,919 | 4,109,924 | −0.049% |
| `to_toml_medium` | 28,913,644 | 28,927,645 | +0.048% |
| `stream_medium` | 251,122,390 | 251,226,398 | +0.041% |
| `parse_inline_merge` | 363,892,231 | 364,008,341 | +0.032% |
| the other 17 | — | — | −0.020% … +0.006% |

Two readings of that table, and both are the point of the dispatch.

- **The image and the pinned box agree where it counts.** WSL with `RUSTUP_TOOLCHAIN=1.97.1` read
  `from_toml_medium` at 150,441,866; the enforcing image reads 150,444,810. That is 2,944
  instructions, 0.0020% apart, on the scenario that moved most in the file — which demonstrates the
  residual `AGENTS.md` says the ISA pinning closed, on the one case where a wrong answer would have
  been visible in a six-percent floor rather than in noise.
- **The writer moved, and not as a cost of the fix.** `to_toml_medium` loosens by 0.048%, ten times
  inside the tolerance, and PR #465's own changelog entry says why: the writer never consults the
  header bookkeeping the change removed. `stream_medium` and `parse_inline_merge` are YAML scenarios
  no commit in this window touched, so they are the image's own repeat-to-repeat spread. That is
  also why no second dispatch was asked for: the previous refresh ran one because two floors sat at
  +0.494% and +0.478%, within a whisker of the line, and nothing here is within an order of
  magnitude of it.

What this retires, and what it cannot see. The tolerance is 0.5% and the numbers are the image's
own, so a local reading six percent above `from_toml_medium` is now a regression report rather than
a cross-image artefact — the older framing, which absorbed the gap by carrying 2%, has no residue
left in this file. And the refreshed floor cannot see PR #467 at all: the bench document holds no
multi-line string, so the delimiter bound is priced by the corpus that judges it and by nothing
else.

## The TOML string path was not allocating twice either (2026-10-17)

2026-10-13 closed that idea for the JSON reader (`The string path was not allocating twice`). The
same idea came back for TOML a week later, because there the scanner genuinely does hand its text to
a second function: `parse_basic_string_content` builds a `String`, the value lands in a `CowTable`,
and `cow_table_to_node` puts it into the hub. The profile that reopened it is the one above - four
allocator rows are 28.54% of `from_toml_medium`'s 150,441,807 loop instructions - and "the second
copy is what the allocator measures" reads like the obvious explanation.

It is wrong, and two independent checks say so.

- **The code path already steals.** `scalar_projection` returns `text: String`, and the scalar
  branch of `toml_value_to_node` writes `value: text.into()` — `Arc<str>::from(String)`, the
  stealing conversion, whose precondition holds here for the same reason it held in JSON: the
  escape-free scanner produces its `String` with `self.text[start..end].to_string()`, so
  `capacity == len`. The escaped path builds by pushing and may carry slack, but that is glibc
  handing out a rounded-up block, not a duplicated buffer.
- **The control engine has no second function and pays the same.** `from_json_medium` was profiled
  the same pinned compiler, same `MEASUREMENT_ENV`, same setup subtraction, and its `value()`
  returns a `CustomNode` directly — no intermediate at all. Its four allocator rows are **29.49%**
  of 92,673,499 instructions against TOML's 28.54%. A share that does not change when the alleged
  duplication is absent was never measuring the duplication.

What the pair does isolate is on the key side, not the value side: TOML builds a `String` per key
segment in `parse_key_segment` (3.86%), inserts and clones those keys through
`IndexMap<String, TomlTable>` (two rows, 7.95% together), and copies 5.81% of its instructions in
`__memcpy_avx_unaligned_erms` where JSON copies 4.27%. That ~1.5pp gap is the honest size of the
remaining candidate, and it is a key-path change — sharing or interning segment names instead of
allocating one per level — not a string-value one.

Not measured: no key-path prototype, so the 1.5pp is read off two profiles rather than demonstrated
as recoverable. Recorded instead of done: no code changed. And the shape of the negative result is
worth keeping — the cheapest test of a "remove the copy" hypothesis is a sibling engine that has no
copy to remove, which costs one build and no refactor.

## The width of the band `to_python_small` is drawn from (2026-10-18)

The open question was how much of that scenario's near-tolerance reading is the host. Measured: one
binary, one session, one box (WSL2, `rustc 1.97.1`, the ISA pin the baseline records unchanged),
`--only to_python_small --repeats 9`, nine callgrind samples of identical bytes — 56,507,825,
56,539,957, 56,565,344, 56,567,233, 56,595,698, 56,612,182, 56,645,182, 56,656,063, 56,749,142.
Spread 0.425%, median 56,595,698, against a tolerance of 0.50%.

Two things follow, and both are about reading the guard rather than about the code.

- **The band nearly fills the line on this host.** Nine samples of a binary that the committed floor
  was drawn from keep a maximum 0.425% above their own minimum, so a change touching nothing on the
  `to_python` path can read up to roughly +0.25% from sampling alone. Three readings of this
  scenario today, on code whose difference from it is one predicate, came out +0.35%, +0.23% and
  +0.14% against the then-committed floor; that is the band this section is about, not a
  regression. It is why every claim in this file now compares against a base control built from the
  parent commit in the same session.
- **The repeat count is part of the comparison, not a precision knob.** Against the same
  `0cea1256` floor (`to_python_small` 56,613,975, generated as a largest-of-3 on the image), this
  binary read *below* the floor with three repeats (56,577,476, −0.06%) and *above* it with nine
  (56,749,142, +0.24%). `--repeats` larger than `generated_by.repeats` widens the kept maximum by
  construction; treating that as "a more accurate measurement of the same quantity" turns sampling
  into a regression.

What was deliberately not done. The tolerance stays 0.50%: the band belongs to one scenario on one
host, and the 2026-10-16 image comparison in this file found this same scenario agreeing to 0.09
percentage points, so the line is not the wrong part. No median-of-N for it either — the committed
value is the *largest* sample on purpose (`scripts/ir_gate.py`: "normal variation cannot exceed its
own envelope"), and a median floor would let ordinary variation trip the guard. No per-scenario
tolerance, which would be a second number to defend for every future change.

So: when a pull request moves `to_python_small` near the line, re-measure with `--repeats 3` against
a parent-commit control from the same session and read the median beside the max. A movement that
shows in both, on the pinned compiler, is code. One that shows only in the maximum of a bigger
sample is the band.

## Where one character wrote 0.6% of the YAML writer (2026-10-18)

PR #516 makes an empty plain scalar write the null text `~` instead of nothing. Its CI head was
green on `Test matrix (all legs)` and red on the instruction-count leg: against the committed floor
the four block-style writer scenarios read +0.73%, +0.77%, +1.02%, +0.72%, while `main` one commit
earlier read +0.10%, +0.15%, +0.36%, +0.06%. Two runs of one workflow on the enforcing image, so the
pair is directly comparable: `serialize_medium` 32,370,918 on trunk, 32,570,924 on the PR head. A
uniform +0.62…0.65% over four shapes of the same document is a per-scalar constant, and the fix's
whole diff is six lines inside the scalar writer.

Two hypotheses were tested and one survived. Both trees were built in one session on one host (WSL2,
`rustc 1.97.1`, the baseline's ISA pin), swapping only `crates/pyrs-yaml-core/src/serializer.rs`,
and every scenario below came back with `spread 0.000%` over its three samples.

| variant | `serialize_small` | `serialize_medium` |
|:--|--:|--:|
| trunk (`main`) | 4,139,896 | 32,370,862 |
| flag hoisted out of the short-circuit | — | 32,570,894 |
| same branch, no write in it (control) | 4,141,896 | 32,390,862 |
| `output.push('~')` before `write_scalar` | 4,165,896 | 32,570,893 |
| `output.push('~')` after `write_scalar` | 4,137,896 | 32,360,862 |
| the same, with the test hoisted into a boolean | 4,181,896 | 32,690,872 |

- **The hoisted `meta.tag()` was not it.** Reading the boxed fidelity slot for every scalar instead
  of only for the empty ones moved `serialize_medium` by 32 instructions of 32.5M. The cost was
  already there, so that "obvious" regression was a wrong guess written down and then refuted.
- **The branch is almost free.** The control keeps the nested condition and deletes the write:
  +20,000 of 32.4M, +0.06%.
- **The write's position is the whole cost.** The same one character, in the same never-taken
  branch, reads 32,570,893 before `write_scalar` and 32,360,862 after it — 210,000 instructions,
  0.65%, on a document that contains no empty scalar at all. `--profile serialize_medium` agrees:
  every function's self count is identical between the two trees except `serialize_node_internal`,
  which carries the inlined scalar writer, at 6,134,000 against 5,932,000.

The reading is the two positions; what follows is the story that fits them, not a probed fact about
LLVM. The buffer mutation ahead of the call lands between the writer's two statements that need to
know where the line starts and what the line holds, so the state it cached about `self.output` is
invalid at the point the call wants it; after the call the buffer was going to be mutated anyway, so
there is nothing left to invalidate. What is certain is the arithmetic, and it is enough: a fix that
changes no bytes on the measured fixtures can still cost 0.6% by where its write sits.

- **Computing the test once is the expensive choice.** Sharing the
  `value.is_empty() && matches!(style, Plain)` answer between the branch and the write — one boolean
  instead of asking the same question twice — reads 32,690,872, which is +1.01% over the version
  that asks it twice and +0.99% over a tree with no fix at all. Both readings are whole files rather
  than an isolated instruction pattern, so the mechanism is not pinned down; the number is. The
  shape the reuse-instinct prefers is the one that costs a percentage point, so the change ships the
  row above it, which asks the same question twice and pays nothing.

So the procedure for an instruction-count red on a correctness pull request is: pair the head
against its own trunk commit in one session, then bisect the shape with a no-op control before
believing any explanation — and before asking for a floor refresh. The two channels then agree: the
enforcing image read this head at `serialize_small` +0.05%, `serialize_medium` +0.12%,
`serialize_block` +0.33%, `serialize_block_scalars` +0.05%, `serialize_anchors` +0.03% — each
*below* the same image's reading of trunk one commit earlier (+0.10%, +0.15%, +0.36%, +0.06%) — and
its `serialize_medium` sample (32,360,953) is 86 instructions from the local one. The leg reported
success and no floor moved.

## The leg outside the verdict caught a shape change (2026-10-19)

`Python benchmarks` went red on trunk at `806811c5` (the merged #522) and stayed red on the next
head, while the three commits behind it (`d3fa3d92`, `5753b1e6`, `05a7446f`) all read green on the
same leg. Job 117935026394 names it in one line:
`tests/test_benchmark_api.py::test_to_toml - assert '[server]' in ...`.

The fixture is `[server]` + `[database]` + `[database.pool]` + fifty `[[items]]` blocks. #522 made a
root table that sits before a later root pair go inline, because keeping the mapping's order is what
TOML's own ordering rule leaves open; an array of tables reaches the writer as a sequence, so it
counts as that later pair, and `server` and `database` are emitted as inline tables. Nothing was
lost: on this tree `tomllib.loads(to_toml(from_toml(src)))` equals `tomllib.loads(src)` and the key
list matches the source's, so the failure is the assertion, which was describing the pre-#522
spelling rather than a property of the emitted document.

The assertion now asks the two things the writer promises, and the pair is measured rather than
assumed. Reloading the emission and comparing values catches a lost or altered key; comparing the
reloaded key list catches the order. Swapping two emitted root lines by hand keeps the first
comparison equal and flips the second, so neither assert is decoration and neither is a spelling
test.

It also lives in the normal suite now
(`tests/test_toml.py::TestToToml::test_benchmark_fixture_roundtrips_by_values_and_order`), not in
the benchmark body: this module is deselected unless `--codspeed` is passed, so a correctness
assertion here is invisible to every ordinary run and to the required check. The benchmark body is
back to the measured call, with the TOML-to-YAML setup outside it.

What was named rather than fixed here has since been ruled on. `Test matrix (all legs)` — the single
required check — reported `success` on both heads while this leg was red, because the benchmark legs
are not in the matrix, and the red arrived on trunk instead of stopping it. The ruling splits the
question in two. Benchmark *correctness* belongs to the normal suite, which is where the assertion
now lives, so the matrix already judges it and the benchmark channel is not asked to. The
instruction-count gate is now a leg of the matrix itself (`ci.yml`'s `ir` job), because it repeats
to ~0.001% on one binary and answers a merge question. The two CodSpeed jobs stay advisory in
`codspeed.yml`: their sub-10% noise makes a red worth investigating, not a merge decision.

## The gate's lower edge had no check at all (2026-10-19)

`ir_gate.py` compared one side of the baseline. A scenario that grew past the tolerance failed; a
floor that sat *above* what the tree now does was never a verdict, so however much the engine
outgrew a floor, the gate kept accepting that much headroom as normal. The 2026-10-14 refresh
measured what that accumulated into: six writer scenarios 22–46% below their floor and four readers
4.7–11.2% below, drift from four optimization PRs landing between refreshes. Re-tuning the
tolerance would have hidden it rather than fixed it — the missing comparison was the defect.

The comparing path now reads both edges. `floor_staleness` returns two lists: a scenario more than
2% below its committed floor is named as `FLOOR_STALE` in the log and the run continues, and more
than 5% below is a failure — a floor a twentieth out of date is no longer called a baseline. Growth
stays `drift_report`'s question, and a scenario missing from either side is skipped, because "not
in baseline" is already its own report. The failure text points at the dispatch job, not at
`--update`: a local re-baseline would move the floor to this host, which is the drift the rule
exists to stop.

The first dispatch under the rule (run 39242949404, artifact `ir-baseline-39`) needed no
`--accept-drift` and moved one scenario past the new line in each direction: `to_toml_medium`
28,939,722 → 26,879,029, −7.12%, which the old gate accepted and the new one would have failed;
`from_toml_medium` 135,689,929 → 131,611,879, −3.00%, the warning side. Every other scenario stayed
inside +0.33% upward (`serialize_block`, the largest move) and −0.005% downward, and the run
reported 0.000% spread for every scenario. The local WSL reading of `to_toml_medium` was
26,741,057 — 0.51% under the image's number in the cheaper direction, which is the pinned-compiler
pattern recorded above, not the number a floor takes. Both harness digests moved with the refresh
because trunk advanced since the previous floor; the committed `.ci/ir-baseline.json` is the
artifact byte for byte.

## The Rust bench set reached the ranking channel's blind spots (2026-10-19)

`codspeed.yml` runs two legs, and the Rust one has been green the whole time:
`cargo codspeed build && cargo codspeed run` over `crates/pyrs-yaml-core/benches/yaml_bench.rs`,
simulation mode, the report posted as `CodSpeed Performance Analysis`. Green is not the same as
covering. Everything the file measured was YAML text in, YAML or Python out; the two spokes this
engine has grown most since

- TOML and JSON - and the resolver that decides what a plain scalar *means*, were invisible to it,
so the channel that answers "is this slower than last week" had never seen them move.

Six benches close that: `resolve_yaml11_type_many` (30 texts over the 1.1 profile, the shapes the
profile was completed for) and `needs_quotes_over_scalars` (the quoting predicate every plain scalar
pays), then `from_toml_medium`/`to_toml_medium` and `from_json_medium`/`to_json_medium`. The two
writer benches read `pyrs_yaml_core::bench_inputs::MEDIUM_TOML` / `MEDIUM_JSON` and the same
`bench_inputs` documents the Ir harness measures, which is the standing rule (`AGENTS.md`: one copy
of a fixture, two harnesses), and their names match the Ir scenarios on purpose - a reader comparing
a wall-clock movement with an instruction-count movement should be looking at the same pair of
numbers. Measured on this box with `cargo bench --bench yaml_bench`: `from_toml_medium` 4.199 µs and
`to_toml_medium` 699.9 ns per call.

What this does not change is the enforcement layer. CodSpeed keeps ranking and the trend; the
zero-regression verdict stays with `.ci/ir-baseline.json` and `scripts/ir_gate.py`, because a
wall-clock number on shared runners is not reproducible below ~10% (the record is
`docs/dev/perf.md`'s own history and `AGENTS.md`). A new bench is a new thing the ranking channel
can see moving; it is not a new gate, and none of these six has a floor.

## The byte searches the readers own, moved sixteen at a time (2026-10-23)

"Use SIMD for character classification" is an objective line, not a measurement, and the line does
not say which scanner pays for it. The probe asked that first: for each format, the time of the
shipped byte loop over the very bytes it scans, next to the time of the whole parse of the same
document, on three shapes of input. The stop sets were copied from the readers, the scalar loop and
the vector were asserted equal before either was timed, and the numbers below are that program's
output (development host, release, median of eleven interleaved runs). The shares come from one run
before either change landed, and they move a few points between runs; the before/after times quoted
later are each from the run immediately before that crate's change.

| corpus | scan share of the parse | vector scan | shape |
| --- | --- | --- | --- |
| JSON `"…"` 5000 B × 400 | 25.7% | 10.5x faster | one long string per record |
| JSON CJK 5000 B × 400 | 33.6% | 11.1x faster | three-byte code points |
| JSON `"…"` 100 B × 4000 | 3.3% | 10.6x faster | a realistic log value |
| JSON5 `'…'` 5000 B × 400 | 34.6% | 8.7x faster | single-quoted dialect |
| TOML basic `"…"` 5000 B | 31.7% | 9.2x faster | one long string per key |
| TOML literal `'…'` 5000 B | 25.5% | 8.2x faster | no escape arm to stop on |
| TOML multiline `"""…"""` | 11.8% | 8.9x faster | the delimiter is a quote *run* |
| TOML bare-CR pre-scan, 2 MB | 20.3% | 12.8x faster | every input, any string length |

Three things in that table are decisions rather than results. **The share is a function of string
length, not of format**: the same JSON records scanned at 25.7% when each holds one 5000-byte string
and at 3.3% when each holds a 100-byte one, because the short-value parse is node construction and a
faster scan of the same bytes cannot touch it. So the win is real for the log and event-sourcing
shape the objective names and near-zero for a config file, which is exactly what the after-numbers
show: 2 MB of JSON 2.47 → 1.97 ms, 6 MB of CJK 6.17 → 5.27, JSON5 single-quoted 1.53 → 1.11, and
2 MB of TOML basic strings 2.36 → 0.99, literal 2.41 → 0.96, multiline 3.37 → 1.01 ms.

**The TOML bare-CR walk was the surprise, and it re-scopes the item.** `parse_ir` reads the whole
source once per call looking for a carriage return that must be part of a CRLF pair - a loop whose
comment argued it is cheap because CR is rare. Rarity of the answer is not cheapness of the search:
on 2 MB it was 20.3% of the parse and on a 0.45 MB config of short values still 3.6%, on every
input, whatever it contains. Vectorizing the search while leaving the pairing question in the caller
takes it to 1.6% and 0.3%, and it is the only part of this change that helps a short-value document
(2.98 → 2.54 ms).

**YAML was refuted before it was attempted.** granit owns the tokenizer, so the only question is
what is ours, and the probe answers it with the discriminator that separates the two readings: full
parse minus a receiver that only counts events. Over 2 MB of long plain scalars that is 0.78 ms of
1.72 ms, but a single `to_vec()` of the whole source costs 0.52 ms - two thirds of our share is the
`Arc<str>` copy running at memcpy speed. Over 50,000 nodes of 40 bytes the same subtraction is
24 ms of 39 ms against a 0.66 ms copy floor, which is per-node work that no byte count drives.
Neither is a scan, so there is nothing to vectorize on our side of the YAML read path, and
`crates/pyrs-yaml-core` is not in this change.

What enforces is unchanged: the wall clock above is the shape of the win, and the verdict on whether
a change made a hot path do more work stays with `.ci/ir-baseline.json`. Measured there on the
baseline's pinned compiler it moves down, not sideways - and this time on the image that enforces
it, because the change carries the refresh artifact: `from_toml_medium` 125,580,997 → 120,534,055
(−4.02%), `from_jsonc_medium` −3.69%, `from_json_medium` −3.55%, `from_json5_medium` −1.09%, each
the largest of three samples at 0.000% spread. No floor sits above the tree it now describes, so the
`FLOOR_STALE` warnings the engine change left are gone.

The shares above came from a throwaway probe, and that is why the probe's shape is now a committed
input: `bench_inputs::long_string_json` / `long_string_toml` / `long_string_yaml` generate one
record list with 256-character values, and `crates/pyrs-yaml-core/tests/ir_fixtures.rs` pins that
they stay letters-only, that all three mean the same list, and that the medium fixtures stay short.
Both channels read them at both ends of the two bridges. The ranking channel takes four benches -
2.599 µs and 3.499 µs a call to read, 950 ns and 1.174 µs to write, one run on this box - and the
enforcing channel takes four floors, which the next change's own refresh re-measured on the same
image: `from_json_long_strings` 112,664,928 → 112,987,111, `from_toml_long_strings` 109,254,466 →
109,535,451, `to_json_long_strings` 58,550,997 → 58,550,735, `to_toml_long_strings` 72,438,982 →
72,438,675. The writer halves exist because the gate asked for them: `scripts/quality_matrix.py`
reports a bridge numbered on one side only as `ir-bridge-unidirectional`, and the reader-only first
version was caught by exactly that. Per source byte of that one image run the long documents cost a
fifth of `from_json_medium` and an eighth of `from_toml_medium`, and their writers two fifths of the
medium twins: there the scan, not the node, is the cost centre.

**Growing the harness moves scenarios the harness never enters - measured twice.** The head before
this one carried the same engine code and the 22 committed scenarios; on the enforcing image its
`stream_medium` read 254,022,296 against a floor of 253,988,248, `+0.01%`. The head that landed
as PR #599 then added four scenarios - none of them the event path - and the same job read
`stream_medium` at 255,598,118, `+0.63%`, past the 0.50% line. One variable, one image, a leg that
fails on a scenario
the change never touches. This is the effect `tests/ir_fixtures.rs` opens with, now with a pair of
readings instead of a single surprise, and the answer is the documented one rather than a wider
line: the refresh dispatch, run with `accept_drift stream_medium` so that every other upward move
still refuses to be written. The second measurement adds the other half of the lesson - the effect
is not a law. Growing the same harness from 26 scenarios to 30 crossed the line for nobody: the
largest upward move was `from_toml_medium` at `+0.363%`, and `stream_medium` itself read
`−0.575%`, so its floor came down with the rest of them. What a scenario change owes the gate is the
run, not a prediction about it.

**What a local reading costs against these floors.** Both builds pinned the baseline's
`rustc 1.97.1` over identical sources, and the digests still differ (`9618e51ff9bde8a0` on this WSL
host, `dcf0cf4a439c5288` on the image), with readings 0.95% apart on `from_json_medium` and 0.76% on
`from_json_long_strings`. A digest difference is therefore a *build* difference, not proof that the
code moved, and a local run that reports a marginal failure against these floors is an environment
question - the enforcing reading is the image job's. The permanent guards are the four new numbers
plus the two differential tests in each crate, which ask every offset of a corpus holding every byte
value, and the run that passes the same 200 tests with the vector path compiled out - the path the
aarch64 wheels take.

**The outbound scan was the other half of that pillar, and it is a vector now.** (fi) recorded four
writer floors at the long-value shape precisely so this sentence could be measured rather than
proposed: `write_json_string` is 75.12% of `to_json_long_strings` and `push_quoted` 69.01% of
`to_toml_long_strings`, and the classification inside each is 84.5-97.9% of that. Three scans
changed. `write_json_string` and `write_single_quoted` ask one SSE2 pass for the first byte needing
a decision - {delimiter, `\`, `b < 0x20`}, const-generic so strict JSON, JSONC and JSON5 each
monomorphize their own delimiter - and copy the clean prefix before the char loop resumes verbatim;
`write_basic_body` had asked `any(forbidden_single_line)`, which answers a yes/no question and then
threw the walk away by re-running it, so it gained a position over {`"`, `\`, C0, DEL`} and the same
prefix copy, with DEL given its own equality mask because the saturating subtract that catches the
C0 range cannot see 0x7f at all. The floors those three moves take down, read on the enforcing
image by the refresh artifact this change carries, largest of three samples: 58,550,735 →
22,038,711, 72,438,675 → 28,254,711, 60,056,647 → 23,544,666 and 60,480,707 → 23,968,721. Two
boundaries travel with them. The medium shape is where a 16-byte question costs more than it saves,
and the image said so precisely: `to_json_medium` rose to 21,492,584, +0.52%, the one scenario the
dispatch had to be told to accept, while `to_jsonc_medium` (+0.48%) and `to_json5_medium` (+0.47%)
stayed inside the line, `to_toml_medium` came down 0.84%, and `parse_inline_merge` - which the
development host had put at +0.55%, a YAML scenario this change does not touch - read +0.00% there.
That pair of readings is the case for the channel: a host number can name a mover that does not
exist. And `to_json`'s same-session wall-clock pair does not separate at the timer's 100 ns
precision (1.199 µs → 399.9 ns on TOML, 943.5 → 431.1 on JSONC, 987.2 → 474.9 on JSON5, but
931 ns both ways on JSON): where the clock cannot see it, the claim is the floor.

The same surface now reaches the two dialect bridges this release also moved:
`bench_inputs::long_string_jsonc` (2,557 bytes - the same eight members with a line comment and a
block comment between them) and `long_string_json5` (2,550 bytes - single-quoted values, which is
the body the other instantiation of the vector scans, bare keys, a trailing comma on every member),
priced by `from_jsonc_long_strings`, `from_json5_long_strings`, `to_jsonc_long_strings` and
`to_json5_long_strings`. Recording the writer halves at this shape before anything proposes to
vectorize the outbound scan is deliberate: that change will then cost a number rather than an
argument. Both halves then say something about the host. The same head, built on this WSL box
(binaries `787413133303e751` and `eac9031ad74758f6`) and on the enforcing image
(`199deee5c61c85a1` and `0cf64e33e5957ca4`), put the two dialect *readers* `0.373%` and `0.525%`
apart while its two dialect *writers* came within `0.001%`: the half a local reading moves is the
allocation-heavy one, and a writer floor is nearly portable. Growing the scenario set is the act
measured above, and this growth asked for no `accept_drift` name at all.

## What the TOML read path spends its instructions on (2026-10-25)

`(fr)` published a loss: `rtoml` reads TOML about 1.9× faster than `load_toml` - 16.511 µs and
26.471 µs against 31.753 µs and 50.603 µs on the two fixtures. What that 32 µs is came from the
channel that decides such sentences, `scripts/ir_gate.py --profile from_toml_medium --top 14` on
this WSL host with `RUSTUP_TOOLCHAIN=1.97.1`: callgrind, loop 120,990,808 instructions (full
121,473,407 minus setup 482,599), functions printed as named.

| share | frame(s) | what it is |
|--:|:--|:--|
| 26.20% | `_int_free` 8.60, `_int_malloc` 7.35, `malloc` 6.32, `free` 3.93 | allocator traffic, not one function |
| 9.70% | `IndexMap<Arc<str>, pyrs_toml::TomlTable>::…` 4.88, 4.82 | the **intermediate** table tree |
| 6.53% | `cow_table_to_node` 4.14, `toml_value_to_node` 2.39 | walking that tree into the final one |
| 5.75% | `__memcpy_avx_unaligned_erms` | copying source runs |
| 4.86% | `parser::Parser::parse_key_segment` | the scanner's key path |
| 4.28% | `IndexMap<CustomNode, CustomNode>::…` | the final AST's own map |
| 5.31% | `<CustomNode as Hash>::hash` 2.75, `drop_glue::<CustomNode>` 2.56 | key hashing and teardown of the final AST |

The finding is the middle pair. `crates/pyrs-toml/src/lib.rs` opens by claiming "direct AST
construction, no intermediate value tree", and the profile says the crate builds a
`CowTable`/`TomlValue` tree first and converts it afterwards: about 16% of the scenario is a layer
the module docstring denies. That docstring is owed a correction, and it should travel with the
change that makes it true rather than being halved by itself.

Two levers, in the order their prices suggest:

- **Drop the intermediate tree**, parsing straight into `CustomNode`. Roughly 16% of instructions,
  on every TOML entry point at once - `load_toml`, `from_toml`, the TOML half of a round-trip -
  priced by the committed `from_toml_*` floors and guarded by the TOML parity, property and fuzz
  tiers. Not free: `parser.rs` is 2,941 lines with 84 references to the three intermediate types,
  and TOML's table-definition rules (dotted keys, arrays of tables, redefinition refusal) are
  exactly what a value tree makes easy.
- **Skip both trees for `load_toml`**, with a direct text-to-Python scanner behind a fall-back - the
  `json_fast::try_load` shape `(fq)` just extended to the third JSON loader. This is the only lever
  that can approach `rtoml`, because it also removes the final AST's 9.6% and the AST-to-Python walk
  with it. Its price is a second implementation of the dialect's value grammar, which `(fq)` could
  pay for a strict *subset* of a superset and this would have to pay for far more of the grammar.

What the arithmetic says: parity with `rtoml` needs about half of the present cost, so the first
lever is worth doing on its own merits and is not the fix for the published row. Nothing is
implemented here; the point of writing the number down is that the next attempt at this row starts
from 16% and 9.6% instead of from a guess.

## The tree floor, the reporting layer, and the reader that skipped both (2026-10-25)
<!-- landed: crates/pyrs-toml/src/read.rs::read_toml -->
<!-- landed: crates/pyrs-yaml/src/py/toml_direct.rs -->

`(fs)` decided to price both TOML levers in a throwaway clone before writing the real one, on this
host, Python 3.12.15, one release build, the `bench_snapshot.py` batch protocol (nine batches of at
least 0.1 s, fastest reported). Seams added for the measurement only: `proto_toml_tree` (scan plus
the intermediate tree, no output object) and `proto_toml_node` (scan, tree, `CustomNode`, no Python
walk), plus a sink that built a `dict` from the event stream.

| us | hot 1816 B (27 notes) | hot without notes | config 2165 B (no notes) |
|---|---:|---:|---:|
| scan + tree (floor) | 18.4 | 17.2 | 29.5 |
| `rtoml`, whole parse | 18.9 | 18.4 | 29.6 |
| + `CustomNode` | 27.6 | 24.4 | 45.9 |
| `load_toml` as it was | 30.9 | 31.9 | 55.9 |
| event stream to `dict`, reporting kept | 39.2 | 35.7 | 53.2 |
| event stream to `dict`, notes and positions stripped | 33.5 | 31.7 | 48.4 |

Three readings, in the order they decided things.

**The floor is the counterparty.** Scan plus intermediate tree costs 18.4 / 29.5 us against
`rtoml`'s whole parse at 18.9 / 29.6, and the ratio holds in nine of nine cells across three
re-runs (0.93 to 1.01). Everything the published row paid was therefore *after* the tree, so
neither "drop the tree" (which moves the scan's output shape) nor a faster encoder of the same
verdict was the target: the target was the node model and the walk off the node model.

**Reporting is not a fast path.** The event stream reached the same `dict` for every document, and
slower: 39.2 us against 30.9 on the note-heavy fixture, because the producer converts a byte offset
to line and column for every event and allocates an `Arc<str>` for every note a reader of values
discards. Removing both (the fourth and fifth rows) took the note penalty out and left a wash:
13 to 15 per cent faster on the container-heavy document with no notes at all, 8 to 15 per cent
slower on `hot`. A reporting layer reports at its own price, and that price is about what the AST
costs, so `(fs)` rejected the event route for `load_toml` and kept the shape it implied: read the IR
straight into Python, through no vocabulary.

**What landed, and what it is worth.** `pyrs_toml::read_toml` walks the IR into a caller's sink and
the binding's sink builds `dict` / `list` / scalars with the same three-step reading the node walk
applies (stated tag, then schema, then a registered type), so the scalar answer is shared rather
than restated. Paired in one process - the AST route kept alive by registering a handler for a tag
TOML never emits - the direct route measured 1.15 to 1.34x on `hot` and 1.31 to 1.35x on `config`.
The published snapshot reads 31.753 -> 29.115 us and 50.603 -> 44.654 us, with `rtoml` measured in
the same run at 16.511 -> 19.798 and 26.471 -> 30.527: the host drifted about 20 per cent between
the two runs, so the honest figure is the ratio to the in-run control, 1.92 -> 1.47 and
1.91 -> 1.46, about 1.31x on both fixtures.

The controls, because a fast path that never serves is the failure this family has been bitten by
twice. `crates/pyrs-toml/tests/value_reader.rs` flattens the tree and the new reader to the same
token list the stream comparison already uses and requires them equal over the seed corpus (17
seeds, both dialects, floor of 16 documents named in the assert), requires a refused document to
reach no value, and pins the shapes a corpus of round-trip text does not guarantee: `!!int` for
`0b1101`, `!!timestamp` and `!time` for the temporals, `Literal` for a multi-line string, and a key
named `n` or `y` arriving `DoubleQuoted` because the hub quotes any name a 1.1 reader would type.
`tests/test_toml.py::TestValueReader` then compares `load_toml` against the hub's own export and
reimport over eight documents, and proves by *value* that a registered tag handler still reaches the
load path - the guard routes to the AST model, and the rewrite is visible. No timing gate was added
for the guard: selecting the route per call means registering and unregistering a handler inside the
timed reference, which inflates that reference by about 0.7 per cent and would let a silent fallback
pass a floor set against it.

What was still open when that was written has since been measured rather than argued.
`scripts/ir_gate.py --profile`, on the `rustc 1.97.1` the baseline pins, ranks the loader route
`load_toml_medium` - loop 144,813,562 counted instructions, −0.054% against the committed floor of
144,891,981, so this reading and the gate describe the same build. Its two
`IndexMap<Arc<str>, TomlTable>` insertion rows carry 5,902,000 and 5,828,000: 8.10% of the loop. The
identical pair appears in the engine route `from_toml_medium` (loop 121,169,608) at 9.68%, which is
the evidence that the IR is one tree both consumers read rather than two costs. The second tree the
hub route builds - `cow_table_to_node` 5,008,000, `IndexMap<CustomNode, CustomNode>` 5,178,000,
`<CustomNode as Hash>::hash` 3,332,000, `drop_glue::<CustomNode>` 3,100,000, `toml_value_to_node`
2,892,000 - is a further 16.1% of `from_toml_medium` and is already absent from the loader route.

So not building the IR is worth a named 8% of `load_toml`, not the ~1.5x that still separates the
row from `rtoml`; the larger named rows in the loader's own profile are `parse_ir` (6.83%) and
`parse_key_segment` (4.06%), which a tree-free parser would still execute. The allocator rows
(`_int_free`, `malloc`, `_int_malloc`, `free`, about 15% together) are deliberately not charged to
the tree: that inference was tested and refuted once already, and the profiler prints functions
exactly as named rather than grouped by a pattern over a name. Rewriting `parser.rs`'s table
strategy to buy a named 8% is therefore not scheduled. The sentence in `crates/pyrs-toml/src/lib.rs`
that this paragraph used to leave owed has instead been corrected to state what the scanner does -
it builds one intermediate representation, and every consumer reads it once.

## The line index walked one character at a time, on the channel that had no choice (2026-10-26)
<!-- landed: crates/pyrs-event/src/lib.rs::newline_lanes -->
<!-- landed: crates/pyrs-event/src/lib.rs::zero_lanes -->

The refresh job's `profile` input (#633) answered its first question on the image that enforces
the floors, and the answer was a share rather than a hypothesis. Of `documents_long_strings`'
328,990,899 counted instructions, 170,394,000 (51.79%) sit in `<pyrs_event::LineTable>::line_col`
and 62,170,000 (18.90%) in `LineTable::new` - 70.69% of the scenario in two functions that walk a
document one character at a time. The short-value pair beside them reads 20,218,000 (21.09%) and
6,842,000 (7.14%) of 95,854,941. Both scenarios are line-delimited JSON, where the index is built
once and the cursor then advances for every event: neither job had a word-sized path, and together
they were the largest term the harness could name.

The change gives both one. `zero_lanes` marks the empty lanes of a `u64`, `newline_lanes` turns
that into the lanes holding `\n`, and the tail of the document stays a scalar walk. `line_col`
advances the column by eight while a word holds no byte at or above `0x80`, because eight
single-width characters are eight columns, and breaks out to the character walk the moment a word
could hold a multi-byte one. The line index needs no UTF-8 awareness at all: a `0x0A` byte cannot
appear inside a multi-byte sequence, since every continuation byte is at or above `0x80`.

Measured twice on one host, `rustc 1.97.1` (the compiler the baseline names), valgrind 3.22.0, the
harness binary `ir_gate-32fe5bfcd93ee66a` before (`a77bbab866280a80`) and after
(`8bd4697974c4e351`), so the two readings are the same target and different code:

| scenario | before (instr) | after (instr) | move | `line_col` | `LineTable::new` |
|---|---:|---:|---:|---|---|
| `documents_long_strings` | 308,642,512 | 126,840,512 | **-58.84%** | 170,394,000 → 29,618,000 | 62,170,000 → 21,144,000 |
| `documents_short_strings` | 75,506,559 | 64,952,559 | **-13.98%** | 20,218,000 → 13,060,000 | 6,842,000 → 3,446,000 |

Every number in the last two columns is a row the profiler printed, not arithmetic over the rows
that happened to appear. The pair accounts for the whole drop in each scenario - in the long case
232,564,000 became 50,762,000, which is exactly the 181,802,000 the loop lost - and their share of
it fell from 75.35% to 40.02%. `ir_gate::main` reads 30,001,920 in both runs, unchanged to the
instruction, which is why its percentage doubles from 9.72% to 23.65%: the harness's fixed cost did
not grow, the walk under it shrank. The short scenario moves a third as much because the pair owns
less of it - 27,060,000 of 75,506,559 (35.84%) before, against 75.35% of the long one - so the same
cut buys 10,554,000 instructions there rather than 181,802,000.

**The over-report is the bug, and the property found it.** A zero-lane test of the shape
`(w - 0x0101..) & ~w & 0x8080..` cannot be trusted alone: a borrow out of a zero lane turns the
lane above it into `0xFF`, so `0x01` followed by `0x00` reads as a break. `newline_lanes` therefore
checks each candidate against the byte it claims, and that check runs only on a word that has a
candidate - every word without one still costs the single test. The differential property
`a_word_at_a_time_sweep_matches_a_rescan_byte_for_byte` compares `starts` and `line_col` against a
byte-by-byte rescan at every position of an arbitrary document; its first counterexample was
`"a0a\0\n\u{b}¡"`, a phantom line start the source never wrote. Without the byte check this change
would have shipped an index that invents lines.

**The floors moved with the code, on the image that enforces them.** Run #64 of "Ir baseline
refresh" measured this head on `ubuntu24` image `20261023.334.3` with the pinned `rustc 1.97.1` and
wrote the file; no local `--update` was used, which is the point of the channel. The committed
floors move `documents_long_strings` 328,990,934 → 147,188,912 (**-55.26%**) and
`documents_short_strings` 95,854,968 → 85,300,932 (**-11.01%**), and two scenarios no local profile
had named move with them - `stream_json_medium` **-14.90%** and `stream_json_stop` **-27.17%** -
because the same line index serves the whole JSON stream route, not only the NDJSON one. The other
38 scenarios move by 0.0051% or less, so the instrument is still measuring the same thing it
measured before.

**What the image's own attribution says.** The refresh's `profile` input named
`documents_long_strings`, and its after-table has `line_col` at 29,618,000 and `LineTable::new` at
21,144,000 - the same two numbers the local table printed, to the instruction, on a binary whose
engine rows match. The loops still differ: 147,188,890 there against 126,840,512 here, and
17,341,472 of that 20,348,378 gap is one libc row, `__memcpy_avx_unaligned_erms` (21,567,856 on the
image against 4,226,384 here). That is the shape of (gk)'s residual, now measured on the enforcing
image rather than argued from off it: the engine is not what disagrees between the two hosts.

**Not claimed.** (gk)'s host residual is narrowed, not closed - the third divergent scenario,
`from_json5_medium`, was not profiled on this head, so its +1.20% term stays where (gk) left it.
What this section does settle is (gk)'s question about where the work sits: the term that dominated
the long scenario is no longer the term that dominates it, and the rows above it now are the
harness's own `main`, libc's `__memcpy_avx_unaligned_erms` and the parser.
