---
title: Changelog
description: All notable changes to pyrs-yaml, formatted per Keep a Changelog and Semantic Versioning.
tags:

- **A key with a comment above it was unreachable by name** —
  `doc["key"]`, `"key" in doc` and merge expansion all resolve a node by hash, and
  `CustomNode::hash` folded `NodeMeta`'s normalised comment view while
  `CustomNode::eq` compares the raw `comment` slot, so two nodes could compare equal
  yet hash apart and `IndexMap` answered "no such key" for a key the document plainly
  holds. `NodeDecor` is documented as excluded from `Hash` / `PartialEq`, so the hash
  was the side out of contract: `CustomNode::hash` now folds only the fields its own
  equality compares (`comment` / `anchor` / `tag`), through a new
  `NodeMeta::hash_custom_node_identity`, while `NodeMeta::hash` keeps mirroring
  `NodeMeta::eq`'s #117 normalisation so both pairings stay self-consistent. Every key
  past the first was affected — a document's opening note is reported against the
  enclosing mapping, which is exactly why single-key fixtures never showed it.
- docs
status: new

---

## Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

### [Unreleased]

#### Added

- **An instruction-count gate the CI can actually hold** — the `CodSpeed`
  workflow now runs an `Instruction-count baseline` job that measures the engine's
  hot paths in *counted instructions* (`callgrind` Ir) and fails on more than a
  two-percent rise over `.ci/ir-baseline.json` — that line calibrated to the drift
  actually observed between two Linux images (a WSL-generated baseline checked on a
  GitHub runner sat +1.45% on the most allocation-sensitive scenario). It exists because the wall-time
  comparison the divan suite reports is not reproducible below roughly ten
  percent: three consecutive pushes, each doing strictly *less* work than the one
  before, were scored −7.7%, −10.5% and −9.8% for the same benchmark set. Ir
  repeats to about ±0.001% on the same binary, so the line means something; the
  whole gate costs about six seconds. Run `python scripts/ir_gate.py` to check,
  `--update` to re-baseline deliberately. Its scenarios read the same documents as
  the divan suite through `pyrs_yaml_core::bench_inputs`, so the two measurements
  cannot drift apart.
- **Note survival is a gate now, not a hope (`crates/pyrs-yaml-core/tests/note_survival.rs`)**
  — the round-trip tier's oracle is text idempotence, which every stable-but-short-a-note
  document passes, and that blind spot is why five silent note losses stayed green behind
  it. A deterministic test now replays the committed YAML seed corpus on every
  `cargo nextest`, requiring every note the reader recorded to appear in the emission
  **and** every input to settle in one round — 37 note-bearing seeds carry the assertion
  today, and the corpus grows it automatically as crashes become seeds. Its limit is
  stated in the file and proven by mutation rather than argued: making the reader report
  success without attaching a note reddens the pinned shape tests and leaves *this* gate
  green, because a note lost during ingest never reaches the AST the gate measures.
  Counting `#` in the source instead would redden correct input — the corpus holds
  `!###0 …`, a tag whose suffix is `#` characters — and a gate that reddes correct output
  is worse than none, so the reader-side half stays with the fuzz tier and the per-shape
  pins, and neither half claims to cover the other.
- **A localized-script purity gate (`scripts/check_cjk_localisation.py`)** — the `ja`,
  `ko` and `zh` changelogs are now machine-checked to stay in their own writing system:
  no kana outside `ja`, no Hangul outside `ko`, and no simplified-Chinese-only Han in
  `ja`/`ko` (the pairs Japanese writes with a different codepoint — 積/积, 連/链, 視/视,
  層/层). Translated entries drift into neighbouring scripts, and a reader of that locale
  often cannot tell, because the intruding glyph looks like a variant of the intended
  one. It runs as the prek hook `cjk-localisation`, honours the filenames the hook
  passes, and refuses to call an empty scan a pass: the first version pinned its heading
  pattern to the root file's depth, matched nothing on the nested localized pages, and
  printed a green OK that meant nothing. Proven by injecting violations and watching the
  hook fail before trusting it — and on the run that made it real it caught two
  pre-existing intrusions (a simplified 折叠 in the `ja` page, a katakana ウ inside a
  Hangul word in `ko`).
- **Weekly fuzz schedule in CI** — `.github/workflows/fuzz.yml` runs all
  four libFuzzer targets every Saturday (plus on demand and whenever `fuzz/`
  itself changes), seeding the ephemeral per-run corpus from curated
  `fuzz/seeds/` (historical crash inputs + hand-written shape coverage), and
  uploading crash artifacts on failure to feed the
  crash → regression-test → seed → fix pipeline. Machine corpora still never
  enter git.
- **A `cargo-fuzz` harness for the engines (`fuzz/`)** — four
  coverage-guided libFuzzer targets over the native frontends: `parse_yaml`
  (single + stream), `yaml_roundtrip` (parse → serialize → re-parse and
  serialize-idempotence), `parse_json` (all three dialects crossed with all
  three writers, each output re-parsed), and `parse_toml` (1.0/1.1 plus
  writer re-parse). Only the targets are tracked; corpora and crash
  artifacts are generated locally per session and stay gitignored (crash
  findings land as regression tests, not as corpus files). The harness paid
  for itself inside its first minute — see the comment-scanner fix below.
- **`pyrs-ast` / `pyrs-schema` are `no_std`-capable** — the two foundation
  crates that every format engine sits on now build against `alloc` alone:
  `indexmap` and `thiserror` lose their default `std` features and a new
  opt-in `std` feature re-enables `std::error::Error` impls plus the
  `RandomState` hasher. `std` stays on by default so every existing consumer
  keeps the exact `IndexMap<K, V, RandomState>` node-map type it already
  wrote; `no_std` users opt out with `default-features = false` and get a
  fixed-seed hasher instead. Proptest node strategies move behind a new
  `test-strategy` feature so a plain build carries no property-test cost.
  A `no-std-check` CI job cross-compiles the workspace for a bare-metal
  target to keep the property honest.
- **`pyrs-json` / `pyrs-toml` are `no_std`-capable too** — the two native
  format engines build against `alloc` alone: `std::sync::Arc` moves to
  `alloc`, the `String`/`Vec`/`format!` prelude is spelled explicitly via
  `#[macro_use] extern crate alloc`, parse-time key/value stores reuse
  `pyrs-ast`'s `NodeMap` hasher alias, and `canonical_float`'s integral
  check becomes core-only (`f64::trunc` is an std inherent). The
  `no-std-check` job now cross-compiles all four crates.
- **`pyq` ships as a prebuilt binary with every release** — a new `pyq` job in
  `publish.yml` builds the native CLI for six platforms and attaches the
  archives to the GitHub Release, so users no longer need a Rust toolchain to
  get a standalone binary.
- **Type stub drift gate (`scripts/check_stub_drift.py`)** — the committed
  `python/pyrs_yaml/pyrs_yaml.pyi` is machine output that ships inside every
  wheel, but CI only asserted that it exists and is tracked (`release-guard`),
  so a binding signature change could leave the public typing contract silently
  behind. A new `stub-drift` job in `validate.yml` regenerates the stub through
  the declared route (`uv run maturin generate-stubs`) and fails on any content
  difference. Two transforms keep the tracked stub fully derived instead of
  hand-patched: the trailing whitespace prek's hook strips at commit, and one
  declared fidelity fix for the two `__next__` returns where maturin 1.14.1
  drops the `Option` the bindings actually return. Every declared fix asserts
  its expected match count, so a changed signature or a fixed upstream fails
  loudly instead of being rewritten silently. `mise run stubs` now writes
  through the same pipeline.

#### Changed

- **`pyq validate` accepts `--input` and honours the real format** — it used
  to hardcode the YAML parser and had no way to say otherwise, so every
  non-YAML config it was pointed at (`pyproject.toml`, `package.json`, …)
  was rejected outright. It now takes `--input auto|yaml|json|jsonc|json5|toml`
  and routes through the shared loader, which is what made the command
  usable outside the repo it was written in.
- **`pyq` prebuilt binaries build in manylinux2014 containers** — the Linux
  targets compile natively inside the official CentOS 7 images via `cross`,
  pinning a glibc 2.17 floor that covers CentOS 7 / Ubuntu 16.04 / 18.04 /
  Debian 8 / 9 with zero hand-rolled cross toolchains. An earlier zigbuild
  recipe pinned x86_64 one minor lower (2.16, the measured `getauxval`
  floor), but the first real run of `publish.yml` — the workflow never
  executes on ordinary PRs — exposed the whole zig stack was incomplete
  (`cargo zigbuild: no such command`, an armv7 leg mislinked by the host's
  default `-fuse-ld=lld`, smoke tests running binaries the runner could not
  exec); the container recipe replaces all three failure modes with one
  community-standard tool, at a floor two releases lower than the runner's
  own glibc.
- **PR-tier fuzz is blocking now (`fuzz.yml`)** — the step-level
  `continue-on-error` on pull requests was a ratchet kept only while `main`
  carried drift this tier correctly flagged (crash-f44eca1d, after the
  predecessors fixed in #256/#258/#261/#262). Re-checked on 2026-10-04 against
  this tree: all four targets replay the committed seed corpus clean (5/59/6/5
  seeds, the pinned nightly-2026-08-15, cargo-fuzz 0.13.2, 60 s discovery), so a
  new crash now fails the PR that introduces it instead of surfacing the
  following weekend. Measured on 2026-10-05 after the merge-key override landed: the
  75-file seed replay is clean on all four targets, while a fresh 60 s discovery window
  still reaches the note-relocation family (`crash-a916de77`, 48 bytes) — recorded in
  `ROADMAP.md` as the open finding, unseeded and unfixed, because a seed is a promise the
  input passes. The key-metadata gap the ledger listed next (crash-86a9ae7b)
  was verified closed by single-input replay; the drift found while checking —
  `yaml_roundtrip` crash-ac5d9043 — is recorded in `ROADMAP.md` as the current
  open finding, unseeded and unfixed.

#### Fixed

- **A block value made of nothing but line breaks no longer decays to empty** — `>+8\r\r#`
  reads as a folded scalar whose value is a single line break with `Keep` and an explicit
  indent of 8. The writer kept the indicator, emitting `>+8\n\n`, which re-reads as the
  same value but as `Clip` with no indicator; the next round then wrote `>\n\n`, whose
  value is `""` — Clip strips trailing breaks, so the one break the value consists of had
  nowhere to live. The two rules that make an *empty* body idempotent (drop the
  unrecoverable indentation indicator; write the chomping that re-reads the same) were
  keyed on emptiness, and an all-break body is not empty (libFuzzer `yaml_roundtrip`,
  `crash-2f6b1eff`, 6 bytes — already minimal: the input no longer crashes, so `tmin`
  cannot shrink it). Both writers now treat "no content line" as the shared condition, the
  shape reaches a fixed point in one round, and the value survives it. Cost: the first version scanned the value a second time and measured +0.65% on
  `serialize_block_scalars` - +2.11% on the runner, over the instruction-count gate's 2%
  tolerance, so the gate caught the change before merge. Sharing the writers' existing
  first-content-line computation brings it to +0.15%, inside the gate.
- **A merge key can no longer move an anchor behind its alias** — expanding `<<:`
  prepended the merged pairs at the front of the mapping, so a document that defines
  `&b` on an earlier own key and uses `*b` inside the merged map emitted `*b` before
  `&b` was defined: text our own parser refuses (`found unknown anchor`), breaking the
  engine's "we never emit unparseable output" contract (`crash-9b77aea4`, 78 bytes,
  minimised to 15). The expansion now inserts at the index the `<<:` occupied, which is
  authored order — and index 0 when the merge key is first, so the documented
  `<<: *defaults` shape is untouched (487 -> 489 Rust tests, 1781 Python tests, all
  passing unchanged).
- **A key's trailing note no longer migrates onto its value's line** — when a
  value had to move down to take a leading note of its own, the note that belongs
  to the *key* was still appended to whatever line finished last, i.e. the value's
  line. Re-reading assigns a trailing note on a tag-only scalar's line to the
  **value** as a leading note, so the note changed owner each round and the
  emission never settled: `b: ! # &` + `#~` gave `b:\n  # ~\n  !   # &`, whose
  re-read gave `b:\n  # ~\n  # &\n  !` (libFuzzer `yaml_roundtrip`
  crash-1b01ac3f, 93 bytes minimised to 11). The key's note now stays on the
  `key:` line, which is both the slot a reader reports it from and a one-round
  fixed point. Costs nothing measurable: the instruction-count gate moved +0.05%.
- **A merge can no longer repeat a key its mapping already owns** — an untagged `y` that
  carries a note and a merged `y` were two different `IndexMap` keys, so both survived into
  the emission and `to_yaml` printed `y:` twice at one level: text our own parser refuses,
  breaking the engine's "we never emit unparseable output" contract (`crash-3495cc86`,
  72 bytes, minimised to 19). `prepend_merged_pairs` had long claimed its input was
  "filtered against existing keys by the caller"; nothing filtered. The expansion now drops
  what the mapping overrides — identity being the *emitted* value of an untagged scalar key,
  the same rule `push_node`'s duplicate check applies, so the two can never disagree about
  which pairs may coexist — and a dropped pair's notes are re-homed instead of vanishing
  with it (`former-crash-3495cc86.seed`, pinned by `a_merge_never_repeats_a_key_the_mapping_owns`
  and by the corpus gate). Withdrawing the override reddens both and nothing else in 283.
- **A note beside a root node that carries only properties is no longer dropped** —
  `!x # note`, `&a # note` and the 69-byte `!###0` wall of notes kept losing their
  text, and no oracle could see it: a document that is stable and short a note is a
  perfectly stable document. Two orders, two bugs. granit delivers that note *before*
  the `Scalar` event, where `attach_inline_comment` had no candidate to hang it on and
  still answered "handled", so the caller never carried it forward; and for
  `!m` CR `...` SP `# -o` the note arrives *after* `DocumentEnd`, where binding it back
  onto the finished root as an inline note produced `!m   # -o` — a spelling whose own
  re-read reports the note before the node and homes it as a leading note, so the round
  trip never settled. Only a real attachment may now report success, a note that
  follows the end of a non-container document is carried forward, and a pending note
  left when the document finishes rides the root as a leading note instead of being
  discarded (`former-crash-7918272c.seed`, 11 bytes, and `former-crash-ce106ccc.seed`,
  69 bytes, pinned by `a_note_beside_a_property_only_root_survives_and_settles`, which
  asserts the exact emission, the survival of the text and a one-round fixed point for
  five shapes plus both seeds). Withdrawing the honest return value reddens that test
  alone. A container root deliberately keeps its inline home — `a: 1` + `# trailing
  note` re-reads from the last value line, and that is what the two pins on
  `flush_trailing_comment` hold.
- **A `#` that is only text no longer evicts a note from its line** — the writer puts
  a container's own inline note on the line it has just finished, but it first asked
  whether that line "already has a `#`" by scanning raw bytes. A quoted scalar holding
  one (`"+#": !-`) answered yes, so the note was demoted to a line of its own — and a
  bare note line below a value is read back as the *following* node's leading comment,
  which moved it inside the value block on the second round. The scan now respects
  quoting and YAML's whitespace rule (`line_has_comment_marker`), so the note rides the
  pair line and one emission is the fixed point (`former-crash-22cb5f67.seed`, 15 bytes,
  pinned by `a_quoted_hash_does_not_demote_the_containers_note` plus
  `comment_marker_scan_respects_quoting`). Withdrawing the new scan reddens exactly the
  end-to-end test.
- **A tagged container now lifts every note on its first entry's marker spine** — the
  lift that clears notes out from under an anchor/tag header line read only the first
  key's *own* leading stack, so a stack riding a marker inside the body stayed below
  the header and the reader hoisted it on the next round. The lift now takes the whole
  spine (`former-crash-e6551c75.seed`, 60 bytes, and the 43-byte `former-crash-8f7085b0.seed`,
  pinned together by `every_spine_note_clears_a_tagged_containers_header_line`). The
  ledger had deferred this on the reasoning that the spine walk would pull notes out of
  *nested* markers and break the pinned compact-key shapes; withdrawing the walk and
  re-running showed exactly one test reddens and every neighbour in the family passes
  both ways, so the blast radius had been inferred rather than measured.
- **`pyrs-toml` builds for a bare-metal target again** — the stacked-note work had put
  `std::mem::take` into the parser and writer of a `#![no_std]` crate, which every host
  build forgave and the `no-std-check` job would not. Six call sites now use
  `core::mem::take`, and `cargo build --locked --no-default-features --target
  thumbv7em-none-eabi -p pyrs-ast -p pyrs-schema -p pyrs-json -p pyrs-toml` is green
  locally, which is the gate that job runs.
- **A tagged container no longer splits its note stacks across its own header line** —
  the lift that moves a block container's first-entry notes above the anchor/tag header
  it prints refused to run when the container carried notes of its own, because at the
  time a node owned a *single* leading slot and a second stack there would have
  overwritten the first. That slot is a `Vec` now, so the refusal stopped protecting
  text and only cost a round: `# a` + `!5b4?` + `?` + `# b` + `k: v` wrote `# b` under
  the header and had to be re-read to hoist it. Measured first — the fixed point of
  every shape in the family is *all* stacks above the header, in source order — then
  the guard was removed rather than tuned, and the characterization test that had
  pinned the old placement was re-derived (`notes_stack_above_a_tagged_containers_own_note`
  now asserts the order, the survival of both notes and a one-round fixed point;
  `both_note_stacks_land_above_a_tag_header_in_one_round` replays the window's
  32-byte carrier `former-crash-fbc8f2ae.seed`). Restoring the guard reddens exactly
  those two tests.
- **Every note stack on a marker's spine lifts to the marker line, not just the first** —
  `hoist_marker_note` walks the chain of `?` markers a single line opens and stopped at
  the first stack of leading notes it found. A chain can carry more than one, and
  granit reports each of them at the marker's own level: in crash-f8525a9e the spine is
  three markers deep with `#` on the middle mapping and `!!"#~` on the innermost `~`
  key, so hoisting one left the other a level deeper and the emission settled only on
  its second round. The walk now accumulates, outer before inner — which also closed
  the 99-byte crash-c9031de4, whose notes sit on the same spine (`former-crash-f8525a9e.seed`,
  `former-crash-c9031de4.seed`, pinned by
  `every_note_on_the_marker_spine_lifts_to_the_marker_line`). The first hypothesis for
  this input — that the hoist missed a note stored in the legacy `comment` slot — was
  tested and **refuted** (the emission did not change a byte); `take_leading_notes` was
  still switched to the normalised `leading_comments()` view, because reading only one
  of the two storage conventions is the fork `standalone_slice()` exists to prevent, but
  it is justified on that ground alone and not credited with this fix.
- **A container's own inline note is now written on a line that can hold one** — the
  writer printed a non-standalone `comment` on a block container as a bare note line
  below the block, but a reader never reports an inline note from an empty line: the
  re-read hands that text to the node which ended the block, as its *leading* note,
  so the first emission was never a fixed point. `:<TAB>!-<CR>... #-o` produced
  `~: !- \n# -o\n` and only the second round produced
  `~:\n  # -o\n  !- \n` (libFuzzer `yaml_roundtrip` crash-11ced252, 13 bytes). The
  note now borrows the line the block just finished — `~: !-   # -o`, stable in one
  round and exactly the slot granit reports it back from. Ownership of that slot is
  tracked where lines are written instead of guessed from the output text, so the
  borrow is refused for a block scalar's body line (an appended note would turn into
  content), for a wrapped continuation, and for a line already carrying a note.
  `a_containers_inline_note_lands_on_the_last_value_line` pins the acceptance and
  `a_block_scalar_body_never_borrows_the_containers_note` the refusal; a mutation
  check (un-append the note) reddens exactly the first of them while the other 274
  tests stay green. Through the TOML hub this also sharpens a documented boundary:
  `[sec] # note` no longer escapes to the document head — it stays inside its own
  table as a trailing comment on the table's last entry, still one round to a fixed
  point, so `TestSectionHeaderCommentBoundary` is rewritten to pin survival, the
  in-table position, and that stability.
- **A marker line now performs both of its note lifts** — an explicit key can carry a
  note on its key node *and* leave a second one riding the first entry of the key
  body, and the reader reports both at the marker's own level. The writer chose
  between the two lifts with an `if`/`else if`, so whenever the key owned a note the
  body note was printed one indent deeper and climbed a level on re-read; the emission
  settled only on its second round (libFuzzer `yaml_roundtrip` crash-456176be, 40
  bytes: `?` + `### standab:` + `?` + `# ! y%% yam2:#l: tr` + `~: ~`, whose tree
  keeps the first note on the key mapping and the second on its inner `~` key). The
  lifts compose now, in source order, above the `?`, and one emission reaches the
  fixed point — pinned by `a_marker_carries_both_its_own_note_and_its_bodys_first_note`
  reading the committed seed, and attributed the same way: un-composing them reddens
  that test alone. Two inputs which shared the symptom, crash-f8525a9e and
  crash-c9031de4, stayed red after the fix, so they are a different geometry (their
  note rides a nested marker, not a scalar key) and remain open.
- **An `&` inside comment text can no longer donate its name to a real anchor** — granit
  reports only a numeric `anchor_id`, so the display name is recovered by scanning left
  from the node's content, and that recovery refused a `&` only when the token *directly*
  before it was a comment opener. A comment body may hold anything: `bg: &b` followed by
  `# !! &?` re-read as `bg: &?`, because the `&?` sits past a `!!` rather than right after
  the `#`. A renamed anchor silently orphans every alias that referred to it — the same
  data-loss class as #265 and crash-04fddeb8, and the third time this cycle the guard was
  written narrower than the rule it stood for. The refusal now asks the question YAML asks:
  does a comment start anywhere earlier on this line? A `#` embedded in a scalar still
  opens nothing, and the one shape that could over-refuse — a quoted `#` earlier on a line
  that also carries an anchor — has no reachable form, because node properties always
  precede the value. Pinned by `anchor_name_before_ignores_ampersand_anywhere_in_comment_text`
  (both directions of the predicate) and
  `anchor_keeps_its_name_across_a_comment_line_holding_an_ampersand`, which reads its bytes
  from the new seed `fuzz/seeds/yaml_roundtrip/former-crash-68adf94c.seed` and asserts the
  anchor token, the comment text, and a one-step fixed point; the artifact replayed
  CRASH→CLEAN, and every earlier guard in the file still holds.
- **Every standalone comment line above a key survives, not just the last one** — the
  AST held leading notes in a single slot (`NodeDecor.leading_comment: Option<Comment>`)
  and the YAML receiver, the JSONC parser and the merge pass each *overwrote* it, so a
  stack of comment lines kept one: `# alpha` + `# beta` + `key: 1` serialised back as
  `# beta` + `key: 1`. Nothing downstream could see it — the lossy text is stable, and
  the round-trip tier's oracle asks only for stability after re-serialising, so this is
  the first class that tier cannot express. `NodeDecor.leading_comments` is now an
  ordered list; `NodeMeta::standalone_slice()` is the one normalised read (the list, or
  the legacy `comment(standalone = true)` spelling, as a slice — a slice, not a `Vec`,
  because `NodeMeta::eq` / `Hash` run on every `IndexMap` probe of every mapping).
  `leading_comment()` and Python `Node.leading_comment` still report the first note, so
  nothing existing changes; `Node.leading_comments` is the new full view. Following that
  thread found three more silent losses of the same shape, each fixed, tested and
  seeded: a document carrying nothing but comments dropped all of them
  (`#&l<TAB><TAB>:` → `null`, because `DocumentEnd` never fires without a node), the
  null-key fold deleted the folded entry's comment with it, and a consumed merge key —
  or the mapping it merged — took its comments down with it, a hole the merge-identity
  fix had just made reachable. Notes are re-homed now, never discarded. The TOML spoke carried the same
  overwrite in its own pending-note slot (`# a` + `# b` + `k = 1` kept one), and an empty container's
  single inline slot dropped the rest of a stack (`# d1` + `# d2` → `{}  # d1`); both keep every note
  now, which also changed a documented boundary: a comment on a table-header line can still not stay on
  that line, but it is relocated to the document head instead of vanishing, and one TOML round through
  the hub is already stable there. Only the note-survival oracle that found all of this waits for the
  remaining attachment work, so it does not land red ahead of it.
- **A merge key is recognised by what it is, not by what it carries** — the merge pass
  looked `<<` up in the pair map by whole-node equality, so a `<<` whose key node carried a
  comment was invisible to it: `<<: #*` + `y:` resolved to `{'<<': {'y': None}}` while
  the same document written `<<:` + `y: ~  # *` resolved to `{'y': None}`. Because the
  writer *moves* notes between exactly those positions, a round trip changed the
  document's meaning and the pair vanished on the next round (libFuzzer `yaml_roundtrip`
  crash-69931a77, minimised by `cargo fuzz tmin` to 10 bytes; crash-0a6fe677,
  crash-2d3dab18 and crash-f88c2382 closed with it — putting whole-node equality back
  reddens all four together, which is the attribution rather than the shared assertion).
  The key is now matched the way YAML resolves it — an untagged plain `<<` — and the entry
  is addressed by position, not by value; that also fixes the tail walk handing back a
  merged-in clone whenever the clone compared equal to an own key. Style and tag still
  decide: a quoted `"<<"` and a tagged `!x <<` remain ordinary keys, pinned by
  `a_quoted_or_tagged_merge_lookalike_stays_an_ordinary_key` and
  `TestMergeKeyIdentityIgnoresMetadata`. Seeded as
  `fuzz/seeds/yaml_roundtrip/former-crash-{69931a77,0a6fe677,2d3dab18,f88c2382}.seed`.
- **A note under a container's own tag line no longer swaps places with it** — granit
  reports a standalone note written *below* an anchor/tag header line as that tagged
  node's leading comment, so a note the writer left there moved *above* the header on the
  next round and a single emission step never reached a fixed point (libFuzzer
  `yaml_roundtrip` crash-77a8039b, 28 bytes: `!5b4?` then `# yrrrrrrrrrrrr%3c` then
  `~: ~`; measured to settle only at round 2). Such a note is now lifted above the header —
  the one line the reader hands it back from — and taken out of the body copy so it is not
  written twice; a tagged block sequence gets the same lift for its first item. The lift is
  capped where the reader runs out of slots: a container that already carries a note of its
  own has one line above its header, and a second would fall into the same single leading
  slot, so that shape keeps its placement rather than trading drift for lost text
  (`a_note_is_not_stacked_above_a_tagged_containers_own_note`) — and it is recorded as its
  own open finding, because that input measurably loses a note today. Seeded as
  `fuzz/seeds/yaml_roundtrip/former-crash-77a8039b.seed`.
- **A trailing note is no longer bound across the line break its own node swallowed** —
  granit spans a block collection *through* the line break that ends its line, so a
  span end can already sit on the next line. The "is this note on a later line?" test
  scanned only the gap between the candidate's end byte and the note, found no `\n`
  inside `?\n`, and bound the note to the deeper node; the writer emitted it inside
  that block, the re-read handed it to the shallower entry, and ownership climbed a
  level every round (libFuzzer `yaml_roundtrip` crash-0e1c4378, minimised to 10 bytes
  `b:<LF> ?<LF>? #i`). The test now backs the candidate end over the blanks the span
  swallowed before looking for the break. Two guards hold the boundary: crash-105de752
  (47 bytes) went CLEAN with the same change and the attribution was *measured* —
  turning the trim off reddens it again next to crash-0e1c4378, so they are one root
  cause rather than one shared assertion — while
  `a_note_on_a_multi_line_nodes_last_line_still_trails_it` pins the opposite direction,
  because a note on the last line of a multi-line node must still trail it. Both
  inputs are committed as `fuzz/seeds/yaml_roundtrip/former-crash-{0e1c4378,105de752}.seed`.
  Method note, worth more than the fix: the first version of this rule read the
  receiver's char-index → byte-offset table as if it were a line table, so on pure-ASCII
  input (where the table does not exist) it returned `None` and changed nothing at all.
  It read correctly and did nothing; only the still-red assertion said so.
- **A note trailing a mapping key is no longer silently dropped** — granit reports a
  comment between a simple key and its `:` on the *key* node, but once the pair is
  written as `key: value` that position has no spelling, and the YAML writer lost the
  note outright (`? a # note` + `: b` emitted `a: b`). It is now emitted after the
  value — the one slot a reader can report such a note from — so nothing disappears,
  and the line is a fixed point there; a value carrying a note of its own keeps it,
  because one line has one trailing slot. The round-trip tier cannot see this class
  (the text was stable, merely short a note), so `a_note_trailing_a_key_survives` plus
  `test_from_jsonc_keeps_comments_as_yaml_notes` pin it. Probing it also turned up
  three places still claiming `from_jsonc` "strips comments" — a statement the
  changelog recorded as stale since #112/#115 — in the binding's `from_jsonc` /
  `from_json5` docs and in the generated stub; the new stub-drift gate caught the
  stale stub the moment the docstrings were corrected, and the stub was re-derived
  through the declared route rather than hand-edited.
- **A note that trails an explicit-key marker is written where the reader reports
  it** — granit attaches such a note one level shallower than the node it lands on,
  so the indented spelling climbed a column every round and `to_yaml` never settled
  (libFuzzer `yaml_roundtrip` crash-ac5d9043, minimised by `cargo fuzz tmin` to 8
  bytes `? ? ? #~`; the `&##` anchor in the found input was incidental). The writer
  now bubbles a marker-line note up to the marker line that owns it, while a note
  granit lexed on its own line stays exactly where it is — that geometry already
  round-tripped, and a guard pins it so the fix cannot widen.
- **A mapping folds its null keys, and only its null keys** — a `~` key and an
  empty key are the same key, but `IndexMap` compares whole nodes and the two
  spellings differ in metadata, so both stayed, both rendered as `~:`, and the
  reader folded them on re-parse: such a document lost a line every round (libFuzzer
  `yaml_roundtrip` crash-00e31785, minimised to 9 bytes `: &b #*\r:`). Ingest folds
  them to the single entry a re-read produces. That fold first deleted data, because
  `is_null_key` tested only the scalar text: a quoted `"NULL"` / `""` key or a tagged
  `!a null` key counted as null too, so `{"": None, "NULL": None}` lost its empty key
  through the JSON5 and TOML round trips (`tests/test_property_dialects.py`) and
  proptest reported `!a null:` + `!A null:` as a bogus duplicate. Both predicates now
  ask what YAML asks — implicit resolution applies to untagged plain scalars only.
- **A block scalar with an empty body no longer advertises a chomping indicator** —
  granit reports the *default* chomping when it re-reads a header that has no content
  to act on, so writing `|+` / `>+` for an empty scalar drifted to `|` / `>` on the
  next round and `to_yaml` never reached a fixed point (libFuzzer `yaml_roundtrip`
  crash-89d81d99, 5 bytes `>+8<CR>#`; crash-b5dcc38f, 55 bytes, `ancho: |+`). The
  writer already drops the *indentation* indicator for exactly that reason; it now
  drops the chomping indicator the same way, and nothing is lost — an empty body has
  no trailing break to keep or strip, and the AST keeps whatever was parsed.
- **A comment line no longer donates its `&` to the anchor name** — granit reports
  only a numeric `anchor_id`, so the display name is read back by scanning left from
  the node's own content for the nearest boundary `&` (the recovery #265 tightened for
  tags). A standalone comment sitting *between* that anchor and the content was never
  excluded, and `&` is legal comment text: `chi&&&: &~:` followed by `# &l` re-read
  with the anchor renamed to `&l` — the real name `~:` vanished and every alias
  pointing at it was silently orphaned, the same data-loss class as #265 reached from
  the other token that may contain `&`. A `&` whose line has already opened a comment
  is now refused as a candidate, exactly as one inside a tag is; the note itself is
  kept. (libFuzzer `yaml_roundtrip` crash-04fddeb8.)
- **A tag suffix holding a flow indicator no longer breaks the document** —
  granit hands the reader the *decoded* suffix, so the source tag `!a%2cb` arrives
  as `a,b`. Tag emission re-encoded only what RFC 3986 forbids, and `,` `[` `]` `!`
  are perfectly legal URI characters — but they are precisely the characters
  granit's `is_tag_char` refuses, so the suffix scan stops at them and, at flow
  level 0, the scanner then demands a blank or a line break. Our own `to_yaml`
  output was therefore rejected outright ("while scanning a tag, did not find
  expected whitespace or line break"; libFuzzer `yaml_roundtrip` crash-e92ce66f,
  43 bytes, the same root cause as `!5%2cy7 `). The write set now comes from the
  reader instead of from the URI grammar: a shorthand tag percent-encodes those
  four, while a verbatim `!<uri>` keeps them raw because `is_uri_char` accepts them
  there — `!<tag:yaml.org,2002:str>` still round-trips byte-identically.
- **Unicode whitespace no longer masquerades as a YAML blank** — six places in the
  YAML pipeline asked `char::is_whitespace()` / `str::trim()`, which is Unicode-based
  and also matches NBSP (U+00A0), U+0085 and U+2028/U+2029, none of which YAML treats
  as separation (granit's blank set is SP and TAB only). The consequences were silent,
  not cosmetic: a document holding a single NBSP fell through the empty-document fast
  path and re-read as `null` instead of a scalar (libFuzzer `yaml_roundtrip`
  crash-512814, 5 bytes: a BOM then a NBSP); `resolve_core_type` and
  `resolve_yaml11_type` trimmed content away, so `<NBSP>42` resolved to the *integer*
  42 and `<NBSP>yes` to `true`, and a multi-line NBSP scalar resolved to `Null`, which
  made the writer skip quoting and emit raw line breaks that collapsed on re-read —
  emission never settled (crash-b44481b2, 7 bytes); a plain scalar folded for wrapping
  lost an NBSP at the break; `anchor_name_before` cut `&a<NBSP>b` down to `&a`,
  silently orphaning every alias that used the full name; and comment bodies lost an
  NBSP at either edge. All six now test against `pyrs_schema::is_yaml_blank`, the
  reader's own set. The JSON-family resolvers keep Unicode whitespace on purpose —
  JSON5 really does treat it as structural whitespace.
- **An anchor next to a tag whose URI contains `&` keeps its name** —
  `anchor_name_before` recovers the display name granit never reports by scanning left
  from a node for the nearest boundary `&`. But `&` is a legal URI character, and `-`
  (needed for `- &a v`) is in the boundary set, so for the writer's own
  `&anchor !tag` ordering the rightmost qualifying `&` sat *inside* the tag: `&F !-&l`
  re-read as anchor `l`. The name mutated on every round so emission never reached a
  fixed point, and because a renamed anchor silently orphans every `*F` alias that
  referred to it, this was a data-loss class, not just a formatting drift. A `&` whose
  whitespace-delimited run opens with `!` is now skipped as tag content (found as
  libFuzzer `yaml_roundtrip` crash-f44eca1d, 36 bytes minimised to 12).
- **A BOM inside a double-quoted scalar is escaped, not written raw** — the escaper's
  catch-all tested `is_control() || is_yaml_noncharacter()`, and U+FEFF satisfies neither
  (it is a `Cf` format character, and the noncharacter mask excludes it), so it fell
  through to the verbatim push and emitted a raw BOM inside quotes, where YAML does have
  an escape. The parser rejects a mid-document BOM on input, but the edit API reaches the
  writer directly (`set("$.key", "a<BOM>b")`), producing output that no longer parsed; it
  now emits `"a\ufeffb"`, which reads back as the same text. Found while chasing
  crash-2d14c6f6 - the comment and anchor positions have no escape syntax at all and are
  handled separately by ingest-side filtering.
- **A byte-order mark inside a comment or anchor no longer breaks the document** —
  U+FEFF is *restricted* to a stream's own leading byte-order mark and may not appear
  inside a document. granit surfaces it inside decoded comment text, and our own
  `anchor_name_before` text scanner swept it into an anchor name too. Both positions are
  emitted bare (`# note`, `&name`) with no escape syntax available, so re-emitting the
  BOM produced text our own parser rejected outright ("a BOM must not appear inside a
  document"; libFuzzer `yaml_roundtrip` crash-2d14c6f6, 55 bytes). Comment and anchor
  text are now filtered to in-document characters on ingest, so the AST is the single
  already-safe form and every writer stays correct by construction - the same "record
  the re-readable form" rule already applied to contentless comments (#248) and stranded
  notes (#256/#258). A note keeps its readable text (`# a<FEFF>b` -> `# ab`); one with
  nothing left is dropped rather than emitted unparseably.
- **Tag suffixes are re-encoded on write, so a decoded tag still parses** — granit hands
  the reader the *decoded* tag suffix, so the source tag `!y5%7c` arrived as `y5|`. The
  writer emitted that decoded text verbatim, but `|` is not a permitted tag character, so
  the output no longer parsed at all ("while scanning a tag, did not find expected
  whitespace or line break") and the round trip broke immediately (libFuzzer
  `yaml_roundtrip` crash-b91536ce, 7 bytes `!y5%7c `). Tag emission now percent-encodes
  characters outside the tag URI set (`%` included, so a literal percent cannot start a
  fresh escape), restoring a readable spelling that is identical on every round. Tags
  needing no escape are emitted unchanged.
- **A note on a block item's dash line binds to the item it annotates** — a trailing
  comment (`Placement::Right`) was always attached to the most recently created node,
  even when it sat on a later line. In `- :\u{feff}:\n- #e` the note on the second item's
  dash line landed on the *first* item's value, so the writer spilled it inside the first
  item's block; re-reading bound it to the second item instead, and ownership flipped on
  every serialize round (libFuzzer `yaml_roundtrip` crash-aee06aca). `attach_inline_comment`
  now compares the comment's line against the backwards candidate's: a note on the same
  line still binds inline, and a note on a block scalar's header line still binds (granit
  spans the node at its *content*, a later line than the header), while a note on a later
  line is carried forward as the next node's leading note. Anchors, block scalars and
  empty-container slots are otherwise untouched.
- **A trailing comment on a block container survives the round trip** — the writer
  cannot hang an inline note (`meta.comment`, `standalone = false`) on the same line as a
  *block* mapping or sequence, because no line is left after the last item, so it spills
  the note onto its own trailing line. On re-read granit reports that shape as a standalone
  comment with no node following it, and the receiver left it stranded in the pending slot,
  so the note vanished on the second serialize (`&"\n-\r... #-o` -> `&" \n- ~\n# -o\n` ->
  `&" \n- ~\n`; libFuzzer `yaml_roundtrip` crash-96fa252c). A comment still pending at
  `DocumentEnd` is now flushed onto the finished document's root - the very slot the writer
  read it from - so the round trip stays stable *and* keeps the comment.
- **Block-header detection is anchored to the scalar's byte span** — the re-derivation
  (#250) scanned upward from the content line by the parser's line number and took the
  first `|`/`>`, so a `|` inside a key (`"k:yam  |1": |` or the plain `k:yam  |1: |2`) or
  a content-line sigil was misread as the header, parsing a bogus indentation indicator
  and drifting `|` -> `|1` across rounds (libFuzzer `yaml_roundtrip` crash-cad17b2b,
  extending crash-bdf3f15f). Detection now anchors to the scalar's own source byte span,
  reads only the physical line above its content, and takes the first sigil whose tail
  satisfies the block-header grammar (at most one indentation digit and one chomping sign,
  then only spaces or a `#` comment to end-of-line). Correct by construction for every
  shape, immune to granit's `\r` line shifts (it treats only `\n` as a break), and it
  removes the quadratic upward re-scan from the block-scalar hot path. Well-formed
  `key: |`/`key: |2`/`key: |-2 # c` headers are unaffected.
- **Width-folding no longer corrupts a long plain scalar's spacing** — a plain
  scalar longer than the wrap `width` is folded at spaces, and a folded line break
  re-parses to a single space. Folding beside a run of 2+ spaces (or a tab) left
  trailing spaces that re-read to a different space count, so the value drifted
  across rounds (libFuzzer `yaml_roundtrip` crash-9ee754bf: `…999  y|` folded to
  `…999 \n y|` then `…999\n y|`). `write_plain_scalar` now emits a value that
  contains a multi-space run or a tab unwrapped (one long, lossless line);
  single-space values still fold and stay stable, and the value is always exact.
- **A block scalar's indentation indicator is detected above its content** —
  `detect_block_header` scanned upward from the block's first content line but
  started *on* it, so a content line containing `|`/`>` could be parsed as the
  header. granit counts only `\n` as a line break, so a source `\r` kept
  `key: |2` and a `|`-bearing content line on one logical line; re-emitting them
  split onto `\n`, shifting which line the scan hit and flipping the indicator
  `|2` <-> `|` every round (libFuzzer `yaml_roundtrip` crash-bdf3f15f). The header
  is now located only on a line strictly shallower than the block content, so
  content is never mistaken for it; load-bearing indicators (content deeper than
  the declared one) are preserved and the value is unchanged.
- **A plain scalar equal to a document indicator is now quoted** — granit reads a
  leading-space `...` as the string `"..."`, but the writer emitted it bare, and
  `...` at the start of a line is the document-end marker, so the value re-read as
  null (`...` -> `null`) and drifted every serialize round (libFuzzer
  `yaml_roundtrip` crash-41acfbbe). `needs_double_quoted` now treats a value that
  is exactly `...`/`---` OR begins with `... `/`---` (marker + space) as needing
  quotes — the prefix case folds a following token onto the marker line and then
  fails to re-parse at all (crash-08f05e25: `... k` -> "invalid content after
  document end marker"). `---`/`-…` was partly caught by the leading-`-` rule; `...`
  starts with `.` so it slipped past; no other plain scalar is affected.
- **A contentless comment is no longer stored or emitted** — granit surfaces a
  bare `#` / `#` as an empty `Event::Comment`, but it will not re-read one, so
  the writer emitted a stray `#` line that the next parse dropped: a trailing
  `#` that drifted the document every serialize round (libFuzzer
  `yaml_roundtrip` crash-0de6be17). Both the AST and stream receivers now skip a
  comment whose trimmed text is empty, so a comment with nothing to say is
  neither recorded nor emitted; non-empty comments are untouched.
- **An empty block scalar no longer carries a stray indentation indicator** —
  an empty `|`/`>` body has nothing to measure an indent against, so granit drops
  an explicit indicator on re-parse, yet `detect_block_header` had read the `2` of
  `|2` into the AST and the writer re-emitted it, so `|2` drifted to `|` every
  serialize round (libFuzzer `yaml_roundtrip` crash-d4ea8a23). Both block writers
  now omit the indentation indicator when the value is empty, making the empty
  shape idempotent; non-empty block scalars keep their indicator unchanged.
- **A mapping key now keeps its anchor, tag, and empty-key quoting on emit** —
  `write_scalar_for_key` wrote the key's scalar token but dropped the key node's
  anchor and tag (the properties a value scalar emits) and left an empty plain key
  bare, so a key like `&f& !&&f&&&` (an anchor and a tag on the empty string)
  emitted `:` and re-read as the null `~` scalar — anchor and tag lost, and the
  round-trip drifted `: ~` → `~: ~` (libFuzzer `yaml_roundtrip` crash-62bcff6f).
  Keys now carry their anchor/tag exactly like values, and an empty key is quoted
  (`""`) so it re-reads as the empty string rather than null. Complex (`? `) keys
  were already correct — they route through the property-emitting node writer.
- **Anchor names are recovered per node from granit's own anchor site, not a
  whole-text pre-scan** — granit does not surface the `&name` text of an anchor,
  so the parser had recovered it by running a hand-written quote/escape/comment
  state machine over the raw source (`extract_anchors`) and pairing the *Nth*
  scanned name to the *Nth* anchored event with a counter (`anchor_name_idx`).
  That positional pairing desynced the moment the state machine mis-classified a
  single byte class — a bare apostrophe (`bas'e`), an embedded `&` (`sbb&e`), a
  single-quote backslash — each previously its own fix and its own libFuzzer
  `yaml_roundtrip` crash, and each of which then mislabeled every *later* anchor.
  The pre-scan is gone: granit's event marks a node anchored (`anchor_id != 0`)
  and gives its exact source span, so the name is now read back locally at that
  span as the same maximal `is_anchor_char` run granit's scanner used
  (`anchor_name_before`), keyed by granit's authoritative id. Recovery is
  position-isolated, so an unreadable byte class can only affect that one node,
  never shift another anchor's name — the whole drift family is closed by
  construction rather than patched per shape. It also drops one full document
  scan from every parse. The BOM-in-an-*anchor-name* emit-representability gap is
  a distinct root cause and stays tracked separately.
- **A backslash no longer escapes the closing quote of a single-quoted
  scalar in the anchor scan** — `extract_anchors` ran its escape state
  machine inside single quotes too. YAML single-quoted scalars have no escape
  processor (only `''`), so the `\` before a key's closing `'` (a
  backslash-terminated single-quoted key like `'a\'`) was read as escaping the
  `'`, the quote never closed, and every `&anchor` after it was hidden — the
  value's anchor vanished on re-parse and the round-trip drifted (libFuzzer
  `yaml_roundtrip` crash-12f01ee0). Backslash escaping now applies only inside
  double-quoted scalars; anchor-free, single- and double-quoted documents scan
  exactly as granit reads them.
- **A `&` embedded in a plain scalar is no longer read as an anchor** —
  `extract_anchors` harvested every `&` outside quotes, including one inside a
  plain scalar (the `&` of a bare key like `sbb&e`). granit starts an anchor
  only where a node can begin, so that phantom `&e` name was pushed onto the
  ordered `anchor_names` list and desynced the index-based id→name pairing in
  `register_anchor`: later real anchors got mislabeled (`&b` re-emitted as
  `&e:`) and the round-trip drifted (libFuzzer `yaml_roundtrip` crash-83cc68c6).
  Anchor extraction now gates the `&` on the same node-boundary test the quote
  state machine uses (line start or after `\t:,[]{}-`), so `sbb&e` stays a
  plain key. Anchor-free and correctly-anchored documents scan identically.
- **Duplicate keys are rejected by value, not by full node** — the AST
  `IndexMap` keys by the whole `CustomNode`, so two scalar keys with the same
  text but a different trailing comment / style / anchor (`key # a` vs
  `key # b`) stayed distinct: no duplicate fired at parse, yet the serializer
  drops key decor and emitted two identical `key:` lines that our own parser
  then rejected on re-parse (libFuzzer `yaml_roundtrip` crash-3b0a7d1d — the
  emitted document was unparseable). Duplicate detection now identifies a
  scalar key by its value, the same identity `to_yaml` emits, so such inputs
  are rejected on the first parse. The `<<` merge key stays exempt: YAML
  permits a mapping to repeat it.
- **Empty block containers serialize as a mapping value inline** — an empty
  `Mapping`/`Sequence` has no block form, but a block-style empty value was
  emitted as `key:` with the `{}`/`[]` on the next indent. Re-reading that
  yields a *flow* collection, so `flow_style` flipped and the next round
  inlined it — `key:\n  {}` vs `key: {}` drifted every serialize (libFuzzer
  `yaml_roundtrip` crash-d0e84310). Empty containers now always emit inline
  (`key: {}`), including a value carrying an anchor/tag (`key: &a {}`); the
  next-line pre-emit is skipped for them so the header is never doubled.
- **A bare apostrophe in a plain key swallowed every later anchor** —
  `extract_anchors` runs a quote-state machine to skip `&` inside quoted
  scalars, but it toggled on any `'`/`"` even one embedded in a plain scalar
  (the `'` of a bare key like `bas'e` or `a'`). That phantom quote stayed open
  for the rest of the document, so the pre-scan returned no anchor names and
  `register_anchor` handed every node `None` — anchors silently vanished from
  the emit and the round-trip drifted (libFuzzer `yaml_roundtrip`
  crash-68da2420). Quote *opening* is now gated on a token boundary (line
  start or after `\t:,[]{}-`), matching granit; a quote inside a plain scalar
  is literal content, while a real quoted scalar still hides its `&`.
- **Literal block scalars force an indent when the first line is blank** —
  the AST stores a `|`/`|N` body de-indented and drops the source's explicit
  indicator, so a value whose first content line starts with a blank but whose
  later lines are shallower (` 1|l\n:t\n`) re-emitted with no indicator let
  granit take the deeper first line as the block indent and read the shallower
  line as a dedent — the output no longer re-parsed (libFuzzer `yaml_roundtrip`
  crash-e432d4b8). The literal writer now mirrors the folded writer and forces
  an indentation indicator in exactly that case, so auto-detection is skipped
  and the leading blanks stay content. Documents without a blank-first-line
  body serialize byte-identically as before.
- **Folded writer kept the break *after* a more-indented line** — granit's
  fold rule tracks a `leading_blank` flag that a more-indented line (a
  continuation starting with a space or tab) sets so that *both* the break
  before it *and* the break after it stay un-folded. The emit-side run rule
  only knew the first half: it keyed the suppression off the previous line,
  so a more-indented line followed by a plain one over-padded the run by one
  newline and each serialize round gained a blank line (libFuzzer
  `yaml_roundtrip` crash-b7a2285e). The rule now keys on the line just
  written, so an r-newline run touching a more-indented neighbour on either
  side emits exactly r physical newlines; pinned with full value fidelity
  (re-parse preserves the scalar value), not just byte idempotence.
- **Folded scalars re-read their own newlines** — granit's folded read
  re-reads k blank lines (leading or between text lines) as exactly k
  newlines, but the line-splitting writer emitted one blank too few per run,
  so every folded value with multi-newline runs shrank a newline each
  serialize round (libFuzzer `yaml_roundtrip` crash-490c4beb: 4 → 3 → 2 → …;
  crash-6288e5be drifted the leading blanks the same way). The writer is now
  fold-aware — a run of r newlines occupies r blank lines, with the header
  break counted for leading runs and one blank less before a more-indented
  continuation line (which preserves its own break) — closing every run length
  by construction (verified stable for inner, leading and more-indented runs 1
  through 5; a first content line starting with a blank forces an explicit
  indentation indicator so its blanks stay content).
- **Block scalar emission is now closed under re-parse** — two granit-read
  shapes the serializer never matched: a `Clip` block scalar whose value
  carries trailing blank lines round-trips only under the `Keep` indicator
  (a Clip read strips trailing blank lines — the only header form that
  re-reads to the same value at any document position; libFuzzer
  `yaml_roundtrip` crash-c18cb1fd), so the emit promotes it — and an inline
  comment on a block scalar rides the header line (`y: |  # c`) instead of
  its own line, where it was absorbed as block content (crash-cfb3fa83).
  Both rules are pure emit-side normalizations: every previously stable
  document still serializes byte-identically.
- **Anchor-name grammar aligned with granit — root-fixes the whole drift family** —
  `extract_anchors`/`scan_anchor_name` had grown two hand-written branches
  granit's scanner does not have: a quoted-anchor form (`&"a b"` with spaces)
  and a value-indicator rule (a `:` before space/EOL ends the name). granit
  reads the name as a single maximal run of `is_anchor_char` (`:`/`#`/`"`/`&`
  are ordinary name chars; the run ends only at whitespace / line break / flow
  indicators — granit's own issue14 tests). Every divergence shifted the
  id↔name pairing and broke a round-trip; the four entries below (#215/#218/
  #227/#228) were symptoms of this one cause. The scanner now mirrors granit
  exactly (maximal run + the anchor token is skipped atomically so a `"`/`#`
  in the name no longer desyncs quote/comment state) and `write_anchor_tag`
  emits `&name` bare — closure holds by construction, subsuming the per-shape
  workarounds; quoted anchors (never round-trippable) are dropped.
- **Quoted anchor names could swallow a line break** — `scan_anchor_name`'s
  quoted branch treated any later `"` in the buffer as the closing quote, so
  `&"X-<CR>:&"X-` read the name across the carriage return into `X-\r:&`. The
  serializer emitted that raw and the re-parse wrapped one extra layer each
  round (a growing libFuzzer `yaml_roundtrip` non-idempotence, 11 bytes).
  granit ends an anchor token at a CR/LF break, so a closing quote past a line
  terminator no longer qualifies as a quoted anchor — names stay single-line
  and re-emit stable.
- **Anchor names ending in `:` were emitted in an unstable form** —
  `write_anchor_tag` wrote every anchor as a bare `&name` token. When the
  parsed anchor name ends in `:` (reached through an unterminated quoted
  anchor like `&"X-::…:`), the trailing `:` merged with the emitted space into
  a value indicator and was dropped on re-scan, so each serialize round lost
  one character — a 42-byte libFuzzer `yaml_roundtrip` find where
  `fmt(fmt(x)) != fmt(x)`. Unsafe names (trailing `:`, embedded whitespace or
  flow indicators) are now emitted as quoted `&"name"` anchors, which the raw
  scanner reads up to the closing quote, preserving the exact bytes across
  rounds.
- **Nested self-referential merge anchors overflowed the native stack** — a
  mapping anchored with `&b` whose body re-uses `*b` (directly or through a
  second `&b`) fed `resolve_mapping_merges`' tail recursion with the path
  cycle-guard already popped, so every walk re-expanded a fresh clone of the
  anchor and the descent grew without bound (libFuzzer `parse_yaml`, 58-byte
  `bas: &b … <<: *b …`). The tail walk now recurses only into the mapping's
  *own* children — merged-in clones are resolved under the guard in the
  expansion loop — and a `MAX_MERGE_DEPTH` budget turns any residual runaway
  into a graceful stop, matching the parser's container-depth and serializer
  `max_depth` guards.
- **The raw anchor scanner invented anchors the emitted text cannot keep** —
  `extract_anchors` accepted `:` followed by space/end-of-line into anchor
  names (`&&&&:` → `&&&:`), harvested anchors from comment text, and rescanned
  the overlapping `&` characters inside accepted names (`&&&&` yielded
  phantom `&&&`, `&&` and `&` anchors), shifting the id→name pairing of every
  later anchor. Each quirk let serialized documents re-parse to different
  anchor names — the 12-byte libFuzzer find `&&&&:<LF>#&&&:&` drifted a
  character per round. Names now end at a value-indicator colon, comment text
  is skipped, and accepted anchor tokens are never re-scanned.
- **Double-quoted scalars were decoded twice** — granit delivers
  double-quoted values already unescaped, but both receivers ran
  `unescape_double_quoted` over them again: `a: "\\n"` (the literal two
  characters `\` `n`) silently collapsed to a newline, and each
  serialize/re-parse round stripped one backslash (libFuzzer
  `yaml_roundtrip`: `!-# \\f"<TAB>0:!`). Both call sites are now pass-
  through, with stream/AST unit tests pinning the single-decode contract.
- **Unterminated-quote anchor names swallowed the rest of the line** —
  given `&"X-<CR>:`, `extract_anchors`' quoted scan collected to end-of-line
  because no closing quote ever arrived, putting a raw CR and colon into the
  anchor name; the serializer emitted `&X-\r:` verbatim, and granit ends
  anchor names at whitespace, so re-parsing yielded `X-` — serialize
  idempotence (`fmt(fmt(x)) == fmt(x)`) broke on a 6-byte input found by
  `fuzz/yaml_roundtrip`. An unterminated `"` now stops at exactly the
  character granit's unquoted anchor token stops at; genuine `&"quoted
  anchor"` names (spaces included) are unchanged.
- **JSON comment scanners could panic mid-character** — the line-comment and
  unterminated-block-comment scans in `ws()` stepped `pos` one *byte* at a
  time, so a trailing multi-byte char (e.g. U+FEFF) could leave `pos` inside
  it; the next `&text[pos..]` slice panicked with "not a char boundary"
  (found by `fuzz/parse_json` in ~25 seconds: `\r\r{aMNaN/*0\u{feff}`).
  Line comments now advance a full code point and an unterminated block
  comment rewinds to its `/`, so every failure path is a typed error again.
- **Linux free-threaded (`cp314t`) wheels ship in the Release** — the wheel
  matrix only built free-threaded artifacts for Windows and macOS, so Linux
  users on the GIL-less interpreter had nothing to install: the GIL-enabled
  `cp38-abi3` wheels are ABI-incompatible with `Py_GIL_DISABLED` builds and
  the `abi3t` wheel only starts at CPython 3.15. The `linux` job now builds
  manylinux cp314t wheels for x86_64 from the image's own free-threaded
  interpreter and smoke-tests the wheel in a `3.14t` venv before it is
  attached (aarch64 stays out: maturin must execute the target interpreter
  for a non-abi3 wheel, and that exec fails under qemu-user).
- **The `pyq` release job actually builds its Linux artifacts** — the
  cross-architecture legs register qemu binfmt handlers for `cross`'s
  emulated containers, and their smoke tests exec the fresh binary *inside*
  the manylinux image: the host has the qemu translator but no foreign
  `/lib/ld-linux-*.so` loader, so a direct host exec of the aarch64/armv7
  binaries dies before `main`. Every leg builds and self-verifies before its
  archive is uploaded.
- **Block scalars keep their explicit indentation indicator** — a body
  written as `key: |2` had the `2` silently dropped on write, so the output
  re-parsed differently from the input whenever the first body line sat
  deeper than the rest (exactly the `4RWC.yaml` shape: first line indented
  6, continuation 4). The reader auto-detects indentation from the first
  body line when no indicator is present, so losing it is a real semantic
  change, not cosmetic. The indicator now rides the AST and is re-emitted in
  spec order (`c-b-block-header`: chomping first, then indentation, so a
  stripped scalar is `|-2`), and the writer measures the body from the line
  that carries the header rather than from the parent node's column.

#### Performance

- **Tag emission is table-driven, and escaping no longer allocates** — moving tag
  encoding onto the reader's own character classes put the per-byte membership test
  on the serialize hot path, so it became a 128-entry compile-time table (the
  alphanumeric test folds into the same lookup) and the `%XX` escape is written from
  a hex-digit table instead of `format!`, which allocated a fresh `String` for every
  single escaped byte. Measured in one process with alternating best-of-6×40 batches
  over the suffix corpus the serializer actually meets: 10.53 ns → 2.55 ns per
  11-suffix sweep (4.1×). The two related substitutions landed ahead as well — the
  schema resolver's edge trim 0.47 → 0.20 ns (2.3×, because YAML's blank set is five
  comparisons where Unicode `trim` consults a character-property table), and the
  anchor-name character test 1.61 → 1.14 ns (1.4×) once it grew an ASCII fast path.
- **A document with many null keys parses in linear time** — the fold described above
  first rescanned the mapping for every null key. That is invisible on an all-null
  document (its folded entry sits at slot 0) and quadratic on the shape that matters:
  2k distinct keys followed by 2k null keys grew 12.4x for a 4x input (99 ms at
  8k + 8k). The mapping now remembers its null key's slot, keeping the scan only as a
  correctness fallback: 11.0 ms for that same input and 4.14x growth — the same slope
  a distinct-key document already had (4.02x).
- **Cross-process divan tables cannot settle a sub-10% question on this box** — the
  same binaries differed by up to ±38% run to run (`parse_medium` read +77% in one
  batch and −20% in another), so the predicate numbers above come from an in-process
  A/B instead, and the end-to-end claim is left to the CodSpeed gate that runs on
  every PR touching `crates/**`. `cargo nextest run --all` stays 449/449 and every
  committed fuzz seed replays to the same bytes.

### [v0.17.0] — 2026-10-01

#### Added

- **pyq CLI parity flags** — `pyq fmt` gains `--indent N` (block indent,
  default 2), `--width N` (plain-scalar soft-wrap column, 0 disables),
  `--sort-keys` (serializer-level whole-document key sort) and
  `-i/--inplace` (rewrite the file), exposing the
  `pyrs-yaml-core::SerializeOptions` knobs and matching the Python CLI's
  `fmt --indent`. `pyq to-json` gains `--jsonc` / `--json5` dialect
  output (mutually exclusive), wiring the `pyrs-json` comment-preserving
  and JSON5-spelling writers (`to_jsonc_text*`, `to_json5_text*`) into
  the CLI.
- **pyq JSONC/JSON5 input dialects** — `--input jsonc|json5` joins the
  `Format` enum (auto-detection also keys off `.jsonc` / `.json5`
  extensions) and routes through the native `pyrs-json` dialect
  parsers, so comments and JSON5 spellings ride the AST; combined with
  `to-json --jsonc` this gives a comment-preserving
  JSONC→JSONC round-trip in one command. `--all-docs` rejects the
  single-document dialects with a stable message.
- **`pyq diff` / `pyq merge`** — semantic document comparison and
  right-biased deep merge (yq `*+` shape), new in the native CLI.
  `diff` walks both ASTs comparing resolved values, structure and tags
  (comments/quoting/layout never appear) printing `-/+ /~` path lines,
  exit 0 equal · 1 differs. `merge` overlays mappings recursively,
  appends sequences (or swaps them with `--replace-arrays`), and emits
  round-trip YAML; both commands read any supported input dialect via
  `--input`/extension detection.

#### Changed

- **GitHub Release is created by `publish.yml`** — the release page used to be
  made by hand (`gh release create`) after each publish, which is one more step
  to forget and one more place for the published version and the tag to drift
  apart. The `release` job now runs `gh release create` after `uv publish`
  succeeds, gated on the same `refs/tags/` condition, with generated notes and
  the built wheels attached — notes and assets come from the tag PyPI was
  published from. `workflow_dispatch` runs are unaffected (no release, no
  PyPI publish), matching the existing behavior.
- **`README.md` / `README.zh-CN.md` document the native `pyq` CLI** — both
  READMEs gained a `pyq` subsection next to the Python CLI section: how to
  install it from a checkout, three worked examples, and the full command
  list, pointing at the pyq guide for detail.

#### Fixed

- **`pyrs-json` module documentation** — it still claimed comments were
  stripped on read and never re-emitted; since #122 they ride the AST
  comment slots and the JSONC/JSON5 writers reproduce them.

### [v0.16.0] — 2026-10-01

#### Added

- **JSONC block-comment hot-sample bench** — objective §测试覆盖 5
  names "block-comment" as a required hot sample; previously only
  inline `//` comments were measured. New fixture drives
  `test_load_jsonc_block_comments` with 50 pairs + header/footer
  blocks, catching scanner regressions on the `/* ... */` path via
  CodSpeed.
- **PyYAML + ruamel.yaml cross-library parity for YAML** — objective
  §测试覆盖 3 names both as oracles; previously only benchmarks and a
  printout. `tests/test_yaml_crosslib.py` closes the correctness gap:
  20 canonical documents × 5 parity surfaces + 2 documented divergences
  (dup-key strictness, YAML 1.1 legacy booleans schema-scope) = 122
  tests. Optional deps skipif clean.
- **tomlkit cross-library parity for `load_toml`** — objective §测试
  覆盖 3 names tomlkit as an oracle; previously tomlkit only appeared
  in benchmarks. `tests/test_toml_crosslib.py` adds 24 tests covering
  11 canonical constructs with three-way agreement (pyrs/tomlkit/
  tomllib), pins the `>i64` spec-strict divergence, and asserts the
  `-2^63` boundary. Optional dep, skipif clean.
- **orjson as strict-JSON oracle for `load_json`** — objective
  §测试覆盖 3 required correctness parity against orjson; previously orjson
  only appeared in benchmarks. 16 canonical documents assert byte-for-byte
  agreement, and 12 non-strict forms (comments, trailing commas, single
  quotes, `NaN`/`Infinity`/`-Infinity`, hex, leading zero, `+.5`, `5.`) are
  rejected by both orjson and `load_json`. orjson rejects the bare literals
  that stdlib `json.loads` accepts under `allow_nan=True`, so it is the
  stronger RFC 8259 oracle. Optional dep, `skipif` clean.
- **CLI ↔ Binding parity gate (`tests/test_cli_binding_parity.py`)** — the
  Pillar 1 invariant is now an executable contract: CLI's registered
  commands are checked against a fixed 18-name inventory, every `to-X` /
  `from-X` verb requires its matching `YamlDocument.to_X` / `from_X` /
  `load_X` sibling, the `load_*` family symmetry (json/jsonc/json5/toml)
  is asserted, and editing/validate/compliance verbs map to live Python
  API. A drift on either surface now fails CI.
- **`load_json` property tests + CodSpeed benches** — Hypothesis
  (`test_load_json_matches_stdlib_json` + `test_load_json_matches_load_jsonc_on_strict_domain`)
  pins the strict loader against `json.loads` for every generated canonical
  document and asserts both loaders agree on the strict domain; a widening of
  the fast path or a fallback drift now surfaces as a property failure.
  Three CodSpeed benches (`test_load_json_large` / `_floats` / `_escapes`)
  mirror the load_jsonc samples so the strict binding layer is tracked.
- **`load_json` (strict) — completes the `load_*` family parity** — the binding
  exposed `load_jsonc` / `load_json5` / `load_toml` but the strict RFC 8259
  counterpart was missing. `pyrs_yaml.load_json(s)` now matches `json.loads` on
  canonical documents and rejects the JSONC/JSON5 extensions (`//`, `/* */`,
  trailing commas, single quotes, bare `Infinity`/`NaN`, `0x…`) with a typed
  `YamlParseError`. The fast path shares `json_fast::try_load` with
  `load_jsonc` (which bails on every non-canonical byte — zero widening risk);
  declined constructs route through the STRICT `from_json` AST parser. Closes
  the last gap under the CLI ↔ Binding parity entry: every CLI format now has
  its `load_*` sibling, Pillar 1 complete. Re-exported and in `__all__`;
  `.pyi` regenerated via `maturin generate-stubs`.
- **Dialect writer fixed-point properties** — `fmt_pbt.rs` promised a writer
  fixed point in its header but never implemented one. Four proptests now hold
  it for JSON/JSONC/JSON5/TOML (the sole input filter drops hand-built ASTs whose
  distinct keys spell the same JSON name — outside RFC 8259's object domain).
  The gate immediately surfaced three real comment-fidelity defects (Fixed).
- **Hot-spot benchmark corpus** — seven CodSpeed wall-time benches target the
  historically fragile serialization paths: YAML block-scalar documents (all six
  header spellings `|`, `|-`, `|+`, `>`, `>-`, `>+`) and comment-dense documents,
  TOML multi-line strings / radix integers / underscore separators / exponents /
  datetimes, and JSON5 exotic number forms (hex, `+.1`, `5.`, `Infinity`, `NaN`,
  single quotes, trailing commas). Fixtures live in `tests/data/yaml_samples.py`,
  benches in `tests/test_benchmark_api.py`. Building this corpus is what surfaced
  the nested block-scalar indentation bug fixed below.
- **Text-level re-parse gate (`prop_output_always_parses`)** — the Rust proptest
  suite now asserts that every generated AST serializes to text the parser
  accepts again. The AST-vs-AST round-trip property silently skipped shapes
  whose serialized text could not re-parse (`try_roundtrip` returns `None`),
  leaving an entire defect class invisible; the new gate caught six real
  serializer bugs on its first runs (see the Fixed entries below), each now
  additionally pinned by targeted Rust unit tests and a Python regression class
  (`TestNestedBlockScalarIndent` in `tests/test_roundtrip_bugs.py`).
- **TOML datetime offset range** — a numeric UTC offset is now range-checked
  (hours 00..=23, minutes 00..=59); `+12:60` / `+24:00` are rejected instead of
  accepted. Shape was checked but values never were (toml-test
  `invalid/datetime/offset-overflow-minute`; illegal in both 1.0 and 1.1).
- **TOML table-redefinition strictness** — a table created implicitly by a dotted
  key is now closed: a later `[header]` may not re-open it (`[fruit]` + `apple.color`
  then `[fruit.apple]`; `[t1]` + `t2.t3.v` then `[t1.t2]`), and a table cannot be
  redefined as an array (`[tbl]` then `[[tbl]]`). Invalid under BOTH 1.0 and 1.1
  (toml-test `invalid/table` duplicate-key/redefine); legit super-tables and sibling
  dotted keys still parse (mis-acceptance 30 -> 21, valid decode-match unchanged).
- **toml-test conformance harness** — `tests/test_toml_test_suite.py` runs the
  official [toml-test](https://github.com/toml-lang/toml-test) corpus the same way
  `test_yaml_suite.py` runs the YAML suite: untracked local artifact, `skipif` on
  absence, measured-floor gates, and a type-tag adapter for decode comparison.
- **TOML temporal types decode correctly** — date-only and time-only values now
  carry distinct `!date`/`!time` tags (date-times keep `!timestamp`) so they use
  `date`/`time.fromisoformat` instead of crashing. toml-test surfaced a bare time
  (`07:32:00`), a seconds-less time (`13:37`), and a lowercase-delimiter datetime
  (`1987-07-05t17:45:00z`) each raising `ValueError` on valid TOML; `!time` now
  pads omitted seconds, `!timestamp` canonicalizes lowercase `t`/`z`, and both
  normalize a fractional-second field of any precision (TOML `.6`; pre-3.11
  `fromisoformat` wants 3 or 6 digits) to microseconds.
- **TOML control-character strictness** — raw C0 codes (NUL, FF, DLE, US, ...) and
  DEL (U+007F) are now rejected inside basic, literal and multi-line strings (only
  tab, plus newlines in multi-line forms, stay legal). toml-test's `invalid/control`
  corpus surfaced 13 such mis-accepted documents; comment bodies now reject the
  same control codes (5 more `comment-*` documents) and a bare carriage return
  (0x0D outside a CRLF pair) is rejected by a central entry scan (4 more `*-cr`
  documents). The conformance harness now reads corpus bytes verbatim — its earlier
  `read_text()` applied universal-newline translation, rewriting bare CR to LF and
  masking the whole class.
- **TOML number-literal strictness** — leading-zero decimals (`01`, `-01`), a sign
  on radix-prefixed integers (`+0x1F`, `-0b101` — `signed-int` is decimal-only),
  and trailing/double underscores (`1_`, `1__0`) are now rejected. toml-test
  `invalid/integer` + `invalid/float` surfaced 23 mis-acceptances (total 71 -> 48);
  the prior "radix integers can be signed" behavior was a spec violation.
- **TOML inline-table key-collision strictness** — an inline table rejects a dotted
  key that equals, extends, or is shadowed by a defined path (`{ a = 1, a.b = 2 }`,
  `{ a.b = 1, a.b.c = 2 }`); sibling paths (`{ a.b = 1, a.c = 2 }`) stay legal.
  toml-test `invalid/inline-table` duplicate-key/overwrite surfaced these (total 48 -> 39).
- **TOML non-ASCII string crash fixed** — the basic and multi-line basic string parsers
  advanced byte-wise and could slice mid-character on multi-byte input (U+00A0, …),
  panicking; both now consume whole characters. Found via the toml-test corpus; completes
  #153 for the single-line/JSON paths.
- **Format fuzz + robustness fixes** — new `proptest` property tests fuzz the
  TOML/JSON/JSONC/JSON5 parsers and writers (no-panic + re-parseability). They
  found and fix a mid-character slice panic in the TOML and JSON string parsers
  and a JSONC/JSON5 inline-`//`-comment bug that swallowed the following `,`/`}`.
- **YAML merge/alias property fuzz** — the `proptest` suite now generates well-formed
  anchor/alias/merge-key docs (single/sequence aliases, inline-map merges, and the
  scalar/null sources the #166 fix rejects), including a self-referential anchor and
  repeated alias refs. `arb_custom_node()` emits `meta.anchor` but never an `Alias`,
  so the alias/merge paths (the #163/#166 structure class) were previously unfuzzed
  in-process. `prop_merge_alias_never_panics` asserts parse + merge resolution never
  panic or overflow and that any parsed tree re-serializes and re-parses cleanly.
- **CLI format parity** — the CLI gains `to-toml`/`from-toml`, `to-jsonc`/`from-jsonc`
  and `to-json5`/`from-json5`, mirroring `to-json`/`from-json`, so every binding
  format is reachable from the command line.
- **JSON string-escape fast path (perf)** — `load_jsonc` decodes the eight simple
  two-byte escapes inline instead of bailing the whole doc to the AST path; escape
  JSON stays on the fast path (~15x faster than the AST route). Matches
  `json.loads`; `\u`/invalid escapes still route through the AST path.
- **JSON float fast path (perf)** — `load_jsonc` parses canonical floats
  (decimals/exponents) directly into Python objects instead of bailing the whole
  doc to the AST path; values match `json.loads` exactly (correctly-rounded
  parse). A new benchmark exercises the branch for regression tracking.
- **Faster JSON string serialization (perf)** — strings with no escapable byte
  are bulk-copied in one `push_str` instead of per-char UTF-8 re-encoding;
  string-heavy `to_json` ~35% faster (41 -> 27 ns/item), byte-identical output.
- **`YamlDocument.to_toml()`** — documents emit TOML straight from the AST,
  mirroring `to_json`/`to_jsonc`/`to_json5`, instead of the `to_toml(doc.to_yaml())`
  round-trip. Byte-identical output; the writer is ~4.3x faster than `tomli_w`.
- **`to_json` native serializer (perf)** — `YamlDocument.to_json` now uses the
  native engine instead of `to_dict()` + `json.dumps`. Byte-identical for ASCII,
  ~10x faster (1200-item ~1450µs → ~120µs, beating `json.dumps`). Non-ASCII now
  raw UTF-8 (like `to_jsonc`/`to_json5`) instead of `\uXXXX`; still valid JSON.
- **JSON object keys written directly (perf)** — the writer emits mapping keys
  straight into the buffer (no per-key `String` alloc); compact `to_json` ~2x
  faster (~120µs → ~60µs), byte-identical, now #2 in the field.
- **JSON load fast path** — `load_jsonc` parses canonical strict JSON straight
  into Python objects, skipping the `CustomNode` AST (~5-6x faster, now ahead of
  stdlib `json.loads`). Non-canonical input (floats, escapes, comments, huge
  ints, trailing commas) falls back to the general path; values unchanged.
- **TOML multi-line string fidelity** — a TOML multi-line string is projected
  as a `ScalarStyle::Literal` YAML block (surviving the text hub) and re-emitted
  by `to_toml` as a `"""` block instead of an escaped single line. Values
  round-trip byte-for-byte and the output is idempotent; a single-line string
  stays single-line. No AST struct change (reuses `Literal`).
- **TOML document-level comment fidelity** — `to_toml` now emits the root
  mapping's leading comment, so a document-opening standalone `# note` survives
  a TOML → hub → TOML round trip instead of being dropped (the TOML counterpart
  of the JSON writer's `emit_root_leading`). Native TOML parses and comment-free
  documents are unaffected.
- **JSON5 Unicode identifier keys** — unquoted object keys accept the full
  Unicode `ID_Start` / `ID_Continue` set (not ASCII only), so `from_json5` /
  `load_json5` parse `{ é: 1, 名: 2, हिन्दी: 3 }`. Built on the
  `unicode-ident` tables (rustc’s own lexer crate) for exact per-script
  conformance including combining marks. Gated on JSON5, so strict
  `from_json` / `from_jsonc` still require quoting. Lone UTF-16 surrogates stay
  rejected. Adds a dependency (`unicode-ident`).
- **JSON5 Unicode structural whitespace** — `from_json5` / `load_json5` accept
  the whitespace JSON5 adds to RFC 8259's four: VT, FF, NBSP, every Unicode `Zs`
  separator, the LS/PS line terminators and ZWNBSP (U+FEFF). Built on `std`
  `char::is_whitespace` (minus NEL U+0085) plus U+FEFF — no new dependency.
  Strict `from_json` / `from_jsonc` still reject all of them.
- **JSON5 line continuation and `\'` escape** — double-quoted JSON5 strings
  accept a backslash-newline line continuation (removing both) and an escaped
  single quote (`\'`); strict JSON / JSONC still reject both. Mirrors the
  single-quoted handling from #125, completing JSON5 string fidelity.
- **JSON5 string escapes `\v` and `\0`** — `from_json5` accepts
  vertical tab (`\v`) and NUL (`\0`) in double- and single-quoted
  strings; strict JSON / JSONC still reject them. Completes JSON5
  grammar with #120 (numbers) and #124 (value semantics).
- **JSON5 value semantics on load** — `load_json5` resolves the
  JSON5-only number forms (`0x1F`→31, `+7`→7, `5.`→5.0, `Infinity`/`NaN`)
  to real numbers via a new `Schema::Json5`, leaving strict JSON/JSONC
  loaders unchanged and `to_json5_text` still emitting the source
  spelling.
- **JSON5/JSONC reachable through the public API** —
  `pyrs_yaml.from_json5` / `load_json5`, and `YamlDocument.to_jsonc()` /
  `to_json5()` (native-engine emit, so comments and JSON5 styles
  survive). Also fixes a reachability gap: `from_jsonc` / `load_jsonc`
  were never re-exported from the `pyrs_yaml` package, so
  `pyrs_yaml.from_jsonc(...)` raised `AttributeError`; they now appear
  in `__all__`. `to_jsonc`/`to_json5` gained `emit_root_leading` so a
  document-level standalone comment is preserved. New JSON-family
  benchmark coverage added to `test_benchmark_api.py`.
- **JSON5 writer (`to_json5_text` / `to_json5_text_pretty`)** —
  contract-B step 2. Serializes the AST back to JSON5, restoring
  single-quoted strings and the `0x…`/`.5`/`+7`/`Infinity`/`NaN` numeric
  forms the parser preserves, plus `//` comments. Keys are always quoted
  (lossless). Shares an internal `Mode` (Json/Jsonc/Json5); strict and
  JSONC output unchanged.
- **JSON5 numeric forms on the parser** — `from_json5` (new
  `allow_json5_numbers` axis) accepts hex (`0xDECAF`), leading/trailing
  dot (`.5`, `5.`), leading `+` (`+7`), leading zero (`07`), and bare
  `Infinity` / `NaN` / `-Infinity`, each keeping its exact source text
  for a future JSON5 writer. STRICT / JSONC stay gated off and reject
  them as before. `from_jsonc`'s stale "comments are stripped" doc was
  corrected.
- **TOML inline-table interior comment fidelity** — PR #119 captures
  interior `# ...` comments in inline tables (own-line above a member
  → leading, same-line after its value → trailing) and threads them
  through the IR so they round-trip instead of vanishing. Undecorated
  inline tables keep the compact one-line form; a decorated one nested
  in an array goes multi-line. Also repaired a latent #114 bug where
  `skip_all_blank` mistook a comment's own terminator newline for a
  blank line.
- **YAML receiver writes standalone comments into
  `decor.leading_comment`** — PR #117b moves the last engine (the
  granit-parser receiver) onto the slot convention PR #114 / #115
  established. Standalone notes on scalars / mappings / sequences
  land in `NodeMeta::decor.leading_comment` rather than the older
  `comment(standalone = true)`. Hand-built fixtures stay equal thanks
  to #117's normalisation, and `CustomNode::remove_comment` now
  clears **both** slots so Python `Node.remove_comment()` still
  behaves correctly on YAML-parsed docs.
- **Cross-slot standalone normalisation + Python
  `Node.leading_comment`** — `NodeMeta::eq` / `Hash` treat the
  standalone note as one concept across the newer `leading_comment`
  slot (TOML / JSON) and the older `comment(standalone = true)` slot
  (YAML receiver + hand-built fixtures). Setters / removers act
  atomically on both, and the YAML serializer reads the normalised
  view so `to_yaml(toml_ast)` keeps its leading notes. Python
  `Node.leading_comment` getter / setter / remover mirror
  `Node.comment` and now surface TOML / JSONC standalone notes.
- **TOML 1.1.0 grammar** — `from_toml` parses to the v1.1.0 spec
  (2025-12-18). Four additions: **(A1)** multi-line inline tables +
  trailing commas; **(A2)** `\xHH` basic-string byte escape;
  **(A3)** `\e` = U+001B; **(A4)** optional seconds in time and
  date-time. `TomlDialect::V1_0` / `from_toml_v1_0` remain as strict
  1.0.0 escape hatch. A parser-side off-by-one in the space-separated
  date-time detection was fixed alongside.
- **JSON dual-slot comment fidelity** — the JSONC parser writes
  standalone (`// ...` on its own line) notes onto the new
  `leading_comment` slot from #114, while inline (`// trailing`) stays
  on `comment`. Object members and array elements can now carry both
  notes on the same node — impossible under #112's single-slot model.
  The writer prefers `leading_comment` and falls back to `comment`
  with `standalone = true` for hand-built fixtures.
- **TOML blank-line + dual-slot comment fidelity** — `NodeMeta` grows
  `leading_comment: Option<Comment>` and `blank_before: bool` (both
  excluded from structural `Hash` / `PartialEq`), letting a section
  header or AOT element carry both the standalone note above and the
  inline trailing note after `]` without either displacing the other.
  `to_toml` reproduces blank-line groupings; the very first pair never
  emits a leading blank line. Hand-built / YAML-origin nodes still
  rendering via the writer's fallback read.
- **JSON5 dialect** — `pyrs_yaml_core::json::from_json5(text)` and
  `from_json_with_options(text, JsonParseOptions)` accept the full
  JSON5 axis set: trailing commas, single-quoted strings, unquoted
  identifier keys, and line/block comments. Each axis is togglable via
  `JsonParseOptions`; `STRICT`, `JSONC`, and `JSON5` constants ship as
  presets.
- **JSONC/JSON5 bindings and CLI** — `pyrs_yaml.from_jsonc(str)`
  returns YAML text; `pyrs_yaml.load_jsonc(str)` returns a Python dict
  / list. `pyq from-json` grows `--jsonc` and `--json5` flags so
  `tsconfig.json` / `settings.json` files feed straight into the verb
  pipeline.
- **JSONC comment preservation** — `from_jsonc` now attaches captured
  `//` line and `/* */` block comments onto the AST's
  `NodeMeta::comment` (standalone on the key node, inline on the value
  node), mirroring the TOML model from PR #109. The matching pair
  `to_jsonc_text(node)` / `to_jsonc_text_pretty(node, indent)` emits
  them back at their original positions; block comments collapse to
  `//` on output. Strict writers `to_json_text` / `to_json_text_pretty`
  remain byte-identical, so consumers opt into preservation selectively.
- **pyq multi-document edits** — `-A/--all-docs` now covers every edit
  command (set/delete/rename/move/append/insert/sort-keys) plus
  `to-json -A` (JSON array, Python parity). Each document splices
  against its own segment of the stream (`MultiDocEditor` +
  `DirtyUnit::shifted`): untouched documents and all `---` separators
  keep their exact bytes, a plan miss skips its document (Python try/
  skip semantics; all-miss still exits 1), and one layout-dirty document
  falls back alone instead of de-pinning its neighbours.
- **JSONC parsing** — `pyrs_yaml_core::json::from_jsonc(text)` and
  `from_json_with_options(text, JsonParseOptions)` accept `// line` and
  `/* block */` comments at any whitespace position (the dialect used by
  TypeScript's `tsconfig.json` and VS Code's `settings.json`). Comments
  are stripped, not preserved. Trailing commas and other JSON5-only
  forms remain rejected so the accepted language stays a strict superset
  of RFC 8259. `from_json` is unchanged (strict mode by default).
- **TOML comment fidelity** — the parser now captures both standalone
  (`# ...` on its own line above a pair or section header) and inline
  (`key = value # ...` / `[name] # ...`) comments and attaches them onto
  the shared AST through `NodeMeta::comment`. `to_toml(from_toml(src))`
  re-emits those comments at their original positions so `pyq edit` and
  `YamlDocument.set()` no longer strip annotation notes from a TOML
  round trip. Whitespace fidelity (blank-line separators between pairs)
  stays on the writer's default layout per the design doc.
- **TOML numeric source fidelity** — `to_toml(from_toml(src))` preserves
  hex (`0xDEADBEEF`) and octal (`0o755`) integer spellings and exponent
  form floats (`1e10`, `-3.14e-2`) verbatim. Underscore separators,
  explicit `+` signs, negative radix (`-0x1F`) and binary (`0b101`)
  canonicalise to decimal because YAML Core schema cannot re-read them,
  keeping the shared AST interoperable with the YAML pipeline. Comment
  fidelity and JSONC arrive in follow-up PRs per the design doc.
- **pyq feature completion** — the CLI reaches parity with the Python
  CLI's surface: `rename`/`move`/`append`/`insert` splice edits, `validate`
  (parse check, or against schema-language rules via `--schema rules.yaml`),
  `frontmatter` (`--body-out` splits the body), and `-A/--all-docs` on
  `get`/`fmt`/`to-json` for multi-document streams. Fixing the wiring
  surfaced a core engine bug: `move_path` returned only the destination
  INSERT unit, so the spliced text kept a duplicate of the moved subtree
  (invisible whenever documents fell back to re-serialization); it now
  returns both units and the bindings apply them through their batch
  splice path.
- **`pyq` filter verbs** — structured jq-style post-processing on the
  match stream: `--select 'PATH OP LITERAL'`, `--sort-by PATH` /
  `--desc`, `--unique`, `--first` / `--last`, `--skip N` / `--take N`,
  and `--join SEP`, applied in the fixed pipeline order
  `select -> sort -> unique -> slice` then `join` on `get` and the
  `from-*` converters. Deliberately flags, not an expression language:
  the predicate is one micro-grammar parse (micro ~40 lines), mixed
  comparison kinds are `false` (documented divergence from jq's total
  order), and startup stays instant.
- **`pyq completion`** — print a shell completion script for bash, zsh,
  fish or PowerShell (`pyq completion bash > ...`), powered by
  `clap_complete` (approved addition to the CLI crate's dependencies;
  it stays inside the `pyrs-yaml-cli` binary and does not touch the
  Python distribution).
- **`pyq sort-keys`** — sort the keys of the mapping at any path (`$` for
  the root), in place or to stdout, through the same core plan/splice
  engine as `set`/`delete` - closing the parity gap with the Python
  CLI's `sort-keys` command.
- **Command-line interface** — a new `pyrs-yaml` command (opt-in via
  `pip install "pyrs-yaml[cli]"`, requires Python 3.10+) exposing the library
  from the terminal: `fmt` (round-trip reformat preserving comments/anchors/
  order), `get` (JSONPath queries with `--format yaml|json|text`), `set` /
  `delete` / `rename` (path-based edits with `--inplace`, `--string`,
  `--create-missing`), `validate` (CI-friendly exit codes), and `to-json` / `from-json` conversions. All
  commands read stdin via `-` and default to stdout output. Implemented in
  pure Python (`python/pyrs_yaml/cli/`) on top of
  [Cyclopts](https://github.com/BrianPugh/cyclopts) as an optional extra, so
  the base install keeps zero extra dependencies and Python 3.8 support.
- **CLI expansion** — `sort-keys` (sort mapping keys at a path), `move`
  (relocate a subtree to an existing destination), `frontmatter` (extract
  Markdown front matter as YAML, optional body split), and `compliance`
  (YAML Test Suite report with `--json`) commands; `-A/--all-docs`
  multi-document mode on `fmt`/`get`/`set`/`delete`/`rename`/`sort-keys`/
  `validate`/`to-json`; and mutually exclusive `validate --schema <name>`
  vs `--schema-file <path>`. The undocumented `python -m
  pyrs_yaml.compliance` entry point was removed in favor of the
  subcommand.
- **`YamlStream` is now importable** — `from pyrs_yaml import YamlStream`
  works as documented in the API reference and type stubs; the class was
  previously returned by `YAML().load_stream*()` but never exported from the
  native module.
- **CLI `move --all-docs`** — `move` now accepts `-A/--all-docs`, applying the
  subtree move to every document where both paths resolve (same semantics as
  `set`/`delete`/`rename`), so the multi-document flag covers all edit
  commands.
- **Docs ↔ API consistency guard** — `tests/test_docs_api.py` scans every
  locale doc page for `pyrs_yaml.…` chains, `import pyrs_yaml…`, and
  `from pyrs_yaml … import …` claims, and fails if any referenced symbol does
  not exist at runtime (~965 claims checked).
- **Optional third-party type plugins** — `!duration` (`pendulum.Duration`),
  `!arrow` (`arrow.Arrow`), and `!ulid` (`ulid.ULID`) auto-register when the
  corresponding library is installed (`_register_third_party` in
  `python/pyrs_yaml/plugins/_builtin.py`). Each uses a distinct tag so existing
  `!timestamp` / `!date` / `!uuid` handlers are unaffected; a plain stdlib
  `timedelta` is never matched by `!duration`.
- **pydantic-settings YAML source** — `PyrsYamlConfigSettingsSource`
  (`python/pyrs_yaml/settings.py`) is a drop-in replacement for
  `pydantic_settings.YamlConfigSettingsSource` that parses with pyrs-yaml
  (YAML 1.2 core schema) instead of PyYAML. It is exported lazily so
  `import pyrs_yaml` never requires pydantic-settings; install with
  `pip install "pyrs-yaml[settings]"` (Python 3.10+). `dump_pydantic` and
  `parse_as` now use the same lazy module-level `__getattr__` export pattern.
- **`pyq` — native Rust CLI crate** — `crates/pyrs-yaml-cli` (workspace
  member, clap-based) puts `pyrs-yaml-core` behind a jq/yq-style command
  line with no Python at runtime: `fmt` (comment-preserving round-trip),
  `get <path>` (JSONPath-lite with `--json`/`--raw`), `set <path> <value>`
  and `delete <path>` (yq-style edits with `--create-missing` and
  `-i/--inplace`, round-trip-preserving output), `to-json` (order
  preserved), `to-toml`, `from-json` / `from-toml` / `from-ini`; input
  format by extension with `--input` override, stdin via `-`, non-zero
  exit with the core's stable error text.
- **TOML and INI exchange formats** — hub-and-spoke multi-format support
  with YAML as the single editable representation: `from_toml`/`to_toml`
  convert TOML text ⇄ YAML text (Rust `toml_edit`), `load_toml` reads TOML
  directly into Python values (datetimes via the built-in `!timestamp`
  plugin; TOML strings never re-resolve), and `load_ini` reads INI via
  stdlib configparser (strict, read-only). TOML output rejects
  inexpressible shapes with stable errors; round-trip editing stays
  YAML-only by design.

#### Changed

- **granit-parser 1.1 → 1.3** — bumped the YAML event parser from 1.1.0 to
  1.3.0. The upgrade is a semver-compatible minor bump inside the 1.x line:
  1.2.0 added optional `Options` fields for exotic-document limits, 1.2.1
  tightened a few parse results to match the YAML spec, and 1.3.0 added two
  defaulted `Input` methods (`fetch_block_scalar_line` and
  `take_quoted_scalar_ascii_chunk`) that let the scanner step over block and
  quoted scalar bytes faster. The project consumes the parser through
  `Parser::new_from_str` and implements only `EventReceiver` /
  `SpannedEventReceiver`, never `Input`, so no source changes were required —
  the new trait methods resolve to their default implementations. Full suite
  green: `cargo nextest run --all` (359), `pytest` (1436 + 43 numpy), pure-Rust
  `--no-default-features` build, and the YAML test-suite compliance gates
  unchanged.
- **Native JSON & TOML cores** — the `serde_json` and `toml_edit`
  dependencies are gone. `pyrs-yaml-core` ships an RFC 8259 JSON engine
  (byte-level scanner, verbatim number spelling so `from_json → to_json` is
  byte-stable and precision-preserving, typed line/column errors, strict
  rejections for trailing commas, leading zeros, lone surrogates, unescaped
  control characters, and multi-root documents) and a TOML 1.0 engine
  covering the full grammar — bare/quoted/dotted keys, basic/literal/
  multiline strings, decimal / hexadecimal / octal / binary integers with
  underscore separators, floats including `inf`/`nan`/exponent forms, and
  offset/local date, time, and date-time — every rejection surfacing as
  `ParseError::Syntax` with 0-indexed `line`/`col` in the granit-parser
  house style. Public surface unchanged; round-trip tests and
  `tests/test_toml.py` green with the native path.
- **Internal duplication cleanup** — benchmark fixtures composed from shared
  blocks, PyO3 path-edit methods delegate to the existing
  `apply_metadata_edit` helper, and repeated file-read/error-mapping and
  line-offset boilerplate collapsed into shared functions. No public behavior
  change; the duplicate-code rate measured by jscpd drops from 5.25% to 3.45%.
- **`YamlDocument.validate()` caches compiled validators** — the first
  successful validation against a schema (JSON text *or* dict) caches a
  compiled `jsonschema` validator; later calls skip schema parsing, meta-schema
  checking, and validator construction. Dict schemas are keyed by object
  identity with a deep-copy snapshot guard: in-place mutation is detected by
  `==` and transparently recompiles. The cached path raises
  `exceptions.best_match(validator.iter_errors(instance))`, identical to
  `jsonschema.validate()` semantics. WSL wall-time: `document_validate` −98%.
- **Structural dedupe of parser/serializer kernels** — mapping and sequence
  rendering share one `write_container_node` skeleton (output byte-identical,
  `serialize_*` medians −5~11%); single- and multi-document parse entry points
  share one `load_ast` error contract; the schema resolution chains share
  `bool_word`/`numeric_tail` and YAML 1.1 no longer re-checks the core's
  null/bool words per scalar; anchor registration (`register_anchor`) and the
  standalone-vs-inline comment taxonomy (`is_standalone_placement`) are
  single-sourced across the AST and stream receivers. Repo duplicate rate
  3.38% → 2.60%.

#### Fixed

- **`\u` / `\x` escapes followed by a multibyte char panicked the parser** —
  fixed-width escape readers sliced `&self.text[pos..pos+width]` by byte
  offset; a multibyte char after a JSON `\u` or TOML `\xHH`/`\uXXXX`/`\UXXXX`
  landed the slice mid-character and aborted (the #153 slice-crash sibling).
  They now byte-slice + UTF-8-validate, rejecting cleanly. Found by the
  dialect fuzz; pinned by deterministic Rust regression tests.
- **A literal `<<` key with a non-merge value was silently dropped** —
  `load(safe_dump({"<<": None}))` returned `{}`, losing the key. The merge
  resolver consumed any `<<` as a merge even for a Null/Scalar source. A `<<`
  is only a merge when its value is an alias-to-mapping, inline mapping, or
  sequence of those; Null/plain-scalar `<<`, and an alias-free `<<` that yields
  nothing (`<<: []`, `<<: [1, 2]`), now stays an ordinary key and round-trips.
  Alias / alias-bearing paths are always consumed (#166 guard holds; a
  `<<: [*a, *a]` cannot re-expand); yaml-test-suite holds 405/406. Surfaced by
  the round-trip property fuzz, pinned by deterministic Rust regression tests
  for null, scalar, and sequence forms.
- **TOML deep nesting overflowed the native stack and aborted the process**
  — the TOML parser had no nesting budget (JSON has `DEFAULT_MAX_DEPTH`,
  YAML has `parse` `max_depth`), so `parse_value` → `parse_array` /
  `parse_inline_table` recursed without limit. A deeply nested array/inline
  table crashed the interpreter outright (verified: exit `0xC00000FD`
  STACK_OVERFLOW) — the TOML analogue of the #166 YAML merge overflow. The
  parser now tracks `depth` and returns `ParseError::MaxDepthExceeded` past
  1000, mirroring JSON. Covered by an in-process Python boundary test, a
  subprocess crash canary, and a big-stack Rust unit test.
- **Dialect writers/parsers lost or misplaced document-level comments** — a
  file-leading `// note` was misclassified as inline and stolen by the first
  JSONC/JSON5 object member (vanishing on empty containers) instead of landing
  on the root the writer annotates; the JSON-family **and TOML** writers now
  trim comment bodies at emit to match the parser's trimmed storage (an
  untrimmed comment used to oscillate whitespace across passes); and a
  comment-only TOML document keeps its leftover note on the empty root instead
  of serializing to `""` (a hand-built non-empty root carrying both a root and
  a first-element note is outside TOML's representable domain and filtered from
  the property). Leading comments across all five formats now reach a
  byte-stable fixed point (three pinned Rust tests).
- **Nested block scalar bodies kept their parent's indentation** — a literal or
  folded scalar under a nested key emitted its body lines one fixed indent step
  from column zero instead of one step below the `b: |` header line, so every
  nested block-scalar shape re-parsed as an error or a wrong value. The writer
  now threads a `block_base` (the parent line's column) through every emission
  site; round-trip is text-exact for all seven nesting shapes. Found by the
  TOML hot-spot bench.
- **The serializer emits only re-parseable YAML** — five spelling defects caught
  by the new text-level gate: double anchor/tag headers when a mapping value
  moved to its own line pre-emitted a header the child also wrote; block
  scalars inside flow collections (or key position) now demote to
  double-quoted; flow containers starting their own line and the complex-key
  value marker `:` lost their parent indentation; complex keys with a
  standalone comment/tag serialized ambiguous text (the note now moves above
  `?` and the body gets its own deeper lines); and plain scalars with edge
  whitespace or embedded flow indicators (`,[]`) inside flow collections are
  now quoted instead of corrupting the token. Tagged empty containers fold
  their header onto the `{}`/`[]` line; compact dash items refuse to inline
  commented values. Pinned by nine Rust tests and Python regressions.
- **TOML rejected the legal minimum i64 integer** — `from_toml`/`load_toml` failed
  on `-9223372036854775808` (`i64::MIN`): the signed path parsed the unsigned
  magnitude first and overflowed before negation ran. The sign now parses with
  the digits (`i64::from_str` accumulates negatively), signed floats keep their
  exponent spelling, and the old negate pass is gone. Found via the new
  Python-side Hypothesis dialect fuzz (`tests/test_property_dialects.py`, with
  stdlib `json`/`tomllib`/`pyjson5` oracles and type-strict equality; it also
  pins the two known AST-ambiguous spellings — bare JSON5 `Infinity`/`NaN`
  literals and >i64 digit strings). Rust regression:
  `toml::parser::tests::i64_lower_bound_negative_integer_is_accepted`.
- **Wrongly-indented flow sequence continuation is rejected again** — upgrading
  the YAML parser to granit-parser 1.3 (see *Changed*) silently began *accepting*
  a multi-line flow collection whose continuation line is indented no further
  than its enclosing block key (yaml-test-suite `9C9N`: `flow: [a,` then `b,` at
  column 0), regressing strictness `405/406 → 404/406` — invisible to the suite's
  ≥95% threshold gate, so it passed green CI. An in-tree post-parse guard in the
  AST receiver now tracks the enclosing block indentation and rejects an
  under-indented flow continuation, restoring `405/406`. The guard uses only
  spans the parser already computes, so correctly indented multi-line flows are
  untouched. `9C9N` is now pinned as a per-case hard gate (literal input, no
  `skipif`) in `tests/test_yaml_suite.py`, plus a Rust unit test
  (`parser::tests::flow_continuation_under_indented_is_rejected`).
- **Self-referential merge keys no longer overflow the native stack** — a `<<`
  whose expansion points back at its own anchor (`a: &a` containing
  `b: {<<: *a}`) expanded forever inside `resolve_merge_keys`, exhausting the
  native stack and killing the whole interpreter process (Windows exit
  `0xC00000FD`, a segmentation fault). The cycle guard was applied only while
  *collecting* merged pairs, never while *walking* the expansion, so recursive
  re-entry was never caught. The anchor guard is now path-scoped: an anchor's
  name stays on the recursion path while its expansion is walked, and a merge
  that resolves back to an ancestor already on that path terminates as an empty
  expansion instead of recursing. An acyclic AST cannot carry PyYAML's cyclic
  dict, so a self-merge now bottoms out at `{}` rather than crashing. Four
  related merge-semantics defects were fixed in the same pass: a null / scalar /
  sequence merge source no longer survives as a literal `<<` key, an inline
  mapping used directly as a merge value (`<<: {x: 1}`) now merges, and a
  non-alias element in a merge sequence (`<<: [*a, {y: 2}]`) keeps its inline
  map. Covered by six Rust and nine Python regression tests (`merge::tests`,
  `tests/test_gaps.py::TestSelfReferentialMerge166`).
  Reported by [@bourumir-wyngs](https://github.com/bourumir-wyngs) in #166.
- **NumPy serialization no longer reads Python memory without the GIL** — the
  ndarray writer borrowed the array's data buffer via `unsafe { as_slice() }`
  and then iterated that borrowed slice *inside* `py.detach`, i.e. after
  releasing the GIL. `&[T]` is `Send` no matter its provenance, so the borrow
  checker could not catch it, but the memory is owned by Python and another
  thread could resize or write the array concurrently — an unsound data race /
  UB that only surfaces under concurrency. The buffer is now snapshotted into
  Rust-owned memory while the GIL is still held (`slice.to_vec()`), and only
  the scalar→node conversion runs off-thread. This was the *only* `unsafe`
  buffer borrow in the binding layer; every other `py.detach` site was audited
  and touches Rust-owned state only (AST, source text, `BufWriter<File>`).
  Covered by `tests/test_numpy.py::TestNumpyConcurrency`.
  Reported by [@bourumir-wyngs](https://github.com/bourumir-wyngs) in #165.
- **Repeated alias references no longer resolve to `None`** — `to_dict()`
  expanded aliases behind a *global* visited-anchor set that was never
  cleared, so only the first reference to any anchor produced a value and
  every later one silently degraded to `None`:

    ```yaml
    a: &x 1
    b: *x      # 1
    c: *x      # was None, now 1
    ```

    The blast radius was wider than "the second reference": two sibling
    references inside one container poisoned each other too (`{a: &x {p: 1},
    b: {q: *x}, c: {q: *x}}` gave `b` a value and `c` a `None`). The guard is
    now scoped to the current recursion path — pushed for the duration of one
    expansion, popped afterwards — so repeated and sibling references each get
    their own fully built value while genuine cycles still terminate. `<<`
    merge resolution and the AST itself were audited and are unaffected. Six
    new PyYAML-parity cases in `tests/test_direct_load.py` pin the agreement,
    and two tests that had pinned the buggy output as expected behaviour were
    rewritten. Reported by [@bourumir-wyngs](https://github.com/bourumir-wyngs)
    in #163.
- **Document header comments no longer vanish on nested first values** —
  the parser kept one shared comment slot for all in-progress containers,
  so a nested container start clobbered a standalone header note before
  it reached the AST (dropped at parse time; invisible to `to_dict`,
  fatal for `dump`). Slot handling is now a per-container stack.
- **Splice edits no longer duplicate leading comments** — when a
  regenerated region text carried the pair/item's own standalone note,
  the replaced range skipped the old note line and both survived; the
  plan now widens the range over note lines (`pyq set`/`delete` and the
  bindings' splice path share the fix).
- **`!timestamp` accepts a trailing `Z` on every supported Python** —
  `datetime.fromisoformat` only recognizes the UTC-`Z` suffix from 3.11;
  the plugin now normalizes `...Z` to `+00:00` first, fixing
  `Invalid isoformat string` on 3.8–3.10 for YAML `!timestamp` scalars
  and TOML datetimes via `load_toml` / `from_toml`.

#### Performance

- **Direct event→Python load materialization** — `safe_load`, `safe_loads`
  and `YAML().safe_load*` build Python objects in one pass over the granit
  event stream instead of constructing the full AST and walking it again in
  `convert.rs`; schema resolution, raw-text mapping keys and duplicate-key
  errors stay identical. Anchored/tagged/merge/multi-document shapes fall
  back via zero-cost pre-vetoes. WSL wall-time: scalar-heavy `safe_load`
  −21~25%, family overall −13~18%, fallbacks unchanged.
- **Anchor extraction byte gate** — `extract_anchors` returns empty after a
  single `&` byte-containment check; anchor-free documents skip the
  per-character quote state machine. Rust-side `parse_*` benches improve
  11–18% at the median; the scan itself drops 1.5µs → 38ns.
- **Interned stream-event dict keys** — fixed per-event keys from
  `parse_stream` / `load_stream` reuse interned strings (`pyo3::intern!`).
  WSL wall-time: `parse_stream` −34%, `parse_stream_multidoc` −39%,
  `load_stream` −22%.
- **Decomposition micro-benchmarks** — new `granit_events_*` benches isolate
  the pure granit event-pipeline cost from AST construction (bench-only).
- **Multi-document parse without per-document clones** — `on_document_end`
  moves the completed document into the collection instead of deep-cloning it.
  WSL wall-time: `parse_all_docs` −9.7%, `safe_loads` (multi-doc) −9.5%,
  `YAML().safe_loads` −6.7%.
- **Streaming write reuses one buffer across documents** — new
  `direct_dump_into` writes each document into a reused `String`, and
  `dump_iterable` skips `normalize_doc`'s re-copy in the normal case.
  WSL wall-time: `dump_stream_multi_doc` −27.2%, `dump_stream` −4.4%.
- **Scalar fast paths in the AST builder** — `unescape_double_quoted` returns
  early without backslashes; `detect_chomping` pulls lines lazily instead of
  collecting the whole document per block scalar. WSL wall-time: `to_dict`
  family −4~9%, scalar-type loads −3~4%, no regressions.

#### Docs

- **Corrected the numpy guide's 0-D scalar section (all locales)** — the
  snippet claimed 0-D arrays "reshape to a single-element list"
  (`assert data == [42]`); shipped behavior (pinned by `tests/test_numpy.py`)
  serializes bare scalars (`assert data == 42`). The en page gained a warning
  admonition for the 0-D `bool` → `1.0` rust-numpy quirk.

### [v0.15.0] — 2026-08-19

#### Added

- **Node metadata setters/getters** — `Node.comment` / `Node.anchor` / `Node.tag` read properties and `set_comment` / `set_anchor` / `set_tag` (plus `remove_*` variants), backed by new path-based edit operations. Editing an alias or a missing path raises; standalone comments on inline scalar values and sequence items are now serialized on their own indented lines (fixes pre-existing broken round-trip for `child:\n  # c\n  val` and `- a\n# c\n- b`).
- **Verbatim tags** — `set_tag("!<tag:yaml.org,2002:str>")` now produces a verbatim tag (empty handle), and verbatim tags parsed from source survive round-trip: `Tag`'s `Display` emits `!<...>` wrapping for empty-handle tags, `parse_tag` recognizes the `!<...>` form, and stream events serialize tags through `Display`.
- **Schema file IO and listing** — `load_schema(name, path)` reads a schema definition from a file and registers it; `list_schemas()` returns all registered schema names (built-in `failsafe`/`json`/`core`/`yaml1.1` plus custom).
- **Node style/format setters/getters** — `Node.scalar_style` / `Node.flow_style` / `Node.chomping` read properties and `set_scalar_style` / `set_flow_style` / `set_chomping` methods. ScalarStyle/Chomping now derive `Copy`. Non-scalar nodes return `None` / are no-op; aliases and missing paths raise.
- **Schema structural validation** — a `validate` section in a schema definition adds structural checks (path-qualified scalar types, `sequence_of`/`mapping_of` containers, `required`); `validate_against_schema(data, schema_yaml)` raises `YamlValidateError` listing every failure.
- **`Node.copy()`** — deep-copies a subtree as a standalone Python value (dict/list/scalar), detached from the document, for pasting via `set_value()`.
- **Deep editing API** — `doc.set_many({path: value})` sets multiple paths (with wildcard `[*]` and deep-scan `..` support) in a single splice burst; `doc.sort_keys()` orders mapping keys in place; `Node.move(new_path)` relocates a subtree; `Node.path` / `Node.find_first()` / `Node.value_eq()` add path access, first-wildcard lookup, and value comparison.
- **Property-based testing for 0.14+ features** — Rust proptests for `validate_node`, schema parsing, style-settings round-trip; Python hypothesis tests for `set_many` wildcards, metadata-edit preservation, `sort_keys` idempotency. `hypothesis` moved to the `test` group so CI runs property tests.
- **Serializer fix** — standalone comments on empty flow containers (`key: {}` / `key: []`) no longer produce invalid YAML; demoted to inline.

#### Changed

- **NumPy re-enabled on free-threaded (cp314t) wheels** — the `--no-default-features` flag is removed from the cp314t build lines; rust-numpy 0.29 supports free-threaded Python, so `numpy.ndarray` serialization is now available on free-threaded wheels when NumPy is installed (auto-detected at runtime).

#### Docs

- **Corrected stale references across all locale docs (en/zh/ja/ko)** — `saphyr-parser` → `granit-parser`, YAML compliance 98.1% → 99.75% (405/406 suite cases), ABI3 support 3.9–3.13 → 3.8–3.15 (py3.9+ → py3.8+), and benchmark tables updated to current CodSpeed CI numbers (parse 21–43×, serialize 55–177× faster than PyYAML). Rust-side benchmark sections migrated from Criterion to divan (`benches/yaml_bench.rs` → `crates/pyrs-yaml/benches/yaml_bench.rs`).

### [v0.14.1] — 2026-08-15

#### Fixed

- **Single-quoted scalars with backslash + control/noncharacter** — such values now use double-quoting; single quotes cannot escape control chars/noncharacters.
- **Noncharacters and BOM quoted** — `needs_quotes` / `needs_double_quoted` now require quoting for U+FFFE/U+FFFF/plane-end noncharacters and U+FEFF (BOM).
- **Double-quoted escape width** — code points above U+FFFF now emit the 8-digit `\Uxxxxxxxx` form (the 4-digit `\u` form is BMP-only).
- **Folded plain-scalar continuation indent** — continuation indent is derived from the value's start column so nested sequence/mapping items stay indented past the parent block indent.
- **Multi-byte wrap boundary** — `wrap_plain_scalar` floors the wrap slice to a char boundary instead of panicking on 4-byte UTF-8 straddling.
- **`hypothesis` in publish test requirements** — `.ci/requirements-test.txt` pins `hypothesis>=6.113.0` so the publish workflow can run the property suite.

#### Added

- **`scripts/fuzz_panics.py`** — high-volume local Hypothesis fuzz harness with a hostile strategy across dump/parse/edit/idempotency.

### [v0.14.0] — 2026-08-14

#### Added

- **YAML Schema Language** — define custom schemas with `rules` mapping
  regex patterns to YAML types; register via `register_schema()`.
- **Inline dict schema** — `schema` parameter accepts `dict` in `YAML()`,
  `parse()`, `safe_load()`, etc.
- **Community Plugins** — `CustomType` base class with
  `from_yaml`/`to_yaml`/`can_parse`/`validate`; register via `register_type()`.
- **Built-in plugins** — `!timestamp` (datetime) and `!set` registered by default.

#### Changed

- **Schema resolution is pluggable** — `SchemaResolver` trait + `Schema`
  enum with global `SchemaRegistry`. Built-in schemas retain zero-cost dispatch.
- **`node_to_pyobject` and `direct_dump` check registered `CustomType`s** —
  tagged scalars convert via `from_yaml()`; objects serialize via `to_yaml()`.
- **`get()` is literal-key only** — `YamlDocument.get()` no longer guesses
  JSONPath for keys containing `.` or `[`; every key is treated as a
  top-level mapping key, consistent with `__getitem__`/`__setitem__`.
  Path access stays available via `find()`/`node()`.

#### Fixed

- **Quoted scalars always load as strings** — implicit type resolution now
  applies only to plain scalars (YAML 1.2): `safe_load('"true"')` returns the
  string `"true"`, not `True`. The serializer keeps negative numbers
  round-tripping through the document (`to_yaml`) path.
- **Lone-quote keys round-trip** — mapping keys that are a single `'` or `"`
  are emitted as quoted scalars instead of unparseable YAML.
- **Empty collections emit `{}`/`[]`** — dumping empty mappings/sequences no
  longer yields an empty document that re-parses as `None`.

### [v0.13.0] — 2026-08-10

#### Changed

- **Rust MSRV raised to 1.96 and edition bumped to 2024** - both crates now
  declare `rust-version = "1.96"` and `edition = "2024"`; CI pins the
  `build`/`test-freethreaded` jobs to Rust 1.96 for deterministic wheel builds
  and adds an `msrv-check` job running `cargo check`/`cargo test` at the MSRV
  to prevent silent MSRV drift (the `rust-lint` job stays on `stable`).
  The floor is set above PyO3 0.29's own baseline (rustc 1.83) for std API
  headroom (e.g. `assert_matches!`, stabilized 1.96) with no code migration
  needed. `TAG_REGISTRY` (tag handler storage) refactored to
  `std::sync::LazyLock`, dropping the `Mutex<Option<...>>` indirection.

#### Performance

- **`safe_dump` / `from_dict` / `dump_file` / `dump_iterable`: direct writer**
  — Python→YAML serialization without intermediate `CustomNode` AST.
  Single-pass `direct_dump` replaces the old two-pass `pyobject_to_node` +
  `to_yaml`. 7x faster on `safe_dump` (28ns→4ns), 6x faster on `from_dict`
  (35ns→6ns). (#60)
- **`safe_load` / `safe_loads` / `to_dict`: fast-path skip anchor tracking**
  — when input has no `&` characters, skip `collect_anchors` + anchor
  resolution and use the simpler `node_to_pyobject_simple` path. (#59)
- **`resolve_core_type`: first-byte dispatch whitelist** — non-numeric/
  non-boolean first bytes return `Str` immediately, avoiding schema
  resolution overhead for the common case. (#59)
- **granit-parser migration** — saphyr-parser replaced with granit-parser
  1.0.1 for native `Event::Comment` emission, eliminating the full-text
  `scan_yaml()` pre-scan. parse_small -18%, parse_large -21%,
  roundtrip_large -18%.

#### Fixed

- **`float_to_yaml_string` round-trip fix** — appends `.0` when Rust
  Display drops the decimal (`42` → `42.0`) so floats round-trip as
  floats instead of becoming ints.
- **Reverted `count_nodes` pre-allocation** — the full AST traversal cost
  more than the reallocations it avoided (serialize_10mb was ~14% slower);
  buffer growth is left to the Vec.

#### Added

- **`max_depth` on stream & frontmatter APIs** — `parse_stream(yaml, on_event, max_depth)`,
  `read_markdown(path, schema, max_depth)`, `read_markdown_str(content, schema, max_depth)`
  accept `max_depth` (default 1000). Stream parsing now enforces the nesting-depth limit
  via core `parse_stream_with_options` (previously stream events had no depth limit).
- **Pydantic integration** — `dump_pydantic()` serializes a Pydantic model
  to YAML string via `model_dump(mode='json')` + `safe_dump`; `parse_as()`
  parses YAML string into a Pydantic model instance. Both use lazy imports,
  no hard dependency on pydantic. (#61)

#### Internal

- **Split `py/mod.rs`** — monolithic 1786-line module broken into
  `document.rs` (YamlDocument), `yaml_instance.rs` (YAML class),
  `functions.rs` (module-level functions), `stream_iterator.rs`,
  `walk_helpers.rs`. `mod.rs` reduced to 128 lines. (#61)
- **`needs_quotes()` guard + `double_quoted_scalar()` constructor** —
  strings like `'true'` / `'42'` / `'null'` now emit as double-quoted
  scalars under the core schema instead of being misread on re-parse
  (`pyobject_to_node` + `json_value_to_node`).
- **CodSpeed benchmarks unified on `codspeed-divan-compat`** —
  `exclude-allocations` removes allocator noise; cross-library benchmarks
  consolidated into `tests/test_benchmark_crosslib.py` with shared
  `tests/data/yaml_samples.py` fixtures and streaming coverage.

### [v0.12.1] — 2026-08-06

#### Added

- **`set(create_missing=True)`** - missing intermediate mapping keys along
  the edit path are created as nested mappings (e.g. setting `a.b.c` on
  `a: 1` creates `b` and `c`); index segments that miss are still an error,
  and a scalar intermediate along the path still raises.
- **`doc.walk()` / `doc.scalars()`** - Rust-backed depth-first AST traversal
  yielding `Node` objects, avoiding per-node `to_dict()` resolution.
  `walk()` returns all nodes; `scalars()` returns only scalar/null nodes.
- **Rust core module tests** - 39 new tests covering `editing::navigate`
  (key_eq, navigate, navigate_mut, normalize_index, mapping_key_index),
  `editing::region` (line helpers, node_is_flow, extend_delete_over_comments,
  nav_err), `editing::dirty` (DirtyKind/DirtyUnit constructors), and
  `editing::metadata` (with_metadata_from, needs_quoting).
- **Python doc.walk() edge case tests** - 9 new tests for empty doc, null
  values, deeply nested, flow collections, mixed types.

#### Changed

- **Monorepo workspace** - source code split into `crates/pyrs-yaml-core/`
  (pure Rust, no PyO3) and `crates/pyrs-yaml/` (PyO3 bindings). Root
  `Cargo.toml` is now a workspace. Old `src/` directory and `build.rs`
  removed.
- **pyproject.toml** - added `tool.maturin.manifest-path` pointing to
  `crates/pyrs-yaml/Cargo.toml`.
- **Parse hot paths** - single-pass comment/anchor extraction, lazy
  duplicate-key detection, `shift_insert` merge prepending, and skipped
  `DocumentEnd` deep-clone for single-document parses cut large-document
  parse cost ~19% (CodSpeed: parse[large] +13.9%, parse[medium] +16.6%,
  roundtrip[large] +12.2%).
- **`Arc<str>` scalar storage** - `CustomNode::Scalar` and comment/event
  text share allocations via `Arc<str>`; AST nodes shrink 8 bytes and
  clones become refcount bumps instead of deep copies.

#### Fixed

- **`set(create_missing=True)` nested chain build** - the created mapping
  chain no longer duplicates the first segment as a nested key level.
- **`set(create_missing=True)` eligibility** - freshly created keys are now
  eligible for the value write (the eligibility check no longer runs after
  the synthetic pair is inserted).
- **Standalone comments before simple mapping keys** - round-trip
  previously dropped standalone comments attached to simple-key nodes;
  now preserved (two regression tests).

### [0.11.7] - 2026-08-04

#### Changed

- **stub-build-check replaced with release-guard** - the always-red container
  build (`validate.yml`) that deliberately failed to reproduce the v0.10.0
  `--generate-stubs` failure mode is replaced with three static assertions
  that **pass** when the repo is correct: `grep` guards `publish.yml` against
  `--generate-stubs`, `git ls-files` asserts the committed `.pyi` is tracked,
  and `test -f` checks `py.typed` exists. The job now gives green CI on
  correct state, red only on regression.

#### Added

- **Numpy free-threaded tracking** - ROADMAP.md now tracks `rust-numpy` free-
  threaded support status (PyO3/rust-numpy#476) as a dependency for re-enabling
  ndarray serialization on cp314t wheels when the Rust binding matures.

### [0.11.6] - 2026-08-04

#### Changed

- **Free-threaded (cp314t) wheels are now numpy-free** - built with
  `--no-default-features`, so rust-numpy is excluded entirely (smaller
  binary, no runtime probe). `safe_dump` on a `numpy.ndarray` raises
  `YamlTypeError` on free-threaded builds; GIL builds (Python 3.8-3.15)
  keep full ndarray serialization.

#### Added

- **Free-threaded CI validation** - `test-freethreaded` job now builds
  and tests with `--no-default-features`, matching the shipped
  free-threaded wheel configuration.
- **Install docs** - `docs/{en,zh,ja,ko}` note that free-threaded
  wheels are numpy-free (ndarray serialization unavailable on cp314t).

### [0.11.5] - 2026-08-04

#### Changed

- **Parser robustness items 3/4/5 closed via Phase 0 strictness audit** — the 70-probe corpus (indentation, block-mapping keys, flow context) compared against a PyYAML oracle showed **no fixable accepted-but-invalid case** (64/70 match; the 6 divergences are deliberate YAML 1.2 / yaml-test-suite requirements where PyYAML is the outlier, and one deliberate duplicate-key strictness). Compliance stays at **99.75% (405/406)**. Full write-up in `ROADMAP.md` §v0.11.5 and `tests/test_strictness_audit.py`.

#### Added

- `tests/test_strictness_audit.py` — 70-probe strictness regression corpus pinning current rejection/acceptance behavior (both directions), so future parser changes cannot silently regress strictness or over-reject.

### [0.11.4] - 2026-08-04

#### Fixed

- Duplicate null/empty mapping keys no longer error (`: a\n: b`, `~: a\n~: b`) — matches yaml-test-suite 2JQS; real duplicate keys still raise `YamlDuplicateKeyError`
- Compliance harness: correctly-rejected invalid YAML now counts as pass (was lowering the rate despite compliant behavior)
- Compliance harness: `convert_special_chars` tab decoding via regex — any run of `—`/`‖` + `»` is one tab, fixing tab-encoded suite cases

#### Changed

- YAML Test Suite pass rate gate raised from >75% to **≥95%**; current rate **99.75%** (405/406)
- Known deviation documented: `ZYU8` (`%YAML 1.1 1.2`) is rejected by design (invalid per YAML 1.2 grammar, matches PyYAML/libyaml)

### [0.11.3] - 2026-08-03

#### Added

- Streaming write: `YAML.dump_stream(file_obj, iterable)` / `YAML.dump_file(path, iterable)` with document-level constant memory, auto `---` separators, and `explicit_start`/`explicit_end` flags
- `YamlDocument` `with` context manager: snapshot/rollback transaction scoping
- `compliance_report()`: public YAML Test Suite pass-rate reporting (version-consistent)

#### Changed

- Edit-burst line-offset cache: internal O(N+edit) carry-through in the splice layer (public API unchanged)
- `compute_compliance` moved from tests to `pyrs_yaml.compliance`; version no longer hardcoded

#### Fixed

- Changelog mirror drift guard: prek hook + CI job assert root/mirror `[Unreleased]` sync
- Publish stub pre-validation: CI reproduces v0.10.0-class `--generate-stubs` container failures before Release

### [0.11.2] - 2026-08-03

#### Added

- `YAML.load_stream(file_obj)` / `YAML.load_stream_file(path)`: lazy event iterators with O(anchors + chunk) memory

#### Performance

- **Parse no longer computes splice eligibility** — the O(document) layout check now runs lazily on the first edit via `YamlDocument.splice_checked`, restoring the v0.11.0 regression: parse_comments -59%, parse_anchors -42%, parse/roundtrip/edit -10~35% all back to v0.10.0 levels
- **Linear-cursor layout check** — replaces per-node binary search over precomputed line offsets (monotonic source-order traversal)

#### Changed

- `parse_with_options` returns `CustomNode` (was `(CustomNode, bool)`); splice eligibility is now internal to `YamlDocument` and computed on demand

### [0.11.0] - 2026-08-02

#### Added

- **Surgical Serialization** — byte-level source span tracking on every AST node; segment-based splice — edits regenerate only the touched region, untouched text is byte-copied
- proptest fidelity property tests (new dev-dependency)
- 10MB edit-flush benchmarks (divan)

#### Changed

- `flush_source` now splices segments; falls back to full serialization for flow-style regions, non-default layout documents, merged keys, CRLF/BOM documents, and after materialization (single-burst model)
- Splice edits preserve `---`/`...`/directive marker lines as untouched bytes (full serialization previously dropped them — deliberate behavior difference)

### [0.10.0] - 2026-08-01

#### Added

- **In-place editing** — edit parsed documents without losing formatting metadata:
    - Path API: `doc.set(path, value)`, `doc.insert(path, index, value)`, `doc.append(path, value)`, `doc.delete(path)`, `doc.rename(path, new_key)` with JSONPath-style paths (`$.a.b[0]`); root sugar via `doc["key"] = value` and `del doc["key"]`
    - Node API: `doc.node()` / `doc.find(path)` return `Node` objects with `set_value` / `append` / `insert` / `delete` / `rename`, plus tree traversal (`parent`, `children`, `walk`, `filter`)
    - Full metadata preservation — replaced scalars keep comment/anchor/tag/quoting; renamed keys keep position and comments; mapping order preserved on delete
    - Atomic edits — failed operations leave the document (and its revision) untouched
    - Lazy source re-sync — `source()` / `to_yaml()` / `reparse()` re-serialize only after a successful edit
    - Stale-node detection — `Node` access after a document edit raises `YamlDocumentError` (with `RuntimeWarning`)
    - New exceptions: `YamlEditError`, `YamlPathError` (i18n across en/zh-CN/ja-JP/ko-KR)
    - Alias-aware editing — setting an alias's own path replaces it in place; editing through an alias raises `YamlEditError`
- **Edit benchmarks** — 6 new divan benchmarks in `benches/yaml_bench.rs` (set/insert/delete on small–large documents)

#### Changed

- `YamlDocument.source()` now returns `str` and lazily re-serializes after in-place edits

### [0.9.0] - 2026-08-01

#### Added

- **Python 3.13, 3.14 and 3.15 support** — PyO3 `abi3-py38` wheel covers Python 3.8-3.15 (GIL build); `abi3t` + `abi3t-py315` provide free-threaded stable ABI
- **Free-threaded CPython (no-GIL) support** — `#[pymodule(gil_used = false)]` declares module as thread-safe for free-threaded Python; `Py_GIL_DISABLED` cfg flag gates numpy (rust-numpy has no free-threaded support yet — numpy feature must be disabled for free-threaded builds via `--no-default-features`)
- **CI free-threaded job** — new `test-freethreaded` workflow job validates compilation and tests against Python 3.14t
- **`pyo3-build-config` build dependency** — enables `#[cfg(Py_GIL_DISABLED)]`, `#[cfg(Py_3_15)]` etc. compiler flags via `build.rs`
- **`numpy` made optional** — feature-gated behind `numpy` feature (default enabled); excluded automatically under `Py_GIL_DISABLED`
- **`allow_duplicate_keys`** — `YAML(allow_duplicate_keys=True)`, `parse(..., allow_duplicate_keys=True)`, `parse_file`, `safe_load`, `safe_loads`, `parse_all_docs` all accept the flag; duplicate mapping keys raise `YamlDuplicateKeyError` by default, `last value wins` when allowed
- **`SerializeOptions` expansion** — `doc.to_yaml_with_options()` gains `width` (line wrapping, 0 = off), `indent_mapping`, `indent_sequence`, `indent_offset` alongside existing `indent_size`/`explicit_start`/`explicit_end`/`sort_keys`/`max_depth` (`src/py/mod.rs:432`)
- **Tag handler registry** — `register_tag("!custom")` decorator and imperative forms + `clear_tag_handlers()`; scalar nodes carrying a registered tag are transformed through the handler (`src/py/tag_registry.rs`)
- **Tag handler chaining with priority** — multiple handlers per tag run in ascending `priority` order; `YamlTagSkip` lets a handler pass through to the next, fallback keeps the original value
- **Pydantic integration** — `parse_as(Model, yaml, **yaml_kwargs)` parses YAML and validates against a Pydantic v2 model; raises `ImportError` with guidance when pydantic is absent (`python/pyrs_yaml/pydantic.py`)
- **`.pyi` type stubs** — auto-generated by maturin and committed so `register_tag`, `parse_as`, `to_yaml_with_options` and the new exceptions are visible to type checkers

#### Changed

- CI Python matrix expanded: 3.8-3.14 across ubuntu, windows, macos
- Stable ABI: `abi3-py39` → `abi3-py38` (wider Python 3.8+ support), added `abi3t` + `abi3t-py315` (free-threaded stable ABI)
- `pyproject.toml` classifiers updated with 3.13, 3.14, 3.15 entries
- **CI optimization: redundant Rust compilation eliminated** — a single `rust-lint` job runs `cargo clippy` + `cargo test` once; the build job produces one abi3 wheel per OS which test jobs install instead of running `maturin develop`, removing Rust compilation from 21 matrix jobs (~86% fewer compiles); `Swatinem/rust-cache` added to all jobs
- **pydantic test dependency** — `pydantic>=2.10.6` added to `[dependency-groups] test` and `.ci/requirements-test.txt` (SSOT via `uv sync` in ci.yml)

#### Fixed

- **Windows DLL loading** — removed `#[cfg(test)]` block from `src/py/tag_registry.rs` which broke `import pyrs_yaml` on Windows (`250b8d0`)
- **Python 3.8 compatibility** — `from __future__ import annotations` in `pydantic.py` (`63d2495`)
- **CI pydantic skip** — `pytest.importorskip("pydantic")` so tests pass when pydantic is not installed (`7be011d`)
- **CI glob expansion on Windows** — `shell: bash` for `pip install dist/*.whl` (PowerShell does not expand `*`) (`2f7778d`)
- **Non-string tag handler returns now raise `YamlTagError`** — a handler returning a non-`str` value (previously silently ignored, keeping the original scalar) now errors with `Tag handler '!x' must return a string` (`src/py/mod.rs:resolve_tags`)
- **`to_yaml_with_options` indent wiring** — `indent_mapping`/`indent_sequence`/`indent_offset` are now honored by the serializer (previously dead fields); each defaults to `indent_size`/0 when omitted (`src/serializer.rs`)
- **`width` no longer hangs for tiny values** — `width < continuation indent` falls back to emitting the remainder unwrapped instead of looping forever (`src/serializer.rs:write_plain_scalar`)
- **`remove_tag(name)`** — new function to unregister a tag handler; complements `register_tag`/`clear_tag_handlers` (`src/py/tag_registry.rs`)
- **`duplicate-key` errors are i18n'd** — `YamlDuplicateKeyError` messages now flow through `format_i18n_error` across all 4 locales (`src/i18n/locales/*.yml`)

### [0.8.0] - 2026-07-30

#### Added

- **`YAML()` instance API** — `YAML(typ="rt"|"safe"|"full", schema="core"|"yaml1.1", max_depth=1000)` with reusable configuration; `.parse()`, `.safe_load()`, `.safe_loads()`, `.parse_file()`, `.parse_all_docs()` methods
- **Python `Node` API** — `Node` class with `find()`, `filter()`, `walk()`, `to_yaml()`, `parent`, `children`, `root_type`, `value` for AST navigation; JSONPath-like query language (`$.key.sub`, `$.arr[0]`, `$..deep`)
- **`doc.version` metadata** — `YamlDocument.version()` returns the YAML spec version (default "1.2")
- **`MergedView`** — `doc.merged()` returns a read-only dict-like view with merge keys resolved
- **Lifecycle warnings** — `Node.release()` to explicitly invalidate a node; stale access emits `RuntimeWarning` + `YamlDocumentError`

#### Changed

- `parse()` / `safe_load()` now delegate to `YAML().parse()` / `.safe_load()` as syntactic sugar
- `YamlDocument` now stores `version` field for document metadata

### [0.7.1] - 2026-07-30

#### Added

- **ryaml benchmark comparison** — `tests/test_benchmark.py` now benchmarks against `ryaml` (Rust YAML library) alongside PyYAML and ruamel.yaml; `benchmark_compare.py` rewritten as a feature comparison report (`tests/test_benchmark.py:25-28`, `.github/workflows/ci.yml:219`)
- **CI compliance threshold raised** — YAML Test Suite compliance gate increased from 70% to 75% in `test_compliance_report()`; valid parse rate gate at 95% (`tests/test_yaml_suite.py:251`)
- **CI dependency consolidation** — added `.ci/requirements-test.txt` and `.ci/requirements-test-lite.txt` for unified test dependency management across publish workflow and local dev
- **Benchmark modernization** — migrated from `pytest-benchmark` to `pytest-codspeed` for faster C-extension-based statistical benchmarking; all CI jobs now use `-r .ci/requirements-test.txt`
- **Rust benchmarks migrated to Divan** — replaced `codspeed-criterion-compat` with `codspeed-divan-compat` v5.0.1; 16 benchmarks rewritten from Criterion groups to `#[divan::bench]` attributes (`Cargo.toml`, `benches/yaml_bench.rs`)

#### Changed

- CI benchmark job installs `ryaml` for cross-library comparison
- `benchmark_compare.py` now delegates timing to `pytest-benchmark` and serves as a feature comparison/reporting tool

### [0.7.0] - 2026-07-29

#### Added

- **Serializer `max_depth` guard** — `serialize_node_internal` now tracks recursion depth and raises `YamlMaxDepthError` when exceeding the limit (default 1000), matching the parser's protection (`src/serializer.rs:135-145`)
- **Serializer hot-path optimization** — 5 optimizations targeting block-style serialization for ~4.9% roundtrip speedup:
    - Inlined `write_anchor_tag` and `write_inline_comment` None checks (eliminates method calls for ~99% of nodes)
    - `write_indent` hot/cold path split (direct index for cached levels ≤64)
    - `write_plain_scalar` fast path for short ASCII alphanumeric strings (≤8 chars)
    - `write_scalar_for_key` direct dispatch for Plain scalars (avoids dispatch chain)
- **pytest-benchmark migration** — Python benchmarks migrated from raw `time.perf_counter()` to `pytest-benchmark` for statistical rigor, structured JSON output, and CI integration (`tests/test_benchmark.py` + updated `tests/test_performance.py`)

#### Changed

- `pytest-benchmark` replaces raw `timeit` in Python benchmarks
- CI benchmark job now runs `pytest --benchmark-json` instead of standalone script

#### Removed

- `write_inline_comment` method — inlined at all call sites
- `Comment` import from serializer — no longer needed

### [0.6.0] - 2026-07-27

#### Added

- **Async serialization** — `safe_dumps_async`, `safe_dump_async`, `safe_loads_async`, `safe_load_async` via `asyncio.run_in_executor` (`python/pyrs_yaml/async_dump.py`)
- **JSON Schema validation** — `YamlValidateError` exception + `YamlDocument.validate(schema)` method (accepts `str` or `dict`); delegates to Python `jsonschema` module
- **`YamlDocument.to_json()`** — serialize document to JSON string (uses Python `json.dumps`)
- **Incremental re-parse** — `YamlDocument` now stores source text (`doc.source()`); `doc.reparse(resolve_merges=True, schema="core")` re-parses in-place
- **29 new tests** across `test_async.py` (8), `test_validate.py` (14), `test_reparse.py` (7)

#### Changed

- `YamlValidateError` registered as new custom exception (inherits `ValueError`)
- `rust_i18n::i18n!` macro path updated to `"src/i18n/locales"`
- `validate_translations()` test paths updated to match new locale directory

#### Removed

- Deleted redundant `src/i18n/en.ftl`, `src/i18n/zh-CN.ftl` (never referenced by rust-i18n)
- Moved `locales/*.yml` → `src/i18n/locales/` (co-located with i18n module)

#### Dependency Changes

- Runtime dependency: `jsonschema>=4.25.1`
- Dev dependency: `pytest-asyncio>=0.23` (moved from runtime, no longer pinned)

### [0.5.0] - 2026-07-27

#### Fixed

- **`Serializer::write_node`** — `.unwrap()` on `values.iter().next().unwrap()` in `block_mapping`/`block_sequence` replaced with safe indexed access to eliminate potential panic on edge-case ASTs
- **`YAML_SCHEMA` constant** — typo `yamorg2002` corrected to `yamlorg2002` (matches YAML 1.2 spec URL)
- **Development documentation** — `AGENTS.md` updated with mandatory `uv run` prefix for Python commands and direct `cargo` for Rust commands

### [0.4.0] - 2026-07-27

#### Added

- **132 new gap-filling tests** — comprehensive coverage for previously untested APIs
- **i18n function tests** — `set_language`, `get_language`, `list_languages`, `detect_language`, `negotiate_language`
- **`parse_all_docs`** dedicated test suite — single doc, multiple docs, empty, comments
- **`parse_file` success case tests** — basic parsing, comments preservation, file-not-found error
- **`to_yaml_with_options` tests** — `explicit_start`, `explicit_end`, `indent_size`, `sort_keys` order preservation
- **`to_dict()` method tests** — scalar root, nested, list, bool, null, anchor resolution, empty mapping/sequence
- **YamlDocument dunder method tests** — `__repr__`, `__str__`, `__contains__`, `__len__`, `__iter__`, `__getitem__`, `root_type()`
- **Bytes input tests** — `parse(b"key: value")`, UTF-8 bytes, invalid UTF-8 error
- **Unicode & special character tests** — CJK, emoji, roundtrip, CRLF line endings, duplicate keys
- **`safe_load`/`safe_loads` feature coverage** — anchors, merge keys, block scalars, flow collections, special floats, type resolution
- **`from_dict` edge cases** — special characters in keys, nested lists, None values, empty dict/list
- **`from_json` round-trip** — nested structures, arrays, invalid JSON error
- **`dump_file` tests** — success path, invalid path error
- **YAML Test Suite individual case tests** — octal, hex, scientific notation, NaN, infinity, merge keys, explicit/implicit keys, bool/null variants, block scalar strip (`|-`), flow collections
- **`resolve_merges` parameter tests** — preserving `<<` when disabled, resolving by default
- **Flow collections roundtrip** — root-level and nested flow mapping/sequence
- **Anchor on non-scalar nodes** — mapping anchors (`&defaults`) and sequence anchors (`&items`)
- **Sequence indexing tests** — positive index, out-of-range error
- **Merge key integration** — roundtrip with resolved and unresolved merge keys
- **Tag preservation** — `!!seq` and `!!map` tag test coverage
- **Comment preservation** — inline and standalone comment tests on complex structures

#### Changed

- Fixed version sync: `python/pyrs_yaml/__init__.py` `__version__` updated from 0.2.0 to 0.4.0 to match Cargo.toml/pyproject.toml
- Removed stale 0.2.0 wheel artifacts from `dist/`

### [0.3.0] - 2026-07-27

#### Added

- **NumPy ndarray serialization** — `safe_dump()` / `safe_dumps()` / `from_dict()` / `dump_file()` now support `numpy.ndarray` of all dimensions (0-D through N-D)
    - Supported dtypes: `int8/16/32/64`, `uint8/16/32/64`, `float32/64`, `complex64/128`, `bool`
    - Multi-dimensional arrays serialize as nested YAML lists with correct indentation
    - Complex numbers serialize as `(re+imj)` string format
    - `0-D` scalar arrays reshape to 1-D and serialize as a single-item list
    - `PyUntypedArray` + `PyArrayDyn` via `numpy` Rust crate for zero-copy dtype dispatch
    - GIL released during slice iteration for maximum performance
- **`quoted_scalar()`** — new `CustomNode::quoted_scalar()` constructor for values requiring single-quoted YAML style
- **Type resolution for quoted scalars** — `resolve_yaml_type` now applied to `SingleQuoted`/`DoubleQuoted` scalars for correct round-trip of quoted negative numbers
- **Comprehensive NumPy test suite** — 42 tests covering all dtypes, dimensions (0-D through 4-D), negative numbers, infinity, NaN, empty arrays, and edge cases
- Flow collections (`{}`/`[]`) round-trip support with `flow_style` field on Mapping/Sequence AST nodes
- `parse()` accepts both `str` and `bytes` input
- `parse()` supports `resolve_merges` parameter to opt out of merge key expansion
- `parse_all_docs()` for multi-document parsing via saphyr events
- `to_yaml_with_options()` with `indent_size`, `explicit_start`, `explicit_end`, `sort_keys` parameters
- `get()` supports default value parameter
- `dump_file()` for writing YAML to files
- Criterion benchmarks in `benches/yaml_bench.rs` (parse/serialize/roundtrip)
- GitHub Actions CI with matrix testing (3 OS x 4 Python versions)
- Anchor name parsing expanded to full YAML 1.2 spec (dots, colons, hashes, quoted anchors)
- `__version__` attribute, `py.typed` PEP 561 marker

#### Fixed

- **Negative number round-trip** — YAML 1.2 block sequences cannot contain plain scalars starting with `-`; negative numbers are now quoted during serialization and correctly parsed back as integers/floats
- **N-D array support** — replaced `PyArray1<T>` with `PyArrayDyn<T>` to support arrays of any dimension, not just 1-D
- **Correct nesting depth** — multi-dimensional arrays now produce exactly N levels of nesting (shape[1..] handles inner dimensions, root dimension wrapped by `plain_sequence`)
- Alias resolution in `to_dict()` and `safe_load()` — aliases now resolve to referenced values instead of `None`
- `safe_loads()` no longer uses naive `split("---")` — uses saphyr's document events
- Mapping/Sequence tags no longer discarded during parsing
- `format_scalar_for_key()` now handles Literal/Folded block scalar styles

#### Changed

- Added `numpy` crate (v0.29) as a dependency for ndarray type dispatch
- Upgraded PyO3 from 0.21 to 0.29
- Replaced 15+ boilerplate `CustomNode` constructions with `plain_scalar()`/`plain_mapping()`/`plain_sequence()`/`plain_null()` constructors
- Serializer extracted `write_anchor_tag()` and `write_inline_comment()` helpers
- Parser extracted `detect_flow_style()` helper
- Removed dead code: `ParseOptions`, `find_inline_comment`, `find_standalone_comment_before`, `format_yaml_type` (test-only)
- Consolidated 6 duplicate test files, moved 9 diagnostic scripts to `scripts/`
- Improved error messages with key/index/type context

### [0.1.0] - 2026-07-25

#### Added

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
