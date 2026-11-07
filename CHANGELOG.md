# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- **A refusal names the format that refused it** — `ParseError::Syntax` is built by the YAML, JSON,
  TOML and schema readers alike, and its `Display` prefixed every message with `YAML parse error:`.
  A TOML refusal therefore read `YAML parse error: TOML parse error: unterminated array …`, the four
  JSON doors called a JSON refusal a YAML one, a schema grammar refusal was labelled a YAML parse
  error although the YAML parsed, and `register_schema` carried the prefix twice on a YAML
  pass-through. The label now belongs to whoever writes the message. Measured over 53 door answers
  before and after: the 36 YAML ones are byte-identical and 16 change, each a JSON, TOML or schema
  door losing a label that was never its own (`docs/dev/quality-ledger.md` (kc)).
- **The editing pages named three refusals that do not happen** — a write through an alias was
  documented as raising `cannot-edit-alias`, and `set()` as answering `anchor-is-referenced`;
  measured on the shipped build, `set`, `delete` and `rename` all rewrite `use: *x`, and no `set`
  route reaches `cannot-edit-alias` at all — that token lives on the delete, rename, move and
  metadata paths. The alias *parent* is still refused, as are the twelve metadata verbs on an alias
  node. `api/exceptions.md` now lists the fifteen reachable reason tokens with the call that answers
  each, and `validate()`'s two dependencies are stated where users meet them: nothing this project
  ships installs `jsonschema`, and a schema passed as text is read by `json.loads` first, so a
  YAML-spelled schema raises `json.decoder.JSONDecodeError`.
- **A note under a tag header that opens the document settles on the first emission** — a note that
  cannot ride its line is written above the header the reader reports it on, and the walk that finds
  that header asked for the break *before* the header line, so a header starting the document was
  never crossed: the note stayed in the gap, where the re-read files it on the root, and the next
  emission moved it above the header. `!base:` over an unindented sequence measured
  `!base: ` / `# fuzz 47` / `- etss: &a0 !s: ~  # fuzz 47` against a second round that put the note
  line first; crash-92f3fc65 and crash-e392c402 (81 and 82 bytes) and their 29-byte reduction now
  come back identical in one round, `fuzz/seeds/edit_yaml` replays 47 of 47 clean and
  `cargo nextest run --all` 895 passed. The batch's third input is a different placement — an inline
  note the reader homes one node up — and stays open as issue #732.
- **A note lifted off a nested `?` marker no longer lands in a header's gap** — `hoist_marker_note`
  brings a note that rides an inner marker up to the marker's level, which is where the reader
  reports it, and the lifted line was then appended at the writer's tail. Where that marker sits in
  the value of a pair headed `&a !tag:`, the tail is the gap under the header; granit spans the node
  through the break that ends that line, so the re-read filed the note one owner higher and the
  second emission lifted it above the header. The gap rule `note_line_above` already applies is now
  the rule for that write too, taken after the pending tag-only line is closed. Replaying every
  input issues #732 and #733 name moves 17 `yaml_roundtrip` inputs from `serialization not
  idempotent` to a fixed point (44 bytes the smallest) and the audited set from 36 failing to 19;
  `yaml_roundtrip` replays 128 of 128 seeds, `edit_yaml` 47 of 47, and `cargo nextest run --all`
  896 passed. The other 19 stay open on those issues.
- **A block scalar's chomping indicator survives a note line above it** — a block scalar with an
  empty body was read through the line in front of it: the header search backs up from the content
  to the last non-blank line, found the note instead of a header, answered Clip and dropped the
  author's `+`. The document was not a fixed point in one round — `# Irrr` over `>+4` emitted `>+`
  and then `>` — so the indicator disappeared from the text the engine wrote. The header scan now
  falls back to the sigil's own line, and only after the backup above finds no header, which leaves
  a content line that merely starts with `>` on the normal path. Measured: the two fuzz inputs that
  drew this (`crash-aa8a0447`, `crash-95b66309`) leave the failing set, `yaml_roundtrip` 9 to 7 with
  `edit_yaml` unchanged at 14, and both are archived as seeds — corpus 284 to 286, `seed_reach`
  still 11 unreachable of the 242 seeds its four round-trip targets then held (the gate now walks
  six targets and reports 10 of 259).
- **A 1000-deep TOML document converts with a typed error, not a Rust panic** — `from_toml` reads a
  dotted-key chain exactly 1000 levels deep, because that is the reader's budget, and then asks the
  YAML writer for the text form, which refuses at the same number. The refusal used to land in
  `serializer::to_yaml`'s `expect`, so Python saw `PanicException: serialization failed:
  MaxDepthExceeded(DepthError(1000))` — not an `Exception`, so it passed straight through every
  `except Exception` in the language. The four conversions (`from_json`, `from_jsonc`,
  `from_json5`, `from_toml`) now share one fallible path that answers `YamlMaxDepthError` under the
  existing `max-depth-exceeded` key; the JSON dialects never showed the symptom because their reader
  stops one level earlier. Measured: depth 999 still emits 1,000,001 bytes through all four, and
  three new `tests/test_max_depth.py` cases pin the boundary — routing the helper back through
  `to_yaml` reddens one with exactly that panic. What this does not fix: the YAML reader runs
  granit's two default nesting limits of 255 (`block_nesting_limit` and `flow_nesting_limit`,
  both counted per container) while every other depth budget the library advertises is 1000, so a
  conversion past the 255th container still hands back text its own reader refuses
  (`docs/dev/quality-ledger.md` (jq)).
- **The schema `type` refusal named five spellings and accepted nine** — `YamlTypeKind::from_name`
  answers to `boolean`, `integer`, `double` and `string` as well as the five canonical names, so an
  author who typed `type: integer` was served, while one who typed `type: number` was told the
  valid set was `null, bool, int, float, str` and sent looking for a synonym that had never been
  missing. The refusal now appends `(aliases accepted: boolean, integer, double, string)`; the
  accepted set is unchanged, and the guide's five-row table is left as it is pending a ruling on
  whether the aliases are surface or accident (`docs/dev/quality-ledger.md` (jz)). Measured:
  `register_schema` accepts all four aliases and refuses `number` and `Int`; the new named test
  asserts both the alias list in the message and the four spellings as accepted, so dropping either
  half reddens the run.

### Changed

- **The docs toolchain moved to the newest `zensical` its range allows** — `zensical` 0.0.72 →
  0.0.74, with `zensical-ui` 0.0.41 arriving beside it, under an unchanged `zensical>=0.0.69`
  constraint. A green `Docs` run is not evidence about a page: the locked 0.0.56 published pages
  whose front matter is not a mapping while 0.0.69 fails the build outright, so the bump is judged
  by `scripts/check_doc_metadata.py` (165 pages carry metadata the generator can read) and by a
  strict four-locale build.

### Internal

- **`edit_yaml` counts what a name answers instead of asking whether the name died** — a rename or a
  move acts on the one pair the lookup's identity pass resolves, and where the document spells that
  key text twice the sibling goes on answering the name through the pass the lookup falls back to:
  "the old name is gone" is not a claim the emission makes. That was the open class of the three
  remaining 2026-11-02 `edit_yaml` findings (issue #732), and `!base k: 1` beside `k: 2` reproduces
  it in 19 bytes. The tier now asks `duplicate_fold_count` — the sibling counter the `on_duplicate`
  policy already uses — for exactly one pair fewer under the name each verb left, and `move` states
  that claim for itself instead of inheriting `delete`'s `all` guarantee. Asking "no pair moved"
  instead reddens 15 of the corpus's 45 `edit_yaml` seeds, so the count carries well past the two
  reductions written for it; the engine answer is pinned by
  `rename_moves_one_pair_of_a_name_the_document_spells_twice` and the move beside it. The three
  inputs and the two reductions join that corpus, which replays clean under `-runs=0`, and the
  matrix reads 931 Rust `#[test]` over 302 seeds.

- **A seed its target refuses is now counted instead of credited** — six fuzz targets open their
  body with a typed refusal as a pass (`yaml_roundtrip` and `edit_yaml` on `parse(Schema::Core)`,
  `json_roundtrip` and `parse_json` on the three JSON dialects, `toml_roundtrip` and `parse_toml` on
  both TOML gates), so an archived input the reader refuses replays clean under `-runs=0` while the
  writer promise it was filed for is never asked, and the corpus total still counts the file.
  `crates/pyrs-yaml-core/tests/seed_reach.rs` asks each target its own engine call and reddens when
  `.ci/seed-reach.txt` and the measurement disagree in either direction; over the 259 seeds in those
  six directories it measures 10 unreachable, and `scripts/quality_matrix.py` derives a
  `seed-unreachable:<target>` hole for every target the list names — three now, all registered with
  the state that removes them. `parse_json` and `parse_toml` joined that population after this entry
  was first written: `parse_json.rs:36` and `parse_toml.rs:57` `continue` past a refusal with every
  assertion behind them, the shape the four round-trip targets have rather than the "asserts on the
  refusal itself" the exclusion claimed — and that claim had also sold the remedy short, since
  moving a refused seed into `fuzz/seeds/parse_toml/` relocates the blind spot rather than closing
  it. The inputs stay in the corpus: 5 of the 10 answer `max depth exceeded (max=1000)`, which is
  PR #694's own postcondition, so they protect that refusal and are credited with nothing beyond it.
  One registry line has since been re-mined out: `shape-literal-and-folded` was `l = |`, illegal in
  both profiles, and rewritten as a multi-line literal string beside a basic one it parses, replays
  clean under `-runs=0` and reaches the writer - so `seed-unreachable:toml_roundtrip` now names two
  seeds of three (ledger (jo)). `edit_yaml` is measured one precondition short of its harness on
  purpose — the target also wants a NUL separator, a complete two-byte record, a non-empty path list
  and one verb that answers ok — and the gap is sized rather than assumed: replayed against the
  target instrumented to panic at each precondition, 62 of its 62 seeds reached an assertion, so the
  weaker predicate and the real one agree on today's corpus (`docs/dev/quality-ledger.md` (jm)). Two
  more registry lines went the same way: `cr-only-break-note-ownership` was refused for its `- :`
  empty key rather than its CR breaks, and `special-keys` for a bare `@literal`; re-mined - into a
  sequence whose note line falls between two entries across lone CRs, and into the same four scalar
  spellings with the reserved one quoted - each now reaches the assertion and replays clean under
  `-runs=0`. The plain `@`-leading spelling the second file used to ask is unreachable from legal
  YAML at all and stays pinned by the Rust test that builds the node directly. The reading today is
  Two registry lines have since closed outright. `shape-comment-only-document` was widened to a
  comment *before* a value, which is the half the pairing test already asked of literals and the
  half the target can actually read; `former-crash-comment-feff` lost the bytes that only made it
  refuse and kept the BOM-after-comment adjacency it was filed for. Each replays clean under
  `-runs=0` on its own target and each now reaches an assertion, so the ids are gone from
  `.ci/quality-holes.json` rather than re-worded. The reading today is 10 of 259
  (`docs/dev/quality-ledger.md` (jp), (jt)).

- **Twenty-three inputs fuzz found and nothing kept are now in the corpus** — the 2026-11-02 and
  2026-11-03 findings that replay clean today (15 `edit_yaml`, 8 `yaml_roundtrip`, 45 to 234 bytes)
  were never archived, and an absent seed protects nothing: the matrix counts files, so the gap was
  invisible rather than measured. Each was filed verbatim under
  `fuzz/seeds/<target>/former-crash-<short>.seed` and re-asked of its own target's engine —
  `seed_reach` lists none of the twenty-three among its 10 unreachable of 259 walked, so all of
  them reach the assertion they were archived for instead of passing on a refusal, and the replay
  tier runs 198 of 198 with zero artifacts. The 23 findings that still fail stay open on their
  issues, including the
  `edit_yaml` container-note shape that needs a placement ruling.

- **A number's source spelling now has a named limit in the export spokes** — a tree reaching
  `to_json5` or `to_toml` kept the text on the first round and canonicalized on the second:
  `k: 0Xeeeeeeeeeeeeeeeeeeeeeeee4e6` (26 hex digits, which the core schema resolves as a *string*)
  exported as the bare token `{"k":0Xeeeeeeeeeeeeeeeeeeeeeeee4e6}`, so the export changed its type,
  and `k: +83e6` wrote `k = +83e6` and then `k = 83e6`.
  `crates/pyrs-yaml-core/tests/exports_settle.rs` pins both shapes as the state they are, because
  neither writer can answer the question itself: measured on this tree, `from_json5` and `parse`
  hand back the same `Scalar { style: Plain, tag: None, fidelity: None }` for a JSON5 number and for
  a YAML string that merely looks like one. Carrying the source text in the node's `fidelity` slot,
  canonicalizing always, or making it per call is an open decision, recorded with its measurements
  in `docs/dev/quality-ledger.md` (je).

- **The merge-chain note collision is pinned by a named test** — a `<<` whose merge source holds
  nothing but another `<<` reads two different ways depending on whether the outer note rides its
  line: `<<: #a` over `  <<: ":!"` yields one level, `# a` above `<<:` yields two, for the same
  tree. Both spellings settle in one round, so neither the settlement oracle nor a property tier
  could see it. `test_note_slot_currently_changes_merge_consumption` records both current answers
  and names the assertion to collapse once the rule is chosen; no behavior changed here. Two shipped
  rules collide in it — `collect_inline_pairs` contributes a source's merge keys while the v0.16.0
  "yields nothing, so an ordinary key" rule keeps them, and `is_merge_key` states that a comment is
  metadata, not identity — so which reading is correct is an open decision, written up with its
  measurements in `docs/dev/quality-ledger.md` (jg).

- **The writer's reserved-indicator rule is now asked end to end, not only decided** —
  `serializer.rs::leading_indicator` answers whether a text can open a plain token, and
  `needs_double_quoted_agrees_with_the_scan_for_every_code_point` pins that decision against an
  independent restatement of all 17 characters. Nothing asked the consequence: that a scalar whose
  text begins with one of them is emitted so that it re-reads as the same text, on the key side and
  the value side, in one round — the claim `fuzz/seeds/yaml_roundtrip/special-keys.seed` used to
  stand for and no longer carries, since its bare `@literal` makes the whole document a refusal
  (`.ci/seed-reach.txt` names it).
  `a_plain_scalar_leading_with_a_reserved_indicator_survives_the_round_trip` walks 17 × 2 sides: 34
  of 34 hold today, and the test is priced by mutation — deleting `b'@'`, `b'*'`, `b'|'`, `b'>'` or
  the backtick alternative from the rule reddens it, naming the emitted text and the reader's
  message. Deleting `b'?'` does not, and that is a boundary rather than a gap: `?x` is a legal plain
  scalar to our own reader, so that member is interop conformance, not our round-trip contract.

- **A comment-only JSON document is now refused by name, in all three dialects** — the corpus input
  that pinned it (`fuzz/seeds/json_roundtrip/shape-comment-only-document.seed`) is skipped by it
s own
  target (`json_roundtrip` `continue`s when no dialect accepts the text) and by the JSON corpus gate
  that re-asks every seed, so nothing asserted the rule while the corpus still counted the file.
  Auditing the 15 registry lines then unreachable for what defends their class left this as the only
  naked one: the 5 depth-guard inputs are asserted by the typed `MaxDepthExceeded` refusal PR #694
  shipped, the CR-break and reserved-indicator classes by named tests in
  `settles_in_one_round.rs`, the TOML multi-line shapes by four reader and writer tests, and the BOM
  boundary in both directions. `comment_only_documents_are_refused_by_every_dialect` pairs the
  claim — `// nothing here`, `// c`, `/* x */` and `# hash` then `// c` are refused by JSON, JSONC
  and JSON5, while the same comment before a value reads in the two comment dialects and stays
  refused by strict JSON (measured: 12 refusals, 6 acceptances). The other 10 lines keep their
  registry entries: they record lost corpus credit, not lost defence.

- **The doc wrapper's CJK safety is measured on the shipped pages, not only on injected ones** —
  `--fix` reflows prose to 100 display columns and its typography rules were fired by synthetic
  injections in `tests/test_doc_wrapping_gate.py`, while nothing asked the same question of the
  11,000 lines that ship. The claim carried from earlier sessions — that `--fix` glues Hangul and
  splits katakana, so localized pages must be wrapped by hand — traced to a comment in a scratch
  file, not to the tool: `join_lines` asks each boundary whether a space there is residue and glues
  only where it is not, and `_is_hangul` exists so Korean breaks land on word spaces.
  `test_reflowing_the_shipped_localized_pages_changes_only_line_breaks` joins real paragraphs of the
  three localized changelogs into over-long lines, runs the function `fix()` writes with, and holds
  five invariants: text preserved modulo whitespace, backticks preserved, no Korean word cut, zero
  residue after the pass by the checker's own summed count, and a second pass that moves nothing.
  3 of 3 pages pass; neutralizing `strip_residue` reddens two, letting Hangul break reddens a third,
  the tool restored byte-for-byte after. Getting the measurement right took two traps out: the
  checker reports one finding per *line* with a gap count inside it, so comparing `len()` measures
  lines rather than typography.

- **The prose-width gate now sees the lines it used to excuse** — the exemption asked whether some
  legal cut would leave a line's *first* piece inside 100 display columns, so a line opening with a
  wider code span or link answered no however much prose followed it, and `_wrap_line` then put the
  whole remainder on that same line and called it done. Measured here: 18 unreported lines past the
  limit on seven of the eleven governed pages, the widest a 1,410-column ledger paragraph with 169
  legal breaks that `--fix` returned unchanged. The fixer now emits the unavoidable token and wraps
  what follows, and the check concedes only a line that *is* one atomic token — so the two halves
  still agree, and anything reported is something the fix shortens. The reflow moved 13,024 to
  13,083 ledger lines and one changelog line, proved break-only by text modulo whitespace and by
  backtick, heading and table-row counts; 19 lines stay excused at 101 to 153 columns, each one
  span.

- **The 256-1000 depth band got named guards, because no tier could see it** — behind the conversion
  fix, the same sweep measured the rest: 27 conversion-shaped callables against 1000-deep documents
  answer with typed errors and no `PanicException` at all, while a tree grown by `set` refuses to
  pass depth 999 and then exports 1,000,100 bytes of YAML that this library's own reader refuses at
  line 257 (`recursion limit exceeded`). Nothing in the suite could have found that: the property
  generator is `prop_recursive(6, 128, 8)`, the round-trip fuzz targets re-read with each format's
  own 1000-level budget, and YAML fuzz inputs have already passed granit's 255, which counts
  containers in either style. Two guards now pin the band —
  `crates/pyrs-yaml-core/tests/depth_budgets.rs` (254 levels round-trips in one pass, 300 emits and
  then refuses at the read, the writer answers in type at its own cap, and the reader stops at 255
  containers whichever style spells them) and `tests/test_binding_serializer_gate.py` (no production
  binding path may call the infallible `serializer::to_yaml`, with a use-count check so an empty
  scan cannot read as a pass); each was priced by mutation, one red test each. Whether the budgets
  should agree is still an open decision (`docs/dev/quality-ledger.md` (jr)).

- **A fuzz finding now closes on a reached assertion, not on a refusal** — the report step
  wrote "close this issue when the seeds replay clean", and ten of the 259 seeds its six reaching
  targets walk replay clean *because* their target refuses the input, so an issue could be closed
  with no promise ever asked. The sentence now demands both halves, and
  `tests/test_fuzz_file_issues.py::test_the_closure_criterion_demands_that_the_seed_reaches` keeps
  it there: dropping the reach clause reddens that one test and leaves the other sixteen green (17
  passed once restored). Measured against the strengthened criterion the two findings open today
  both satisfy it - #732's three `edit_yaml` inputs and #733's two `yaml_roundtrip` inputs are all
  archived as seeds, none appears among the ten unreachable lines, and each replayed at `rc=0` under
  `-runs=0` against its own target. `QUALITY_MATRIX.md`'s fuzz row stated the weaker claim and now
  states the qualified one (`docs/dev/quality-ledger.md` (ju)).

- **`parse_yaml` now asks the two YAML entry points to refuse alike** — the target called `parse`
  and `parse_all` as two `let _ =` statements, so its only claim was "no input panics the parser",
  while `load_ast`'s own comment says the shared helper exists so the single-document door and the
  stream door cannot drift. What is asserted now: a stream refusal carries the identical message at
  both doors, at most one accepted document means the single door accepts, and two or more must
  answer `ParseError::SingleDocumentExpected`. Five named cases in
  `crates/pyrs-yaml-core/tests/reader_door_agreement.rs` hold each branch, and they caught the first
  version's mistake on its first run — the prose token `multi-document-input` is the binding's
  mapping (`py/mod.rs:51`), not the engine's rendering (`pyrs-ast/src/error.rs:55`). Measured on the
  file as shipped (sha `3165fb959464`): the 144 archived YAML inputs replay as 145 runs with no
  panic, and the mutated expectation crashes on the 18-byte `---\na: 1\n---\nb: 2\n` - an input the
  archive did not hold until this change, which is why the oracle's first pricing replayed clean on
  both the shipped and the mutated build. Discovery totalled 88,704 runs across 8 jobs with an empty
  artifact tree, and the two thinnest targets 14,786 and 13,165 runs with the same. Because the
  pull-request tier replays seeds with `-runs=0`, this is a merge gate
  (`docs/dev/quality-ledger.md` (jv)).

- **A JSON5 non-finite document now walks the events target's refusal arm** — `json_events_replay`
  asserts the two walks agree, and `fuzz/fuzz_targets/json_events_replay.rs:105` carries
  `(Err(_), Err(_)) => {}` for the case where both accept the document and the strict writer refuses
  it, with a comment naming JSON5 infinity as that case. No archived input reached it: an
  instrumented `-runs=0` replay of all 21 seeds printed a writable verdict for every document both
  walks accepted. The 16 bytes of `{"a": Infinity}` are filed as
  `fuzz/seeds/json_events_replay/shape-json5-non-finite.seed`, so the pull-request tier walks the
  refusal rather than only describing it, at 22 seeds and 23 runs clean. The same probe priced the
  claims that were already sound — the note-slot equality is reached non-vacuously by 12 seeds, with
  leading- and inline-owned notes both present — and left the asymmetric-writability arm at `:106`
  named as unwalkable by construction, since both trees come from one document
  (`docs/dev/quality-ledger.md` (jw)).

- **`parse_schema`'s corpus walks the grammar now** — the target holds one claim (no panic on any
  input) and held one seed, so new inputs buy no assertion credit; what they buy is the discovery
  tier's starting shapes and a record of which refusals exist. Fourteen inputs joined
  `fuzz/seeds/parse_schema/`, one per refusal site — thirteen, derived from the door's own code —
  plus one accepted alias type, each measured through the shipped extension first: a non-mapping
  root, the YAML pass-through, the five non-scalar positions, a `rules:` that is not a sequence,
  a rule that is not a mapping, an unknown `type:` name, a rule without `pattern`, one without
  `type`, and a pattern that will not compile. The directory holds 15 and a direct `-runs=0`
  replay of those files alone reads `Done 17 runs` at rc=0 with an empty artifact tree; the
  matrix reads 302 seeds and its thinnest target is now `parse_yaml` at 6
  (`docs/dev/quality-ledger.md` (kb)).

## [v0.19.0] — 2026-11-04

<details markdown="1">
<summary>policies named at the call site · duplicate keys · note placement · floors keyed by runner
image</summary>

Measurement history — which run refused what, which attribution was later retracted — lives in
[`docs/dev/quality-ledger.md`](https://github.com/759401524/pyrs-yaml/blob/main/docs/dev/quality-ledger.md).
These notes state what changed for a caller.

### Added

- **`duplicate_keys=` names the reader's duplicate-key policy** — one flag, four per-identity
  answers today: `a: 1` beside `a: 2` raises `YamlDuplicateKeyError`, `: 1` beside `: 2` folds to
  the last pair, `0set: 1` beside `!base 0set: 2` keeps both and refuses only `to_dict()`, `'a': 1`
  beside `a: 2` raises. `as_is` (default) *is* those answers; `error` also refuses null and merge
  keys, `last_wins` is what `allow_duplicate_keys=True` always meant, `first_wins` keeps the earlier
  pair and re-homes the folded pair's notes and anchors onto it. Both spellings at once is refused,
  an unknown value names the four it means, and every read entry point and the twelve CLI commands
  take `--duplicate-keys`. The fuzz targets parse with the default, so the three named alternatives
  are not yet in that tier.
- **`note_placement=` and `on_duplicate=` put the remaining ambiguities at the call site** — under
  an `&a` / `!tag` header the writer parked a pair's trailing note in the header's gap where readers
  file it above; `note_first` (default) writes it where the read puts it, so the first emission is
  the fixed point, and `header_first` keeps the old placement, documented as not settling. Two keys
  can spell one name, told apart only by a tag or anchor, so deleting one left the other answering
  that name on re-read; `delete(path, on_duplicate=...)` answers `one` (default), `all` or `error`
  (`duplicate-key-target`, tree untouched), and `delete_path_with_policy` re-runs the anchor guard
  so a definition an alias still names is refused rather than emitted dangling.
- **`load_json_lines(text)` reads NDJSON one line at a time** — an iterator resolving one document
  per `__next__`, so a stream of 10,000 records reads in 3.861 ms whole and 0.343 ms to the first
  record against 8.440 ms for per-line `load_json`; blank, note-only and continued lines raise
  naming the line. The opt-in `parallel` feature gives each shard its own sink and thread: 200,000
  lines take 112.3 ms through one sink against 18.9 ms across 24 (5.96x). `parse_file` gains an
  opt-in `mmap` path that drops one whole-file allocation, off by default.
- **Streams stop when the consumer says so, and every event names its bytes** —
  `parse_stream(..., on_event=...)` used to collect everything before the first callback, so `False`
  still paid for the whole scan; `EventSink` now returns `ControlFlow`. What `False` stops is a
  format fact: JSON/JSON5 halt before the next event's bytes, TOML halts emission (so a refused
  document still raises first), YAML's adapter stops delivery after the unabortable parse, leaving
  `load_stream` as YAML's incremental surface. Every `StreamEvent` carries `offset`/`end`, and the
  event dicts gain both keys. The dumpers accept the tuple the loader hands back
  (`safe_dump(safe_load("? [a, b]\n"))` raised on accepted input); a nested key sequence still
  raises `unhashable type: 'list'`.
- **The test layer gained the faces that were dark** — cross-format note bridges (`json -> jsonc`
  and four other one-way pairs had never been exercised), a property tier for the JSON Schema
  surface over YAML-typed keys, JSON tree equality across the three dialects, generative Python
  leaves that name the families the old generator could not reach (`bytes`, dates, `Decimal`,
  unbounded ints), and an API call-surface gate that refuses a public callable with no call site —
  it found `get_plugin` and `YamlDocument.flush_source` never called. The fuzz corpus got its
  housekeeping: unseeded artifacts archived as `former-crash-*.seed`, one issue filed per target
  with findings, and per-target seed counts published and gated.

### Fixed

- **An alias whose only definition a fold dropped gets one where the use can reach it** — the
  duplicate-key fold dropped the pair defining `x` while the surviving `&x` rode a key after the
  alias, so `*x` came out above every `&x` and the reader refused its own emission with
  `found unknown anchor`. The repair now asks whether the name is defined *before this use*, the
  merge pass inlines the survivors it deletes, and `delete_path` reads key-node anchors too. A key
  rewritten from an alias folds where it lands, and an anchor name ending in a quote keeps that name
  instead of opening a quoted node on re-read.
- **Five round-trip shapes reach a fixed point on the first emission** — a first entry's note inside
  a `?` marker body no longer climbs above the marker, a block sequence's first item keeps its
  leading stack under its own dash, a marker lift no longer crosses a header it should not, a note
  no longer splits a value across two lines when a tag-only closure lengthens the line under it, and
  deleting a container's last pair no longer drags its note onto the pair line. Each was a
  `yaml_roundtrip` or `edit_yaml` crash input; the inputs are archived as seeds.
- **Keys only node metadata tells apart are refused** — the reader compared tagged keys as whole
  nodes while the writer chooses style and anchor, so its own emission came back `duplicate key`.
  Keys are now tracked by tag and value (YAML's scalar equality) and a merge asks the same; two
  container keys the writer spells alike ask the writer what a container key spells as and refuse
  the source when two read back equal. Projections key by text, so `c: 0` beside `!A c: "!"` emitted
  entries the TOML reader rejects and `json.loads` folded to one: both writers now raise
  `YamlSerializeError` and `to_dict`/`safe_load` raise `YamlTypeError` naming the text. `to_yaml()`
  is unchanged.
- **A repeated JSON key keeps the notes inside the value it replaces** — the duplicate fold kept the
  first position and the last value but carried only the notes on the dropped key and value nodes,
  so a note inside a dropped container went with it while the event stream still reported it.
  `subtree_notes` now collects a node and everything under it in source order, and a noted key
  between repeats leaves the last value rather than the second-to-last. An element's two note groups
  land in one slot instead of overwriting each other, and a root line-end note is written by every
  entry point: `{"a": 1} // after` no longer round-trips to `{"a": 1}`.
- **JSON5 quotes a string that only looks numeric, and its refusals name its own grammar** — the
  bare-token test stripped one leading sign before its numeric checks, so
  `+-33337333333373333333733333` was emitted as a bare token this engine's reader refused; only a
  leading `+` over an unsigned body is a JSON5 spelling, so `+7` and `+1.5` keep their bare token
  and such a string is quoted. In key position the message asked for a quoted key while JSON5 also
  takes a bare identifier; the wording now comes from the dialect's own flag, and strict JSON and
  JSONC keep theirs.
- **TOML notes that had no slot to live in are read** — `to_toml("sec: # note\n  x: 1\n")` returned
  `[sec]\nx = 1\n`: the section pass read only the mapping's own inline slot, while the hub hands
  over the key-line note on the key node. Five member note slots were likewise unread, and one
  element note was written twice. Every pair pays: `to_toml_medium` +1.58 % and
  `to_toml_long_strings` +1.25 % on the enforcing image.
- **A dotted path is bounded like every other descent** — `parse_inline_table` laid member key paths
  out outside `parse_value`, which is where `DEFAULT_MAX_DEPTH` acted, so 1,059 segments were
  accepted by `from_toml` and refused by the 1.1 reader, and a long path could overflow the native
  stack (`0xC00000FD` on a 2 MiB thread). Both descents share one budget now: 1,000 segments
  round-trips and settles, 1,001 is a typed `max depth exceeded`.
- **A registered handler owns the scalar it claims** — both writers asked the array route and the
  number protocol before the registry, so `numpy.ndarray` handlers lost and a class carrying only
  `__bool__` was written on a different line than the AST route emits; the check was written for the
  float shape alone, so a `python_type` with `__index__` was bypassed and `Money(500)` dumped as a
  bare integer. Handler lookup now comes first in every position; an `int` or `int` subclass keeps
  the integer spelling and a `str` handler still leaves text alone.
- **Dumping a NumPy array no longer ends the process on Python 3.15** — the array route cast the
  object through `rust-numpy` and read its memory, which the `cp315-abi3t` wheel cannot do soundly;
  `safe_dump(numpy.arange(8))` died with a segmentation fault (exit 139). The route now asks the
  array for its own `dtype`, `shape` and `tobytes()` and decodes that buffer in Rust, so no artifact
  touches the numpy C API. Two defects fell with it: a transposed view wrote the untransposed grid,
  and the number protocol answered a one-element array with the dimension dropped, so the emitted
  shape depended on the installed numpy.
- **A note can no longer break its line, and a block scalar can spell the break it carries** — the
  generator drew note text from printable ASCII only, so no property test could reach the flattening
  a one-line note carrier owes; drawing newlines too reddened four properties because both writers
  put the text after a break into the document. Both writers flatten a break to one space, and a
  block scalar that carries a break now quotes it instead of losing it to `~`.
- **A one-line document streams in linear time again** — `LineTable::line_col` recounted characters
  from the line's start for every event, so a source with no newlines paid O(n) per event: a 1.5 MB
  minified JSON array spent 4.9 s reporting its 224,006 events. It streams in 19 ms now. JSON and
  TOML share the table; YAML computes its own positions and never had this cost.
- **Gates and guards that could report success over measuring nothing** — six checker scripts
  printed OK on an empty glob, an unread path list or an `--only` name that matches nothing, and all
  six exit 2 now; ten suite assertions that passed on an empty iterable gained a floor naming the
  measured count. A memory bound that asked for `psutil` through an `importorskip` no requirement
  file ever named — every leg reported a skip that read like coverage — is a stdlib `tracemalloc`
  scaling invariant now (3,168 bytes traced at both 2,000 and 200,000 lines, against 199,854,440 for
  a buffering walk). The `yaml_edit` leaderboard row blamed "newer releases" while the pinned build
  raised on all three fixtures: 0.2.0 is installed and the row is gated by what it measures.
- **Reported fuzz findings stop disappearing** — the download unpacked the archive roots into the
  crash directory, so two phantom targets named `artifacts` and `collected` were minted and one
  archive's three inputs were filed as two issues; counting used `grep -c .` under
  `-eo pipefail`, so an empty archive ended the step before filing; and the panic signature stopped
  at the first colon, which is how five consecutive scheduled runs came back red with nothing
  triageable. The target is the segment after either root, zero is a reportable count, and the
  signature keeps location and first message line.
- **Published docs corrections** — `Node.is_valid()` was said to refuse a modified document but
  tests only the document reference; the same `repr` conflates a stale node, a missing path and a
  path the dict projection refuses, which reads through `to_dict()`. `rumdl fmt`'s MD038 "fix"
  deleted the space *inside* a code span, silently rewriting five mirrors and the ledger while the
  hook printed `Passed`; MD038 is disabled now. Localized changelog pages put their sections at the
  version heading's own level, so the site rendered each as a sibling of its version, and the mirror
  check reads that too.

### Changed

- **Asking a single-document route for a stream is refused** — `parse`, `parse_file`, `safe_load`,
  `to_toml` and the `YAML()` equivalents read one document, and a two-document input returned
  whichever the walk ended on: `a: 1\n---\nb: 2\n` came back as `b: 2`. They now raise
  `YamlParseError` opening `multi-document-input` at the second document's position and point at
  `parse_all_docs`, `safe_loads` and `parse_stream`. PyYAML and ruamel.yaml raise on the same ten
  shapes and still accept a leading `---`, a trailing `...`, a bare `---` or empty input, which the
  guard must not over-fire on. `-A/--all-docs` says the same in the CLI.
- **The instruction-count gate now carries one floor per runner image where the counted region
  touches CPython** — at one tree with equal harness digests `load_toml_medium` read 142,467,241 and
  144,904,012, a 1.71 % spread a single floor cannot hold, so `.ci/ir-baseline.json` gained
  `image_floors` and `scripts/ir_gate.py` judges a run by its own image's row; an image with no row
  is refused with exit 2 rather than guessed. Growing the harness moves floors the change never
  enters, so scenarios past the tolerance are named by a refresh run's refusal rather than
  predicted, and `tolerance_hint` stays 0.5 %.
- **Every Rust benchmark runs in its own process, and both lockfiles moved to the newest version
  their ranges allow** — `codspeed-divan-compat` 5.0 → 6.0 forks every Divan benchmark, so allocator
  state left by one no longer colours the next. Nothing that decides a merge moved, but a CodSpeed
  trend across this bump is not one measurement. `similar` 3.2.0 → 3.2.1 is the only Rust step left
  under the eight crates' MSRV 1.96 floor, and nine Python packages moved; `maturin` stays at 1.15.0
  and `pyo3` on the 0.29 line, since the newest `pytest`, `hypothesis`, `pydantic` and `numpy` need
  an interpreter above the 3.8 floor.
- **The `no_std` host tests run on Windows as well as Ubuntu, and the unit-test leg runs the
  configuration its name states** — wheels ship for three hosts while the job was pinned to one. The
  six-crate invocation unified `std` across the packages it reached, so a claim about 112 tests
  running without std had been measured on the std build; three manifest edges close it and
  `tests/test_no_std_gate.py` reads the manifests rather than trusting a `-p` list. The std-free
  command runs 724 tests over 23 targets with 0 failures on Windows.
- **The bump bot neither offers the floor's compiler nor mis-spells its own config** — the weekly
  `github-actions` group rewrote `dtolnay/rust-toolchain@1.96` to `@1.120`, the ref `msrv-check`
  installs as its compiler; that name is excluded now, and three `groups` entries using the
  `ignore`/`allow` vocabulary in `update-types` (which takes only the short forms) are corrected and
  pinned by a test.

### Performance

- **Writer output stopped paying for intermediate allocations** — every writer began with
  `String::new()`, whose seven reallocations to 512 bytes are most of a config-sized write. On the
  gate's pinned `rustc 1.97.1`: `to_json_medium` 22,884,635 → 17,718,056 (-22.6 %),
  `to_toml_notes` 16,468,905 → 12,071,715 (-26.7 %), `serialize_block` 14,518,712 → 10,899,301
  (-24.9 %), `serialize_note_flow` 64,411,669 → 59,227,943 (-8.1 %) — all sixteen writer scenarios
  down, the smallest `to_toml_long_strings` -5.2 % — while the unchanged tree reads
  `to_json_medium`'s floor to +0.002 %. One-pass note flattening leaves `to_jsonc_notes` and
  `to_json5_notes` 38.27 % and 38.11 % under the floors their PR committed. No output byte changes.
- **String bodies, the TOML line and the escape scan run sixteen bytes at a time on SSE2** — SSE2 is
  in the x86_64 baseline: no new dependency, no `std`. On the host, 2 MB of JSON with 5000-byte
  strings 2.47 → 1.97 ms, 2 MB of TOML basic strings 2.36 → 0.99 ms, literal 2.41 → 0.96, multiline
  3.37 → 1.01; on the enforcing image `from_toml_medium` -4.02 %, `from_jsonc_medium` -3.69 %,
  `from_json_medium` -3.55 %. The escape question measured 75.12 % of `to_json_long_strings`, and
  the four long-value writer floors read -62.36 % to -60.37 %; short values are another shape (about
  26 %) and one paid: `to_json_medium` +0.52 %.
- **A line of JSON is indexed eight bytes at a time** — `pyrs_event::LineTable` counted one UTF-8
  character per step, and on the enforcing image that walk read 51.79 % of `documents_long_strings`.
  A `u64` answers eight bytes at once, each candidate lane checked against the byte it claims.
  `documents_long_strings` 328,990,934 → 147,188,912 (-55.26 %) and `documents_short_strings`
  95,854,968 → 85,300,932 (-11.01 %), plus `stream_json_medium` -14.90 % and `stream_json_stop`
  -27.17 % because the same index serves the whole JSON stream route. Positions are unchanged.
- **`load_toml` stopped building the tree it cannot keep, and `load_json5` took the AST-free route
  its siblings already had** — TOML text went scan → table tree → `CustomNode` → Python, so a plain
  dict paid for a format-preserving node model; `read_toml` now walks the IR into a caller's
  `TomlSink`, and a read while any handler is installed takes the AST route. Against `rtoml` the
  published row went from 1.92x and 1.91x behind to 1.47x and 1.46x. `load_json5` had been paying
  the whole `CustomNode` round-trip (2.063 µs against 0.473 µs for `load_json` over the same 123 B);
  it wins 6.25x on 300 items and 9.34x on 1,200 against its own dialect route, values unchanged.
- **The loader routes, the three edit verbs and the stream stop have instruction floors** — no
  harness entered them before: `load_toml` reads 1.20x and 1.29x of `from_toml_medium` and
  `from_toml_long_strings` (the Python half no counted number had described), `load_json_*` floor
  the fast JSON route at 0.74x and 0.86x of their node-model twins, and `stream_json_medium` against
  `stream_json_stop` puts the stop at 25.09 % of the walk. The note routes and metadata carriage are
  floored too: `edit_set_metadata` 42,838,349 against `edit_set_nested` 36,483,730 on a
  26-character-smaller document is the carriage, not a bigger tree.
- **The gate's widest reading was the interpreter, and it is pinned** — three refresh runs recorded
  the binding harness under one binary digest and spread `load_json_long_strings` 1.121 % across
  nine samples while the engine scenarios on the same bytes agreed inside 0.00001 %. The term was
  CPython's per-process hash seed: under `PYTHONHASHSEED=0` all seven binding scenarios collapse to
  0.0000-0.0004 %, and a refresh on that head reports all 42 scenarios within 0.0010 % of
  themselves.
- **An empty block body writes the chomping it declares** — map equality reads `chomping`, so the
  reader kept `? hF&a?k !tag o: |+1` beside `? hF&a?k !tag o: |`; the writer forced `Clip` on every
  empty body, spelled both `|`, and its own reader refused the emission as `DuplicateKey`.
  `effective_chomping` now returns the declared chomping for an empty value, so two container keys
  that differ only there stay two keys.

</details>

## [v0.18.3] — 2026-10-22

<details markdown="1">
<summary>the 1.1 numeric families · `%YAML` negotiation · timestamp tags · a std-free engine ·
outside readers for the writers · `pyq` answering jq and yq</summary>

### Added

- **`pyrs-yaml-core` builds with no standard library** — the parser, the round-trip serializer, the
  comment-preserving edit layer and the splices run on `core` + `alloc` alone over a `granit-parser`
  that was already std-free. `std` stays on by default, so the binding, the CLI and the benches keep
  the API they had, and `ci.yml` builds the crate for `thumbv7em-none-eabi` while running its unit
  tests on the host; what `std` adds is the three surfaces that need an OS — the translated
  error-message catalog, the runtime schema registry and the regex-backed rule language — and a
  `no_std` caller selects a schema by building the `pyrs_schema::types::Schema` variant directly.
  The engine tracks seen scalar keys in its own ordered node map because `alloc` carries no hasher,
  which costs 74 allocations against the std build's 67 on `parse_medium`, one more per mapping, and
  `tests/alloc_gate.rs` pins a ceiling per feature set.
- **`pyq` answers the `jq` and `yq` command line** — an argv that is not one of pyq's own
  subcommands is read as jq 1.8 or mikefarah yq v4 (by invoked name, or forced with `--as jq|yq`)
  and translated into the equivalent subcommand. Accepted forms are judged byte-for-byte against the
  real tools when they are on `PATH` — 24 invocations including `.a.b`, `.list[] | select(.x > 1)`,
  `del(.a.n)`, `.a.n = 9`, `-o json`, `-r`, `-N`, and a probe that refuses any reference inside this
  workspace. What the shell cannot answer with the same document is refused by name and exits 2: jq
  functions (`keys`, `length`, `map`, `to_entries`), jq edit expressions,
  `--arg`/`-s`/`-c`/`-S`/`--stream`, and yq's
  `-n`/`-e`/`-P`/`--split-exp`/`--front-matter`/`--xml-*`/`--lua-*`. `pyq get --no-doc` is the one
  behaviour difference the probe found, because yq v4 prints a multi-item stream without `---`
  separators, and that is now the dialect default.
- **The write side of every format is judged by a reader it does not own** —
  `tests/test_json_writer_suite.py` emits 1,500 generated trees through `to_json`, `to_jsonc` and
  `to_json5` and requires Python's `json` and `pyjson5` to hand back the tree that went in;
  `tests/test_yaml_writer_suite.py` does the same through `safe_dump`, `from_json` and the edit
  layer against PyYAML and ruamel.yaml (400 trees per route); `tests/test_toml_writer_suite.py`
  hands `to_toml` output to `tomlkit` and `tomllib` — 1,074 of 1,200 trees emitted with 0 refusals
  and 0 value changes, and the 126 that never reached a stranger were one typed refusal,
  `toml-cannot-represent-null`. A fixed point satisfied only by our own reader is not a fixed point,
  which is the class two of the three fixes before it were.
- **`Document.validate()` is measured against the official corpus** —
  `tests/test_document_validate_suite.py` replays JSON-Schema-Test-Suite at commit `0b4a7fa` (46
  files, 1,302 cases) and reaches the corpus's verdict on 1,261 of them under `jsonschema` 4.26.0.
  `refRemote.json` is excluded whole as a contract boundary (no resolver is exposed), and ten
  `dynamicRef`/`vocabulary`/`\p{...}` cases are named as boundaries of the delegated engine; the
  same corpus replayed through `jsonschema.validate()` directly agrees on all 1,302, so no exclusion
  is attributable to the conversion. The exclusion set is keyed on the installed version, and both
  directions are asserted — a named case that starts agreeing reddens the run, and so does an
  unnamed one that stops agreeing.
- **The edit layer has a fuzz target of its own** — `fuzz_targets/edit_yaml.rs` is the workspace's
  ninth target and the first to drive a *change* rather than a document: it reads
  `document NUL script`, picks paths the reader really found, and requires every accepted edit to
  re-parse and to leave the value where it says it wrote it. It reached the anchor-defect above on
  its first seed replay.
- **A second registry holds the residuals the matrix cannot measure** —
  `.ci/known-residuals.json` records each deliberately open behaviour with a repro node id, an owner
  and a checkable exit; the repro carries `@pytest.mark.known_residual` and
  `tests/test_known_residuals.py` fails in both directions, so fixing one reddens its
  characterization test and forces the entry out in the same change.

### Changed

- **A declared `%YAML` version is honoured on request** — `honor_version_directive=True` on `parse`,
  `parse_file`, `parse_all_docs`, `safe_load`, `safe_loads` or `YAML(...)` lets a `%YAML 1.1`
  declaration select the 1.1 profile (`flag: yes` reads `True`, `012` reads `10`) while `1.2`, `1.3`
  and an undeclared document still resolve under Core and an explicit `schema` wins over the
  declaration. With the flag on, an honoured declaration is re-emitted as `%YAML <version>` before
  its `---` — after edits and `reparse()` too — so a Core reader cannot silently retype the text.
  The default is unchanged, and `schema` now defaults to `None` so "not given" and "explicitly Core"
  are distinguishable.
- **The `schema="1.1"` profile resolves the numeric families 1.1 defines, and the writer quotes what
  it types** — `1_000` is the integer 1000, `0b101` is 5, `+0x1f` is 31, `+.inf` is infinity and
  `017` is 15, each the answer PyYAML's loader gives the same text. The quoting guard had asked only
  this engine's own Core reader, so `safe_dump({"v": "1_000"})` emitted `v: 1_000` and both PyYAML
  and ruamel handed back the integer 1000; `8_0`, `0b101`, `+0x1f`, the legacy bool words
  `yes`/`on`/ `off`/`y`, the date-only timestamp `1990-06-23` and underscore-prefixed radix forms
  like `0x_1f` were the same defect in 26 spellings per route. Output changes only for those
  spellings: `1_000` now writes as `"1_000"`. Timestamps and sexagesimal stay text, and `0o17` stays
  typed here while PyYAML reads it as text, because the profile is Core *plus*.
- **A TOML integer keeps the spelling it was written with** — the parser cleaned the token before
  recording it, so `to_toml(from_toml("a = 1_000"))` came back as `a = 1000` even though underscores
  and an explicit sign are legal TOML. The token text is kept whole now, and the projection tags a
  spelling only when YAML Core resolves a different value: `0x1f` and `+5` stay untagged while
  `1_000`, `0b1101_0110` and `-1_000` carry `!!int`. Values are unchanged and agree with `tomllib`;
  an untagged quoted `"42"` stays a string.
- **A note in an event stream says which node it belongs to** — `StreamEventType::Comment` carried
  one flag answering two questions at once: whether the note had a line to itself (a fact about the
  source) and which node it is filed under (a fact about the tree). `owns_line` keeps the first and
  a new `NoteOwner` carries the second (`FollowingLeading`, `FollowingInline`, `PrecedingLeading`,
  `PrecedingInline`). Python callers of `parse_stream` see the old `style` key unchanged and gain an
  `owner` key. Two visible details follow from the producers answering it from the node they filed
  the note under: a TOML note written after a comma now reports `style: "inline"` where it reported
  `standalone`, and a TOML table's notes arrive after its key event rather than before it.
- **Temporal values leave under the tag every reader knows** — `safe_dump()` wrote `!date` and
  `!timestamp`, which are local tags with no constructor outside this library: measured on trunk,
  `yaml.safe_load` and `ruamel.yaml` both answered such a document with `ConstructorError`, so the
  value was not mistyped, it was unrecoverable. `date` and `datetime` now leave as `!!timestamp`,
  which those readers resolve to a `date` or a `datetime` on the same fork this reader makes, and
  the TOML projection moves with them (visible as `tag: "!!timestamp"` in `parse_stream`). The old
  tags stay registered and keep resolving, so a document an earlier version wrote still reads back,
  and a document that arrived with one is re-emitted unchanged. `!time` remains a local extension:
  YAML defines no tag for a bare time of day.
- **`sort_keys` on a scalar path is a documented no-op, not a silent one** — `sort_keys("$.a")`
  answered `None` and changed nothing while `sort_keys("$.missing")` raised `YamlEditError`, and
  nothing the package shipped said so. The stub states it now, and the residual leaves
  `.ci/known-residuals.json` with both sides asserted.
- **Every performance number on the site comes from one committed measurement** — the wall-clock
  tables on the four locales' pages had been transcribed by hand, and the same calls now measure
  22–108× faster than the 2026-08-17 figures behind them. `scripts/bench_snapshot.py --update` times
  nine batches of at least 0.1 s and commits the reading with its interpreter, library versions and
  fixture sizes to `.ci/perf-snapshot.json`; `--docs` rewrites all twelve tables from it and
  `tests/test_perf_docs.py` compares every cell back. The prose had drifted separately, so every
  speed range quoted as prose on all 24 pages that carry one is derived by the same gate. The pages
  now state 29–34× parsing and 223–368× serialization against PyYAML, 68–81× and 541–876× against
  ruamel.yaml, and 45–59× / 102–125× on the round-trip guides.
- **The instruction-count gate is a merge decision, and it reads both edges** — the
  `Instruction-count baseline` job lived in `codspeed.yml` while `Test matrix (all legs)` fans in
  over `ci.yml`'s jobs, so a measured regression was a report rather than a refusal; the job moved
  and the verdict now waits on it. The gate had read only the upper edge, so the accepted slack
  widened every time the engine got faster — the 2026-10-14 refresh found six writer floors 22–46 %
  below their tree. A scenario more than 2 % below its floor now warns `FLOOR_STALE` and more than
  5 % fails, naming the refresh dispatch rather than `--update`, which would move the floor to the
  local host.
- **The defence matrix is derived, and its authority column measures a call site** —
  `scripts/quality_matrix.py` computes `QUALITY_MATRIX.md`'s format table from the files that
  declare the defence and adds an `external authority` column, and `--write-rows` now writes the
  tier counts from the same measurement that checks them (that gap cost PR #549 all 14 test legs).
  Measured against the derivation, `schema validation` was under-reported and the property tier is
  27, not 25. The column stopped crediting an import: a `test_*_suite.py` earns reader credit only
  by reaching into that reader, a corpus suite only by naming its corpus, and a `partial` parity
  cell is now a registered hole rather than a footnote, because the sentence "the JSON family has a
  cross-library tier" was quotable about a leg where the module imports nothing and skips.
- **Cross-library reference readers now reach every leg the package promises** — the 3.8 floor and
  3.14+ compared JSON5 and JSONC against nothing while the leg reported green, and TOML's three-way
  agreement skipped on 3.8 through 3.10 because `tomllib` arrives with 3.11. The pure-Python `json5`
  (`>= 0.17.3`, `py3-none-any`) and the `tomli>=2.0` backport are declared in the test group, so all
  24 TOML comparisons and the dialect's spec table run everywhere; stdlib `json` is an RFC 8259
  oracle on the legs that cannot install a reader (measured: 29 comparisons run where 0 ran before).
  Where the two JSON5 references disagree, this engine's reading is pinned rather than hidden: a
  surrogate pair combines into one character here, and a lone surrogate is refused here.
- **The MSRV floor and the wheel matrix now say what they ship** — `msrv-check` installed `@stable`
  and called that the minimum, so a dependency needing 1.97 would have passed the job and failed at
  a user's install; it pins the 1.96 the eight crates declare, and the floor was measured on
  `cargo +1.96.0 check --workspace --locked` and `... test --workspace --locked` before the pin
  moved. The aarch64 glibc and musllinux wheel legs build on native arm64 runners, with s390x the
  only suite still emulated. `.ci/requirements-test.txt` — the only requirement file Publish
  installs — named neither `tomlkit` nor `json5`, and this release's first Publish run reddened five
  wheel legs at collection before a wheel uploaded.

### Removed

- **The invented rule validator is retired; `Document.validate()` is the only schema surface** —
  `validate_against_schema()` and `validate_against_registered_schema()` are removed, and a
  `validate:` section in a schema definition is no longer read. Both evaluated a rule language this
  repository invented while `Document.validate()` already delegated to Python's `jsonschema`, so two
  validators meant two error contracts and two answers to "is this document valid". Callers move to
  `Document.validate(schema)`, whose `YamlValidateError` carries the reference engine's message, or
  to `jsonschema.validate()` directly. Scalar-type resolution is untouched: `register_schema()`,
  `load_schema()`, `list_schemas()`, `YAML(schema=...)` and the `rules:` list still apply. The CLI's
  `validate` binds `--schema`/`--schema-file` to one JSON Schema parameter and checks syntax only
  when neither is given. `TypeSpec`, `ValidateKind`, `ValidateRule` and `SchemaValidationError` are
  deleted, the matrix's `schema validation` row leaves with them, and the fuzz target becomes
  `parse_schema` because it fuzzes the surviving grammar.

### Fixed

- **Eight note placements now settle on the first emission** — a note a block container could not
  spell above its own `&a` / `!tag` line climbed one header per round; a container's note written
  between a header and its body moved above the header; a compact item's note settled above its
  sequence's header; a mapping note over a nested sequence landed on an item's dash; a note on a
  block-sequence item pushed the whole mapping onto its own lines; a tag-only line (`- !fuzz2`,
  `{a: !t }`) left a note with no body to sit on; and a sequence's own note moved into its item.
  Seven of the eight were `edit_yaml` crash inputs, archived as seeds so the fix inherits the input;
  the mapping-note case is pinned by its test name. Nothing was lost across any round, which is
  exactly why no other tier saw them.
- **An empty value is spelled as the null it resolves to** — `k:` and `-` re-read as a null whose
  own spelling is `~`, so the first emission settled only on the second round. The writer now writes
  the null text wherever the body would be empty (nested value, root value, sequence item, flow
  pair, flow item). A tagged empty is left alone, because the tag suppresses resolution and its body
  keeps the `''` spelling that settles it; an anchor is not the same case, so `- &a3` is written
  `- &a3 ~`.
- **An edit that orphans an anchor is refused, not written** — renaming an anchor, replacing its
  node without carrying its metadata, moving a definition away, or `set`-ing over a container whose
  member carried `&x` all answered success and emitted text this library's own reader refused
  (`found unknown anchor`), and each refusal answers `anchor-is-referenced`. The guard was already
  in the engine; these four doors bypassed it, so the question is now asked where the drop happens.
  It is deliberately wider than the set of aliases that would really dangle: an alias binds the
  *nearest preceding* anchor of its name, so a rename a later `&x` would have kept alive is refused
  too. Over-refusing costs one more edit; under-refusing writes a file nothing can read.
- **An anchor the duplicate-key fold removes is inlined, not abandoned** — the fold drops the entry
  whose *value subtree* defines `&x`, the repair pass was handed only the dropped entry's own
  anchor, and the emission named a definition nothing defined. The fold now registers every
  definition leaving the tree, so the surviving use is inlined with its anchor and the output reads
  back in one round.
- **A merge cannot give a mapping a second null key** — `~`, `null`, the empty key and a bare `Null`
  node all resolve to null, and the reader folds two null keys on purpose; the merge filter asked a
  different question (text for untagged scalars, whole-node equality otherwise), so a `<<` source
  contributed a null key beside the one the mapping owned and the emission wrote `~:` twice. Found
  by the property tier at 20,000 cases.
- **JSON5 and JSONC accept what their own grammar allows** — `\xHH` escapes parsed nowhere while the
  shipped stub said `load_json5` "Accepts the full JSON5 grammar" (the two digits accumulate one
  byte at a time, because `from_str_radix` would accept `\x+1`; strict JSON and JSONC keep rejecting
  the form); a backslash-u escape inside an unquoted key and a backslash before CRLF, LS or PS
  inside a string parse now; a double-quoted JSON5 string carries every C0 byte except CR and LF,
  where RFC 8259's rule had been applied to the dialect instead of to strict JSON; and a key that is
  nothing but a Unicode space (`U+2028`) is no longer lost to `null`, because the JSON resolver
  trims plain scalars with Unicode rules while the quoting rule knew only ASCII space and NBSP. All
  25 Unicode `White_Space` code points are swept in key and value position across the three
  dialects, and the strict loaders are held to the RFC for the same bytes.
- **A JSON string spelled with a long integer keeps its quotes across the hub** — Core answers `Str`
  for a digit run too long for an `i64` while JSON bounds no integer literal, so
  `"9999999999999999999"` projected as a plain scalar and `to_json` wrote it back as a number. The
  projection now asks JSON's own number grammar before writing a string plain.
- **A value the target format cannot spell is refused or re-spelled, never invented** — a YAML float
  spelling TOML has no grammar for (`083e6` — Core's exponent form allows a leading zero, TOML's
  `float-int-part` does not) is rendered from the value instead of passed through; a signed zero
  writes as `0`, because TOML has no negative zero and `ist = -0` re-read as `0`; a root table that
  TOML cannot place after its pairs keeps its position inline rather than reordering the mapping;
  and an emptied block sequence keeps its dash line.
- **A schema that omits `extends` resolves the way its documentation says** — the default is `core`,
  but the parser only reached Core when the key was present with an empty value, so a schema that
  left it out typed every scalar as its own text and both directions of the verdict inverted without
  complaint: `type: int` refused `port: 80` while `type: str` accepted it. A base name this engine
  cannot find still leaves no fallback, which is a separate call about a public error surface.
- **A verbatim `!<tag:yaml.org,2002:X>` reaches the plugin registered as `!!X`** — YAML 1.2 §6.8
  makes the two spellings one tag, and the loader already honoured both for the five tags it names
  by hand; for every other name the registry is keyed on rendered text, so
  `!<tag:yaml.org,2002:widget> 7` ignored a `!!widget` plugin and loaded the integer `7`. The
  spelling the document used is tried first, so the second look is paid for only on a miss, and a
  URI outside YAML's namespace keeps its current reading.
- **`pyq` answers a range as a range and `--json` the same way twice** — `[0:2]` fell through to the
  key branch, so `.list[0:2]` looked up a key named `0:2` and answered `null`; the grammar rejects a
  `:` it cannot address and the shell refuses the construct by name, while a quoted bracket is still
  a key. `pyq get --json` indented a single match and wrote each of many matches on one line, so
  `.servers` and `.servers[*]` disagreed about what the flag means; every match now gets the same
  rendering. Under `--as jq|yq` a path the document does not carry prints `null` and exits 0, as
  both real tools do, while native `pyq get` keeps reporting the miss and exiting 1.
- **Documentation that had quietly drifted was corrected** — the localized API pages documented
  `parse_file`, `parse_all_docs`, `safe_load` and `safe_loads` with one or two parameters where the
  shipped stub declares five and six, `honor_version_directive` appeared in no user-facing page, and
  `reparse()` was documented with a `schema` default the extension does not use; the signature gate
  now compares both sides through `ast` in all four locales. `safe_dumps_async` was importable but
  missing from `pyrs_yaml.__all__`. The benchmark pages said CodSpeed runs `WallTime` when it has
  run `mode: simulation` since it was added, so the millisecond tables are marked as wall-clock
  readings from one machine rather than CodSpeed output — including that the "small" Rust document
  is 19 B against the Python table's 95 B, so same-named rows are not comparable.

</details>

## [v0.18.1] — 2026-10-17

<details markdown="1">
<summary>free-threaded 3.15t wheels · Linux arm64 · merge-key notes kept</summary>

### Added

- **Free-threaded wheels for Python 3.15t and Linux arm64** — the release now builds a
  `cp315-abi3.abi3t` wheel for Windows, macOS and glibc Linux (x86_64 and arm64), so 3.15t and later
  install a prebuilt artifact instead of compiling from source; Linux arm64 gains its `cp314t`
  and `abi3t` wheels from a native arm64 runner, where the interpreter exec that qemu could not
  perform is a native call. Each wheel is installed, and the suite run, on the interpreter it
  targets.

### Fixed

- **An alias-valued merge key keeps its inline note** — `<<: *d  # note` lost the note at ingest:
  an alias has no metadata slot, and the reader reported the note attached anyway. It now rides the
  pair's key, the slot the block writer emits it from, so the document round-trips unchanged; with
  merges resolved it re-homes onto the mapping's surviving entry.

- **A key's inline note below a tag-only line keeps its owner** — when the pair's value carries a
  note of its own, the key's note is written on a line above the pair; if the line above is a
  value spelled as nothing but its tag (`k: !:`), the reader handed the note to that value and the
  document settled only on its second emission. The writer now closes that line with the value's
  own `~` first, working from an offset captured when the pair started (libFuzzer
  `yaml_roundtrip` crash-08cf4fe4).

</details>

## [v0.18.0] — 2026-10-17

<details markdown="1">
<summary>five-format event streams · explicit tags · round-trip fidelity · faster I/O</summary>

### Added

- **Five-format event streams** — `parse_stream` gains a keyword-only
  `format="yaml"|"json"|"jsonc"|"json5"|"toml"`; the dicts keep the same seven keys, with `None`
  where a format has nothing to say, and YAML's `anchor`/`alias` stay YAML-only. The default is
  unchanged, event for event. `pyrs_json::parse_stream` and the validating
  `pyrs_json::from_events` are public, and TOML gained a tree-free producer.
- **The schema language can name a container** — `type: map` and `type: seq` assert a node's
  shape without constraining its contents; `mapping_of: seq` says every value of a mapping is a
  sequence; `mapping`, `object`, `sequence`, `array` and `list` are accepted spellings.
- **`no_std` foundation crates** — `pyrs-ast`, `pyrs-schema`, `pyrs-json` and `pyrs-toml` build
  against `alloc` alone. A new opt-in `std` feature restores `std::error::Error` and the
  `RandomState` hasher, and stays on by default.
- **Prebuilt `pyq` and Linux free-threaded wheels** — the release builds `pyq` for six platforms
  (Linux manylinux2014, glibc 2.17) and attaches the archives to the GitHub Release; the wheel
  matrix now ships Linux free-threaded wheels.
- **CI adds Python 3.15** — the matrix covers 3.8 through 3.15, free-threaded interpreters
  included.

### Changed

- **Standard `!!` tags decide the loaded type** — `!!str 1.20` is the string `"1.20"` (it was the
  float `1.2`, losing the trailing zero), `!!float 1` is `1.0`, `!!bool yes` is `True`,
  `!!binary aGk=` is `b'hi'`, and an `!!int` past i64 stays an integer. A text the tag cannot
  satisfy raises `YamlTypeError`; unknown application tags stay lenient.
- **A bare `!` states that a scalar takes no schema reading** — `! 12` loads as the string `"12"`,
  per YAML 1.2 §6.8. PyYAML and ruamel resolve it implicitly and answer `12`; this release follows
  the specification, and a plugin on `!` no longer overrides the tag. An explicit 0.18.0 behavior
  change.
- **`version()` reports the document's `%YAML` directive** — `%YAML 1.1` reports `1.1`, `%YAML 1.3`
  reports `1.3`, and an undeclared document still reports `1.2`. The writer does not re-emit the
  directive.
- **A `bytes` value is written as `!!binary`** — the built-in plugin emitted the local tag
  `!binary`, which PyYAML could not read and which broke on an empty payload; the local spelling
  stays accepted on load.
- **TOML binary integers keep their spelling** — `b = 0b1010` round-trips as `0b1010` (it was
  rewritten as `10`), carried as `!!int 0b1010` so every format reads one type. `0b1101_0110`
  still loses its separators, and a hand-built untagged `0b1010` still reads as a string.
- **Strict JSON refuses what RFC 8259 cannot spell** — the JSON writers raise on non-finite floats
  instead of respelling them; the JSON5 reader rejects leading zeros its grammar forbids; the
  JSON5 writer no longer emits a number its own reader refuses.
- **`granit-parser` 1.3 → 2.0** — the major bump needed no source change; the full comment,
  anchor and compliance tier passes, and `cargo tree` confirms the graph carries 2.0.
- **A `CustomNode` is 144 bytes, not 208** (128 with `no_std`) — metadata moved behind accessors
  and the rarely used anchor/tag slots share one pointer: `from_json_medium` −4.8%,
  `serialize_anchors` −4.0%, and the two sizes are pinned as part of the hot-path contract.

### Fixed

- **Comment, anchor and tag ownership** — a note rides its own node's line through markers, tags,
  nested containers and compact `- key: value` forms; a first entry's note is lifted above the
  header; an anchored flow member keeps its name; anchors survive `#` that only looks like a
  comment, unbalanced apostrophes and brackets. An alias binds the anchor in front of it, in key
  position too, and nested or aliased merge keys are applied instead of dropped.
- **Block scalars settle in one emission** — an empty block scalar keeps the header indicators it
  wrote, no trailing break is invented or dropped, and the indicators are detected correctly.
- **TOML round-trip** — a dotted key inside an inline table nests; a root-level inline table is no
  longer rewritten as a section header; a table header is identified by its segments, not by its
  dot-joined text; a comment inside a multi-line array stays with its element; a note on an
  inline-table member is written; multi-line strings keep their quotes, escaped breaks and
  trailing delimiters.
- **Nothing a writer emits is refused by its own reader** — `to_toml` emitted invalid TOML in
  several shapes (now gated); JSON5 output is read back by the JSON5 reader; the comments the
  JSONC/JSON5 writers used to drop now survive, at a measured `to_jsonc_medium` +4.77%,
  `to_json5_medium` +3.78% and `to_json_medium` +0.94%.
- **Duplicate and null keys** — a duplicate JSON key no longer takes its comment with it; a
  repeated name collapses by name even when a note rides the key; duplicate keys are rejected by
  resolved value; `null:` and `'null':` are different keys. A document with many null keys parses
  in linear time (2k distinct + 2k null keys grew 12.4× for 4× input).
- **A mapping key means what the same text means as a value** — key identity follows the reader's
  resolution, not the AST's node kind; a plain spelling that resolves to a number is not exempt;
  the pinned model grows from 15 to 18 key classes.
- **Schema validation reads the tag first** — `!!int 0b1010` validates as the integer 10,
  `!!int hello` is refused instead of re-typed, and a rule that names a path asserts the node that
  path selects rather than walking past the wrong kind. `$` is a rule target, and non-ASCII rule
  paths no longer abort the process.
- **The Python contract ships intact** — the committed stub parses and type-checks under mypy and
  ty, declares all ten extension exception classes, and matches the running object's parameter
  names for all 40 module functions; published signatures on 28 API pages are checked against the
  package; an inline dict schema is accepted and every key survives; the generated reference
  renders the four public helpers it had omitted (`to_toml`,
  `validate_against_registered_schema`, `validate_against_schema`, `validate_custom_types`).
- **Edge inputs** — CR-only breaks no longer move notes on the second round; Unicode blanks, BOMs
  and tag suffixes no longer corrupt a document; stream positions count characters, not bytes, for
  CJK and emoji keys; a Python integer wider than 64 bits no longer leaves the dumper as a float.
- **`pyq`** — `validate` accepts `--input` and honours the format it actually read; the test
  harness no longer panics when a refusal closes stdin.
- **The defence tier is measured end to end** — the quality matrix's counts come from the tree,
  the JSON and TOML fuzz targets assert that their own writers' output re-loads, PR-tier fuzzing
  is blocking, and allocation ceilings plus instruction floors sit on the hot paths.
- **The release's `pyq` assets get the toolchain they ask for** — `publish.yml` wrote
  `targets: ${ matrix.platform.std }`; a single brace is not a GitHub expression, so rustup
  received `${`, `matrix.platform.std` and `}` as three literal target names and every `pyq` leg
  stopped at toolchain setup. The step passes `${{ matrix.platform.std }}` now, and
  `tests/test_publish_toolchain.py` fails if the form returns.
- **The release's wheel tests get the tools the suite calls** — the wheel jobs run the whole suite
  inside environments that carried neither `git` (the Alpine containers) nor `tomli-w` (every
  leg), so nine repository gates and the script-import test failed after the wheels were built.
  `.ci/requirements-test.txt` ships `tomli-w`, the container steps install `git`, and the suite
  body is now `.ci/pytest-wheel.sh`, called by every container leg: embedding it in
  `docker run ... sh -c '...'` ended at the apostrophe in a comment (`container's`), so Alpine ran
  `apk add` and nothing else while the runner's toolcache Python ran `-m virtualenv`. The script
  calls the venv's binaries by path because Alpine's `/bin/sh` has no `source`, and
  `tests/test_publish_toolchain.py` fails if any of these inputs goes away.

### Performance

- **Readers do less per byte** — the TOML reader allocates each scalar once (148 → 97 allocations,
  16,362 → 15,612 bytes on `from_toml_medium`); the JSON reader's unescaped strings borrow the
  source (74 → 48 allocations, 9,675 → 9,444 bytes); JSON and TOML scanners copy runs instead of
  decoding per character; the scalar resolvers stop trimming text that has nothing to trim.
- **Writers allocate less** — the TOML writer emits into the buffer instead of formatting
  temporaries (it had counted 53.2M instructions against `to_json_medium`'s 21.3M for the same
  document); the YAML writer decides quoting in one byte pass; tag emission is table-driven and
  escaping no longer allocates.
- **The event stream borrows its scalar text** — `StreamEventType::Scalar` no longer copies every
  value out of granit's `Cow`; the move measured −9.192% on the event scenario.
- **Instruction floors, measured on the pinned `rustc 1.97.1`** — against the committed baseline:
  `from_toml_medium` −9.80%, `from_json_medium` −13.37%, `from_jsonc_medium` −11.28%,
  `from_json5_medium` −9.61%. The release wheels are built by the compiler the floors were
  measured on.

</details>

## [v0.17.0] — 2026-10-01

<details markdown="1">
<summary>pyq parity flags · JSONC/JSON5 input · pyq diff/merge · automated GitHub Release</summary>

### Added

- **pyq CLI parity flags** — `pyq fmt` gains `--indent N` (block indent, default 2), `--width N`
  (plain-scalar soft-wrap column, 0 disables), `--sort-keys` (serializer-level whole-document key
  sort) and `-i/--inplace` (rewrite the file), exposing the `pyrs-yaml-core::SerializeOptions` knobs
  and matching the Python CLI's `fmt --indent`. `pyq to-json` gains `--jsonc` / `--json5` dialect
  output (mutually exclusive), wiring the `pyrs-json` comment-preserving and JSON5-spelling writers
  (`to_jsonc_text*`, `to_json5_text*`) into the CLI.
- **pyq JSONC/JSON5 input dialects** — `--input jsonc|json5` joins the `Format` enum (auto-detection
  also keys off `.jsonc` / `.json5` extensions) and routes through the native `pyrs-json` dialect
  parsers, so comments and JSON5 spellings ride the AST; combined with `to-json --jsonc` this gives
  a comment-preserving JSONC→JSONC round-trip in one command. `--all-docs` rejects the
  single-document dialects with a stable message.
- **`pyq diff` / `pyq merge`** — semantic document comparison and right-biased deep merge (yq `*+`
  shape), new in the native CLI. `diff` walks both ASTs comparing resolved values, structure and
  tags (comments/quoting/layout never appear) printing `-/+ /~` path lines, exit 0 equal · 1
  differs. `merge` overlays mappings recursively, appends sequences (or swaps them with
  `--replace-arrays`), and emits round-trip YAML; both commands read any supported input dialect via
  `--input`/extension detection.

### Changed

- **GitHub Release is created by `publish.yml`** — the release page used to be made by hand
  (`gh release create`) after each publish, which is one more step to forget and one more place for
  the published version and the tag to drift apart. The `release` job now runs `gh release create`
  after `uv publish` succeeds, gated on the same `refs/tags/` condition, with generated notes and
  the built wheels attached — notes and assets come from the tag PyPI was published from.
  `workflow_dispatch` runs are unaffected (no release, no PyPI publish), matching the existing
  behavior.
- **`README.md` / `README.zh-CN.md` document the native `pyq` CLI** — both READMEs gained a `pyq`
  subsection next to the Python CLI section: how to install it from a checkout, three worked
  examples, and the full command list, pointing at the pyq guide for detail.

### Fixed

- **`pyrs-json` module documentation** — it still claimed comments were stripped on read and never
  re-emitted; since #122 they ride the AST comment slots and the JSONC/JSON5 writers reproduce them.

</details>

## [v0.16.0] — 2026-10-01

<details markdown="1">
<summary>strict `load_json` and outside value oracles · JSONC/JSON5 through the core, the package
and the CLI · TOML 1.1 and the refusals toml-test demanded · notes and spellings that survive a
round trip · the TOML/INI hub · `pyrs-yaml` and `pyq` · native cores · interpreter aborts · fast
paths</summary>

### Added

- **`load_json` (strict RFC 8259) completes the `load_*` family** — `pyrs_yaml.load_json(s)` accepts
  the strict grammar only and answers a typed `YamlParseError` for `//` and `/* … */` comments,
  trailing commas, single-quoted strings, bare `Infinity` / `NaN` and `0x…`. Its fast path shares
  `json_fast::try_load` with `load_jsonc`, which bails on every non-canonical byte, and what it
  declines routes through the STRICT `from_json` AST parser, never `from_jsonc`. Re-exported into
  `__all__` from `pyrs_yaml.__init__` with the `.pyi` regenerated, so every format the CLI reaches
  (`to/from json|jsonc|json5|toml`) has a `load_*` sibling.
- **What the engine answers is now graded by readers it does not own** — value parity with PyYAML
  and ruamel.yaml over 20 canonical YAML Core 1.2 documents × 5 parity surfaces − 2 documented
  divergences (duplicate-key strictness, YAML 1.1 legacy booleans schema-scope) = 122 comparisons;
  three-way pyrs / `tomlkit` / `tomllib` agreement on 24 comparisons of 11 canonical constructs,
  `>i64` pinned spec-strict (TOML v1.0 §Integers: 64-bit signed) and `-2^63` still passing through
  `tomlkit`; `pyrs_yaml.load_json(doc) == orjson.loads(doc)` on 16 documents. These libraries were
  timed side by side and never compared for values, and orjson is the authority because stdlib
  `json.loads` accepts `NaN` / `Infinity` under `allow_nan=True`.
- **JSONC and JSON5 reach the core, the package and the CLI** — `pyrs_yaml_core::json::from_jsonc`
  and `from_json5` sit on `from_json_with_options(text, JsonParseOptions)`, where `JsonParseOptions`
  toggles `allow_trailing_commas`, `allow_single_quoted`, `allow_unquoted_keys` and `allow_comments`
  one axis at a time and ships `STRICT`, `JSONC` and `JSON5` as the presets. JSONC takes `// line`
  and `/* block */` wherever whitespace is legal (the `tsconfig.json` / `settings.json` dialect) and
  nothing else — a strict superset of RFC 8259, with `from_json` still enforcing RFC 8259 — and
  `to_json_text` / `to_json_text_pretty` ignore comments even when the AST carries them, so
  preservation is opt-in. `pyrs_yaml.from_jsonc` / `load_jsonc` / `from_json5` / `load_json5` and
  `YamlDocument.to_jsonc()` / `to_json5()` route through the native engine rather than `json.dumps`
  — `pyrs_yaml.from_jsonc(...)` had raised `AttributeError` — `emit_root_leading` writes the root's
  `leading_comment` before the outermost `{`/`[`, and `pyq from-json` gained `--jsonc` / `--json5`.
- **JSON5 accepts the lexing and the numbers JSON refuses, and gives both back** — unquoted keys
  take the full Unicode `ID_Start`/`ID_Continue` set (`unicode-ident`, a new dependency); separators
  add vertical tab, form feed, NBSP U+00A0, every Unicode `Zs` space, U+2028/U+2029 and ZWNBSP
  U+FEFF to RFC 8259's four; strings add a backslash before a line terminator, `\'` in double
  quotes, `\v` and `\0`; and numbers add `0xDECAF`, `0XFF`, `-0x1F`, `.5`, `5.`, `+7`, `+.25`, `07`,
  `Infinity`, `NaN` and `-Infinity`, each keeping its exact source text on the plain scalar. All of
  it is gated on JSON5: strict `from_json` and `from_jsonc` still require those keys quoted and
  refuse every one of those bytes byte-for-byte unchanged, and a lone UTF-16 surrogate in `\uXXXX`
  stays refused, which a Rust `String` could not hold. `Schema::Json5` answers `0x1F` as `31` and
  `Infinity` as `float('inf')` on load, and `to_json5_text` / `to_json5_text_pretty` restore the
  spelling under one writer `Mode` (Json / Jsonc / Json5): single quotes on any scalar tagged
  `ScalarStyle::SingleQuoted`, the bare numeric tokens, comments normalized to `//`, keys always
  quoted.
- **TOML refuses the documents toml-test marks invalid** — the official corpus replays through a
  type-tag adapter onto toml-test's `{"type": ..., "value": ...}` form and is read as bytes, after a
  `read_text()` had rewritten bare CR to LF before the parser saw it. A numeric UTC offset is
  range-checked (hours 00..=23, minutes 00..=59), so `+12:60` / `+24:00` fail; C0 control codes and
  DEL U+007F are refused inside basic, literal and multi-line strings — only tab, and newlines in
  the multi-line forms, are legal — and in comment bodies, and a bare `0x0D` outside a CRLF pair by
  a central scan at the parse entry; a table created implicitly by a dotted key is closed to a later
  `[header]`, `[tbl]` then `[[tbl]]` fails, invalid under BOTH 1.0 and 1.1, and legitimate implicit
  super-tables still parse (30 → 21); leading zeros (`01`, `007`, `-01`, `01.5`), a sign on a
  radix-prefixed integer (`+0x1F`, `-0b101`, `+0o644` — `signed-int` wraps only a decimal integer)
  and edge or doubled underscores (`1_`, `1__0`, `0x1_`) are refusals now (71 → 48); and an inline
  table rejects a dotted key that equals, extends or is shadowed by a defined path
  (`{ a = 1, a.b = 2 }`, `{ a.b = 1, a = 2 }`) while `{ a.b = 1, a.c = 2 }` stays legal (48 → 39).
- **TOML 1.1.0 grammar, and its date-times finally decode** — `from_toml` parses the v1.1.0 spec
  released 2025-12-18: inline tables may span lines and end with a trailing comma
  (`{\n a = 1,\n b = 2, \n}`), `\xHH` covers codepoints 0x00..=0xFF, `\e` spells U+001B (ESC), and
  seconds are optional, so `t = 14:15` and `dt = 2010-02-03 14:15` round-trip byte-stable.
  `TomlDialect::V1_0` and `from_toml_v1_0` remain for consumers pinning 1.0.0, every 1.0.0 document
  parses identically under both, and the `space_time_sep` off-by-one that left `T`-less date-times
  unrecognised in 1.0 mode is fixed. Date-only (`1979-05-27`) and time-only (`07:32:00`) now carry
  distinct `!date` / `!time` tags (local and offset date-times keep `!timestamp`) and route to
  `date.fromisoformat` / `time.fromisoformat` instead of raising a raw `ValueError` on *valid* TOML;
  `!time` pads omitted seconds for the 3.8 floor, `!timestamp` canonicalizes lowercase `t`/`z`, both
  normalize a fractional field of any precision (TOML allows `.6`; `fromisoformat` pre-3.11 takes 3-
  or 6-digit) to microseconds, and `...Z` becomes `+00:00` because `datetime.fromisoformat`
  recognizes `Z` only from 3.11.
- **A TOML round trip keeps its notes, shapes and spellings** — the parser captures the standalone
  `# ...` above a pair or header and the inline `key = value # ...` / `[name] # ...` note onto
  `NodeMeta::comment`, and `to_toml(from_toml(src))` re-emits them in place, so `pyq edit` and
  `YamlDocument.set()` no longer strip annotation notes; a document-opening `# note` survives too,
  and a comment-free document emits no stray line. Interior inline-table notes land on the member's
  own slots, undecorated tables keep the compact `{ a = 1, b = 2 }` line, a decorated table nested
  in an array promotes to the multi-line inline form, a decorated top-level table to a `[section]`,
  and the repair exposed `skip_all_blank`, which counted a standalone comment's own terminating
  newline as a blank line. A multi-line string (`"""…"""` or `'''…'''`) is projected as a
  `ScalarStyle::Literal` YAML block scalar and re-emitted as a `"""` block instead of one escaped
  line, while a single-line `"a\nb"` stays single-line; `to_toml` reproduces the source's blank-line
  groupings (`a = 1\n\nb = 2` is byte-stable) except before a document's first pair, and hex
  (`0xDEADBEEF`), octal (`0o755`) and exponent floats (`1e10`, `-3.14e-2`, `6.02E23`) keep their
  radix prefix while underscore separators, an explicit `+` and binary (`0b101`) canonicalise to
  decimal, because YAML Core has no rule for them.
- **One standalone note, whichever engine wrote it** — `NodeMeta` grows
  `leading_comment: Option<Comment>` and `blank_before: bool` (both excluded from structural
  `Hash`/`PartialEq`), so a section header or array-of-tables element carries the note above it
  *and* the inline note after `]`, and a JSONC member carries both (`// ...` above onto
  `leading_comment`, `... value // trailing` onto `comment`) — the shape the single-slot model could
  not express. The granit-parser receiver is the last engine moved onto
  `NodeMeta::decor.leading_comment` rather than `comment(standalone = true)`, and `NodeMeta::eq` /
  `Hash` treat one concept wherever it sits, so parsed YAML still compares equal to every hand-built
  fixture. `CustomNode::set_leading_comment` / `remove_leading_comment` / `remove_comment` clear
  both slots atomically, the writers fall back to `comment` with `standalone = true` so hand-built
  nodes render unchanged and strict JSON still emits no `//`, `to_yaml(toml_ast)` /
  `to_yaml(json_ast)` keep their leading notes, and Python gains `Node.leading_comment` beside
  `Node.remove_comment()`.
- **TOML and INI join the hub, and a document writes TOML itself** — `from_toml(toml: str) -> str`
  and `to_toml(yaml: str, schema=...) -> str` convert TOML ⇄ YAML text,
  `load_toml(toml: str) -> dict` materializes Python values with datetimes through the built-in
  `!timestamp` plugin and no re-resolution of TOML strings (a `"true"` value stays a string), and
  `load_ini(text: str) -> dict` reads INI through stdlib configparser — strict, case-preserving,
  read-only by design because INI has no official grammar. TOML output refuses inexpressible shapes
  (null values, non-table roots, aliases, non-scalar keys) with stable messages, and round-trip
  editing of comments, anchors and splices stays YAML-only by design; `YamlDocument.to_toml()` emits
  from the AST in one native call rather than `to_toml(doc.to_yaml())`, byte-identical and ~4.3x
  faster than `tomli_w`.
- **Optional types, an optional settings source, and a missing export** — `!duration`
  (`pendulum.Duration`), `!arrow` (`arrow.Arrow`) and `!ulid` (`ulid.ULID`) auto-register when their
  library is installed, each on a distinct tag so `!timestamp` / `!date` / `!uuid` are unaffected
  and a stdlib `timedelta` is never matched by `!duration`. `PyrsYamlConfigSettingsSource` drops in
  for `pydantic_settings.YamlConfigSettingsSource`, parsing with pyrs-yaml's YAML 1.2 core schema
  instead of PyYAML and exported lazily so `import pyrs_yaml` never requires pydantic-settings
  (`pip install "pyrs-yaml[settings]"`, Python 3.10+); `dump_pydantic` and `parse_as` use the same
  module-level `__getattr__` pattern. `YamlStream` — the class `YAML().load_stream*()` always
  returned — is now importable.
- **`pyrs-yaml`, the command line** — `pip install "pyrs-yaml[cli]"` (Python 3.10+) adds 18 commands
  in pure Python on Cyclopts, so the base install keeps zero extra dependencies and Python 3.8
  support: `fmt` (round-trip reformat preserving comments, anchors and order), `get` (JSONPath with
  `--format yaml|json|text`), `set` / `delete` / `rename` (`--inplace`, `--string`,
  `--create-missing`), `sort-keys`, `move` (to an existing destination), `frontmatter` (Markdown
  front matter, optional body split), `validate` (CI-friendly exit codes, mutually exclusive
  `validate --schema <name>` vs `--schema-file <path>`), `compliance` (YAML Test Suite report,
  `--json`) and `to-json` / `from-json`, all reading stdin through `-` and defaulting to stdout.
  `to-toml` / `from-toml`, `to-jsonc` / `from-jsonc` and `to-json5` / `from-json5` mirror `to-json`
  / `from-json`, and `-A/--all-docs` covers every edit and conversion verb, where a move applies to
  each document in which both paths resolve.
- **`pyq`, a native Rust CLI with no Python at runtime** — `crates/pyrs-yaml-cli` (clap-based,
  `cargo install --path crates/pyrs-yaml-cli`) puts `pyrs-yaml-core` behind a jq/yq-style command
  line: `fmt`, `get <path>` (JSONPath-lite — dot keys, `[n]`/`[-n]`, bracket keys, `*`;
  `--json`/`--raw`), `set` / `delete` (`--create-missing`, `-i/--inplace`) re-serialized so comments
  and the value's style survive, `rename` / `move` / `append` / `insert`, `sort-keys` at any path
  (`$` for the root), `validate` as a parse check or against schema-language rules via
  `--schema rules.yaml`, `frontmatter --body-out`, `to-json` / `to-toml`, the importers `from-json`
  / `from-toml` / `from-ini`, formats resolved by extension (`--input` overrides), stdin via `-`,
  and a non-zero exit carrying the core's stable error text.
- **`pyq`'s filters are flags, not an expression language** — `--select 'PATH OP LITERAL'`,
  `--sort-by PATH` / `--desc`, `--unique`, `--first` / `--last`, `--skip N` / `--take N` and
  `--join SEP` run in the fixed order `select -> sort -> unique -> slice` then `join` — flags, not
  an expression language, so mixed comparison kinds answer `false` (a documented divergence from
  jq); `pyq completion` prints a bash, zsh, fish or PowerShell script through `clap_complete`.
  `-A/--all-docs` spans every edit command and `to-json -A`, each document splicing against its own
  stream segment (`MultiDocEditor` + `DirtyUnit::shifted`) so untouched documents and every `---`
  separator keep their exact bytes; wiring it surfaced `move_path` returning only the destination
  INSERT unit, which duplicated the moved subtree.

### Changed

- **granit-parser 1.1 → 1.3, and the strictness the upgrade quietly cost** — 1.2.0 added optional
  `Options` fields for exotic-document limits, 1.2.1 tightened a few parse results toward the YAML
  spec, and 1.3.0 added two defaulted `Input` methods (`fetch_block_scalar_line`,
  `take_quoted_scalar_ascii_chunk`) that let the scanner step over block and quoted scalar bytes
  faster; the project consumes the parser through `Parser::new_from_str` and implements only
  `EventReceiver` / `SpannedEventReceiver`, never `Input`, so no source change was required. One
  verdict did move: 1.3 began *accepting* a multi-line flow collection whose continuation line is
  indented no further than its enclosing block key (yaml-test-suite `9C9N`: `flow: [a,` then `b,` at
  column 0), taking strictness `405/406 → 404/406` — inside the suite's ≥95% threshold, so CI
  reported green. An in-tree post-parse guard now rejects the under-indented continuation, restoring
  `405/406`, and reads only spans the parser already computes.
- **Native JSON and TOML cores; `serde_json` and `toml_edit` are gone** — `pyrs-yaml-core` ships an
  RFC 8259 engine (byte-level scanner, verbatim number spelling so `from_json → to_json` is
  byte-stable and precision-preserving for large integers and doubles, typed line/column errors, and
  refusals for trailing commas, leading zeros, lone surrogates, unescaped control characters and
  multi-root documents) and a TOML 1.0 engine over the full grammar — bare/quoted/dotted keys,
  basic/literal/multiline strings with every escape form, decimal / hexadecimal / octal / binary
  integers with underscore separators, floats including `inf` / `nan` and exponent forms, and the
  offset and local date, time and date-time — every rejection surfacing as `ParseError::Syntax` with
  0-indexed `line`/`col` in the granit-parser house style. The CLI, bindings and packaging surface
  are unchanged.
- **`YamlDocument.validate()` caches compiled validators** — the first successful validation against
  a schema, JSON text *or* dict, keeps its compiled `jsonschema` validator, so later calls skip
  schema parsing, meta-schema checking and validator construction; dict schemas key on object
  identity behind a deep-copy snapshot guard that notices an in-place mutation by `==` and
  recompiles transparently. The cached path raises
  `exceptions.best_match(validator.iter_errors(instance))`, identical to `jsonschema.validate()`
  semantics. WSL wall-time: `document_validate` −98% (660µs → 13µs).
- **Documentation corrected where it disagreed with the shipped build** — the numpy guide's 0-D
  section claimed 0-D arrays "reshape to a single-element list" (`assert data == [42]`) where the
  shipped behaviour serializes a bare scalar (`assert data == 42`), corrected in
  `docs/{en,zh,ja,ko}/guides/numpy.md` with a warning for the 0-D `bool` → `1.0` rust-numpy quirk.
  Across the locale pages `saphyr-parser` became `granit-parser`, YAML compliance 98.1% became
  99.75% (405/406 cases), ABI3 support 3.9–3.13 became 3.8–3.15, the benchmark tables took the
  current CodSpeed numbers (parse 21–43×, serialize 55–177× faster than PyYAML), and the Rust
  benchmark sections moved from Criterion to divan.

### Fixed

- **Hostile text stopped aborting the interpreter** — the fixed-width escape readers sliced
  `&self.text[pos..pos+width]` by byte offset assuming one byte per character, so a JSON `\u` or
  `\x`, or a TOML `\xHH` / `\uXXXX` / `\UXXXXXXXX` escape followed by a multibyte character, ended
  the slice mid-character and panicked the process; the readers now slice the byte buffer and
  UTF-8-validate, so a malformed escape returns a clean parse error, and the TOML basic and
  multi-line basic string parsers — which advanced byte-wise and could slice a `&str` mid-character
  on U+00A0 or U+0251 — consume whole characters. `0xC00000FD` STACK_OVERFLOW was the other abort:
  the TOML parser carried no nesting budget (unlike JSON's `DEFAULT_MAX_DEPTH` and YAML's `parse`
  `max_depth`), so `parse_value` → `parse_array` and `parse_inline_table` recursed without limit and
  `load_toml("a = " + "["*5000 + "]"*5000)` killed the interpreter with no Python exception. A
  `depth` counter now answers a typed `ParseError::MaxDepthExceeded` past 1000, mirroring JSON.
- **`<<` merges only when its source can merge, so a literal `<<` key survives** —
  `safe_dump({"<<": None})` → `<<: null` → `load` returned `{}` and lost the key (same for `<<: 1`,
  `<<: "x"`, or `<<:` beside other keys), because the resolver treated every `<<` as a merge and
  consumed a Null or scalar source while merging nothing. Per YAML the key is a merge only for an
  alias-to-mapping, an inline mapping, or a sequence of those, so a Null or plain-scalar `<<`, and
  an alias-free `<<` that yields nothing (`<<: []`, `<<: [1, 2]`, `<<: {}`), is an ordinary key and
  now survives the round trip, while alias-bearing paths are always consumed — `<<: [*a, *a]` cannot
  re-expand. Compliance stays `405/406`.
- **Anchor guards are path-scoped, so a self-merge terminates and repeated aliases resolve** — a
  `<<` whose expansion points back at its own anchor (`a: &a` containing `b: {<<: *a}`) expanded
  forever inside `resolve_merge_keys` and killed the interpreter (Windows exit `0xC00000FD`),
  because the cycle guard applied only while *collecting* merged pairs and never while *walking* the
  expansion. An anchor's name now stays on the recursion path while its expansion is walked, a merge
  resolving back to an ancestor on that path ends as an empty expansion, and — since an acyclic AST
  cannot carry PyYAML's cyclic dict — a self-merge bottoms out at `{}`. The same pass settled two
  further semantics: `<<: {x: 1}` now merges, and a non-alias element in a merge sequence
  (`<<: [*a, {y: 2}]`) keeps its inline map. `to_dict()` had the mirror-image defect: aliases
  expanded behind a *global* visited-anchor set that was never cleared, so only the first reference
  produced a value and every later one degraded to `None` (`{a: &x {p: 1}, b: {q: *x}, c: {q: *x}}`
  gave `b` a value and `c` a `None`); the guard is pushed for one expansion and popped after, and
  genuine cycles still terminate.
- **NumPy serialization no longer reads Python memory without the GIL** — the ndarray writer
  borrowed the array's buffer through `unsafe { as_slice() }` and iterated that borrow *inside*
  `py.detach`, after releasing the GIL; `&[T]` is `Send` whatever its provenance, so the borrow
  checker could not see that Python owns that memory and another thread may resize or write the
  array underneath. The buffer is snapshotted into Rust-owned memory while the GIL is still held
  (`slice.to_vec()`) and only the scalar→node conversion runs off-thread. This was the only `unsafe`
  buffer borrow in the binding layer, and every other `py.detach` site was audited to touch
  Rust-owned state only (AST, source text, `BufWriter<File>`).
- **Notes that were dropped, misplaced, duplicated or left untrimmed** — a file-leading `// note`
  before a JSONC/JSON5 value classified as an *inline* note (no newline precedes offset 0 in the
  whitespace scanner's heuristic) was claimed by the first object member instead of the root
  container `emit_root_leading` had annotated, and vanished outright on an empty `{}` or a root
  scalar; the JSON/JSONC/JSON5 **and TOML** writers emitted comment bodies verbatim while every
  parser stored them trimmed, so `text = " "` oscillated trailing whitespace across passes; a
  comment-only TOML document (`# note` with no key to consume it) serialized the empty root to `""`;
  the YAML parser kept one shared comment slot for all in-progress containers, so a nested container
  start clobbered a standalone header note before it reached the AST — slot handling is a
  per-container stack now; and a splice whose regenerated text carried the pair's own standalone
  note skipped the old note line so both survived, which the widened range over note lines no longer
  does. All five formats now reach a byte-stable leading-comment fixed point; a hand-built non-empty
  TOML root carrying both a standalone and a first-element note renders as two adjacent `#` lines no
  TOML text can re-attribute, a shape no real parse produces.
- **The serializer emits only re-parseable YAML, at any depth** — a literal or folded scalar under a
  nested key (`a:` then `b: |` then body) emitted its body one fixed indent step from column zero
  instead of one step below the `b: |` header, so every nested shape serialized text that re-parsed
  as an error or a wrong value, and the scalar writer now threads a `block_base` (the parent line's
  column) through every emission site, text-exact for all seven nesting shapes. Five spellings fell
  to the same gate: a mapping value forced onto its own line pre-emitted the child's anchor/tag even
  where the child (scalars, nulls, flow containers) writes its own header, double-stamping `A: !a`
  then `!a null`; block scalars inside flow collections (`[|`, `{k: >}`) or in key position demote
  to double-quoted, since flow syntax has no room for an indented body; a flow container starting
  its own line lost its indent and a complex key (`? …`) emitted its value marker `:` at column
  zero, closing any enclosing collection; a complex key whose body carries a standalone comment or
  tag serialized ambiguous text (`? # c` mid-line), so the note moves above the `?` marker and the
  body gets its own lines one step deeper; and plain scalars with edge whitespace (`" %"`, `" "`) or
  an embedded flow indicator (`,` `[` `]` `{` `}`) inside a flow collection are quoted. Tagged empty
  block containers fold their header onto the `{}` / `[]` line, and compact `- key: value` items
  refuse to inline a value carrying a standalone comment.
- **TOML accepts the legal minimum i64** — `from_toml` / `load_toml` failed on
  `-9223372036854775808` (`i64::MIN`, in range by TOML definition): the signed path stripped the
  `-`, read `9223372036854775808` as an unsigned i64 and overflowed before negation could run. The
  sign now parses with the digits (`i64::from_str` accumulates negatively), signed floats negate
  with their exponent spelling preserved, and `pyrs_yaml.load_toml("a = -9223372036854775808")`
  returns `{"a": -(2**63)}`. It came out of property tests of TOML/JSON/JSONC/JSON5 against the
  stdlib `json` / `tomllib` / `pyjson5` oracles with type-strict equality, which also pin the two
  known AST-ambiguity spellings: bare JSON5 `Infinity`/`NaN` literals and >i64 digit strings a
  writer emits verbatim by fidelity contract.

### Performance

- **`safe_load` builds Python objects in one pass over the event stream** — `safe_load`,
  `safe_loads` and `YAML().safe_load*` no longer construct a full `CustomNode` AST only to walk it
  again in `convert.rs`; plain scalars resolve through the same schema chain, mapping keys keep
  their raw-text semantics and duplicate-key errors report identically, while anchored, tagged,
  merge and multi-document shapes stay on the AST pipeline behind zero-cost pre-vetoes (a `&` byte
  scan, a separator line-scan, a payload-level `<<` check). WSL wall-time: `safe_load_scalar_types`
  −21~25%, the `safe_load` family −13~18%, fallbacks unchanged. The event-span anchor-replay variant
  was implemented, measured at +48% on anchored docs and reverted.
- **JSON stays on its fast path in both directions** — `load_jsonc` parses canonical strict JSON
  (objects and arrays, `i64` integers, booleans, `null`, escape-free strings) straight into Python
  objects, skipping the intermediate `CustomNode` AST: ~5-6x faster than the AST route and ahead of
  stdlib `json.loads`, with any non-canonical input (floats, escapes, comments, out-of-range
  integers, trailing commas, invalid tokens) falling back to the general path, so values and error
  reporting are unchanged. Floats (a correctly-rounded parse identical to CPython's `float`) and the
  eight simple two-byte escapes now join it — ~15x faster on a log-style payload, with a `\u` or any
  invalid escape still on the AST path. `YamlDocument.to_json` uses the native engine instead of
  `to_dict()` + `json.dumps`, byte-identical for ASCII and ~10x faster (a 1200-item document drops
  ~1450µs → ~120µs), mapping keys go straight into the output buffer rather than a `String` per key
  through `key_text` (~60µs, about 2x again), and a string needing no escaping is bulk-copied in one
  `push_str` (~35%, 41 → 27 ns/item, byte-identical). JSON serialize measures #2 against the
  installed field, behind only orjson. One behaviour change: `to_json` emits non-ASCII as raw UTF-8,
  matching `to_jsonc` / `to_json5`, rather than `\uXXXX` escapes — still valid JSON that
  `json.loads` parses the same.
- **The containers, receivers and error contracts were single-sourced** — mapping and sequence
  rendering share one `write_container_node` skeleton (byte-identical output, `serialize_*` medians
  −5~11%), the parse entries share one `load_ast` error contract, the schema chains share
  `bool_word`/`numeric_tail`, `register_anchor` and the standalone-vs-inline taxonomy
  (`is_standalone_placement`) are single-sourced across the receivers, path-edit methods delegate to
  `apply_metadata_edit`, and the duplicate rate moved 5.25% → 3.45% → 2.60%.
- **Readers skip the work they cannot need** — `extract_anchors` returns empty after a single `&`
  byte-containment check, so anchor-free documents skip the per-character quote state machine
  (`parse_large` 31.2µs → 26.3µs, the scan 1.5µs → 38ns, Rust `parse_*` medians 11–18%); the fixed
  keys `parse_stream` / `load_stream` emit per event (`line`, `column`, `type`, `value`, `style`,
  `anchor`, `tag`) reuse interned objects via `pyo3::intern!` (WSL: `parse_stream` −34%,
  `parse_stream_multidoc` −39%, `load_stream` −22%); `on_document_end` moves the finished document
  instead of deep-cloning it (`parse_all_docs` −9.7%, multi-doc `safe_loads` −9.5%,
  `YAML().safe_loads` −6.7%); `direct_dump_into` writes each document into a reused `String` and
  `dump_iterable` skips `normalize_doc`'s re-copy when the text ends in one newline
  (`dump_stream_multi_doc` −27.2%, `dump_stream` −4.4%); `unescape_double_quoted` returns early for
  backslash-free quoted strings and `detect_chomping` pulls lines lazily (`to_dict` family −4~9%,
  `safe_load_scalar_types[strings/numbers]` −3~4%); and the serializer's indent cache now serves
  block-scalar header writing.

</details>

## [v0.15.0] — 2026-08-19

<details markdown="1">
<summary>Node metadata and style API · schema file IO · deep editing · numpy back on
cp314t</summary>

### Added

- **Node metadata setters/getters** — `Node.comment` / `Node.anchor` / `Node.tag` read properties
  and `set_comment` / `set_anchor` / `set_tag` (plus `remove_*` variants) on
  `python/pyrs_yaml/node.py`, backed by new path-based edit operations in `py/editing/mod.rs` and
  `#[pymethods]` (`_set_comment_path`, `_set_tag_path`, `_set_anchor_path`, `_remove_*_path`,
  `_get_comment`, `_get_anchor`, `_get_tag`). Editing an alias or a missing path raises; standalone
  comments on inline scalar values and sequence items are now serialized on their own indented lines
  (fixes pre-existing broken round-trip for `child:\n  # c\n  val` and `- a\n# c\n- b`).
- **Node style/format setters/getters** — `Node.scalar_style` / `Node.flow_style` / `Node.chomping`
  read properties and `set_scalar_style` / `set_flow_style` / `set_chomping` methods, backed by
  `#[pymethods]` (`_set_scalar_style_path`, `_set_flow_style_path`, `_set_chomping_path`,
  `_get_scalar_style`, `_get_flow_style`, `_get_chomping`). ScalarStyle/Chomping now derive `Copy`.
  Non-scalar nodes return `None` / are no-op; aliases and missing paths raise.
- **Verbatim tags** — `set_tag("!<tag:yaml.org,2002:str>")` now produces a verbatim tag (empty
  handle), and verbatim tags parsed from source survive round-trip: `Tag`'s `Display` emits `!<...>`
  wrapping for empty-handle tags, `parse_tag` recognizes the `!<...>` form, and stream events
  serialize tags through `Display`.
- **Schema file IO and listing** — `load_schema(name, path)` reads a schema definition from a file
  and registers it; `list_schemas()` returns all registered schema names (built-in
  `failsafe`/`json`/`core`/`yaml1.1` plus custom). Exposes the existing `registry::names()` and
  wraps `register_schema` with file I/O.
- **Schema structural validation** — a `validate` section in a schema definition adds structural
  checks (path-qualified scalar types, `sequence_of`/`mapping_of` containers, `required` presence);
  the new `validate_against_schema(data, schema_yaml)` raises `YamlValidateError` listing every
  failure.
- **`Node.copy()`** — deep-copies a subtree as a standalone Python value (dict/list/scalar),
  detached from the document, for pasting via `set_value()`.
- **Deep editing API** — `doc.set_many({"$.path": value})` sets multiple paths (with wildcard `[*]`
  and deep-scan `..` support) in a single splice burst; `doc.sort_keys()` orders mapping keys in
  place; `Node.move(new_path)` relocates a subtree atomically; `Node.path` / `Node.find_first()` /
  `Node.value_eq()` add path access, first-wildcard lookup, and value comparison. Rust primitives:
  `sort_keys_path` / `move_path` / `set_many_path` + `apply_batch_edit`.
- **Property-based testing for 0.14+ features** — Rust proptests for `validate_node` (no panic,
  error paths exist), schema parsing, and style-settings round-trip; Python hypothesis tests for
  `set_many` wildcard equivalence, metadata-edit value preservation, and `sort_keys` idempotency.
  `hypothesis` moved to the `test` dependency group so CI (`uv sync --group test`) actually runs
  property tests.
- **Serializer fix** — standalone comments on **empty** flow containers (`key: {}` / `key: []`) no
  longer serialize to invalid YAML; they are demoted to inline comments (`key: {}  # note`).

### Changed

- **NumPy re-enabled on free-threaded (cp314t) wheels** — the `--no-default-features` flag is
  removed from the cp314t build lines in `publish.yml` and `ci.yml`; rust-numpy 0.29 (already
  pinned) supports free-threaded Python since v0.24.0, so `numpy.ndarray` serialization is now
  available on free-threaded wheels when NumPy is installed (auto-detected at runtime; inert when
  absent). Closes the deferred Research & Exploration item "Numpy free-threaded re-enable".
- **granit-parser upgraded to 1.1.0** — the YAML 1.2 parser dependency moves from the `1.0` to `1.1`
  semver line. Backward-compatible maintenance release: adds `Options`/`emit_comments`
  configuration, new `new_from_*_with_options` parser constructors, fuzz-testing hardening, and a
  performance fix for validating large plain/block scalars (ASCII fast path). No API changes
  required in `pyrs-yaml-core`.

</details>

## [v0.14.1] — 2026-08-15

<details markdown="1">
<summary>quoting and escape edges: single-quote backslash, BOM, noncharacters, multibyte
wrap</summary>

### Fixed

- **Single-quoted scalars with backslash + control/noncharacter** — a quoted value containing a
  backslash was routed to single-quoting, but single quotes cannot escape control characters or
  Unicode noncharacters, so the emitted YAML was unparseable. Such values now use double-quoting
  (`direct_dump` and the shared `write_plain_scalar` single-quote branch).
- **Noncharacters and BOM quoted** — `needs_quotes` / `needs_double_quoted` now treat Unicode
  noncharacters (U+FFFE/U+FFFF and the plane-end twins) and U+FEFF (BOM) as requiring quoting:
  granit drops a plain U+FEFF as a document-start BOM and rejects raw noncharacters even inside
  quoted scalars.
- **Double-quoted escape width** — `write_double_quoted_scalar` escapes noncharacters and control
  chars; for code points above U+FFFF it now emits the 8-digit `\Uxxxxxxxx` form (the 4-digit `\u`
  form is only valid for the BMP).
- **Folded plain-scalar continuation indent** — `wrap_plain_scalar` continuation indent is no longer
  a fixed 2 spaces; it is derived from the value's start column on the current line, so folded plain
  scalars inside nested sequence/mapping items stay indented past the parent block indent (granit
  otherwise reports "simple key expected ':'").
- **Multi-byte wrap boundary** — `wrap_plain_scalar` now floors the wrap slice to a char boundary
  instead of panicking when a 4-byte UTF-8 character straddles the wrap column.
- **`hypothesis` in publish test requirements** — `.ci/requirements-test.txt` now pins
  `hypothesis>=6.113.0` so the publish workflow (which does not install the `dev` dependency group)
  can run the property test suite.

### Added

- **`scripts/fuzz_panics.py`** — high-volume local Hypothesis fuzz harness that bypasses pytest
  `@settings` caps with a hostile strategy (control chars, NBSP, backslashes, long multibyte runs)
  across dump/parse/edit/idempotency.

</details>

## [v0.14.0] — 2026-08-14

<details markdown="1">
<summary>YAML Schema language and pluggable resolution · quoted scalars · empty
collections</summary>

### Added

- **YAML Schema Language** — define custom schemas as YAML files with a `rules` list mapping regex
  patterns to YAML types (`null`/`bool`/`int`/ `float`/`str`), plus an optional `extends` base
  schema. Registered via `register_schema(name, schema_yaml)` and used as `YAML(schema=name)`.
- **Inline dict schema** — the `schema` parameter of `YAML()`, `parse()`, `parse_file()`,
  `parse_all_docs()`, `safe_load()`, and `safe_loads()` accepts an inline dict, serialized and
  registered automatically.
- **Community Plugins** — `CustomType` base class with `can_parse`/`from_yaml`/`to_yaml`/`validate`
  methods; register via `register_type()` (imperative or decorator). Custom types handle tagged
  scalars on load and Python objects on dump.
- **Built-in plugins** — `!timestamp` (maps to `datetime`) and `!set` registered by default in
  `pyrs_yaml/plugins/`.

### Changed

- **Schema resolution is pluggable** — `YamlSchema` enum refactored into a `SchemaResolver` trait +
  `Schema` enum with a global `SchemaRegistry` pre-loaded with the four built-in schemas
  (`failsafe`, `json`, `core`, `yaml1.1`). Custom schemas register via the registry; built-in Core
  keeps its zero-cost `match` dispatch.
- **`node_to_pyobject` and `direct_dump` check registered `CustomType`s** — tagged scalars convert
  via `from_yaml()` on load; matching Python objects serialize via `to_yaml()` on dump.
- **`get()` is literal-key only** — `YamlDocument.get()` no longer guesses JSONPath for keys
  containing `.` or `[`; every key is treated as a top-level mapping key, consistent with
  `__getitem__`/`__setitem__`. Path access stays available via `find()`/`node()`.

### Fixed

- **Quoted scalars always load as strings** — implicit type resolution now applies only to plain
  scalars (YAML 1.2): `safe_load('"true"')` returns the string `"true"`, not `True`. The serializer
  keeps negative numbers round-tripping through the document (`to_yaml`) path.
- **Lone-quote keys round-trip** — mapping keys that are a single `'` or `"` are emitted as quoted
  scalars instead of unparseable YAML.
- **Empty collections emit `{}`/`[]`** — dumping empty mappings/sequences no longer yields an empty
  document that re-parses as `None`.

</details>

## [v0.13.0] — 2026-08-10

<details markdown="1">
<summary>MSRV 1.96 and edition 2024 · direct writer and load fast paths · granit migration</summary>

### Changed

- **Rust MSRV raised to 1.96 and edition bumped to 2024** - both crates now declare
  `rust-version = "1.96"` and `edition = "2024"`; CI pins the `build`/`test-freethreaded` jobs to
  Rust 1.96 for deterministic wheel builds and adds an `msrv-check` job running
  `cargo check`/`cargo test` at the MSRV to prevent silent MSRV drift (the `rust-lint` job stays on
  `stable`). The floor is set above PyO3 0.29's own baseline (rustc 1.83) for std API headroom (e.g.
  `assert_matches!`, stabilized 1.96) with no code migration needed. `TAG_REGISTRY` (tag handler
  storage) refactored to `std::sync::LazyLock`, dropping the `Mutex<Option<...>>` indirection.

### Performance

- **`safe_dump` / `from_dict` / `dump_file` / `dump_iterable`: direct writer** — Python→YAML
  serialization without intermediate `CustomNode` AST. Single-pass `direct_dump` replaces the old
  two-pass `pyobject_to_node` + `to_yaml`. 7x faster on `safe_dump` (28ns→4ns), 6x faster on
  `from_dict` (35ns→6ns). (#60)
- **`safe_load` / `safe_loads` / `to_dict`: fast-path skip anchor tracking** — when input has no `&`
  characters, skip `collect_anchors` + anchor resolution and use the simpler
  `node_to_pyobject_simple` path. (#59)
- **`resolve_core_type`: first-byte dispatch whitelist** — non-numeric/ non-boolean first bytes
  return `Str` immediately, avoiding schema resolution overhead for the common case. (#59)
- **granit-parser migration** — saphyr-parser replaced with granit-parser 1.0.1 for native
  `Event::Comment` emission, eliminating the full-text `scan_yaml()` pre-scan. parse_small -18%,
  parse_large -21%, roundtrip_large -18%.

### Fixed

- **`float_to_yaml_string` round-trip fix** — appends `.0` when Rust Display drops the decimal (`42`
  → `42.0`) so floats round-trip as floats instead of becoming ints.
- **Reverted `count_nodes` pre-allocation** — the full AST traversal cost more than the
  reallocations it avoided (serialize_10mb was ~14% slower); buffer growth is left to the Vec.

### Added

- **`max_depth` on stream & frontmatter APIs** — `parse_stream(yaml, on_event, max_depth)`,
  `read_markdown(path, schema, max_depth)`, `read_markdown_str(content, schema, max_depth)` accept
  `max_depth` (default 1000). Stream parsing now enforces the nesting-depth limit via core
  `parse_stream_with_options` (previously stream events had no depth limit).
- **Pydantic integration** — `dump_pydantic()` serializes a Pydantic model to YAML string via
  `model_dump(mode='json')` + `safe_dump`; `parse_as()` parses YAML string into a Pydantic model
  instance. Both use lazy imports, no hard dependency on pydantic. (#61)

### Internal

- **Split `py/mod.rs`** — monolithic 1786-line module broken into `document.rs` (YamlDocument),
  `yaml_instance.rs` (YAML class), `functions.rs` (module-level functions), `stream_iterator.rs`,
  `walk_helpers.rs`. `mod.rs` reduced to 128 lines. (#61)
- **`needs_quotes()` guard + `double_quoted_scalar()` constructor** — strings like `'true'` / `'42'`
  / `'null'` now emit as double-quoted scalars under the core schema instead of being misread on
  re-parse (`pyobject_to_node` + `json_value_to_node`).
- **CodSpeed benchmarks unified on `codspeed-divan-compat`** — `exclude-allocations` removes
  allocator noise; cross-library benchmarks consolidated into `tests/test_benchmark_crosslib.py`
  with shared `tests/data/yaml_samples.py` fixtures and streaming coverage.

</details>

## [v0.12.1] — 2026-08-06

<details markdown="1">
<summary>set(create_missing) · walk/scalars · monorepo workspace · parse hot paths,
Arc<str></summary>

### Added

- **`set(create_missing=True)`** - missing intermediate mapping keys along the edit path are created
  as nested mappings (e.g. setting `a.b.c` on `a: 1` creates `b` and `c`); index segments that miss
  are still an error, and a scalar intermediate along the path still raises.
- **`doc.walk()` / `doc.scalars()`** - Rust-backed depth-first AST traversal yielding `Node`
  objects, avoiding per-node `to_dict()` resolution. `walk()` returns all nodes; `scalars()` returns
  only scalar/null nodes.
- **Rust core module tests** - 39 new tests covering `editing::navigate` (key_eq, navigate,
  navigate_mut, normalize_index, mapping_key_index), `editing::region` (line helpers, node_is_flow,
  extend_delete_over_comments, nav_err), `editing::dirty` (DirtyKind/DirtyUnit constructors), and
  `editing::metadata` (with_metadata_from, needs_quoting).
- **Python doc.walk() edge case tests** - 9 new tests for empty doc, null values, deeply nested,
  flow collections, mixed types.

### Changed

- **Monorepo workspace** - source code split into `crates/pyrs-yaml-core/` (pure Rust, no PyO3) and
  `crates/pyrs-yaml/` (PyO3 bindings). Root `Cargo.toml` is now a workspace. Old `src/` directory
  and `build.rs` removed.
- **pyproject.toml** - added `tool.maturin.manifest-path` pointing to `crates/pyrs-yaml/Cargo.toml`.
- **Parse hot paths** - single-pass comment/anchor extraction, lazy duplicate-key detection,
  `shift_insert` merge prepending, and skipped `DocumentEnd` deep-clone for single-document parses
  cut large-document parse cost ~19% (CodSpeed: parse[large] +13.9%, parse[medium] +16.6%,
  roundtrip[large] +12.2%).
- **`Arc<str>` scalar storage** - `CustomNode::Scalar` and comment/event text share allocations via
  `Arc<str>`; AST nodes shrink 8 bytes and clones become refcount bumps instead of deep copies.

### Fixed

- **`set(create_missing=True)` nested chain build** - the created mapping chain no longer duplicates
  the first segment as a nested key level.
- **`set(create_missing=True)` eligibility** - freshly created keys are now eligible for the value
  write (the eligibility check no longer runs after the synthetic pair is inserted).
- **Standalone comments before simple mapping keys** - round-trip previously dropped standalone
  comments attached to simple-key nodes; now preserved (two regression tests).

</details>

## [0.11.7] - 2026-08-04

<details markdown="1">
<summary>release-guard static assertions replace the always-red stub check · numpy
tracking</summary>

### Changed

- **stub-build-check replaced with release-guard** - the always-red container build (`validate.yml`)
  that deliberately failed to reproduce the v0.10.0 `--generate-stubs` failure mode is replaced with
  three static assertions that **pass** when the repo is correct: `grep` guards `publish.yml`
  against `--generate-stubs`, `git ls-files` asserts the committed `.pyi` is tracked, and `test -f`
  checks `py.typed` exists. The job now gives green CI on correct state, red only on regression.

### Added

- **Numpy free-threaded tracking** - ROADMAP.md now tracks `rust-numpy` free- threaded support
  status (PyO3/rust-numpy#476) as a dependency for re-enabling ndarray serialization on cp314t
  wheels when the Rust binding matures.

</details>

## [0.11.6] - 2026-08-04

<details markdown="1">
<summary>cp314t wheels drop numpy · free-threaded CI validation · install docs</summary>

### Changed

- **Free-threaded (cp314t) wheels are now numpy-free** - built with `--no-default-features`, so
  rust-numpy is excluded entirely (smaller binary, no runtime probe). `safe_dump` on a
  `numpy.ndarray` raises `YamlTypeError` on free-threaded builds; GIL builds (Python 3.8-3.15) keep
  full ndarray serialization.

### Added

- **Free-threaded CI validation** - `test-freethreaded` job now builds and tests with
  `--no-default-features`, matching the shipped free-threaded wheel configuration.
- **Install docs** - `docs/{en,zh,ja,ko}` note that free-threaded wheels are numpy-free (ndarray
  serialization unavailable on cp314t).

</details>

## [0.11.5] - 2026-08-04

<details markdown="1">
<summary>parser robustness items closed by an audit that found no fixable case</summary>

### Changed

- **Parser robustness items 3/4/5 closed via Phase 0 strictness audit** — the 70-probe corpus
  (indentation, block-mapping keys, flow context) compared against a PyYAML oracle showed **no
  fixable accepted-but-invalid case** (64/70 match; the 6 divergences are deliberate YAML 1.2 /
  yaml-test-suite requirements where PyYAML is the outlier, and one deliberate duplicate-key
  strictness). Compliance stays at **99.75% (405/406)**. Full write-up in `ROADMAP.md` §v0.11.5 and
  `tests/test_strictness_audit.py`.

### Added

- `tests/test_strictness_audit.py` — 70-probe strictness regression corpus pinning current
  rejection/acceptance behavior (both directions), so future parser changes cannot silently regress
  strictness or over-reject.

</details>

## [0.11.4] - 2026-08-04

<details markdown="1">
<summary>duplicate empty keys accepted (2JQS) · compliance counts correct rejections · tab
decoding</summary>

### Fixed

- Duplicate null/empty mapping keys no longer error (`: a\n: b`, `~: a\n~: b`) — matches
  yaml-test-suite 2JQS; real duplicate keys still raise `YamlDuplicateKeyError`
- Compliance harness: correctly-rejected invalid YAML now counts as pass (was lowering the rate
  despite compliant behavior)
- Compliance harness: `convert_special_chars` tab decoding via regex — any run of `—`/`‖` + `»` is
  one tab, fixing tab-encoded suite cases

### Changed

- YAML Test Suite pass rate gate raised from >75% to **≥95%**; current rate **99.75%** (405/406)
- Known deviation documented: `ZYU8` (`%YAML 1.1 1.2`) is rejected by design (invalid per YAML 1.2
  grammar, matches PyYAML/libyaml)

</details>

## [0.11.3] - 2026-08-03

<details markdown="1">
<summary>streaming write · line-offset cache · publish pre-validation · compliance report</summary>

### Added

- Streaming write: `YAML.dump_stream(file_obj, iterable)` / `YAML.dump_file(path, iterable)` with
  document-level constant memory, auto `---` separators, and `explicit_start`/`explicit_end` flags
- `YamlDocument` `with` context manager: snapshot/rollback transaction scoping
- `compliance_report()`: public YAML Test Suite pass-rate reporting (version-consistent)

### Changed

- Edit-burst line-offset cache: internal O(N+edit) carry-through in the splice layer (public API
  unchanged)
- `compute_compliance` moved from tests to `pyrs_yaml.compliance`; version no longer hardcoded

### Fixed

- Changelog mirror drift guard: prek hook + CI job assert root/mirror `[Unreleased]` sync
- Publish stub pre-validation: CI reproduces v0.10.0-class `--generate-stubs` container failures
  before Release

</details>

## [0.11.2] - 2026-08-03

<details markdown="1">
<summary>parse skips splice eligibility · linear-cursor layout check (v0.11.0 regression)</summary>

### Added

- `YAML.load_stream(file_obj)` / `YAML.load_stream_file(path)`: lazy event iterators with O(anchors
    - chunk) memory

### Performance

- **Parse no longer computes splice eligibility** — the O(document) layout check now runs lazily on
  the first edit via `YamlDocument.splice_checked`, restoring the v0.11.0 regression: parse_comments
  -59%, parse_anchors -42%, parse/roundtrip/edit -10~35% all back to v0.10.0 levels
- **Linear-cursor layout check** — replaces per-node binary search over precomputed line offsets
  (monotonic source-order traversal)

### Changed

- `parse_with_options` returns `CustomNode` (was `(CustomNode, bool)`); splice eligibility is now
  internal to `YamlDocument` and computed on demand

</details>

## [0.11.0] - 2026-08-02

<details markdown="1">
<summary>surgical serialization: an edit writes back without re-laying-out the document</summary>

### Added

- **Surgical Serialization** — byte-level source span tracking on every AST node; segment-based
  splice — edits regenerate only the touched region, untouched text is byte-copied
- proptest fidelity property tests (new dev-dependency)
- 10MB edit-flush benchmarks (divan)

### Changed

- `flush_source` now splices segments; falls back to full serialization for flow-style regions,
  non-default layout documents, merged keys, CRLF/BOM documents, and after materialization
  (single-burst model)
- Splice edits preserve `---`/`...`/directive marker lines as untouched bytes (full serialization
  previously dropped them — deliberate behavior difference)

</details>

## [0.10.0] - 2026-08-01

<details markdown="1">
<summary>in-place editing API, with edit benchmarks</summary>

### Added

- **In-place editing** — edit parsed documents without losing formatting metadata:
    - Path API: `doc.set(path, value)`, `doc.insert(path, index, value)`, `doc.append(path, value)`,
      `doc.delete(path)`, `doc.rename(path, new_key)` with JSONPath-style paths (`$.a.b[0]`); root
      sugar via `doc["key"] = value` and `del doc["key"]`
    - Node API: `doc.node()` / `doc.find(path)` return `Node` objects with `set_value` / `append` /
      `insert` / `delete` / `rename`, plus tree traversal (`parent`, `children`, `walk`, `filter`)
    - Full metadata preservation — replaced scalars keep comment/anchor/tag/quoting; renamed keys
      keep position and comments; mapping order preserved on delete
    - Atomic edits — failed operations leave the document (and its revision) untouched
    - Lazy source re-sync — `source()` / `to_yaml()` / `reparse()` re-serialize only after a
      successful edit
    - Stale-node detection — `Node` access after a document edit raises `YamlDocumentError` (with
      `RuntimeWarning`)
    - New exceptions: `YamlEditError`, `YamlPathError` (i18n across en/zh-CN/ja-JP/ko-KR)
    - Alias-aware editing — setting an alias's own path replaces it in place; editing through an
      alias raises `YamlEditError`
- **Edit benchmarks** — 6 new divan benchmarks in `benches/yaml_bench.rs` (set/insert/delete on
  small–large documents)

### Changed

- `YamlDocument.source()` now returns `str` and lazily re-serializes after in-place edits

</details>

## [0.9.0] - 2026-08-01

<details markdown="1">
<summary>CPython 3.13/3.14/3.15 and no-GIL · tag handler registry · pydantic · .pyi stubs</summary>

### Added

- **Python 3.13, 3.14 and 3.15 support** — PyO3 `abi3-py38` wheel covers Python 3.8-3.15 (GIL
  build); `abi3t` + `abi3t-py315` provide free-threaded stable ABI
- **Free-threaded CPython (no-GIL) support** — `#[pymodule(gil_used = false)]` declares module as
  thread-safe for free-threaded Python; `Py_GIL_DISABLED` cfg flag gates numpy (rust-numpy has no
  free-threaded support yet — numpy feature must be disabled for free-threaded builds via
  `--no-default-features`)
- **CI free-threaded job** — new `test-freethreaded` workflow job validates compilation and tests
  against Python 3.14t
- **`pyo3-build-config` build dependency** — enables `#[cfg(Py_GIL_DISABLED)]`, `#[cfg(Py_3_15)]`
  etc. compiler flags via `build.rs`
- **`numpy` made optional** — feature-gated behind `numpy` feature (default enabled); excluded
  automatically under `Py_GIL_DISABLED`
- **`allow_duplicate_keys`** — `YAML(allow_duplicate_keys=True)`,
  `parse(..., allow_duplicate_keys=True)`, `parse_file`, `safe_load`, `safe_loads`, `parse_all_docs`
  all accept the flag; duplicate mapping keys raise `YamlDuplicateKeyError` by default,
  `last value wins` when allowed
- **`SerializeOptions` expansion** — `doc.to_yaml_with_options()` gains `width` (line wrapping, 0 =
  off), `indent_mapping`, `indent_sequence`, `indent_offset` alongside existing
  `indent_size`/`explicit_start`/`explicit_end`/`sort_keys`/`max_depth` (`src/py/mod.rs:432`)
- **Tag handler registry** — `register_tag("!custom")` decorator and imperative forms +
  `clear_tag_handlers()`; scalar nodes carrying a registered tag are transformed through the handler
  (`src/py/tag_registry.rs`)
- **Tag handler chaining with priority** — multiple handlers per tag run in ascending `priority`
  order; `YamlTagSkip` lets a handler pass through to the next, fallback keeps the original value
- **Pydantic integration** — `parse_as(Model, yaml, **yaml_kwargs)` parses YAML and validates
  against a Pydantic v2 model; raises `ImportError` with guidance when pydantic is absent
  (`python/pyrs_yaml/pydantic.py`)
- **`.pyi` type stubs** — auto-generated by maturin and committed so `register_tag`, `parse_as`,
  `to_yaml_with_options` and the new exceptions are visible to type checkers

### Changed

- CI Python matrix expanded: 3.8-3.14 across ubuntu, windows, macos
- Stable ABI: `abi3-py39` → `abi3-py38` (wider Python 3.8+ support), added `abi3t` + `abi3t-py315`
  (free-threaded stable ABI)
- `pyproject.toml` classifiers updated with 3.13, 3.14, 3.15 entries
- **CI optimization: redundant Rust compilation eliminated** — a single `rust-lint` job runs
  `cargo clippy` + `cargo test` once; the build job produces one abi3 wheel per OS which test jobs
  install instead of running `maturin develop`, removing Rust compilation from 21 matrix jobs (~86%
  fewer compiles); `Swatinem/rust-cache` added to all jobs
- **pydantic test dependency** — `pydantic>=2.10.6` added to `[dependency-groups] test` and
  `.ci/requirements-test.txt` (SSOT via `uv sync` in ci.yml)

### Fixed

- **Windows DLL loading** — removed `#[cfg(test)]` block from `src/py/tag_registry.rs` which broke
  `import pyrs_yaml` on Windows (`250b8d0`)
- **Python 3.8 compatibility** — `from __future__ import annotations` in `pydantic.py` (`63d2495`)
- **CI pydantic skip** — `pytest.importorskip("pydantic")` so tests pass when pydantic is not
  installed (`7be011d`)
- **CI glob expansion on Windows** — `shell: bash` for `pip install dist/*.whl` (PowerShell does not
  expand `*`) (`2f7778d`)
- **Non-string tag handler returns now raise `YamlTagError`** — a handler returning a non-`str`
  value (previously silently ignored, keeping the original scalar) now errors with
  `Tag handler '!x' must return a string` (`src/py/mod.rs:resolve_tags`)
- **`to_yaml_with_options` indent wiring** — `indent_mapping`/`indent_sequence`/`indent_offset` are
  now honored by the serializer (previously dead fields); each defaults to `indent_size`/0 when
  omitted (`src/serializer.rs`)
- **`width` no longer hangs for tiny values** — `width < continuation indent` falls back to emitting
  the remainder unwrapped instead of looping forever (`src/serializer.rs:write_plain_scalar`)
- **`remove_tag(name)`** — new function to unregister a tag handler; complements
  `register_tag`/`clear_tag_handlers` (`src/py/tag_registry.rs`)
- **`duplicate-key` errors are i18n'd** — `YamlDuplicateKeyError` messages now flow through
  `format_i18n_error` across all 4 locales (`src/i18n/locales/*.yml`)

</details>

## [0.8.0] - 2026-07-30

<details markdown="1">
<summary>YAML() instance API · Python Node API · doc.version · MergedView · lifecycle
warnings</summary>

### Added

- **`YAML()` instance API** —
  `YAML(typ="rt"|"safe"|"full", schema="core"|"yaml1.1", max_depth=1000)` with reusable
  configuration; `.parse()`, `.safe_load()`, `.safe_loads()`, `.parse_file()`, `.parse_all_docs()`
  methods
- **Python `Node` API** — `Node` class with `find()`, `filter()`, `walk()`, `to_yaml()`, `parent`,
  `children`, `root_type`, `value` for AST navigation; JSONPath-like query language (`$.key.sub`,
  `$.arr[0]`, `$..deep`)
- **`doc.version` metadata** — `YamlDocument.version()` returns the YAML spec version
  (default "1.2")
- **`MergedView`** — `doc.merged()` returns a read-only dict-like view with merge keys resolved
- **Lifecycle warnings** — `Node.release()` to explicitly invalidate a node; stale access emits
  `RuntimeWarning` + `YamlDocumentError`

### Changed

- `parse()` / `safe_load()` now delegate to `YAML().parse()` / `.safe_load()` as syntactic sugar
- `YamlDocument` now stores `version` field for document metadata

</details>

## [0.7.1] - 2026-07-30

<details markdown="1">
<summary>ryaml benchmark comparison · compliance threshold raised · Divan benchmark
migration</summary>

### Added

- **ryaml benchmark comparison** — `tests/test_benchmark.py` now benchmarks against `ryaml` (Rust
  YAML library) alongside PyYAML and ruamel.yaml; `benchmark_compare.py` rewritten as a feature
  comparison report (`tests/test_benchmark.py:25-28`, `.github/workflows/ci.yml:219`)
- **CI compliance threshold raised** — YAML Test Suite compliance gate increased from 70% to 75% in
  `test_compliance_report()`; valid parse rate gate at 95% (`tests/test_yaml_suite.py:251`)
- **CI dependency consolidation** — added `.ci/requirements-test.txt` and
  `.ci/requirements-test-lite.txt` for unified test dependency management across publish workflow
  and local dev
- **Benchmark modernization** — migrated from `pytest-benchmark` to `pytest-codspeed` for faster
  C-extension-based statistical benchmarking; all CI jobs now use `-r .ci/requirements-test.txt`
- **Rust benchmarks migrated to Divan** — replaced `codspeed-criterion-compat` with
  `codspeed-divan-compat` v5.0.1; 16 benchmarks rewritten from Criterion groups to `#[divan::bench]`
  attributes (`Cargo.toml`, `benches/yaml_bench.rs`)

### Changed

- CI benchmark job installs `ryaml` for cross-library comparison
- `benchmark_compare.py` now delegates timing to `pytest-benchmark` and serves as a feature
  comparison/reporting tool

</details>

## [0.7.0] - 2026-07-29

<details markdown="1">
<summary>serializer max_depth guard · hot-path optimization · pytest-benchmark migration</summary>

### Added

- **Serializer `max_depth` guard** — `serialize_node_internal` now tracks recursion depth and raises
  `YamlMaxDepthError` when exceeding the limit (default 1000), matching the parser's protection
  (`src/serializer.rs:135-145`)
- **Serializer hot-path optimization** — 5 optimizations targeting block-style serialization for
  ~4.9% roundtrip speedup:
    - Inlined `write_anchor_tag` and `write_inline_comment` None checks (eliminates method calls for
      ~99% of nodes)
    - `write_indent` hot/cold path split (direct index for cached levels ≤64)
    - `write_plain_scalar` fast path for short ASCII alphanumeric strings (≤8 chars)
    - `write_scalar_for_key` direct dispatch for Plain scalars (avoids dispatch chain)
- **pytest-benchmark migration** — Python benchmarks migrated from raw `time.perf_counter()` to
  `pytest-benchmark` for statistical rigor, structured JSON output, and CI integration
  (`tests/test_benchmark.py` + updated `tests/test_performance.py`)

### Changed

- `pytest-benchmark` replaces raw `timeit` in Python benchmarks
- CI benchmark job now runs `pytest --benchmark-json` instead of standalone script

### Removed

- `write_inline_comment` method — inlined at all call sites
- `Comment` import from serializer — no longer needed

</details>

## [0.6.0] - 2026-07-27

<details markdown="1">
<summary>async serialization · JSON Schema validation · to_json() · incremental re-parse</summary>

### Added

- **Async serialization** — `safe_dumps_async`, `safe_dump_async`, `safe_loads_async`,
  `safe_load_async` via `asyncio.run_in_executor` (`python/pyrs_yaml/async_dump.py`)
- **JSON Schema validation** — `YamlValidateError` exception + `YamlDocument.validate(schema)`
  method (accepts `str` or `dict`); delegates to Python `jsonschema` module
- **`YamlDocument.to_json()`** — serialize document to JSON string (uses Python `json.dumps`)
- **Incremental re-parse** — `YamlDocument` now stores source text (`doc.source()`);
  `doc.reparse(resolve_merges=True, schema="core")` re-parses in-place
- **29 new tests** across `test_async.py` (8), `test_validate.py` (14), `test_reparse.py` (7)

### Changed

- `YamlValidateError` registered as new custom exception (inherits `ValueError`)
- `rust_i18n::i18n!` macro path updated to `"src/i18n/locales"`
- `validate_translations()` test paths updated to match new locale directory

### Removed

- Deleted redundant `src/i18n/en.ftl`, `src/i18n/zh-CN.ftl` (never referenced by rust-i18n)
- Moved `locales/*.yml` → `src/i18n/locales/` (co-located with i18n module)

### Dependency Changes

- Runtime dependency: `jsonschema>=4.25.1`
- Dev dependency: `pytest-asyncio>=0.23` (moved from runtime, no longer pinned)

</details>

## [0.5.0] - 2026-07-27

<details markdown="1">
<summary>Serializer::write_node fix · YAML_SCHEMA constant · development documentation</summary>

### Fixed

- **`Serializer::write_node`** — `.unwrap()` on `values.iter().next().unwrap()` in
  `block_mapping`/`block_sequence` replaced with safe indexed access to eliminate potential panic on
  edge-case ASTs
- **`YAML_SCHEMA` constant** — typo `yamorg2002` corrected to `yamlorg2002` (matches YAML 1.2 spec
  URL)
- **Development documentation** — `AGENTS.md` updated with mandatory `uv run` prefix for Python
  commands and direct `cargo` for Rust commands

</details>

## [0.4.0] - 2026-07-27

<details markdown="1">
<summary>132 gap-filling tests: i18n, multi-document, bytes input, suite cases, fidelity</summary>

### Added

- **132 new gap-filling tests** — comprehensive coverage for previously untested APIs
- **i18n function tests** — `set_language`, `get_language`, `list_languages`, `detect_language`,
  `negotiate_language`
- **`parse_all_docs`** dedicated test suite — single doc, multiple docs, empty, comments
- **`parse_file` success case tests** — basic parsing, comments preservation, file-not-found error
- **`to_yaml_with_options` tests** — `explicit_start`, `explicit_end`, `indent_size`, `sort_keys`
  order preservation
- **`to_dict()` method tests** — scalar root, nested, list, bool, null, anchor resolution, empty
  mapping/sequence
- **YamlDocument dunder method tests** — `__repr__`, `__str__`, `__contains__`, `__len__`,
  `__iter__`, `__getitem__`, `root_type()`
- **Bytes input tests** — `parse(b"key: value")`, UTF-8 bytes, invalid UTF-8 error
- **Unicode & special character tests** — CJK, emoji, roundtrip, CRLF line endings, duplicate keys
- **`safe_load`/`safe_loads` feature coverage** — anchors, merge keys, block scalars, flow
  collections, special floats, type resolution
- **`from_dict` edge cases** — special characters in keys, nested lists, None values, empty
  dict/list
- **`from_json` round-trip** — nested structures, arrays, invalid JSON error
- **`dump_file` tests** — success path, invalid path error
- **YAML Test Suite individual case tests** — octal, hex, scientific notation, NaN, infinity, merge
  keys, explicit/implicit keys, bool/null variants, block scalar strip (`|-`), flow collections
- **`resolve_merges` parameter tests** — preserving `<<` when disabled, resolving by default
- **Flow collections roundtrip** — root-level and nested flow mapping/sequence
- **Anchor on non-scalar nodes** — mapping anchors (`&defaults`) and sequence anchors (`&items`)
- **Sequence indexing tests** — positive index, out-of-range error
- **Merge key integration** — roundtrip with resolved and unresolved merge keys
- **Tag preservation** — `!!seq` and `!!map` tag test coverage
- **Comment preservation** — inline and standalone comment tests on complex structures

### Changed

- Fixed version sync: `python/pyrs_yaml/__init__.py` `__version__` updated from 0.2.0 to 0.4.0 to
  match Cargo.toml/pyproject.toml
- Removed stale 0.2.0 wheel artifacts from `dist/`

</details>

## [0.3.0] - 2026-07-27

<details markdown="1">
<summary>NumPy ndarray serialization (N-D) · quoted-scalar typing · negative round-trip</summary>

### Added

- **NumPy ndarray serialization** — `safe_dump()` / `safe_dumps()` / `from_dict()` / `dump_file()`
  now support `numpy.ndarray` of all dimensions (0-D through N-D)
    - Supported dtypes: `int8/16/32/64`, `uint8/16/32/64`, `float32/64`, `complex64/128`, `bool`
    - Multi-dimensional arrays serialize as nested YAML lists with correct indentation
    - Complex numbers serialize as `(re+imj)` string format
    - `0-D` scalar arrays reshape to 1-D and serialize as a single-item list
    - `PyUntypedArray` + `PyArrayDyn` via `numpy` Rust crate for zero-copy dtype dispatch
    - GIL released during slice iteration for maximum performance
- **`quoted_scalar()`** — new `CustomNode::quoted_scalar()` constructor for values requiring
  single-quoted YAML style
- **Type resolution for quoted scalars** — `resolve_yaml_type` now applied to
  `SingleQuoted`/`DoubleQuoted` scalars for correct round-trip of quoted negative numbers
- **Comprehensive NumPy test suite** — 42 tests covering all dtypes, dimensions (0-D through 4-D),
  negative numbers, infinity, NaN, empty arrays, and edge cases
- Flow collections (`{}`/`[]`) round-trip support with `flow_style` field on Mapping/Sequence AST
  nodes
- `parse()` accepts both `str` and `bytes` input
- `parse()` supports `resolve_merges` parameter to opt out of merge key expansion
- `parse_all_docs()` for multi-document parsing via saphyr events
- `to_yaml_with_options()` with `indent_size`, `explicit_start`, `explicit_end`, `sort_keys`
  parameters
- `get()` supports default value parameter
- `dump_file()` for writing YAML to files
- Criterion benchmarks in `benches/yaml_bench.rs` (parse/serialize/roundtrip)
- GitHub Actions CI with matrix testing (3 OS x 4 Python versions)
- Anchor name parsing expanded to full YAML 1.2 spec (dots, colons, hashes, quoted anchors)
- `__version__` attribute, `py.typed` PEP 561 marker

### Fixed

- **Negative number round-trip** — YAML 1.2 block sequences cannot contain plain scalars starting
  with `-`; negative numbers are now quoted during serialization and correctly parsed back as
  integers/floats
- **N-D array support** — replaced `PyArray1<T>` with `PyArrayDyn<T>` to support arrays of any
  dimension, not just 1-D
- **Correct nesting depth** — multi-dimensional arrays now produce exactly N levels of nesting
  (shape[1..] handles inner dimensions, root dimension wrapped by `plain_sequence`)
- Alias resolution in `to_dict()` and `safe_load()` — aliases now resolve to referenced values
  instead of `None`
- `safe_loads()` no longer uses naive `split("---")` — uses saphyr's document events
- Mapping/Sequence tags no longer discarded during parsing
- `format_scalar_for_key()` now handles Literal/Folded block scalar styles

### Changed

- Added `numpy` crate (v0.29) as a dependency for ndarray type dispatch
- Upgraded PyO3 from 0.21 to 0.29
- Replaced 15+ boilerplate `CustomNode` constructions with
  `plain_scalar()`/`plain_mapping()`/`plain_sequence()`/`plain_null()` constructors
- Serializer extracted `write_anchor_tag()` and `write_inline_comment()` helpers
- Parser extracted `detect_flow_style()` helper
- Removed dead code: `ParseOptions`, `find_inline_comment`, `find_standalone_comment_before`,
  `format_yaml_type` (test-only)
- Consolidated 6 duplicate test files, moved 9 diagnostic scripts to `scripts/`
- Improved error messages with key/index/type context

</details>

## [0.1.0] - 2026-07-25

<details markdown="1">
<summary>initial release: YAML 1.2 AST with full metadata, round-trip, PyYAML-compatible
API</summary>

### Added

- Initial release with YAML 1.2 compliance via saphyr-parser
- Custom AST with full metadata (comments, anchors, tags, chomping, scalar styles)
- Round-trip preservation of comments, anchors, tags, and formatting
- PyYAML-compatible API (`safe_load`/`safe_dump`)
- `from_dict`/`from_json` conversion functions
- `read_markdown`/`read_markdown_str` for YAML frontmatter extraction
- Block scalars (`|`/`>`) with chomping indicators (`|-`/`|+`/`>-`/`>+`)
- Escape sequences (`\n`, `\t`, `\uXXXX`, `\xXX`)
- YAML 1.2 type resolution (null, bool, int, float, infinity, NaN)
- Merge key resolution (`<<: *alias`)
- Complex keys (sequence/mapping as key)

</details>
