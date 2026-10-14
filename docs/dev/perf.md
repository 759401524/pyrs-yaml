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

Reproducing it where the gate itself cannot run: `scripts/ir_gate.py` also builds the binding
harness, and this box's WSL has no `libpython3.12` to link it, so measure the engine harness alone -
`CARGO_PROFILE_RELEASE_STRIP=false cargo bench -p pyrs-yaml-core --bench ir_gate --features ir-gate
--no-run` (the profile's `strip = true` is what leaves `callgrind_annotate` reporting `???:0x…`),
then `valgrind --tool=callgrind <that binary> from_json_medium`.

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
