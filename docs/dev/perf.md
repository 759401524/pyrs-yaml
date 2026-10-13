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
**128**; the `no_std` CI job found that by reddening on a tripwire that pinned 144 alone.
The test `node_layout_is_part_of_the_hot_path_contract` pins all four numbers, so the table above
cannot rot silently. The paired run (2,000 iterations minus `--setup-only`, `m197.bin` vs the
retuned binary):

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
