# Quality ledger

The entry-by-entry record of defects found, fixed and *measured*: what failed, which crash id or
seed proved it, which hypothesis was tried and refuted, and which gate now refuses the shape.
`CHANGELOG.md` points here per entry as `(details: quality-ledger (xx))`; letters are allocation
order, not importance, and entries marked SUPERSEDED keep the wrong reasoning on purpose because
a record that only lists successes teaches nothing.

Not site content: `zensical.toml` builds `docs/en`, and `check_i18n.py` mirrors only pages under
it, so this file has no `zh`/`ja`/`ko` twin and is not in the published navigation. It is an
engineering record, in the same category as `QUALITY_MATRIX.md` and `ROADMAP.md` at the root.

Related documents: [engine boundaries](boundaries.md), [performance status](perf.md).

## Contents

- The placement family - (h) through (v)
- The quality defence, entry by entry - (w) through (bg)
- Note survival: the leading slot became a list
- Shipped milestone scoping - the v0.11.3 to v0.12.0 tables after they shipped

## The placement family, entry by entry (2026-10-04 →)

### Two more note placements closed (2026-10-04, same day, two root causes)

**Update, same day: two further root causes closed four more inputs between them.** The ledger below
predicted the guard's premise was obsolete; it was, and removing it plus deepening one hoist closed
`fbc8f2ae`, `f8525a9e` and `c9031de4`. **(j) The tag-header lift no longer skips a container that
has notes of its own** — `lift_first_entry_notes` refused to run when
`meta.standalone_slot().is_some()`, written when one leading slot could not hold two stacks.
`decor.leading_comments` is a `Vec` now, so the refusal only cost a round: measured first, the fixed
point of every shape in the family is *all* stacks above the header in source order, and the guard
was deleted rather than tuned. The characterization test that had pinned the old placement was
**re-derived, not quietly kept** (`notes_stack_above_a_tagged_containers_own_note` now asserts order

- survival + a one-round fixed point), joined by
  `both_note_stacks_land_above_a_tag_header_in_one_round` reading `former-crash-fbc8f2ae.seed` (32
  bytes). **(k) `hoist_marker_note` now takes every stack on the first-pair-key spine, not the
  first** — it stopped at the first non-empty stack it found, and a chain can carry several: the
  tree of crash-f8525a9e holds `#` on the middle mapping and `!!"#~` on the innermost `~` key, three
  markers deep, while granit reports both at the outermost marker's level. Hoisting one left the
  other a level deeper, so emission settled on its second round; the walk accumulates
  outer-before-inner now, which closed crash-c9031de4 as well (`former-crash-f8525a9e.seed`,
  `former-crash-c9031de4.seed`, `every_note_on_the_marker_spine_lifts_to_the_marker_line`).
  **Attribution, measured both times:** restoring the header guard reddens exactly the two header
  tests; reverting the spine walk to stop-at-first reddens exactly the spine test; nothing else
  moves (275 → 277 tests, all green). **A hypothesis that died for it:** the first explanation
  offered for f8525a9e — that the hoist missed a note stored in the legacy `comment` slot — was
  tested and refuted (the emission did not change a byte). `take_leading_notes` was still moved onto
  the normalised `leading_comments()` view, because reading one of two storage conventions is the
  very fork `standalone_slice()` exists to close, but it is documented on that ground alone and not
  credited with the fix.

**What is left open after all of it is one drift input and one survival input.** *(Retired the same
day: the drift input closed, and the survival input now has a minimal carrier and a named boundary —
see the two entries below.)* `crash-e6551c75` (60 bytes): the container's tag header is written
before its body, and the body's outermost marker hoists notes to that same column *after* the header
line already exists — so round 1 puts `# a`, `# b` below `!5j4?` and round 2 above it. The mechanism
is now named from the tree and the emission, and the candidate fix is to let
`lift_first_entry_notes` share the spine walk (`hoist_marker_note`) instead of reading only
`pairs[0].key`'s own list; it is deliberately **not** attempted in this change, because that walk
pulls notes out of nested markers too and the pinned blast-radius shape `k: / ? / # c / ? a / : b`
depends on where those stay. `crash-ce106ccc` remains a note-survival failure with stable text,
which is the oracle's business, not the drift tier's.

**A new quality gate landed with this: localized script purity (`scripts/check_cjk_localisation.py`,
prek hook `cjk-localisation`).** It polices the `ja`/`ko`/`zh` changelogs for intrusions from
neighbouring scripts — kana outside `ja`, Hangul outside `ko`, simplified-only Han in `ja`/`ko`. It
earned its keep twice on the day it was written: the first version pinned its section-header pattern
to the root file's depth, matched nothing on the nested localized pages, and printed a **green OK
over zero lines** — caught only because a violation was injected before trusting it, which is the
same rule the note-survival oracle follows. After fixing the depth and refusing empty scans, that
injected run also surfaced two pre-existing intrusions in shipped text (a simplified 折叠 in `ja`, a
katakana glyph inside a Hangul word in `ko`), and a third in the very `zh` entry written to describe
the gate. Its allow-list is deliberately conservative: glyphs that are genuine Japanese kanji stay
unlisted, because a gate that reddes correct output is worse than none.

**Both were found by re-reading the artifacts the ledger had already filed, not by new fuzzing** —
and the first measurement of the day was that the committed corpus was clean while the artifact
directory held a fifth input (`crash-c9031de4`, 99 bytes) the discovery window had produced after
the anchor fix landed. Sorting the six open artifacts by replay rather than by memory split them
three ways: `11ced252`, `456176be`, `c9031de4`, `e6551c75`, `f8525a9e` fail idempotence; `ce106ccc`
is stable text and fails only note survival.

### (h) A container's own inline note has to be written on a line that can hold one

— `write_container_node` printed a non-standalone `comment` on a block container as a bare note line
below the block, but no reader reports an inline note from an empty line: the re-read hands that
text to the node which ended the block as its *leading* note, so round 1 was never the fixed point.
`:\t!-\r... #-o` emitted `~: !- \n# -o\n` and needed a second round to reach `~:\n  # -o\n  !- \n`
(crash-11ced252, 13 bytes — the smallest open input, which is why it went first: it is the whole
rule in one line). The note now borrows the line the container just finished (`~: !-   # -o`, one
round), which is precisely the slot granit reports it from. The safety half is the interesting part:
a line may only be borrowed when it can host a trailing note, and that is *tracked where lines are
written* rather than inferred from the emitted text — a block scalar's body line and a wrapped
continuation both look like ordinary lines, and appending into a `|` body would turn the note into
content. `trailing_note_slot_open` is set by the scalar/null/alias/flow writers and cleared by every
note line, and `tail_line_has_note` refuses the one case state cannot see (a quoted scalar whose
text contains `#`). Pinned in both directions —
`a_containers_inline_note_lands_on_the_last_value_line` (acceptance, exact text, note count,
one-round fixed point) and `a_block_scalar_body_never_borrows_the_containers_note` (refusal, and the
block value re-reads intact) — and attributed by mutation: un-appending the note reddens the first
test alone, all 274 others stay green.

### (i) The two lifts a marker line performs have to compose

— an explicit key can carry a note on its key node *and* leave a second riding the first entry of
the key body, and the reader reports both at the marker's own level. The writer chose between them
with `if`/`else if`, so the moment the key had a note of its own the body note was printed one
indent deeper and climbed a level on re-read (crash-456176be, 40 bytes as found: `?` +
`### standab:` + `?` + `# ! y%% yam2:#l: tr` + `~: ~`). The tree was read before the claim, and it
is the tree that names the cause: the source keeps the first note on the key mapping's `decor` and
the second on the inner `~` key, which is exactly the pair the `else if` threw away. Both lifts now
run, in source order, above the `?`; pinned by
`a_marker_carries_both_its_own_note_and_its_bodys_first_note` reading `former-crash-456176be.seed`,
attributed by un-composing them (that test alone reddens).

**The grouping error, recorded because it is the fourth one this cycle.** `crash-456176be` was filed
above as "shows `crash-f8525a9e`'s signature — recorded as a second carrier of that family by that
measured symptom, not as a verified attribution", and even that hedge was too generous: fixing
456176be left `f8525a9e` and `c9031de4` red, so the shared signature was a shared *symptom class* (a
note that rides an explicit-key body) and not a shared cause — their notes ride a nested **marker**
rather than a scalar key, which `hoist_marker_note`'s first-pair-key spine does not reach. The open
list at that moment was `f8525a9e`, `c9031de4`, `e6551c75` (idempotence, one extra round each,
nothing lost) and `ce106ccc` (survival only) — the first two were closed the same day by root cause
(k) below. `e6551c75` is not even in the marker family: two notes compete with a container's own tag
header line, which is the ordering rule `writes_block_header` already implements for one shape and
misses for another. **The 60 s window then produced a fourth carrier of that header family and a
smaller one — `crash-fbc8f2ae`, 32 bytes** (`# yr-` + `# y?` + `!5b54?` + `# yr-` + `# yrrr` +
`~: ~`): the first emission writes one note stack above the tag header and a second stack below it,
and the re-read hoists the lower stack too. That is not a new discovery — it is the *documented
cost* of the guard `a_note_is_not_stacked_above_a_tagged_containers_own_note` pins, which refused
the lift because a second note line would fall into the single leading slot the reader owns and
trade drift for lost text. **The list work removed that reason**: with
`leading_comments: Vec<Comment>` a container can carry both stacks above its header with nothing
dropped, so the guard's premise is obsolete and the guard itself has to be re-derived, not quietly
kept — measure the emission order first, then lift, then re-check every pinned shape in
`writes_block_header`'s family. Next step for the marker family is unchanged — read both trees, name
the difference, then fix one cause.

### Two placements closed by measurement, not by the ledger's guess (2026-10-05)

**The day's first measurement was a replay, not a reasoning.** A 15-byte artifact the window had
left (`crash-22cb5f67`, `+#:` TAB `!-` CR `...` SP `# -o` LF) was replayed and its two trees read
before any claim was written, and that step is the whole difference between this entry and the four
grouping errors above: the artifact was assumed to belong to the header family because its emission
*looks* like crash-11ced252's (the same bytes are quoted in `write_container_node`'s own comment),
and it does not. **(l) A `#` that is only text was evicting a note from its line.** The tree says
the note sits in the **root mapping's own inline slot**
(`comment { text: "-o", standalone: false }`) — nothing to do with a marker, a header, or a leading
stack at all. The writer had the right plan (hang the container's inline note on the line it just
finished, the one slot a reader reports inline notes from) and the right guard
(`trailing_note_slot_open`), but the guard's escape hatch asked whether the finished line "already
has a `#`" by scanning raw bytes — and the line was `"+#": !- `, whose `#` is a character *inside a
double-quoted key*. The refusal fired, the note fell to a line of its own, and the re-read handed
that line to the following node as its leading note, so round 2 moved it inside the value block. The
predicate is now `line_has_comment_marker`, which walks the line with quote state (`''` inside
single quotes, a backslash inside double) and applies YAML's own rule that a `#` opens a comment
only after a space, a tab, or the line start. Measured after the change: one emission is the fixed
point (`"\"+#\": !-   # -o\n"`), pinned by `a_quoted_hash_does_not_demote_the_containers_note` (seed
`former-crash-22cb5f67.seed`) and by `comment_marker_scan_respects_quoting`, which tests the
predicate on its own terms rather than only through a document — the `#` inside `a: "x#y"`, the `#`
that *is* a marker after `a: "x#y"  `, `'doubled''#still'`, `!-#tail`. **Attribution:** putting the
raw scan back reddens exactly the end-to-end test; the predicate test stays green because it does
not go through `tail_line_has_note`, which is why both were written. *(The mutation probe also
caught a stale-build trap worth recording: the first probe run reported "nothing failed" while the
library was in fact failing to compile, and the test binary that ran was an older one. Only the
build log says which code was measured.)*

### (m) `lift_first_entry_notes` now takes the whole marker spine, and the ledger's own deferment was the thing to test

The paragraph above deferred exactly this change, reasoning that the spine walk "pulls notes out of
nested markers too and the pinned blast-radius shape `k: / ? / # c / ? a / : b` depends on where
those stay". That sentence was never measured, and the discovery window produced a second carrier
the same afternoon — `crash-8f7085b0` (43 bytes, `!3b55?b55?` + `? ?` + five note lines + `?` /
`~: ~` / `:` / `~`) — whose first round writes the header then all five notes and whose fixed point
is the notes above the header. With the lift widened, `e6551c75` and `8f7085b0` both reach the fixed
point **in one round**, and `cargo nextest run -p pyrs-yaml-core` with the walk withdrawn reddens
**exactly one test** (`every_spine_note_clears_a_tagged_containers_header_line`):
`notes_stack_above_a_tagged_containers_own_note`, `a_note_under_a_tag_header_is_lifted_above_it`,
`notes_above_an_empty_container_all_survive` and
`seq_item_value_with_standalone_comment_is_not_compacted` pass with the walk and without it, so the
feared blast radius does not exist. Fifth lesson of the same shape: an inferred cost is not a reason
to defer, and the ledger now records it as refuted rather than quietly dropping the sentence. Seeds
`former-crash-e6551c75.seed` (60 B) and `former-crash-8f7085b0.seed` (43 B) are committed as found;
both inputs replay CLEAN.

**The survival input now has a boundary, which it did not have** *(and the boundary has since been
closed — see (n) below; the measurement is kept because it is what made the fix narrow)*.
`crash-ce106ccc` (69 bytes) lost text, but a shape sweep over twelve root-scalar forms says exactly
which: a **property-only node at the document root** — `!x # note` and `&a # note` — arrives with no
note in the AST at all, so nothing can emit it. `abc # note`, `"abc" # note`, `!!str abc # note`,
`&a abc # note`, `k: v # note`, `k: !x v # note`, `k: !x # note`, `- v # note`, `k: # note` and a
comment-only document all keep theirs. So the loss is neither "trailing notes" nor "tags" nor "empty
scalars": it is the empty *content* of a root node that carries only properties, and the 9-byte
`!x # note` is now the smallest carrier rather than the 69-byte artifact. Measured at the AST, not
inferred from the text: `abc # note` finishes with `comment=Some(("note", false))` while `!x # note`
and `&a # note` finish with `comment=None` **and** `leading_comments=[]`, so the note is neither
attached nor pending — it is gone before `finish_document` ever sees it, which puts the next step in
the root-scalar ingest path rather than in the writer. At the time of that sweep `ce106ccc` was
deliberately **not** seeded, because a seed is a promise that the input passes; (n) below kept the
promise and the input is seeded now.

**Two pillar claims checked against the tree rather than against this ledger.** *Crate matrix /
`no_std`:* `pyrs-ast`, `pyrs-json`, `pyrs-schema` and `pyrs-toml` are `#![no_std]` +
`extern crate alloc` with a `std` feature, and `ci.yml`'s `no-std-check` job builds them for
`thumbv7em-none-eabi` — a target with no std at all, which is the only honest form of the claim.
Running that exact command locally is what caught the regression recorded in the changelog: the
stacked-note work had put `std::mem::take` into `pyrs-toml`, which every host build forgives; the
job is `if: github.event_name != 'push'`, so it gates PRs, and it had never been run locally, which
is why the fix ships with the entry. *CI performance gate:* `codspeed.yml` triggers on
`pull_request` and on pushes to `main`, and runs both tiers —
`cargo codspeed build && cargo codspeed run` for the Rust benches and
`pytest --codspeed -m benchmark` for the Python ones — so zero-regression enforcement is on the PR
path, not only on release. **Housekeeping that was itself a hazard:** 32 `cargo fuzz tmin` scratch
files had accumulated in `fuzz/artifacts/`; each was replayed against its own target and only the
CLEAN ones deleted (`deleted=32 kept=0`), leaving the open artifacts as the only occupants of the
directory. The `.cache/bench-head` worktree (a pristine-HEAD comparison tree from an earlier
attribution) was removed; the `Temp/opencode/yaml-rs-wt` worktree predates this task, is not ours,
and is left alone. A seed is a promise that an input passes, which is why `ce106ccc` stayed an
artifact until (n) made it pass — it is seeded now.

### (n) A root node that carries only properties used to throw away its note — twice, in two orders

The boundary the sweep above named is now closed, and reading the *event stream* rather than the AST
is what named it precisely: for `!x # note` granit reports `comment("note", Right)` **before** the
`Scalar` event, while for `abc # note` it reports it after. `attach_inline_comment` had no candidate
at that moment — empty stack, `result` still `None` — and returned `true` anyway, so the caller
never carried the note forward and it vanished with no trace. That is the whole of `crash-ce106ccc`
(69 bytes): its `#` wall is one note that arrives before a tag-only root, and
`former-crash-ce106ccc.seed` now emits
`# #################################################, #######&b #` above `!###0` and stops there.
The second order is `crash-7918272c` (11 bytes, `!m` CR `...` SP `# -o`): the note arrives **after**
`DocumentEnd`, so it was bound back onto the finished root as an *inline* note, the writer printed
`!m   # -o`, and re-reading that reports the note before the node — two ingest orders writing one
note into two slots, which is a drift no amount of writer-side placement rules could settle. Only a
real attachment may report success now; a note following the end of a non-container document is
carried forward; and a note still pending when the document finishes rides the root as a **leading**
note (leading, not inline, because that is the slot its own spelling re-ingests into — the asymmetry
the probe measured). `a_note_beside_a_property_only_root_survives_and_settles` pins five shapes plus
both seeds on exact emission, text survival and a one-round fixed point; withdrawing the honest
return value reddens that test and nothing else in 280. A container root keeps the inline home on
purpose: `a: 1` + `# trailing note` really does re-read from the last value line, which is what
crash-96fa252c's flush rule and `a_containers_inline_note_lands_on_the_last_value_line` /
`a_quoted_hash_does_not_demote_the_containers_note` hold — the first version of this fix refused
*every* post-`DocumentEnd` attachment and reddened both of those pins, so the rule was narrowed to
what the reader actually reports back.

**What the discovery window surfaced next is a different class again, it is open, and it is
deliberately left unexplained.** After all four note inputs went clean, the same 60 s window
produced `crash-3495cc86` (72 bytes), which fails the tier's *re-parse* assertion rather than the
idempotence one: `fmt output failed to re-parse: duplicate key: y`. Measured, in order: the input
parses; `to_dict()` of the input is `{'bg': {'x': 1}, 'y': 1, '<<': {'y': 3}}`; and `to_yaml()`
emits a document holding **both** `y: 1  # !:` and `y: 3  # ::` at one level — two keys our own
reader then (correctly) refuses. So the defect is in the emission, not in the duplicate detection,
and the engine broke its own contract (`to_yaml` never emits text its parser rejects).

**What was tested before touching anything, and what each test settled.** Three hand-written shapes
that look like the bug and are *not*: `y: 1` + `<<: {y: 3}` keeps `<<` as a literal key (`to_dict` =
`{'y': 1, '<<': {'y': 3}}`) and its emission re-parses; `y: 1` + `<<: {y: 3, w: 4}` **does** filter
— `to_dict` = `{'w': 4, 'y': 1}`, the colliding merged `y` is dropped exactly as YAML 1.1 requires,
emission re-parses; and the alias path `b: &b` + `y: 1` + `<<: *b` gives `{'b': {'y': 3}, 'y': 1}`
with a re-parseable emission. PyYAML's `safe_load` agrees with our resolution on all three. Even the
13-byte `y: 1` + `<<:` over an indented `y: 3` re-parses. So `prepend_merged_pairs`'s
documented "filtered against existing keys by the caller" filter really does hold for the shapes a
human writes, and the artifact escapes it through something else — which is why no diagnosis was
written before the reduction, and why the first diagnosis this entry recorded ("the input already
holds two top-level `y` keys, and the duplicate check compares whole nodes") was **retracted after
`tmin`** rather than kept.

**`cargo fuzz tmin` has now run, and it is what replaced that guess.** The artifact reduces to **19
bytes**: `:` LF `# &` LF `y:` LF `# :` LF `<<:` LF `y:` (re-measured as a still-failing carrier:
`fmt output failed to re-parse: duplicate key: y`). Read the reduced pair, not the 72-byte one: the
input is a mapping with a **null key**, a note line `# &`, a keyless `y:` entry, a `# :` line, and
`<<:` whose value is an indented `y` — and our emission is:

```yaml
y: ~
~: ~
# &
y: ~  # :
```

### (o) A merge can no longer repeat a key its mapping already owns — and the reason the earlier probe misled is the part worth keeping

The reduced shape said the defect lived exactly where `prepend_merged_pairs` documents a filter that
was supposed to protect it: "`merged_pairs` is filtered against existing keys by the caller, so
`shift_insert` cannot collide". Nothing filtered — there is no such filter anywhere in the call
path. The first *three* hand-written merge shapes had looked filtered because they were measured
through `to_dict()`, and building a Python dict collapses two equal string keys into one silently:
the AST had been carrying both pairs all along, and only the emission showed it. So the correction
is methodological as much as local — `to_dict` is not an observation of the AST, and the contract at
stake (`to_yaml` never emits text its own reader refuses) can only be checked on the emitted bytes.
That is also why the earlier entry in this ledger, which reasoned from whole-node duplicate
detection and was then "retracted after `tmin`", was reaching for a mechanism that never existed:
the merge expansion was simply not overriding.

Identity for the new override is the **emitted value of an untagged scalar key** — the same rule
`push_node` applies when it raises `DuplicateKey` — because both questions are the same question
(may these two pairs coexist in one emitted mapping?), and any other choice lets the merge layer and
the duplicate detector disagree about output one of them rejects. Tagged and non-scalar keys keep
whole-node equality: `!a y` and `y` re-read apart, so folding them would be its own loss. A dropped
merged pair's notes are re-homed onto the surviving entry rather than discarded — `crash-953bf87a`
and `crash-f453c4e5` are the two precedents that made dropping them a regression in its own right,
and an override that eats a comment just trades one silent loss for another. Pinned by
`a_merge_never_repeats_a_key_the_mapping_owns` over `former-crash-3495cc86.seed` (the 19-byte
minimisation, committed as found: emission, re-parse, one-round fixed point and note survival
asserted together), and the corpus gate reddens with the override withdrawn as well — attributing
the change to those two and nothing else in 283.

**Both tiers are green now, and the ledger's open item has changed shape.** The 75-file seed replay
passes on all four targets, and the 60 s discovery window that produced `crash-3495cc86` now lands
on a different input: `crash-a916de77` (48 bytes) — `bg: &b` + a nested `~:` carrying `# -o` and a
tagged empty `!~-M...`, whose root-level note `# !! &? #!!"#!"chg0#~-.I` is written at column 0 in
round 1 and pulled to indent 4 above the tag in round 2. Nothing is lost and the text settles one
round late, so it is the drift tier's business, not the survival oracle's. Its geometry is the one
(k) and (m) closed for marker chains — a note that trails a nested block at *shallower* indent than
where the reader reports it — appearing here with no marker in between, which is the seam the next
attempt starts from, with both trees read before any cause is named. `crash-a916de77` is kept as an
artifact, unseeded and unfixed: a seed is a promise the input passes.

### (p) A key's note can no longer travel down onto its value's line

What the blocked PR tier was built to do, it then did: after #267 landed, `yaml_roundtrip`'s
discovery window on a PR surfaced `crash-1b01ac3f` (93 bytes → 11 after `tmin`: `b:` SP `!` SP `#`
SP `&` LF `#~`). Its geometry is neither the marker chain of (k)/(m) nor the merge of (o) — the tree
holds the note on the **key** (`key.inline = "&"`) and a separate leading note on the value, and the
drift comes from the writer. `write_mapping_pair` ends by handing a simple key's trailing note
to "the line that just finished", which is the pair line while the value sits on it — but once the
value had to move down to take its own leading note, that "just finished" line is the *value's*, and
a trailing note after a tag-only scalar re-reads as the **value's leading** note. Owner changes ⇒
note climbs a line every round ⇒ no fixed point, and every individual shape in isolation is stable,
which is why only the combination reproduces. Fix is one insertion point: remember where the `key:`
line ended when the next-line branch is taken, and put the key's note there. Measured before and
after — the 93-byte artifact and its 11-byte reduction, the four hand-written shapes that are *not*
the bug (tag+text, anchor+empty, quoted empty, plain own-line: all already stable), the full replay
of all **76** committed seeds across four targets, and the mutation attribution: withdraw the
recorded position and exactly one test reddens out of 283, so no existing behaviour depended on the
drifting placement. Instruction cost is inside the gate's noise (+0.05%, no re-baseline needed).
Open drift inputs after this: `crash-a916de77` (48 bytes, the shallower-indent note above a tag) —
still unfixed, still unseeded, still the next one.

### (q) A merge key can no longer move an anchor behind its alias

`crash-9b77aea4` (78 bytes → 15 after `tmin`: `:` SP `&` `b` LF `<<:` LF SP `<:` SP `*b`) fails the
*re-parse* assertion, not idempotence, and the mechanism is in the merge expansion rather than the
writer: `prepend_merged_pairs` inserted every merged pair at index 0, so a mapping that defines `&b`
on an earlier own key and reaches `*b` through the merged map emitted the **use above the
definition** — `<: *b` then `~: &b ~` — which is not a fixed-point wobble but text our own parser
refuses (`found unknown anchor at char 3 line 1 column 4`). Source order is the invariant that
protects that property (a valid document defines every anchor before any use of it), so the
expansion now inserts at the index the `<<:` occupied instead of the front. The blast radius is
small by construction: a mapping whose merge key is already first gets slot 0, which is what prepend
did, and 487 → 489 Rust tests plus 1781 Python tests pass without a single expectation edited —
including `test_single_merge`, the documented `<<: *defaults` shape. Pinned twice, at the order
(`merged_pairs_take_the_merge_keys_own_slot`, asserting the emitted text) and at the contract
(`merge_expansion_never_emits_an_alias_before_its_anchor`, three shapes, each re-parsed *and*
required to be a one-round fixed point). Both reddened before the fix with the CI error verbatim,
and the mutation attribution runs both ways: forcing slot 0 back reddens those two tests **and** the
seeded replay (`-runs=0`, exit 1, one crash, panic at `yaml_roundtrip.rs:35:29` — the site CI
reported), so the seed is a live guard rather than a souvenir. Seeded as
`former-crash-9b77aea4.seed`, 15 bytes: `tmin` leaves its intermediate candidates beside the final
one, and the first copy grabbed a 44-byte intermediate, so the seed was re-taken by name from the
size listing and verified at 15 bytes — the same class of mistake as seeding `as found`, but
quieter, because the wrong file is also a valid seed.

**Two ledger corrections from the same sweep, recorded because they change what is knowable.**
`crash-1fba6330` (114 bytes, an earlier failing run's artifact) **no longer fails** on this tree:
replayed directly it exits 0. It is deliberately *not* seeded — `tmin` cannot minimise an input that
does not crash, so its root cause stays unattributed, and a seed whose cause is unknown cannot be
paired with the regression test the pipeline requires; what is known is only that #272 or a
neighbour already covers the shape. `crash-a916de77`, which (p) named as next, is in a worse state:
its run's artifact is already expired, so the bytes are gone and its live status is **unverified**,
not "still open" — the sampler will re-surface it or not, and until it does the claim "still
unfixed" is unsupported. What the backlog hunt did produce is a policy fact worth keeping: the last
three merges' Fuzz runs are green because the PR tier is now `-runs=0` replay only, and the sampler
runs on schedule — that split (#266/#273) is what makes a merged tree's greenness mean "no fixed
crash returned" rather than "nothing was sampled".

**One pre-existing harness race, found while proving a gate rather than by intent.**
`pyrs-yaml-cli::cli to_json_dialect_flags_conflict` fails intermittently at default nextest
parallelism with `write stdin: BrokenPipe` (`crates/pyrs-yaml-cli/tests/cli.rs:35`): the CLI child
exits on the conflicting-flags path while the test is still writing its stdin. It passes 3/3
isolated, 71/71 for the crate at `-j 1` and 487/487 for the workspace at `-j 1`, so it is timing,
not behaviour, and no CI tier has flagged it. Left untouched here (one root cause per PR); the fix
is for the test to stop writing stdin when the contract is "reject before reading".

### (r) A block value made of nothing but line breaks keeps its value

The sampler the new budget PR (#275) un-killed had a finding waiting in seconds: `>+8\r\r#` (6
bytes) drifts at the *idempotence* assertion, and the geometry turned out to be neither the header
nor the note family. Measured with a constructed-AST matrix (style × chomping × indicator × value),
because source-level reasoning kept producing wrong hypotheses:
`Scalar { value: "\n", Folded, Keep, Some(8) }` emits `>+8\n\n`, which re-reads as value `"\n"` but
`Clip`/no indicator — and `Clip` on an empty-looking body then yields `""`. So the **value** decays
(`"\n" → "\n" → ""`), not just its spelling. The two emit-side normalizations that make an empty
body idempotent (drop the unrecoverable indentation indicator; write the chomping that re-reads
identically) were both keyed on `value.is_empty()`, and an all-break body is not empty. Both writers
now derive one `no_content_line` flag from the `first_text_line` they already compute,
`effective_chomping` promotes an all-break body to `Keep`, and the shape reaches a fixed point in
one round with its value intact. Verified: 6-byte seed `former-crash-2f6b1eff.seed` replays clean,
and the full corpus (**62** seeds, `Done 63 runs`) replays with zero artifacts; 1781 Python tests
unchanged.

**Two attributions, because two different mutations ask different questions.** Reverting the
*predicate* (`no_content_line` back to `value.is_empty()`) reddened exactly the two new pins.
Forcing the writers' shared `no_content_line` flag to `false` reddened
`a_block_value_that_is_only_line_breaks_stays_itself` **and the pre-existing
`parser::tests::empty_block_header_reaches_a_fixed_point`** — so the rule is load-bearing for
behaviour pinned before this change, which the first mutation alone would have hidden.
(`clip_cannot_carry_an_all_break_block_body` passes the flag explicitly, so it only discriminates
edits to `effective_chomping` itself; a run where it stayed green under the writers' mutation is
correct, not a gap.) Both lists came from `--no-fail-fast`.

**The instruction-count gate then caught me, which is the gate working.** The first cut scanned the
value a second time inside `effective_chomping`, measured `+0.65%` on `serialize_block_scalars`
locally — three identical runs, Ir is exact — and `+2.11%` on the runner: true cost plus the ~1.45%
WSL-to-runner drift the 2% tolerance was calibrated on, landing over the line.
`FAIL serialize_block_scalars: +2.11% over baseline (tolerance 2.00%)`. The honest fix was not a
re-baseline: both writers already compute `first_text_line` for the force-indicator decision, and
`first_text_line.is_empty()` is exactly the predicate, so one computation now feeds chomping and the
indicator together — `+0.15%`, back inside the gate. The changelog's cost sentence was rewritten
across all five mirrors to say this, because the version I first committed ("no measurable cost")
was read off a **truncated local `tail -4`** that had cut the serialize_* rows and shown only the
final "OK" line: the same truncation failure that had already cost me a mutation list, this time
producing a false performance claim in a shipped document.

**Two measurement traps this cycle sprang on me, both recorded because they produce confident false
results.** (1) *`cargo nextest` fails fast by default*: it cancelled at `429/491 tests run` after
the first red, so my mutation "failure list" was really "everything up to the first failure" — and
the second new test, which the mutation must also redden, had simply never run. Attribution claims
need `--no-fail-fast`. (2) *The /mnt/d stale-rebuild trap fired on a restore rather than a build*:
after copying the pre-mutation file back, nextest still reported the mutated tree's 2 failures,
because cargo did not recompile `pyrs-yaml-core` (fixed source passes 491/491 only after
`cargo clean -p pyrs-yaml-core`; `touch` is the cheap prophylactic). The asymmetry is instructive:
mutating the file triggered a rebuild, restoring it did not — so a "restored and still red" reading
would have been a fabricated bug. Cross-check: the mutation numbers stay trustworthy, since the
fixed source cannot produce those 2 reds.

### (s) A folded duplicate `<<` keeps the anchor it defined

The budget fix paid for itself immediately: main's push sampler ran its full 3 rounds and reported
`1 distinct crash signature` instead of being killed, and its artifacts were two inputs for one bug.
`crash-43eca7a3` (18 bytes) is the **re-parse** assertion again — `<<: &b` folded against the real
merge `<<:` below it, and the fold at `parser/mod.rs` re-homed the dropped entry's *notes* while
dropping the *anchor* it carried, so the emission used `*b` with no `&b` anywhere. The rule that
orphaned it is the one that exists for a good reason (crash-973bd522: a mapping holds exactly one
`<<`, and `IndexMap` keyed by whole nodes kept both spellings, so a pair vanished per round) — the
hole was that "the entry disappears" and "the definitions it carries disappear" are different facts,
and only the first was handled.

The repair is not a special case for `<<` but the general invariant the engine already promises: no
alias may name a node the document no longer contains. The receiver now records `(name, node)` for
anything anchor-bearing the fold removes, and `finish_document` inlines that node at every alias
whose name has **no remaining definition** in the finished tree — so ordinary shared aliases are
untouched and the walk only happens when a fold actually dropped an anchored entry (the common path
pays nothing; measured, `parse_small` moved −1.18% → −0.93%, i.e. ~0.25pp of the fold's added
`anchor()` check, inside the gate). `<<: &b⏎<<:⏎: *b` now emits `<<: ~⏎~: &b ~`, stable in one round
with the same `to_dict()`. What the fix gives up is stated in the code and the changelog rather than
discovered later: an orphaned alias becomes the node it named, so value semantics survive but
shared-identity spelling does not — and once the definition is gone there is no identity left to
share. Keys are deliberately not rewritten (`inline_aliases` documents why: swapping a key changes
the map's identity and order mid-iteration, and this fold never leaves an alias in a key slot).

Minimisation needed a build of the *old* code, which is the part worth remembering: a fixed input
cannot be reduced (`tmin` refuses it — it no longer reproduces), so the 18-byte find was minimised
to 15 inside a scratch change with the repair disabled, and that same scratch produced the
attribution: **492 tests, exactly 1 failure — the new pin** — so nothing else was leaning on the
orphaning behaviour. Clippy then caught a `collapsible_if` in the new walker (`-D warnings`), which
the test run would not have shown.

**Still open, found by the same run:** `crash-cf49fe85` (48 bytes, `!-?...` tags with `\r` and
`# -o` notes) fails the **idempotence** assertion on main — first emission
`# -\n~: !-?...   # -o!-\n# -o!:  !!-\n…`. Different family (note/tag placement under CR-terminated
lines), different root cause, so it keeps its own PR: minimise → reproduce → test → mutate → seed →
mirror set. *(It has since closed, and the guess about which family it belonged to was wrong: the
shape that drifts is a bare `!` with no text, not the `!-` tags quoted above — see (t). The 48-byte
artifact was minimised to 13 bytes and the minimised form is what the fix and its tests are pinned
to.)*

### (t) A mapping's own note stops wandering onto a bare-tag value's line

The sampler's other find, `crash-cf49fe85` (48 bytes, minimised to 13: `:` TAB `!` SP `#-` CR `...`
SP `#-`), is the idempotence assertion, and it is #272's rule one slot further out: the
*container's* inline note was appended to the line its body ends on, and when that value renders as
a lone `!` (non-specific tag, no text) a note after it re-ingests as the **value's** leading note —
so the container loses it and the document settles one round late. Candidate emission shapes were
measured rather than reasoned about (parse each, dump where the note lands): for a bare `!` no shape
keeps the note container-owned, and the fixed point round 2 reaches is `~:  # -\n  # -\n  ! `, so
the writer now emits that directly on round 1 while the AST keeps the note on the container as
parsed. A *named* tag (`!-`) is different — measured: its inline note re-reads back to the
container, so it is untouched and `crash-11ced252` / `crash-22cb5f67` still assert their original
text; the first cut of this fix routed both and reddened those two pins, which is what forced the
narrowing to the non-specific tag.

**The instruction-count gate rejected this fix three times, and each rejection named a real
inefficiency.** Per-pair predicate: `serialize_small +2.75%`, `serialize_anchors +3.90%` — obvious
in hindsight, a match on every pair of every mapping. Hoisted to once per mapping: still `+1.90%`,
because the *argument* `pairs.last()` was evaluated eagerly and the "cheapest test first" ordering
inside the function never got to run. Guarded at the call site: `+1.39%` — the remainder was not the
predicate at all but the **loop split** I had introduced to move the hand-off out of the loop.
Putting the rare routing in its own branch and giving the common case back its original single loop
landed `serialize_small +0.54%`, `serialize_block +0.44%`, `serialize_block_scalars +0.31%`, all
nine scenarios inside tolerance. The lesson is not "measure twice": the gate measured what my
reasoning could not see, and each of the three numbers was a *different* cause.

Attribution on the final form (`--no-fail-fast`, full 493): disabling the routing reddens exactly
`a_note_after_a_tag_only_value_settles_in_one_round` and
`every_note_in_the_seed_corpus_survives_one_emission` — the note-survival gate catching the same
defect through a second, independent oracle — and the seeded `-runs=0` replay fails with one
artifact at `yaml_roundtrip.rs:37`. The sorted writer gets its own test (`b: !\na: 1\n# tail\n` with
`sort_keys`) because "the pair that ends the body" is the *emission*-order last pair, which differs
from the insertion-order last one exactly when sorting reorders them — the first implementation had
that backwards and the test is what pinned it. Seeds now 5/64/6/5, all replaying clean;
`cargo clean -p pyrs-yaml-core` before every "green" claim after a restore, per the trap recorded
above.

### (u) The (t) rule was keyed on the wrong thing — SUPERSEDED by (v): the routing this paragraph introduced was later removed; the shipped fix closes the pending line instead and keeps the note's owner. Read (v) first

, and the measurement that showed it came from chasing the next finding.** (t) narrowed its
predicate to the non-specific tag `!`, recording that a *named* tag `!-` "closes its property and
re-reads correctly with the note inline". That claim came from the wrong evidence: what I had
actually observed was that routing `!-` changed two pinned emissions, and I read "the pins moved"
as "`!-` never needed it". Testing (t)'s own shape with each tag spelling instead — `:` TAB `<tag>`
SP `#-` CR `...` SP `#-`, i.e. the two-note arrangement, not the single-note one the pins hold —
settles today only for `!`; `!-`, `!:`, `!x` and `!!str` all drift. The property is about the line
(a value with no text leaves the scanner pending), so the predicate is now "empty plain scalar
carrying a tag", which is smaller than the whitelist and true of it.

The two characterization tests that recorded the inline `!-` text were rewritten to the routed form
rather than exempted, and the reason is in their own history: the routed output `~:⏎··# -o⏎··!-·` is
precisely what `crash-11ced252`'s test had logged as *round two's* output when it was written. Same
note text, still exactly one note, and one emission now settles. That is a user-visible change to
emitted text and the changelog entry says so with the before/after spelling. Verification:
whitelisting `!` again reddens exactly 3 tests — (t)'s tag loop (reporting
`tag "!-" did not settle`) and both rewritten pins — so three independent assertions now hold the
uniform rule; restored, 493/493 with `--no-fail-fast`, clippy `-D warnings` exit 0,
`--no-default-features` 0, fmt clean, 1781 Python tests, and the four seed corpora (5 / 64 / 6 / 5)
replay with zero artifacts — which is the load-bearing result, since it shows both historical `!-`
shapes still reach a fixed point, now in one round instead of two. Cost: none added (Ir identical to
(t)'s numbers; the condition got simpler). Stale test-name pointers were updated in the three
changelog mirrors that carried them and in one live code comment (`attach_inline_comment`'s doc,
which asserted the old inline behaviour as its own justification); ROADMAP's historical paragraphs
((h), (n)) were deliberately left alone — they record who pinned what at the time, and rewriting
that would falsify the ledger, so the rename is noted here instead.

**What is still open, and why this PR does not claim it:** `crash-cf49fe85`'s sibling from the same
sampler run, `crash-c5b367d3` (68 bytes → minimised 23 — “22” here was recalled rather than
measured, and `wc -c` on the committed seed says 23: `bg: !:` TAB / `<<: #*b` / `::` TAB `!)`),
still drifts. Its note is not a container's inline note but the *merged-in pair's own* leading note
— re-homed by merge consumption away from the consumed `<<:` — so it reaches the writer through a
different path than (t)/(u) route. The same swallowing rule applies (a note line directly after a
text-less value line is eaten), but fixing it means the writer having to place *any* node's leading
note differently when the previous line left a value pending, which is line-level surgery on
already-emitted output rather than choosing a slot before writing. Measured and recorded here; not
attempted at the tail of a long cycle.

### (v) What actually ships: close the pending line, and the note keeps its owner

(t) and (u) both answered the same family by moving the note — (t) for a bare `!`, (u) for every tag
— into the value's leading slot. Chasing (u)'s sibling finding showed the slot was the wrong lever.
A value rendered as nothing but its tag (`k: !`, `k: !-`, `k: !!str`) leaves the scanner mid-value,
so *any* note line written next is reported against that value. `Serializer::pending_tag_insert_at`
now records where such a line ends, and `write_note_line` — the single place a note line is emitted,
including the key-leading-note loop in `write_mapping_pair` that had been hand-rolling `#` — writes
the value's own null text first: `bg: !:` + `# *b` becomes `bg: !: ~` then the note line, so the
note stays on the node the AST says owns it and the first emission is the fixed point. That covers
`crash-cf49fe85` (all five tag spellings, via a loop over `:` TAB `<tag>` SP `#-` CR `...` SP `#-`),
`crash-c5b367d3` (23 bytes minimised: a merge key's note re-homed onto the pair it contributes), and
costs nothing where no note line follows — the ordinary documents are untouched, and the two pins
(u) had rewritten are back at their **original** texts, so (u)'s user-visible output change is fully
undone. The one place the two goals genuinely collide — a container's own note that needs a line
*and* wants to keep its owner — is settled in favour of the fixed point, measured rather than
assumed: the owner-preserving spelling `~: !- ~\n# -o` re-reads with the note on the value, so the
next emission goes inline and the text oscillates. That trade is written at the decision site.

**Three measurement lessons from one change, all of which would have produced confident false
results.** (1) *A mutation that does not apply is worse than no mutation*: my first attribution
script asserted on a pre-`cargo fmt` string, matched 0 times, and the run dutifully reported “494
passed, replay clean” on an **unmutated** tree — a clean bill of health for a fix whose tests were
never tested. The rerun verifies the substitution landed (`grep -q` on both lines) before trusting
anything; it then reddens exactly 3 assertions (the seed test, the tag loop,
`every_note_in_the_seed_corpus_survives_one_emission`) and the corpus replay returns `exit=1` with
one artifact. (2) *The gate caught what no placement test did*: my first cut inserted `" ~"`
unconditionally after the tag, producing `!-  ~` (two spaces) which a re-read normalises to one —
every placement test passed, and only the seed-corpus fixed-point assertion noticed. (3)
*`check_cjk_localisation.py` is narrower than its name*: writing `経路`/`행內` into the Korean entry
passed the gate cleanly; found only by grepping for suspect characters by hand afterwards. Both were
corrected in the Korean entry, and the scan surfaced a pre-existing one in a released section
(`多字节` inside a Korean line, ~L1035) that this PR leaves alone rather than silently mixing
concerns. — **corrected by (w): the pre-existing lines were not left alone, and the blind spot
itself is closed.**

## The quality defence, entry by entry (2026-10-07 →)

Everything below is about the defence matrix rather than an engine defect: what was measured, what
it said, and which belief it retired.

### (w) The gate that missed this is now the gate that catches it (2026-10-07)

Lesson (3) was not a one-off to be reviewed harder; it was structural. `ko` was policed by
`SIMPLIFIED_ONLY`, a hand-curated list of thirteen codepoints, so `経` (Japanese shinjitai) and `內`
(traditional) were invisible by construction — and the Korean changelog had shipped **fifteen
lines** of Korean/Chinese mixed prose, including whole clauses
(`热点样本로 指定。以前 inline `//` 만 计量`, `TOML 深네스팅이`, `소비측은保전`). Measured before
designing: with the list replaced by "no Han at all in `ko` prose" and code exempted **per
codepoint** (fenced blocks, inline spans, link targets), the whole tree produces exactly those
fifteen findings and nothing else — `zh` and `ja` are clean across all 41 of their pages each, and
the exemption is load-bearing, because `docs/ko/contributing/site-i18n.md` legitimately shows
`title: 文档标题` in a ```yaml sample and `{ é: 1, 名: 2 }` as parser input. A line-level exemption
would have reddened both, and a gate that reddes correct text is the failure mode this ledger has
now named three times. Scope went from the changelog's `[Unreleased]` block to every page of every
locale (the old narrow scope was justified by "historical entries stay English" — English cannot
violate a script rule, so it was protecting nothing), the `--full` flag went away with it, and the
fifteen lines are repaired here so the audit mode is green rather than theoretical. Two more facts
about the gate were found while wiring it: it **never ran in CI** (only as a prek hook whose `files`
pattern matched nothing outside three changelog paths), and it had **no test of its own** — both
closed (`Validate` → `script-purity`, `tests/test_cjk_localisation_gate.py`). Attribution: disabling
the `ko` Han rule reddens exactly the four Korean cases and turns the dogfood test green, which is
also the proof that the shipped-docs red came from the rule rather than from the fixture plumbing.
Writing the test found a second-order bug in the test harness itself: loading the gate with
`importlib` without registering it in `sys.modules` makes `@dataclass` fail at class-creation time,
so the module could not even be imported for testing. What is deliberately **not** in the rule, with
the measurement that justifies it: variant-form dictionaries for `zh`/`ja` (Japanese legitimately
writes kanji and Chinese legitimately writes Han, so "forbid a class of glyphs" does not generalise
— only a curated per-character table would catch a wrong variant, and that table is exactly the
artifact that goes stale), and CJK punctuation (`。`, `、`, `「」`, fullwidth parens: 21 marks in
Korean prose plus 58 fullwidth parens across 32 Korean lines — a house typography habit, not a
writing-system intrusion). The `「」` and one `。` inside the lines I was already editing were
fixed; the rest is recorded here rather than silently rewritten.

### (x) The sampler surfaced two bugs and the harness reported one (2026-10-07)

The push-to-main fuzz runs had been red since #278 (`37502883147`, `37516581231`, `37518466671`,
`37533230933`), each saying `surfaced 1 distinct crash signature`. Two inputs were in the uploaded
artifact, and they are two root causes: `crash-55c199ef` (25 bytes) and `crash-5561902a` (88 bytes).
`scripts/fuzz_rounds.sh` derives a signature with
`grep -m1 -hoE "panicked at [^:]+|assertion [^ ]+ failed|SUMMARY: libFuzzer: .*"`, and every
idempotence failure in this harness reaches the same `assert_eq!` at `yaml_roundtrip.rs:37`, so
distinct bugs collapse to one key — and the `break` on a repeated key stops the loop early. Read the
job's count as a lower bound, never as a bug tally. Worth its own change (key the dedup on the input
bytes as well, and stop `rm -rf artifacts/$target` before the upload), and deliberately not folded
into a serializer fix.

**The closed one: the compact dash line hand-rolls `key: value` and skipped every note slot.**
`write_sequence_item`'s compact branch copies the pair line itself rather than calling
`write_mapping_pair`, and it had copied only the *text*. Three slots were therefore unreachable
under a `- `: the item's own leading stack, a later pair's leading stack, and a simple key's inline
note. Established by holding variables still, not by reading the diff: `a: !   # n` at a document
root keeps its note, `- a: !   # n` emits `- a: ! `; `"a": !   # n` and `a: !!str   # n` behave
identically to their unquoted forms, so the tag-only value and the quoted key were both innocent.
`crash-55c199ef` reaches the same site one round late — its first emission owns the note through a
node the writer does read, and the re-read hands it to the key, which is why the failure looked like
drift rather than loss. Fixed in the same place the loss was: stacks above the dash, a later stack
at the pair indent, the inline note on its own pair line. Items that were already right are
byte-identical.

**One speculative branch was written and then deleted by experiment.** The first cut also hoisted a
*first key's* leading stack. Twelve crafted inputs (`- # z` / `-` CR / nested under a mapping key /
with an anchor / with a complex key, in both indent spellings) show the reader never gives a leading
note to the first key of a compact item — it lands on the item or on the enclosing sequence — so the
branch was unreachable, untestable, and unattributable. Removed; the comment now records the
measurement instead of the code. Attribution on the three surviving sites is disjoint: withdrawing
each reddens exactly its own test (`a_leading_note_on_a_compact_dash_item_survives`,
`a_note_above_a_later_compact_dash_key_survives`, and the inline slot's three: the shape test, the
multi-pair test, and the seed test on `former-crash-55c199ef.seed`). 500/500 nextest with
`--no-fail-fast` — which includes the committed-corpus gate
`every_note_in_the_seed_corpus_survives_one_emission`, so the 66 `yaml_roundtrip` seeds (the new one
among them) all survive one emission with every note the reader recorded — clippy `-D warnings` exit
0, `--no-default-features` clean, 1794 Python tests. The four targets' deterministic `-runs=0`
replay (`parse_yaml` 5 / `yaml_roundtrip` 66 / `parse_json` 6 / `parse_toml` 5) is this PR's fuzz
job, not a local claim.

**Still open, filed with the artifact kept:** `crash-5561902a` (88 bytes). Its notes move between
column 0 and indent 2 across rounds around a tag-only line nested under stacked complex keys (`?`
chains), which is the *placement* family, not this *slot* family — nothing is lost, the line it sits
on changes, and it settles on the second round. Fixing it needs the writer to know which column the
reader will attribute a note line to, which is a different decision site. — **CLOSED by (y), and the
diagnosis above was wrong: it was neither placement nor the (v) family's ownership conflict, and it
lost nothing on the first round either.**

### (y) `crash-5561902a` closed: an absolute offset into already-written output was never shifted

Minimised locally first, because this input still failed and could therefore be shrunk at all: a
ddmin loop over byte-ranges driven by the *harness's own oracle* (parse, emit, re-parse, emit,
differ?) took 88 bytes to **15** in 117 oracle calls, and ran in a release build in seconds where
the first byte-at-a-time cut in debug had run five minutes without converging (`b: ! #&` LF `#e` LF
`? #!`). The trees tell the whole story: `tree0` holds the note `!` on the second pair's **key**,
`tree1` holds it as a second **leading note on the first pair's value**, and the text moves from
column 0 to indent 2. The cause was in (v)'s own machinery — `close_pending_tag_line` remembers the
tag-only line by `pending_tag_insert_at`, an absolute index, while `write_mapping_pair`'s key-note
slot inserts `# …` at an *earlier* index and never moved it. The distance check then counted
newlines from the wrong place, concluded the pending line was no longer adjacent, and left the value
open — so the next round's note line was swallowed exactly the way (t) described, and the closure
that was supposed to prevent it silently did nothing.

**Two things this corrects.** (1) On (v)'s "ownership is sacrificed to the fixed point": re-measured
on this tree, the shipped spelling settles — `~: !- ~` + a note line emits `~: !- ~  # -o` and holds
for three rounds, as does `~: !-` + a note line. That does **not** refute (v), because its "mutually
exclusive" sentence describes the variant it rejected, and this probe did not rebuild that variant
to re-test it. What is new is that the (y) shape is a third case, measured with the trees rather
than inferred: there the note keeps its owner *and* round 1 is the fixed point, so the two goals do
not conflict once the marker is truthful. (2) Last cycle's cost claim ("+10 instructions") was
partly cross-session. Measured properly — HEAD and candidate built in the same session, only
`serializer.rs` swapped — nine scenarios agree to within 1,283 instructions on a 355.8M one
(<0.0004%), three exactly. The reason the earlier number could not be trusted: **a comment-only edit
swung `serialize_block_scalars` by 1.46% between sessions** (15,702,957 → 15,931,029) on
functionally identical code. So a single Ir reading is not comparable across sessions; only a
same-session A/B is, and the harness now does that by default.

**Attribution**: withdrawing the shift (`*pending += text.len()` → `+= 0`, verified by full-text
equality, restored by hash) reddens
`a_key_note_inserted_before_a_pending_tag_line_keeps_the_closure`
*and* `every_note_in_the_seed_corpus_survives_one_emission`, so the committed-corpus gate
independently guards it. Seeded as `former-crash-5561902a.seed` (15 bytes, the minimised form).
Landed as 501/501 nextest with `--no-fail-fast`, clippy `-D warnings` exit 0,
`--no-default-features` clean, 1794 Python tests, `ir_gate.py` within tolerance; this went in
as #283, whose fuzz jobs ran the deterministic `-runs=0` replay of all four corpora (`parse_yaml` 5
/ `yaml_roundtrip` 67 / `parse_json` 6 / `parse_toml` 5) green on the PR itself.

### (z) The sampler's counting is fixed, and the sampler now has tests of its own (2026-10-07)

Three defects, all measured rather than suspected, all in code that decides whether a merge is
allowed:

- the round loop stopped on a repeated panic signature **and skipped the copy** in that branch, so a
  second input reaching the same assertion was neither collected nor reported — `crash-55c199ef` and
  `crash-5561902a` (two root causes) came back as "1 distinct crash signature". The bytes survived
  in that run only because the `break` happened before the next round's `rm -rf artifacts/`; one
  more round and the second finding would have been deleted.
- the aggregate report read `$d/.signatures` at the archive root, but the file is written under
  `collected/<target>/` (and dotfiles are not what the extraction put there), so the *distinct
  signatures* column has been printing 0 since it was added.
- the same report counted `crash-*` **paths**, and the artifact carries both `artifacts/<target>/`
  and `collected/<target>/`, so it reported 3 inputs for 2 (measured on run `37533230933`).

Now: every artifact of every round is archived before anything else can happen, deduplicated by file
name; the loop continues while it finds new bytes and stops when a round adds nothing; findings land
in `collected/<target>/findings.tsv` (`signature ⇥ artifact`) which the report reads wherever it was
extracted; inputs and signatures are counted separately and the summary states that a signature is a
lower bound, not a tally. Exit codes keep their meanings (0 clean / 1 findings / 2 budget-arithmetic
config error).

**The harness is tested, which it had never been**: `tests/test_fuzz_rounds.py` drives
`fuzz_rounds.sh` against a `cargo` stub replaying a per-round plan, asserting the PR-mode `-runs=0`
single invocation, the corpus-entry removal, the clean-stop, the config-error exit, and the two
counting behaviours. It is Linux-only by design — the script *is* a POSIX shell harness and CI's
ubuntu matrix runs it; the Windows `bash.exe` is the WSL launcher and eats the positional arguments
a `-c` script needs (measured `ARG=[]`), so a platform guard replaced a pile of drive-letter
guessing. Attribution on the harness itself: against the committed script, exactly the two counting
tests fail, and the failure output *is* the CI text —
`round 2: re-found a known signature, stopping` and `surfaced 1 distinct crash signature(s)`.
Against the fixed script: 6 passed.

No CHANGELOG entry: the tier's output is not a shipped behaviour, and a finding nobody has triaged
must not gate an unrelated PR — same reasoning that kept #266's and #275's harness changes out of
the changelog.

**Filed by the run that motivated this change, not fixed here.** Two inputs the sampler found on top
of #283 (`6bb8446f`), both re-measured on that commit and still drifting, neither losing a note (the
shifts are placement, not data):

- `crash-7eb273bc` (24 bytes, `-{TAB}?: !` CR `#U` CR CR `... #` TAB `! - *:`): the emission puts
  the note inline on the tag-only line (`    !   # ! - *:`), the re-read hands that note back as the
  value's **leading** note, and the second emission writes it on its own line above the tag. Same
  neighborhood as #283's key-note slot, different owner — here the note is on the value and the
  drift is inline ⇄ leading.
- `crash-d0745105` (64 bytes): a merge key carrying a note (`<<: #*b`), whose note is re-homed by
  merge consumption and then emitted on its own line at a different indent, dragging the following
  `::` pair with it.

Both fail the same `assert_eq!` at `yaml_roundtrip.rs:37`, so even with this counting fix they will
read as "2 inputs / 1 signature". That is the point of the wording change: the honest statement of
what one run found, rather than a number that looks like a root-cause count and is not.

### (aa) The backlog, triaged by measurement — including one candidate fix already falsified (2026-10-07)

Minimising every artifact still on disk against the current tree (ddmin over byte ranges, harness
oracle, release build) collapses five unfixed inputs into **three shapes**, each of which reaches
its fixed point on the *second* round:

- `: !` CR `#U` CR `... #-` — 13 bytes, from `crash-7eb273bc` (24) and `crash-9733643a` (27). The
  document's trailing note rides the tag-only line (`!   # -`); re-read, the reader files it as the
  value's **leading** note, and the writer emits leading notes as lines, so r2 is `# -` above
  `  ! `.
- `?` / `-` / `#?` / ` ? ` — 10 bytes, from `crash-1445c91a` (54) and `crash-f1643b2d` (55), which
  minimise to the *same* input. The note belongs to the mapping inside a complex-key body and is
  written at that body's indent; re-read hands it to the enclosing sequence, and r2 emits it above
  the `?` marker.
- `<<:` / `<<: #b` / `:` — 15 bytes, from `crash-d0745105` (64). Merge consumption re-homes the
  merge key's inline note onto the folded `~` key, and r2 additionally collapses the pair's indent.

**Falsified by measurement: "a pending line's trailing slot is not open."** `write_scalar_node`
declares the slot open after a tag-only scalar, so the obvious fix was to withhold it when the line
is pending and send the note to a line of its own. It does not settle: `write_note_line` closes the
pending line first (that is its whole purpose), and a comment after a *closed* `! ~` is read as the
value's **inline** note — so `! ~` + note line emits `! ~  # -` on the next round. Two spellings are
self-consistent (unclosed tag line with the note as a line *above* it; closed tag line with the note
*inline*), and this writer produces neither for this shape. The change also reddens five pinned
tests — `a_containers_inline_note_after_a_text_less_value_settles_at_once`,
`a_note_after_a_tag_only_value_settles_in_one_round`,
`a_quoted_hash_key_settles_the_containers_note_at_once`,
`a_note_on_a_dash_key_from_the_ci_artifact_survives_one_round`,
`every_note_in_the_seed_corpus_survives_one_emission` — all of which exist to hold "the container's
note rides the pair line" as the convention. Reverted; the fix has to place the note line **before**
the tag-only line already emitted, which is line-level surgery on output and the same class (u)
declined at the tail of a long cycle. — **Partly superseded by (ab): the withhold-the-slot version
really is wrong, but "that needs line-level surgery, so not at this tail" was too cautious — the
surgery is three lines, and doing it exposed that the spelling this paragraph defends as stable is
not ownership-neutral: the note lands on the key.**

### (ab) Shape A closed — and the old "stable" spelling was losing its owner

`write_scalar_node`'s slot *is* correctly open after a tag-only scalar, so (aa)'s first idea fails
exactly as recorded. That dead end pointed at the real question, which was never "line or inline"
but **which node the reader will hand the note to**. Measured on the candidate placements for shape
A (`: !` CR `#U` CR `... #-`, 13 bytes):

| placement | first emission is the fixed point? | who owns the note afterwards |
| --- | --- | --- |
| inline on the pending tag line (what had shipped) | yes | the **key** — the container lost it |
| note line below the block | no (round 2 needed) | the value |
| close the line, then a note line | no (the next emission pulls the note back onto the line) | the value |
| **note line above the pair (this change)** | **yes** | **the container** |

The spelling earlier fixes pinned as "the stable one" bought stability by silently re-parenting:
`~: !-   # -o` and `"+#": !-   # -o` both re-read with `inline="-o"` on the key. Above the pair is
the first placement with both properties, so those two pins were rewritten rather than kept — their
emitted text changes, and each now also asserts the ownership the old assertion let slip. Shape A's
own inputs go from "settles in two rounds" to "settles in one", with the note on the container
instead of migrating to the value.

Mechanically: `Serializer::pair_line_start` records where the current pair or item began, and
`note_line_above_last_pair` inserts the note line there — through `insert_note_at`, so the pending
marker is shifted rather than left stale (an untracked `insert_str` here would reintroduce #283's
bug). `pending_tag_insert_at` is deliberately *not* cleared: after the insert the tag line is still
the last line and still pending, and the parent container's note has to see that.

**Cost, same-session A/B** (only `serializer.rs` swapped, both force-rebuilt): the serialize
scenarios pay one store per pair — `serialize_small` +4,000 / 5.91 M (+0.068%), `serialize_medium`
+32,025 / 43.3 M (+0.074%), `serialize_block` +14,005, `serialize_block_scalars` +2,000,
`serialize_anchors` +18,000 (+0.044%); the four parse scenarios move 117–4,217 out of 49.5 M–355.8
M. Nine of nine inside the 2% gate, and stated rather than hidden: this is the first fix in the
family that adds per-pair work to the hot path.

**Attribution**: forcing `pending_line_is_last()` to `false` reddens exactly
`a_note_above_a_pending_tag_line_keeps_its_owner`,
`a_containers_inline_note_after_a_text_less_value_settles_at_once`,
`a_quoted_hash_key_settles_the_containers_note_at_once` and
`every_note_in_the_seed_corpus_survives_one_emission` — the corpus gate again, so the committed
seeds hold the behaviour independently. 502/502 nextest, clippy `-D warnings` 0, 1794 Python tests,
two seeds added (`former-crash-7eb273bc.seed`, `former-crash-9733643a.seed`; 69 in
`yaml_roundtrip`).

**Environment trap, recorded so nobody re-learns it:** `cargo test --all --no-default-features`
fails in WSL with `unable to find library -lpython3.12` — the image has `libpython3.12.so.1` but not
the unversioned symlink, so the binding crate cannot link even with the python feature off. Not a
code failure: the same command is exit 0 on Windows and in CI.

One more reason the unit of record is the shape, not the count: two of these inputs minimise to
identical bytes, so "distinct inputs" (the number #284 added) and "distinct bugs" are different
things — the CI job can honestly report the former, and only this ledger can hold the latter.

### (ac) Three writer fixed-point properties fail at 20k cases, and CI never sees them (2026-10-07)

Chasing the Linux-only `pbt::tests::prop_mapping_order_preserved` failure into a deterministic repro
led to running the property suite harder than its default:
`PROPTEST_CASES=20000 cargo nextest run -p pyrs-yaml-core --lib -E "test(~fmt_pbt::)" --no-fail-fast` fails **three** properties —

- `fmt_pbt::prop_json_writers_are_fixed_points`
- `fmt_pbt::prop_json5_writer_is_fixed_point`
- `fmt_pbt::prop_toml_writer_is_fixed_point`

All three are emission-not-idempotent under the hub, i.e. exactly the invariant the fuzz tier
asserts, in the formats the objective names alongside YAML. They are **not** from any recent change:
replacing `serializer.rs` with `main`'s version in the same session, same case count, reproduces the
identical three (blame script, restore verified by hash). CI's default is proptest's 256 cases,
which is why this has been green the whole time — so the finding is two-fold: the properties fail,
and the property tier is under-powered. Do not just raise `PROPTEST_CASES` in CI: that reddens the
line with three unfixed bugs; raise it as they close, one root cause per PR, and keep the seeds that
fall out of them.

The YAML side of the same hunt is closed, and it had **two** sites: an empty block container as a
sequence item emitted `-` then `{}`, which re-read as a flow node and inlined on the next round.
`an_empty_block_container_is_inlined_on_the_dash_line` pins the node writer from the AST (no text
input produces that tree), with the note and anchor variants that had to take the same route; the
Python-object fast path had the identical defect at its own site — `direct_dump`, which mirrors
`is_compact_item` by design rather than sharing it — so
`tests/test_safe_dump.py::test_empty_containers_are_inlined_under_a_dash` pins
`safe_dump([{}]) == "- {}\n"` and the re-dump fixed point for five shapes. That second site is the
one users actually hit, and it was found only by asking the built extension: the Rust test passing
did not mean `safe_dump` had stopped moving.

Ir for this fix is a wash — same-session A/B, only `serializer.rs` swapped: the largest movement
over nine scenarios is 9,027 instructions out of 340.8 M (+0.003%), signs mixed, so the extra
`is_empty_container` test costs nothing measurable. Against that, `PROPTEST_CASES=20000` also showed
the YAML mapping-order property holding at 80x CI's case count once this is fixed, which is what
makes the three `fmt_pbt` failures above stand out rather than hide.

### (ad) Shape B closed — a note riding the first item of a sequence-shaped key belongs to the marker line (2026-10-07)

Re-measured on the post-#286/#287 tree, the 10-byte input `?` / `-` / `#?` / ` ? ` still needed two
rounds, and the tree says exactly who held the note: the reader puts it on the **inner mapping**
(`items[0]`), the writer emits it above the `-` at the body indent, granit reports that line on the
**sequence**, and the next emission then hoists it above the `?` marker — so one level is climbed
per round until the enclosing mapping absorbs it. `hoist_marker_note` already walked that last step
for a *mapping* key (the first-pair-key spine, (k)/(m)), but `write_mapping_pair` gated the lift on
`CustomNode::Mapping { flow_style: false }`, so a block sequence key never got it. Now the gate
covers both containers and the walk has a sequence arm: take the first item's stack, keep going into
that item, outer before inner.

**What this does not claim, stated because the ledger asked for it in (ab).** The chosen placement
is where reader and writer agree, not where the note started: after one emission the note is owned
by the *enclosing mapping*, not by the item that carried it. No spelling keeps it on the item — a
comment line above a `-` is reported on the sequence by the reader, full stop — so the honest
property delivered here is "survives, and settles in one round", with ownership normalised to the
level both sides agree on.

**Measured blast radius, three ways.** (1) The 15 crash artifacts still on disk: main drifts on 3,
this tree on 1, cured exactly `crash-1445c91a` + `crash-f1643b2d`, newly broken 0. (2) The committed
seeds plus the local corpus, 3,842 inputs: main drifts on 2, this tree on 1, newly broken 0. (3) A
14-shape hand-written zoo around sequence keys carrying notes (stacked notes, a note deep on the
item's own marker spine, a nested sequence, an empty-container item, a note on the *second* item, a
note on a sequence in **value** context, a flow key, and the two backlog shapes): 13 parse — the
flow-key one is rejected on ingest, the known open item on flow member comments — main drifts on 3
(`shape_b`, `shape_c`, and `? / - #a / #b / x: 1`, i.e. stacked notes were also climbing), this tree
drifts on `shape_c` alone. Four of those shapes are pinned as assertions in the new test, including
the two controls that must not move: a note above a **second** item stays inside the body, and a
sequence in value context keeps its item's note above the dash where the item path writes it.

**Cost.** Same-session Ir A/B, only `serializer.rs` swapped: parse is unmoved (`parse_medium` +2,074
/ 355.8 M, `parse_block` −3,218 / 146.1 M), and the serialize scenarios move +4,000 to +25,937
(`serialize_small` 5,912,457 → 5,916,457 = +0.068%, `serialize_medium` +0.060%, `serialize_anchors`
+0.064%). None of the nine scenarios contains a `?` marker — `bench_inputs.rs` has zero lines
starting with one — so that residual is codegen (a bigger `write_mapping_pair`, one more match arm
in the hoist), not work executed per pair, and it is reported rather than explained away. Nine of
nine inside the 2% gate.

**Attribution.** Dropping the `Sequence` alternative from the gate reddens exactly two tests: the
new `a_note_on_the_first_item_of_a_sequence_key_adopts_the_marker_line` and
`every_note_in_the_seed_corpus_survives_one_emission` — the second because the shape is now a
committed seed (`former-crash-1445c91a.seed`: the 10 minimised bytes this ledger recorded,
hex-checked against it, and shown by the same A/B to drift on main while settling here), so the
corpus gate holds this behaviour independently of the unit test. 303/303 nextest for the crate on
restore.

**Filed, not fixed, by the same sweep.** `crash-d0745105` / shape C — 15 bytes, `<<:` + LF + one
space + `<<: #b` + LF + two spaces + `:` — is the one remaining backlog input and the only drift
left in the zoo. — **CLOSED the same day by (ae), and the classification above was wrong twice over:
it was not a placement defect, and the rounds disagreed about the data, not the indentation.** And
one committed-corpus input drifts on both sides and is unrelated to this family:
`fuzz/corpus/corpus-local-backup/**/parse_yaml/67007c20fc404c4cb4d9a779167c3ff9f1c39f0a` (14 bytes,
`-` TAB `:` CR `#A#` CR `-` TAB `#A#` CR), a tab-indented dash with a note after a CR-terminated
value — filed as a fourth shape for the next sampling round.

### (ae) Shape C was not a placement bug — a nested merge key was being discarded, and the object view proved it (2026-10-07)

The backlog filed `crash-d0745105` as "merge key's note changes owner + the indent flattens".
Measured through the binding, the two rounds did not merely disagree about a line: they disagreed
about the **data** — `{'<<': {'~': None}}` after one parse, `{'~': None}` after the next. Reading
the collectors first (rather than inferring from the symptom, which this ledger has paid for four
times) found the rule: `collect_inline_pairs` and `collect_merged_pairs_for_anchor` skip any source
key the target "already owns" by asking `IndexMap::contains_key`, which compares **whole nodes** —
and the target's own `<<` entry is still in the map at that moment. So a `<<` inside a merge source
always matched, was skipped, and the source contributed nothing. Because that comparison includes
the key's metadata, a *comment* was enough to change the answer: the noted nested `<<` did not
match, so one level was consumed — which is the only reason the fuzz tier saw this shape at all.

**The compliance reading, measured against both reference libraries (`.cache/crosslib_merge2.py`),
not argued:**

| input | engine before | engine after | PyYAML / ruamel |
| --- | --- | --- | --- |
| `<<: {<<: {x: 1}}` | `{'<<': {'<<': {'x': 1}}}` | `{'x': 1}` | `{'x': 1}` |
| `<<:` first line, an indented `<<:` second, `x: 1` indented further | two literal `<<` levels | `{'x': 1}` | `{'x': 1}` |
| three markers deep | three literal levels | `{'x': 1}` | `{'x': 1}` |
| `<<: {<<: {x: 1, y: 1}, y: 2}` | `{'y': 2}` — `x` lost | `{'x': 1, 'y': 2}` | `{'x': 1, 'y': 2}` |
| `<<:` whose source is a sequence item that carries its own `<<:` | `{'y': 2}` — `x` lost | `{'x': 1, 'y': 2}` | `{'x': 1, 'y': 2}` |
| shape C, with and without the note | drift, and a different dict per round | both `{'~': None}`, one round | ruamel `{None: None}`; PyYAML rejects the input |

So this was never "a different but equally valid spelling": keys the document carried were missing
from the object view. The round-trip tier cannot see that class — its oracle is text equality, and
an under-resolved document is perfectly stable — and the cross-library compliance tier had no
nested-merge case either, so the defect was invisible to both tiers while the note-dependent
spelling leaked into fuzz.

**Fix.** Resolve the source before collecting from it: clone it, run `resolve_merges_recursive` at
`depth + 1`, collect from the resolved copy. The nested merge is then applied *inside the source*,
which also gives the precedence both reference libraries show (`<<: {<<: {x: 1}, x: 9}` is `x: 9`,
because the source's own key overrides there before any pair travels up). The guard
`source_carries_merge_key` looks only at the source's top level: a `<<` deeper inside a *value* is
reached by the ordinary walk, measured on `<<: {a: {<<: {x: 1}}}` and `<<: {<<: {a: {<<: {x: 1}}}}`
(both already correct, and both still correct with the narrow guard) — a whole-subtree scan would
have cost something for nothing.

**Battery, swapping only `merge.rs`.** Committed seeds + local corpus (3,843 inputs): 2 drift
before, 1 after, cured `former-crash-d0745105.seed`, newly broken 0, and no drift *text* changed
anywhere. The 15 crash artifacts still on disk: 1 → 0, i.e. **every surviving artifact now reaches a
fixed point in one round**. Merge zoo of 18 hand-written shapes (nestings, sequence sources,
anchor/alias sources, degenerate `<<: []`, `<<: 5`, `<<:`, quoted lookalikes, self-referential): 1 →
0. Sequence-key zoo of 13: 1 → 0.

**Attribution.** Keeping the clone but dropping the resolve call (a revert that still compiles — the
crate denies `dead_code`, so deleting the helper fails the build instead of the tests) reddens
exactly `a_merge_source_that_itself_merges_contributes_its_nested_pairs`,
`a_nested_merge_adds_the_keys_the_source_does_not_name`,
`a_nested_merge_key_carrying_a_note_means_the_same_thing` and
`every_note_in_the_seed_corpus_survives_one_emission`; the Python side is pinned by
`tests/test_gaps.py::TestNestedMergeSourceIsApplied`. 508/508 on restore.

**Cost, and what the gate cannot see.** Same-session Ir A/B, nine scenarios: largest movement
+27,511 out of 340.8 M (`parse_anchors`, +0.008%), `serialize_*` within ±25 instructions. Stated
rather than implied-green: `bench_inputs.rs` contains `<<` only as `<<: *defaults` — an **alias**
value, which never enters the changed branch — so the committed gate does not cover this path at
all. The work added on that path is one top-level `is_merge_key` scan per inline mapping/sequence
merge value, plus a clone only when a nested `<<` exists. Putting an inline-merge scenario into the
baseline is part of the pillar-four queue below.

**Filed by the same measurement, deliberately not fixed here.** (1) The **alias** source is still
under-resolved, at a different site: `a: &A {<<: {x: 1}, y: 2}` + `<<: *A` contributes `y: 2` while
PyYAML contributes `x: 1` too, because `collect_anchor_mappings` snapshots each anchor body
*before* any resolution and the collector reads that stale copy. Fixing it needs two decisions this
PR should not make in passing: how to thread the cycle `path` when an expansion recurses into its
own anchor (#166 territory), and whether the source's direct keys should override nested
contributions by *value* while keeping the merged pair's *position*. One root cause, one PR — next
one. **CLOSED the same day by (af)**, which measured the chain shapes and took those two decisions
explicitly. (2) **Null-valued keys are not resolved**: `~: 1`, `null: 1`, `Null: 1` all yield string
keys in `safe_load` / `to_dict` / `to_json` where PyYAML yields `{None: …}` (measured table in
`.cache/nullkey3.txt`); null *values* are fine. Pre-existing, unrelated to this change, and a
compliance gap worth its own investigation. (3) `67007c20…` from (ad) still drifts on both sides.

### (af) The same discard at the alias site — and the template chains users actually write (2026-10-07)

(ae) shipped with the alias source left open on purpose; this closes it. `collect_anchor_mappings`
snapshots every anchor body *before* any resolution runs, and `collect_merged_pairs_for_anchor`
reads that snapshot, so a `<<` inside the body met the same whole-node ownership test and was
skipped. Measured object views, engine vs PyYAML: a two-level template chain `base: &b {x: 1}` /
`mid: &m {<<: *b, y: 2}` / `use: {<<: *m, z: 3}` gave `use = {'y': 2, 'z': 3}` where PyYAML gives
`{'x': 1, 'y': 2, 'z': 3}`; three levels (`a: &A {p: 1}` / `b: &B {<<: *A, q: 2}` /
`c: &C {<<: *B, r: 3}` / `<<: *C`) gave `c = {'q': 2, 'r': 3}` and a top level carrying only `r`,
where PyYAML resolves `p`, `q` and `r` in both. Every one of those documents is *stable*, so the
round-trip tier - the oracle that has driven every fix in this family - cannot see the class at all.
It is the everyday GitLab-CI / Ansible shape, which is what makes it worth the second PR.

**Design decisions, and the alternatives that were rejected by reading the code.** Resolve the body
where it is read, not where it is snapshotted, for two reasons. (1) A
snapshot-then-resolve-everything scheme cannot work here: the ownership of the damage is visible in
`resolve_mapping_merges` itself - an `<<` whose value references an alias is *consumed even when it
contributes nothing* (`references_alias`), so by the time a second pass would look, the merge entry
is gone and the keys are unrecoverable. (2) Memoising resolved bodies per anchor name would be
cheaper but wrong: the cycle guard is path-sensitive, and a body resolved under an empty path
expands one level further than the same body resolved while its own anchor is open - which is how
`a: &A {<<: *A}` terminates today (#166). So the walk happens at the call site, with the anchor's
own name pushed onto `path`, and `depth` is threaded through `collect_merge_data` into the collector
rather than restarted, because `MAX_MERGE_DEPTH` is a *total* budget and resetting it per expansion
would silently multiply it.

**Precedence falls out of the placement, which is the point.** Resolving inside the source means a
chain's own key beats what that chain inherits, and the document's own key beats the chain - one
override per level, which is exactly what PyYAML and ruamel produce (`b: &B {<<: {x: 1}, y: 2}` +
`<<: *B` + `x: 9` gives `x: 9` at the top and `{'x': 1, 'y': 2}` in `b`). Pinning that as an
assertion, rather than accepting whatever order a fixed-point loop produced, is what keeps the next
refactor honest.

**Battery.** Because the defect never moved text, the A/B is a *non-regression* check, and it is
reported as such: swapping only `merge.rs` against `main` (which already contains (ae)), committed
seeds + local corpus 3,843 inputs, 15 crash artifacts, a 22-shape merge zoo (now including the
chains and the sequence-of-aliases form), and the 13-shape sequence-key zoo - **drift count
identical on both sides (1 / 0 / 0 / 0), newly broken 0, and not one drift text changed**. The
evidence for the fix itself is the view table plus two new Rust tests
(`an_anchor_that_itself_merges_contributes_the_inherited_keys`,
`a_three_level_anchor_chain_resolves_to_its_deepest_keys`) and `TestNestedMergeSourceIsApplied`'s
two new Python methods, which assert the resolved document rather than its spelling.

**Attribution.** Neutralising the resolve call - while keeping `resolved`, `nested` and `depth`
used, because the crate denies warnings and a mutant that does not compile is not evidence - reddens
exactly those two Rust tests and nothing else (303/305). First two mutant cuts failed for exactly
that reason (`dead_code`, then `variable does not need to be mutable`), which is why the third
mutates the call and not the bindings.

**Cost, on a path the gate does cover.** Same-session Ir A/B, nine scenarios: `parse_anchors`
+232,527 of 340.8 M (**+0.068%**), `serialize_anchors` +335, every other scenario within ±415.
Unlike (ae)'s changed branch, this one runs for every `<<: *alias`, so the committed gate sees it -
which is the point of keeping the two PRs separate rather than claiming one wash for both. The added
work is one top-level `is_merge_key` scan per alias merge, and the clone only where the body really
does carry a `<<`. Precomputing that flag per anchor name was considered and left alone at 0.07%: it
is a cache invalidation problem in disguise (the answer is path-sensitive for the *expansion*, if
not for the flag), and the number does not justify it yet.

**Queue consequence.** The cross-library compliance tier has no nested-merge or template-chain case,
which is why two PRs' worth of data loss surfaced only through a hand-measured table. Adding these
shapes to `tests/test_yaml_suite.py`'s parity scope (or a dedicated `tests/test_merge_parity.py`
with PyYAML/ruamel as the reference) is the standing gate this family needs; it is filed with the
pillar-three items.

### (ag) The merge family got the standing gate it was missing, and its discriminating power was measured before trusting it (2026-10-07)

(af) closed two root causes of merge *data loss* and named the reason neither was caught earlier:
the round-trip oracle passes for a document that is stable and merely under-resolved, and the
cross-library tier had no merge case beyond the plain alias form. `tests/test_merge_parity.py` is
that gate: 17 shapes (nested inline/block/three-deep, per-level precedence, `<<` inside a value,
two- and three-level anchor chains, sequence sources, alias+inline mixtures, quoted lookalikes)
asserted equal against **both** PyYAML and ruamel on the resolved document, plus a one-round fixed
point over every shape including the divergent ones. Comparing `dict`s means insertion order is out
of the assertion by construction, so the engine's documented document-order posture cannot make it
flake — the property under test is *which pairs exist*, not where they sit.

**A gate that stays green against a known-bad engine is decoration, so this one was tested against
the bugs it exists to catch.** Reverting both merge resolutions at once (two mutants, each verified
to compile — the crate denies warnings, so the first cut of each failed on `dead_code` /
`unused_mut` before it proved anything) and rebuilding the extension: **14 parametrised cases fail,
across `test_merge_resolution_matches_pyyaml` and `test_merge_resolution_matches_ruamel` alike,
while `test_every_merge_shape_settles_in_one_round` stays green** — the measured proof that this
class is invisible to the fuzz oracle and visible here. Restoring both fixes: 59 passed.

**Four shapes are listed as deliberate divergences rather than hidden** — `<<: []` and `<<: {}`
(loaders answer with an empty mapping; the engine keeps the user's key), and `<<: 5` / bare `<<:`
(loaders raise `ConstructorError`; the engine treats it as data and re-emits it). They are pinned at
their actual values with the reason in the file, which is what keeps a future "fix for parity" from
deleting text without an argument. **What the set does not cover, stated because a green gate
invites trust:** null-*valued* keys (`~: 1`, `null: 1`) still disagree with PyYAML — filed in (ae),
unresolved — so those shapes are deliberately absent until that closes, and they belong in this file
afterwards.

No changelog entry: the gate is not shipped behaviour, the same rule that kept #284's harness
and #275's budget changes out of the release notes.

### (ah) The defence itself got measured, and the measurement is a gate (2026-10-07)

The question asked was not "find the next crash" but whether thirty-odd root-cause closures
constitute a *system* or a habit: are unit / property / fuzz orthogonal, does the gate set cover the
critical path, do fixes reach root causes, does a closed defect keep itself closed. Answer, from the
repository rather than from recollection: **the method is systematic, the defence is reactive** —
every gate here was built after a defect walked past the gates that existed then, and nothing
measured the coverage of the defence itself. So `scripts/quality_matrix.py` re-derives the picture
from the declaring files (workflows, `prek.toml`, `fuzz/Cargo.toml`, `scripts/check_*.py`, the Ir
bench, `.ci/ir-baseline.json`), `.ci/quality-holes.json` registers what it finds with a dated exit
criterion each, and `tests/test_quality_matrix.py` fails in **both** directions: an undeclared blind
spot cannot land, and a hole that closed cannot stay registered. `QUALITY_MATRIX.md` carries the
assessment and must name exactly the registered ids. Six blind spots came out of the measurement,
none of them reasoned: no CI job ran the hook set (`ci.yml` merely watches `prek.toml` as a path,
and no workflow invoked `prek` at all — the condition that let 15 tracked files reach `main` as
CRLF, 12,263 lines), `hook-skipped-uncovered:cargo-fmt` (`cargo fmt --check` was in no workflow
either), `lint-scope:clippy-all-targets`, `property-tier:default-case-count`,
`fuzz-no-roundtrip:pyrs-json` / `:pyrs-toml` (a parse-only target cannot see a writer), and
`route-parity:node-writer-vs-direct-dump` — the two YAML writers that had to be fixed twice
for #287, with no table pinning them together. Two of the six closed inside this same change:
`hygiene.yml` now runs the hook set including `cargo fmt`, and CI's clippy got `--all-targets` after
measuring that the wider command is already clean on the whole tree. That is why they are absent
from the registry, and the registry check is what makes such an absence meaningful rather than a
silent deletion.

**The instrument was wrong four times before it was believable, and each wrong reading is now a
test.** Matching `^pub fn to_json\b` missed `to_json_text` / `to_json5_text`, so `pyrs-json` looked
like a crate without a writer and the JSON round-trip hole would have been reported as absent — a
false clean, the worst available outcome. Inferring "engine crates" from exported function names
also dropped `pyrs-json` and `pyrs-toml` entirely; the denominator is now `fuzz/Cargo.toml`'s own
dependency list, which is what the fuzz tier is actually responsible for. Matching the Ir scenario
tuples as `("name",` lost `serialize_block_scalars` to a line wrap and reported a *stale baseline*
that did not exist. And the reachability probe first searched whole workflow text, so a hook named
in an explanatory comment counted as wired — the very mistake being catalogued; it now reads only
`run:` steps (block scalars included) and strips quoted spans, since `echo "cargo clippy"` lints
nothing. Separately, `SKIP_PREFIXES = ("fuzz/", …)` excluded `fuzz/fuzz_targets/*.rs` — Rust source
— under cover of excluding fuzz *data*: a wide exclusion written for a narrow reason, i.e. the same
shape as the defect under the previous heading. Discrimination was then proven by four injections (a
target removed from the fuzz matrix, the hook step replaced by `echo`, a registered id renamed, a
phantom id named in the document): each reddened the one test written to notice it, and the tree
came back byte-identical.

**One finding is worth keeping in front of anyone who trusts a green gate.**
`python/pyrs_yaml/pyrs_yaml.pyi` had been assembled carrying a materialised `jj` conflict as
*literal file text* — `<<<<<<< conflict 1 of 1` on line 1, 1,204 lines instead of 597 — inside a
commit that every local check called clean, and `jj status` reported no conflict because the markers
were content, not state. It was caught by noticing an asymmetric diffstat (1,204 insertions against
597 deletions for a "line-ending only" change) and asking what the two sides actually contained. The
file is repaired to `main`'s content with LF endings; #292's genuine stub delta (key types in return
annotations, five hunks) returns with that PR through `maturin generate-stubs` +
`check_stub_drift.py`, which is the only sanctioned route. `check-merge-conflict` is in the hook
set, and the hook set now runs in CI.

### (ai) The route-parity hole closed the next day, and closing it found the defect it was about (2026-10-08)

(ah) registered `route-parity:node-writer-vs-direct-dump` — the two YAML writers
(`Serializer::write_*` over parsed nodes, `py/direct_dump.rs` over Python objects) mirror each other
by design instead of sharing code, and `#287` had needed the same fix at both sites with only review
to notice the second. `tests/test_route_parity.py` is the table: canonical text pinned once, then
three assertions per row — the node writer settles on it, the direct writer emits the same bytes for
the equivalent object, and dumping through the *other* route afterwards is a fixed point. Intended
differences are pinned as differences (quote style, flow style, block scalars, anchors, tags) with
the reason, so "let's make them agree" cannot quietly delete the reason the routes exist.

**Building the table found the class still open at a site `#287` did not reach.**
`safe_dump({"a": {}})` produced `"a:\n  {}\n"` while the AST writer produced `"a: {}\n"` — both
re-read to the same data, which is precisely why the round-trip tier was silent: its oracle is text
equality *within one route*. The same miss had a second symptom behind it: `is_compact_mapping`
accepted only scalars and nulls, so one empty container forfeited the compact dash form for the
whole item — `safe_dump([{"a": {}, "b": 1}])` was `"- \n  a:\n    {}\n  b: 1\n"` against the
writer's `"- a: {}\n  b: 1\n"`. Two predicates, one cause: the fast path had no notion that `{}` /
`[]` have no block spelling.

**Attribution, per site, measured inside the parity table only** (extension rebuilt for each leg,
tree restored by hash): withdrawing the mapping-value inline reddens **12** cases — across
`test_the_direct_writer_emits_the_same_bytes`, `test_the_two_routes_agree_with_each_other` and
`test_dumping_through_the_other_route_is_a_fixed_point`, on the rows whose names begin `empty…` and
`three…`; withdrawing the `is_compact_mapping` clause reddens **9**, on the compact rows; with both
fixes in place the file is **132 passed**. The third failure mode is the interesting one: the
fixed-point assertion also reddens, which means the fast path was not merely spelling differently
from its twin — it was not the settled form, so a document that went `dump → load → dump` through
the AST route moved text while holding the same data.

**Three rows of the table were wrong before they were measured, and the corrections stayed in the
file as comments.** `- a:\n    b: 1` is not how a non-compact mapping under a dash is spelled (both
writers emit `- \n  a:\n    b: 1\n`); `a: '1'` is not the quote style either of them chooses (both
write `a: "1"`); and a folded block is re-wrapped on emission (`a: >` + `f1` + `f2` settles as
`a: >\n  f1 f2\n`), so the pinned text is what the writer settles on, not what the author typed. A
table written from guesses would have been "fixed" by relaxing the assertion; a table written from
output taught three facts about the emitter instead.

**Registered as pending rather than dropped:** `safe_dump({1: "a"})` emits `1: a`, which reads back
as `{"1": "a"}` and is then quoted — the object view resolves values with the schema rules but
leaves keys as text. PR #292 closes exactly that, and the rows are pinned at their current values in
`test_non_string_keys_do_not_survive_the_object_view_yet` with the mechanism named, so #292 must
break them and move the shapes into the parity table. The registry entry was deleted in the same
change that added the test, which is the direction the (ah) check is designed to catch.

### (aj) The JSON and TOML round-trip tier, and the correction of what registered it (2026-10-08)

(ah) filed `fuzz-no-roundtrip:pyrs-json` / `:pyrs-toml` with the line "a parse-only target cannot
see a writer". Opening the two targets said otherwise: `parse_json` calls `to_json_text` /
`to_jsonc_text` / `to_json5_text` and feeds each result back through all three readers, and
`parse_toml` does the same with `to_toml`. The claim was inferred from the target's **name**, which
is exactly the kind of reading this ledger has been warning about, applied to my own note. What was
genuinely missing is narrower and matters more: the re-parse result was bound to `let _ =` and
discarded, so those targets assert *"the reader accepts its own writer"* and not *"the writer's text
has settled"*. A writer can satisfy the first and violate the second forever, and every
comment-relocation defect in this engine lives in that gap.

**Two targets now assert the fixed point, per dialect.** `fuzz/fuzz_targets/json_roundtrip.rs` pairs
reader with writer inside each dialect rather than crossing them — `to_jsonc_text` emits comments
and `to_json5_text` emits hex numbers and bare keys, none of which the strict reader may accept, so
a cross-dialect demand would red correct output, which is the third time this cycle an "obvious"
oracle has been rejected for redding right output. `fuzz/fuzz_targets/toml_roundtrip.rs` reads under
both grammar revisions and re-reads its output with the 1.1 reader, because `to_toml` emits 1.1
spelling. Both are in the `fuzz.yml` matrix, so the PR tier replays their corpora with `-runs=0` and
the schedule explores them; six surfaces now, and the header comment was corrected from "four" in
the same edit rather than left as a stale claim.

**A sampler cannot gate a merge, so the invariant got a deterministic half.**
`crates/pyrs-json/tests/roundtrip_corpus.rs` and `crates/pyrs-toml/tests/roundtrip_corpus.rs` replay
the committed seed corpora on every `cargo nextest`. Measured at commit time: 11 JSON seeds driving
24 dialect rounds and 16 TOML seeds driving 30; each file declares a floor read out of the
measurement (18 and 23) so a corpus that stopped exercising the writers fails instead of passing
vacuously — the same construction as `crates/pyrs-yaml-core/tests/note_survival.rs`.

**Both floors were read out of a deliberate failure, not guessed.** Setting the floor above what the
corpus drives (`999`) makes the assertion message print the measured count — 32 and 30 — which is
how a lower bound gets derived from the machine rather than from imagination. The assertions were
then proven able to bite by mutating the comparison (`once, twice.clone() + "\n"`): both files
reddened on real seeds (`shape-deep-nesting.seed [json]`, `shape-array-of-tables.seed [1.1 input]`)
and came back green after a hash-verified restore. The first cut of that mutant used `twice + "\n"`
and failed to compile instead (`borrow of moved value`) — a mutation that does not build is not
evidence, the same trap (af) and (ag) recorded.

**Result of the new coverage: nothing is unsettled, and the interesting part is how many readings it
took to say so.** 63 dialect rounds over 30 seeds satisfy `once == twice` — 33 from 14 JSON inputs,
30 from 16 TOML — counted by setting each floor to an unreachable number and reading the real count
out of the assertion message rather than guessing one. Three misreadings precede that sentence, all
recorded here rather than edited out:

1. the first draft claimed two JSON5 writer defects, from a truncated `nextest` line whose only
   failure was the deliberate floor assertion;
2. the retraction then read a panic about `json5-number-forms.seed` as "a registered shape settles"
   when that input is not JSON5 at all (`0o17`, `0b101`) and the reader was right to reject it;
3. three shapes were then moved out of the replayed corpus into `fuzz/unsettled/` and registered as
   `writer-not-settled` holes on the strength of an `assert_ne` that had never actually run - the
   test aborted on the first file, which was the invalid input, so the other three never reached the
   assertion.

What ended it was not a reading at all but a temporary probe that printed both emissions for each
shape: every pair equal. The exception directory, its registry entries and the mechanism that
counted it are gone, the three inputs are coverage in the replayed corpus, and the registry is back
to one hole - `property-tier:default-case-count`, still blocked by the three `fmt_pbt` writer fixed
points. The lesson for anyone reading a gate's output next week: an assertion that did not run is
not a result, and the cheapest tie-breaker is a probe that prints values instead of a summary line
that names files.

No changelog entry: a fuzz target and two corpus tests ship no behaviour, the rule that kept the
entry for 284's harness and 275's budget out of the release notes. (The `#` of those issue numbers
was once mid-sentence and once at the start of a wrapped line, where Markdown reads it as a heading;
referring to them without the sign avoids the trap and the drift it caused.)

**(ak) The last registered hole closed, and the third writer defect this cycle found by measurement
rather than by reading (2026-10-08).** `property-tier:default-case-count` was the one entry left in
`.ci/quality-holes.json`, blocked by three `fmt_pbt` writer fixed points said to fail at 20,000
cases. Two of them never failed. With `max_global_rejects` set back to proptest's default 1024 and
the case count at 20,000, all three die at `fmt_pbt.rs:93` with
`Test aborted: Too many global rejects` — the domain filters (`json_object_domain`,
`toml_root_note_ok`) reject shapes the target format cannot spell, and the allowance was calibrated
for 256 cases. The harness was reporting its own reject rate as if a writer had drifted. The dialect
properties now carry a budget of 250,000; nothing about the oracle changed.

**The third failure was real, and it is fixed.** `to_toml` put a same-line note that sits on a
non-last member of a multi-line inline table after the separator comma (`b = 1, # n`). `#` runs to
end of line, so a comma cannot live inside a comment, and the reader re-homes that note as the
*following* key's leading comment — the second emission moved it, so the text never settled. The
note now gets its own line after the member, which is where the parser reports it from; measured
with a temporary probe that printed both emissions: `t = [{\n  b = 1,\n  # n\n  c = 2\n}]\n` twice
over, with the note coming back as `Comment { text: "n", standalone: true }` on `c`. No valid TOML
produces that AST — which is why only a generator reached it, and why the guard builds the node
instead of parsing it.

**Two of my own artefacts did not do what I claimed they did, and both were caught by running them
rather than by reading them.** (i) The first attribution mutant replaced the region up to
`out.push('}')` and re-emitted the loop's closing brace, deleting the statement that closes the
inline table; it reported three failures, one of them a unit test that has nothing to do with note
placement. Diffing the mutant against `main` *before* running it exposed the missing statement, so
the leg was thrown away and rebuilt from `main`'s own text. (ii)
`a_note_between_two_inline_members_keeps_its_own_line`, written to pin the rule, passed with the
rule withdrawn: its input parses the note as the *following* key's leading comment, so the
trailing-note branch never runs. The replacement guard fails in 0.45 s at the cheap tier and is the
only unit test that observes the rule.

**Attribution, all of it measured on `pyrs-toml` + `pyrs-yaml-core`.** Withdrawing the placement
rule: at 20,000 cases exactly one of 354 tests reddens (`fmt_pbt::prop_toml_writer_is_fixed_point`);
at 256 cases with the persisted shrink case present, exactly that one again; at 256 cases with the
persisted case removed, **354 of 354 pass** — the default tier cannot find this defect by sampling,
which is the evidence the new tier exists for. `ci.yml` gains a blocking `property-tier` job running
`PROPTEST_CASES=20000 cargo test --workspace --locked`; verified on this tree (exit 0, 305 core
tests in 63 s). The registry is now empty, and `tests/test_quality_matrix.py` fails if the
measurement disagrees with that in either direction.

**(al) The changelog is now coupled to the changeset, and (aj)'s closing paragraph is the evidence
(2026-10-08).** Read it again - "No changelog entry: a fuzz target and two corpus tests ship no
behaviour": this ledger *deliberately* decided that #296 needed no release note, and no checker
could disagree — `check_changelog_mirrors.py` compares version headers, which only move at a
release, and `prek.toml` runs it on a `files:` pattern that a changeset without a changelog never
matches, so the hook did not fire and the comparison had nothing to say. Meanwhile PR #293's tier of
the same class — "hygiene hooks run in CI", "the quality defence is measured" — did get its entries.
One of the two judgements is inconsistent with the other, and the tie-breaker cannot be prose in a
ledger written by the same hand that made the decision.

`scripts/check_changelog_coupling.py` decides it instead, over the pull request's file list: a diff
under `crates/`, `python/pyrs_yaml/`, `fuzz/`, `scripts/`, `tests/` or touching a shipped manifest
must touch a changelog, and touching one of the five mirrors means touching all five — which
`AGENTS.md` had already required ("never commit partial updates") and nothing had enforced. #296's
note is backfilled in all five, and it is the gate's own first exercise of the rule: the changeset
that adds the gate carries its entry, in five mirrors, or the CI step reddens it.

**Calibrated before adopting, because a gate that reddens dependabot gets bypassed.** Replaying the
rule over the last 40 commits of `main` at commit scope reddened 8 — #296's oracle, the merge-parity
gate, three `fuzz_rounds.sh` harness changes, the stub-drift gate, a proptest domain carve-out and a
`perf(yaml)` allocation change. Adding `.github/workflows/**` or `prek.toml` to the trigger list
added two more that are pure dependency bumps (`docker/setup-qemu-action` from dependabot, the
ruff/rumdl hook pin), so both are out; nothing is lost, because a PR that adds a CI job also adds a
checker or a test under `scripts/` / `tests/`. Scope is the pull request, not the commit: `0713267a`
adds the stub-drift gate and carries no note, its note arrived in a sibling commit — a commit-scope
hook would have red a correct PR for the wrong reason.

**Discrimination proven by mutation, one rule at a time** (`tests/test_changelog_coupling_gate.py`,
29 tests): withdrawing the coupling branch reddens exactly 3 tests (`test_pr_296_shape_reddens`,
`test_backslashes_are_normalised`, `test_cli_exit_codes`) and leaves the completeness cases green;
withdrawing the completeness branch reddens exactly its 2; putting `.github/workflows/` back into
the trigger list reddens exactly the 2 calibration guards. The mirror list is pinned against
`check_changelog_mirrors.py`'s own `FILES` by loading both modules, so the two changelog gates
cannot drift into policing different files.

**(am) The Ir gate's headroom was measured for the first time, and it is thinner than the 2% line
(2026-10-08).** P2-F asked for two things: cover `to_json` / `to_toml` / inline-merge, and
regenerate the baseline in the environment that enforces it. The coverage half is in: 12 measured
scenarios (`parse_inline_merge`, `to_json_medium`, `to_toml_medium` over the existing 9), and
`.ci/ir-baseline.json` carries twelve numbers, so `quality_matrix.py`'s `ir-unbaselined` probe stays
silent. The measurement half produced a finding nobody had asked for:

**Re-measuring the committed scenarios against the current tree moved them by −2.35%
(`parse_anchors`) to +1.61% (`serialize_anchors`), while the three new ones repeated at ±0.0005%
between two runs of the same binary.** The contrast is the point. Callgrind's precision claim in
`ir_gate.py`'s own docstring is intact — 20,434,726 against 20,434,691 — so the spread is
accumulated drift between the baseline's tree and today's, not noise. And drift in that direction is
the unsafe kind: every merged PR that made the engine faster pushed the true cost *below* the
committed number, widening the band the gate accepts. A 2.35% regression on `parse_anchors` today
would print as no change, and `serialize_anchors` has 0.39 of the 2% budget left for work it may
never have grown.

**Two fixes, both measured before claimed.** `ir_gate.py --update --only <scenario>` used to write a
baseline containing only the scenario it had measured — a nine-number file becomes a one-number file
and every other scenario then reads as unbaselined; it now merges into what is committed and refuses
to invent a number it did not measure (used for real here: three measured, twelve committed, the
nine pre-existing values byte-identical). And `.github/workflows/ir-baseline.yml` regenerates the
whole file on `ubuntu-24.04` — the image that enforces it — prints the diff for a human, and uploads
the result as an artifact rather than pushing it, because a baseline is a claim about the code and a
workflow that rewrites it on a schedule turns the gate into a mirror. What that job has not yet done
is run; until its artifact is committed the numbers are still WSL-measured and the 2% line still
pays for the +1.45% cross-image gap. Tightening the tolerance is the next step and belongs after
that commit, not before it.

**The CI run then refuted the last sentence of that paragraph.** #299's own gate measured
`serialize_block_scalars` at **+2.23% on the runner against +0.78% in WSL** — over the line, job red
— while the eleven other scenarios, including the three new ones, agreed between the two images to
0.003% (`to_json_medium`: 20,434,726 against 20,434,780). A machine gap that hits exactly one
scenario and spares the other eleven is not a machine gap. It was `.gitattributes`:
`* text=auto eol=lf` normalises CRLF at checkout on every platform, and it applied to the two files
`check_line_endings.py` deliberately leaves alone *because their CRLFs sit inside raw strings that
are the parser's input*. The runner therefore parsed `BLOCK_SCALAR_YAML` with LF, a working tree
parsed it with CRLF, and `scripts/ir_gate.py` had been attributing the resulting 1.45% to "the
dynamic loader and malloc of the host image" since #293 — with the tolerance sized around a number
that was really a **contradiction between two rules introduced in the same pull request**.

Both fixtures are `-text` now, the rule the fuzz seeds already use for the same reason, and
`tests/test_line_endings_gate.py` pins in both directions that the exclusion list and the checkout
rule name the same files — including that the excluded fixture still carries the CR bytes it is
measured on, because a `-text` line protects nothing if a later commit strips the data. What is
actually left to pay for is drift between the tree the baseline was taken on and the tree being
gated (-2.35% to +1.61%, measured above), which the refresh workflow closes. The 2% line stays for
now on a *correct* reason rather than a wrong one, and `ir_gate.py`'s comment says so.

**Then the second experiment refuted that paragraph too, and this is the last version of the story.
Two of its claims were wrong and both are corrected here rather than edited away.**

1. `-text` did not explain the 1.45%. The push that carried it measured `serialize_block_scalars` at
   16,020,906 against 16,020,878 before it — 28 instructions out of sixteen million. The hypothesis
   is dead, and `scripts/ir_gate.py` says so in the comment that used to carry the old attribution.
2. The premise beneath the whole "measured fixture" story was unstable in a way neither version
   stated. Probing three views of the same path - `git show` of `main`, `git show` of this branch,
   and a Windows checkout - gives 0 CR bytes, 98 CR bytes and 80 CR bytes. The fixture's *stored*
   bytes depended on which tree the last author committed, so #293's note ("their CRLFs sit inside
   raw strings that are the very input the committed Ir baseline was measured against") described an
   accident of someone's checkout, and `check_line_endings.py`'s exclusion has been protecting that
   accident ever since Stripping the CRs from the tree moved `serialize_anchors` by 0.86% and left
   `serialize_block_scalars` 1.45% off the runner, so the local tree's line endings do reach the
   measurement — just not in the direction anyone had guessed.

What is therefore settled is the *practice*, not the mechanism. The runner's own numbers are now the
committed baseline (twelve values, `generated_by` recording image `ubuntu-24.04 20261004.327.1`),
two images of the same commit agree to ≤0.0018%, `tolerance_hint` is 0.5% — about 280× the observed
spread — and `ir_gate.py` prints which machine generated the numbers against which machine is
reading them (a WSL run shows the note). The WSL↔runner gap on two allocation-heavy scenarios is
**open**; what changed is that a marginal failure now arrives with its own provenance instead of
being absorbed by a tolerance sized around an unexplained number.

`tests/test_line_endings_gate.py` gained three pins: the exclusion list must be declared `-text` in
`.gitattributes`; the committed bytes of a measured fixture must equal the bytes in the tree that
produced them (the reproducibility invariant the whole story was missing); and the excluded paths
must still exist. Two earlier drafts asserted, respectively, that the working copy must *keep* its
CR bytes and that the committed blob holds *none* - both were written from a probe of one ref and
generalised; the first is deleted, the second became the equality test above, which is what the
measurements actually support.

**(an) The oldest supported Python was untested territory, and the matrix that seemed to cover it
was not a gate (2026-10-08).** #299 arrived with `test (windows-latest, 3.8)` red and 47 legs green,
and the red one was not about #299 at all. It read:

```text
AttributeError: 'str' object has no attribute 'removeprefix'   # scripts/check_changelog_coupling.py
TypeError: 'type' object is not subscriptable                  # scripts/check_changelog_mirrors.py: _versions -> set[str]
```

Both are #298's file and a much older one, and both are legal on the interpreters that actually run
those scripts - the hook environment and two CI jobs, all 3.12/3.14 - while `pyproject.toml`
promises 3.8. The signature-annotation case is worse than a style slip: without
`from __future__ import annotations`, a `set[str]` return annotation is evaluated at *import* time,
so `check_changelog_mirrors.py` has been unimportable on the supported floor since it was written,
and no gate ever tried. It surfaced by luck: #298's own test file imports a checker, so the 3.8 leg
ran it.

**Two things were done, one of them only half.** `tests/test_scripts_import_on_supported_python.py`
now imports every file under `scripts/` on whatever interpreter runs the tests, checks statically
that any builtin-generic signature carries the future import, and reads the version floor from
`pyproject.toml` instead of memory; it found a third instance - `check_stub_drift.py`'s
`-> tuple[...]` - on its first run. Naming the limit matters: an import cannot see a newer-API call
inside a function body, which is exactly what `removeprefix` was, so the gate's own behaviour tests
remain the thing that catches that half.

The other half is still open and is the more systemic finding: **no CI check is required for a
merge.** Branch protection matches checks by name, and
`gh api repos/<repo>/branches/main/protection` reports `strict: true` with `contexts: []` and
`checks: []` - nothing required at all. So #298 merged with a latent red not because one leg was
missing from a list, but because no leg is on the list, and `test (windows-latest, 3.8)` - the leg
carrying two checkers broken on the supported Python floor - was simply not part of the question the
merge asks. An earlier draft of this entry said "20 of the 21 matrix legs are advisory"; the
measured setting says the true figure is 0 required out of 40+. A fan-in job (`needs: test`, one
name, red if any leg is not `success`) is the software half and has now landed with #300; the
configuration half - putting that one name into branch protection - is a repository setting, not a
pull request. Until it is set, "we test on 3.8 through 3.14 on three operating systems" describes
sampling, not enforcement, and this entry is where that distinction is recorded rather than smoothed
over.

### (ao) The defence measured a gap in itself, and registered it instead of fixing it (2026-10-08)

Every hole closed so far was found by asking what the defence *names*: does a fuzz target exist for
this crate, does a workflow run this hook, does a parity table compare these two writers. This one
came from a different question - what can the instrument **link**?
`crates/pyrs-yaml-core/benches/ir_gate.rs` is the reproducible channel (two runner images agree to
0.0018% on the same commit), and its build graph is the bench owner plus its workspace dependencies:
`pyrs-yaml-core`, `pyrs-ast`, `pyrs-schema`, `pyrs-json`, `pyrs-toml`. `pyrs-yaml` - the crate that
serves the Python API, found by layout via `crates/*/src/py/` rather than by name - is not in it. So
`safe_load`'s AST-to-Python conversion, the path every user takes and the path PR #292 changes,
cannot be gated at all: the only number that covers it is CodSpeed wall time, which this ledger
already documented swinging -7.7%, -10.5%, -9.8% across three pushes that each did strictly less
work.

Registered as `perf-coverage:binding-layer` with the state that removes it (an `ir_gate` harness
compiled against the binding crate, with its own committed number), rather than patched in the same
breath it was found. Two reasons, both earned this cycle: a scenario that needs `PYO3_PYTHON` and a
linked interpreter is not a same-afternoon change, and the registry's value is that the measurement,
not a reader's memory, decides whether the gap still exists. It also settles a question #292 had
been asking in the wrong instrument: its ~+19% local ratio and its -10.52% CodSpeed verdict are both
measurements of a path the reproducible gate cannot see, which is why neither could be adjudicated
against the other.

### (ap) The registered hole closed by the artefact its own exit criterion named (2026-10-08)

`crates/pyrs-yaml/benches/ir_gate.rs` adds `to_python_small`, `to_python_medium` and
`to_python_anchors`: the same fixture bytes the engine harness parses, measured one layer higher,
through a `#[cfg(feature = "ir-gate")]` seam (`bench_to_python` in
`crates/pyrs-yaml/src/py/functions.rs`) that mirrors `safe_load`'s AST path and skips the P3
direct-load shortcut on purpose — measuring the shortcut would report a different quantity than the
one anchor- and tag-bearing data pays for, and an anchor-free fixture would have hidden a
direct-load regression. `required-features` is load-bearing: `cargo codspeed run` executes every
discovered bench target with no arguments, so an ungated scenario binary would exit 2 and redden the
`Rust benchmarks` job. Each iteration has to convert one document or the harness exits 3 — the seam
returns its error instead of defaulting it, because the tolerance punishes growth only, so a harness
that quietly stopped doing the work would otherwise report a large improvement and pass. The entry
deleted itself from the registry because `quality_matrix.py`'s derived graph gained `pyrs-yaml`,
which is the exit criterion being honoured rather than a reader deciding the gap looked closed.

**Building the second harness found a defect in the measurement of the first.** The scenario probe
read `crates/pyrs-yaml-core/benches/ir_gate.rs` by name, so with two harnesses it would have
compared twelve of fifteen names and let `--update` write a baseline missing an entire channel — the
same class as #303's graph probe stopping at the first `[dependencies]` section, caught here by
adding a harness instead of by luck. `ir_harness_channels()` now derives the harness list from the
manifests and compares it against the files on disk in both directions (`ir-harness-missing`,
`ir-harness-undeclared`), reports a scenario name listed by two harnesses (`ir-scenario-duplicate`,
because one baseline number cannot say which channel produced it), and `ir_gate.py` refuses to build
a scenario map from a harness that lists nothing. Each direction is tested by injection, not by
reading the code. The refresh job's artifact also proved the committed baseline was part
transcription: its `generated_by.note` was hand-written and `--update` writes no prose, so the next
refresh would have dropped that paragraph and the diff would have read as a decision — the prose
moved to `QUALITY_MATRIX.md`, and `tests/test_ir_baseline_workflow.py` pins the file's keys to
exactly what the generator writes.

Two limits recorded rather than smoothed over: the binding binary links CPython, so it builds
anywhere and runs only on Linux (measured on Windows — `cargo bench --no-run` succeeds and the exe
then dies at start-up with `0xC000021A`, before printing a scenario list), and
`cargo clippy --all --all-targets` does not compile a `required-features` bench target, so the new
file is guarded by building it, not by the lint job. A third limit was found by running it: the
harness is a standalone executable and has to start the interpreter itself, so the `ir-gate` feature
turns on `pyo3/auto-initialize` — the first refresh run built, listed its scenarios, and only then
failed in the measured loop with "The Python interpreter is not initialized and the
`auto-initialize` feature is not enabled", which is the good outcome (a `--list` that works and a
measurement that refuses stay distinguishable, and `ir_gate.py` exits on a non-zero harness instead
of writing a short baseline). The feature is where that belongs: the shipped wheel runs inside an
interpreter that is already up, so nothing about the extension changes. What (ao) could only
register is now an instrument: the conversion #292 changed has a counted-instruction number,
reproducible to 0.002% inside a job — which is precisely what let it expose the cross-job
disagreement recorded in (aq), rather than leaving that to be argued about.

### (aq) Two runner jobs in this repository measure the same code 1.44% apart (2026-10-08)

Regenerating `.ci/ir-baseline.json` for (ap)'s three scenarios made the enforcing job
(`Instruction-count baseline` in `codspeed.yml`) red on `serialize_block_scalars`, and the failure
is worth more than the red: that job reports 16,020,942 and 16,020,915 across two runs, while
`.github/workflows/ir-baseline.yml` — the job whose whole purpose is to produce numbers for that
gate — reports 15,792,8xx to 15,792,9xx across eight. Two runner images, the same pinned rustc
1.97.1, the same script, sources and fixtures byte-identical (`git rev-parse` of
`crates/pyrs-yaml-core/src/bench_inputs.rs` agrees across the commits involved), and every other
scenario agreeing between the jobs to within 0.08%. Inside either job a scenario is stable to about
30 instructions, so this is not noise: it is the +1.45% this ledger has been calling the
WSL-to-runner gap, showing up between two runner jobs — and the cause is in the next paragraph.

**Four explanations were tested; three are recorded as refuted, and the fourth is the cause.** (1)
Drift between runner images: no — each job reports its own value on both images. (2) The committed
values having been generated off-runner, which is what the first version of this entry asserted: no
— the enforcing job reproduces them, so the file's numbers came from a configuration like the gate's
and the offset belonged to the *refresh* job instead. (3) `Swatinem/rust-cache` letting the refresh
job reuse a warmed `target/`: the step was deleted and the job still reported 15,792,928.

(4) What the binary learns about the CPU at start-up — **confirmed**, by the instrumentation added
to answer the previous three. Both jobs printed the same SHA-256 for the measured binary
(`ir_gate-762abe271a6273f5`, `a0386c17b5f1ef38`), the same valgrind 3.22.0, the same pinned rustc
1.97.1 and the same `nproc`; the hosts were not the same, though both are `ubuntu-24.04` — one
reported `AMD EPYC 9V74 80-Core`, the other `AMD EPYC 9V45 96-Core`. The CPUID flags a VM exposes
decide which string routine glibc binds, and `serialize_block_scalars` is the most copy-heavy
scenario in the set and the only one that moves, which is exactly the signature that mechanism
predicts. Running every measured process under
`GLIBC_TUNABLES=glibc.cpu.hwcaps=-AVX512F,-AVX2,-AVX,-SSE4_2,-POPCOUNT` settled it: on a host of the
other model the enforcing job measured 15,780,929 against a baseline of 15,792,910 — −0.08%, where
the same job had measured +1.44% with the caps unmasked. Regenerated under the pin, the jobs now
agree to tens of instructions on that scenario across three host models — `AMD EPYC 9V74`, `9V45`
and `7763` measuring 15,780,927, 15,780,929 and 15,780,957, which is 0.0002% of spread where the
unpinned pair were 1.44% apart. The line was not widened to absorb the host; the input the host
controlled was pinned instead. The +1.45% WSL-to-runner gap this ledger has carried since the gate
was built, and attributed twice to causes that later measurements refuted, is the same effect — a
laptop hypervisor exposing different flags than a runner VM, and the same one scenario noticing it.

**What the file did lose is prose that had been typed into it.** Its `generated_by.note` cited a
commit that is not PR #299's head and predates the toolchain pin — the job there would have measured
with the image's `@stable` (rustc 1.99.0, which moves this scenario −6.6% and `serialize_medium`
+6.7%) — and its environment string reads `ubuntu-24.04` where `environment()` emits `ubuntu24` for
that image. So `--update` writes every key now, including the sample size the numbers were drawn
with, the reasoning lives in `QUALITY_MATRIX.md`, and `tests/test_ir_baseline_workflow.py` pins the
committed key set to the generated one.

**The new channel was too small to gate, and measuring it is what said so.** At 500 iterations two
runs of one commit put `to_python_small` 0.83% apart — wider than the 0.5% line the scenarios were
about to be held to — while the engine scenarios agreed to 0.0009%. The loop was too short relative
to the variation inside it, so the harness runs 2 000 iterations like its engine sibling and the
within-run spread came down to 0.076~0.24% (engine 0.002%). Each scenario is also sampled three
times and the largest value is what gets committed, because a single draw of a distribution that
wide enforced at 0.5% is a flaky gate; the sample count is recorded in `generated_by` and the
enforcing run reads it from there, since max-of-3 against a single sample is a different instrument
wearing the same name. `ir_gate.py` prints every sample and its spread, so the tolerance is argued
from the job log rather than from a comment.

### (ar) A changelog entry could be committed where no reader looks, and now it cannot (2026-10-08)

401a8057 added a Fixed entry to all five mirrors and filed it above the preamble in `CHANGELOG.md`,
inside the `tags:` list of the en and zh frontmatter, and between the frontmatter and the first
heading in ja and ko. Five placements, one shared property: every one of them sits outside the
changelog body, and every one is invisible to `scripts/check_changelog_mirrors.py`, which compares
version headers while translation leaves those identical wherever the prose goes. The entry is now
under `[Unreleased] → Fixed` in every mirror, in newest-first order — the position was measured
rather than guessed: by commit date it belongs under the `crash-9b77aea4` entry and above the
`crash-1b01ac3f` one — and `placement_errors` rejects both shapes hard: a bullet before the first
version heading, and a bullet whose nearest heading is a version rather than a section. The second
rule was measured before it was asserted (zero violations across the five mirrors on the tree of the
day), which is the difference between a gate and a rule that has to be bypassed in its first week.

**The same blindness in completeness form is registered here, not fixed.** The checker now prints
each mirror's `[Unreleased]` entry counts by section position, and they differ: against root,
`docs/en` is one entry behind and `docs/zh` five (one Added and five Fixed missing, plus one Changed
bullet no other mirror has) — the recorded consequence of `AGENTS.md`'s "never commit partial
updates" being enforced only by a check that cannot see one. `changelog-parity:entry-counts` in
`.ci/quality-holes.json` holds that gap with the backfill as its exit criterion. Turning it into an
assertion was declined in this change for the reason this ledger keeps relearning: a gate that goes
red because three other mirrors are missing translations somebody else did not write gets disabled,
not obeyed. So the divergence is measured, printed on every run of the hook and registered, and
`tests/test_changelog_placement_gate.py` fails if the registered statement stops matching the
measurement — comparing per-section *gaps* rather than counts, so an entry added to all five mirrors
leaves the claim true while adding one to a single mirror makes it false.

### (as) #292 was adjudicated on a channel that could not see it, and re-adjudicated on one that can (2026-10-08)

PR #292 (resolve mapping keys with the same rule as values) is closed, and the closing evidence was
a wall-clock CodSpeed regression of −10.5% on `test_to_dict` while this ledger claimed the Ir gate
saw "~+19%". Neither number was reproducible, so the change was re-measured properly: three variants
through `.github/workflows/ir-baseline.yml` — `main`, the branch as written, and the branch with its
byte-only pre-check switched off — each run twice, landing on two runner images and three host CPU
models. Resolving keys like values costs **+1.45%…+1.62% on the binding channel and 0.00% on every
engine scenario**. The pre-check the branch added to offset that (`might_start_typed` ahead of the
whole-edge trim in `resolve_core_type`, justified in a code comment and never measured) is a
pessimization everywhere the gate can look: **+0.72%…+1.03%** on the binding scenarios it was
written to protect, **+0.86%…+6.66%** on the engine ones, `to_json_medium` worst at +6.66%; switch
it off and each engine scenario returns to `main`'s value within 0.003%. The old +19% was measured
on an instrument that did not exist yet, and the 0.0–0.1 basis-point repetition of every delta
across three CPU models is the cross-check that the pinned environment of (ao)/(ap) actually holds.
Recorded rather than edited out: the behaviour change is ~13× cheaper than claimed here, and the
mitigation written for it was worse than no change.

**The half that still could not be seen is why the gate is wider now.** #292's other change is on
the *reading* side of a cross-format bridge — `load_toml` must quote a key that either YAML schema
would re-type, or the conversion silently changes what the document means — and the scenario set had
`to_toml_medium` (the writer) with no reader twin, so per-key work added there moved nothing the
gate could observe. `from_json_medium` and `from_toml_medium` are in, reading **committed** bytes —
the writer's output for `MEDIUM_YAML`, pinned to it by `crates/pyrs-yaml-core/tests/ir_fixtures.rs`
— a choice made after a measurement that then refuted its own explanation: rendering the input in
setup (so the writer's cost cancels in the subtraction) moved `to_json_medium` +1.52% and
`to_toml_medium` +0.26% with the engines untouched, three times the tolerance on scenarios the
addition never enters, and the extra call site was blamed. Committing the bytes removed the call and
`to_json_medium` moved +1.489% anyway. So the mechanism is recorded as unresolved — the movement
tracks the harness binary changing shape, not the fixture's provenance — while the property is
settled: a harness edit is not neutral for the numbers that harness already produces, and a
re-baseline after one separates method from code by saying so. `serialize_*` and `parse_*` in the
same binary stayed inside 0.042%. The readers exit 3 rather than report a number if the fixture
stops parsing — proven by injecting an unparseable document into one arm — and `quality_matrix.py`
now derives `ir-bridge-unidirectional` for any `to_<format>_*`/`from_<format>_*` scenario whose twin
is missing, with both directions of the rule fired by deletion in `tests/test_quality_matrix.py`.
`to_python_*` is excluded by the rule's own definition (a language binding is not a text format),
which is the kind of exemption a probe needs stated rather than stumbled into. **Consequence
for #292:** reviving it is now a measured decision — land the key resolution without the pre-check,
~1.5% Ir on the one channel users hit, and let the new bridge scenarios price the TOML quoting
instead of guessing at it.

### (at) The key-resolution change landed without the mitigation that made it look expensive (2026-10-08)

(as)'s verdict was applied rather than filed: `py/convert.rs` and `py/direct_load.rs` now resolve a
mapping key with the value path itself, `load_toml` quotes a key that either YAML schema would
re-type, and the byte-only pre-check inside `resolve_core_type` is **not** part of the change - it
was measured as a pessimization on every scenario the gate has, so shipping it would have bought a
regression in the engines to save nothing in the binding. The refreshed baseline says what the
landed shape costs: `to_python_small` +1.539%, `to_python_medium` +1.592%, `to_python_anchors`
+1.553%, and **0.002% or less on the other thirteen**. Among those thirteen is the number (as)
predicted would only exist once the reader scenarios did: **`from_toml_medium` +6.196%** - four
times the YAML key cost, on the half of the bridge that had never been measured at all, from
`plain_text_is_typed` running the Core chain and the 1.1 chain per key. That answer is deliberately
conservative (a TOML or JSON key is a string by its own grammar and quoting is the shared AST's only
marker for it; over-quoting is harmless, under-quoting is a type change), and it is now cheap to
argue about: a one-chain form plus the 1.1 word list is a proposal the gate can price in one
re-baseline instead of one more debate. Registered as the pillar-four follow-up rather than
attempted in the same change - (as) and (ap) are both about what happens when a perf claim gets
shipped ahead of the instrument that can check it.

### (au) A Markdown formatter had been rewriting the ledger's sentences into headings, and no gate could see it (2026-10-09)

Three headings in the tree were not headings. `ROADMAP.md` carried
`## 293's tier of the same class — "hygiene hooks run in CI" … — did`, with the rest of that
sentence sitting in its own paragraph underneath, and two more appeared while landing `(as)` and
`(at)`. Every gate stayed green on all three, for the same reason and a different one each time: the
linter accepts them because they are valid ATX, `check_changelog_mirrors.py` compares version
headers, `check_i18n.py` compares page inventories — a document whose structure is wrong but *legal*
has no way to complain. The cause is what happens when hard-wrapped prose meets a formatter: a
continuation line that begins with an issue reference is, to the parser, a heading, so `rumdl fmt`
promotes it and blanks around it. Reproduced rather than inferred — a scratch file with that exact
shape came back with the line rewritten into `## 292 branch as written, and more`.

`scripts/check_doc_headings.py` asserts the shape now, wired as the eighteenth hook
(`doc-heading-integrity`). Its rule — a heading whose text starts with two or more digits not
followed by a dot — was sized against the prose before it was asserted: run over the damaged text it
reports exactly those three lines (one was already committed on `main`, two appeared in the working
copy while `(as)` and `(at)` were being written), and over the 173 tracked pages of repaired prose
it reports none of the digit headings this repository means (`### 1-D array`,
`#### 0-D Scalar Arrays`, `#### 10. メタデータの操作`, `## 1. Test matrix coverage`). It tracks
fenced blocks, and that came from the same measurement rather than from caution:
`docs/ja/contributing/site-i18n.md` line 60 is a shell comment inside a console block, and a
fence-blind scan would have demanded a fix to a line that is not Markdown at all.
`tests/test_doc_heading_gate.py` (12 test functions) fires the rule on the damaged text, proves the
same sentence *joined back* is silent — so the guard is about the shape and not about citing pull
requests — pins each legitimate digit heading as a non-finding, checks the fence tracking both
inside and after a block, requires the message to name the fix, and asserts the hook is wired.

Worth stating as a category the matrix had not considered. Every guard so far checks that content
agrees with something else — a baseline, a mirror, a registry, a header. This one checks that the
document is still the document the author wrote: an invariant over what a formatter does to prose,
not over a disagreement between two sources.

### (av) The ledger and the changelogs were re-flowed, and the width is a rule now (2026-10-09)

`ROADMAP.md` had a line of 21,850 characters, `docs/ja/changelog.md` one of 884, and 1,299 lines
across the six pages exceeded a terminal window. Markdown does not care about physical line length
and neither does a renderer, so nothing complained: the cost fell entirely on the reader, which is
the one thing a quality ledger is for. The localized pages were the worst because a tool that breaks
only at spaces cannot wrap a Chinese paragraph at all, so a width rule that ignores display columns
is not a rule for this repository.

`scripts/check_doc_wrapping.py` measures 100 **display columns** (a fullwidth glyph counts as two)
and carries the `--fix` side: fences, tables, headings and front matter stay, a list continuation is
indented rather than re-marked, a quote keeps its `>` on every line, code spans and link targets are
atomic, and a paragraph is re-cut as one unit — joining its lines moves whitespace and nothing else,
while cutting line by line handed each line's last word to a continuation until `and` sat alone on a
page. Both halves live in one file on purpose: a check and a formatter written separately drift, and
then the hook rejects what the formatter emitted.

Ownership of a line settled by measurement, after a wrong guess. `rumdl fmt` is the repository's
Markdown formatter and remains so; excluding it left `MD007`/`MD012` findings on six lines it had
been keeping honest. But it does not re-wrap prose, and the draft of this entry assumed it did,
which excused 634 over-length lines as another tool's decision. Fed a 587-character English line it
emits a 587-character line; fed a 319-column Korean one, the same; `line-length = 80` changes
nothing, because that key configures the linter while `.rumdl.toml` disables `MD013` on the belief
that general formatting handles line length. So rumdl owns structure and this rule owns width: the
pass re-flows English, Korean and Chinese alike, taking the six pages to zero avoidable violations.
Four lines stay over the limit and are deliberately not reported, because each opens with a 101 to
127 column code span, a `cargo build --target thumbv7em-none-eabi …` invocation whose every legal
cut leaves the first piece over budget. Splitting the span would corrupt the command that makes the
line long, and a gate nobody can satisfy is a gate people bypass, so the exemption is the fixer's
own capability expressed as a predicate rather than an exception list.

Three rules came from damage this tool caused while being built, which is the honest way to acquire
them. Cutting before an issue reference put `#283, whose fuzz jobs …` at a line's start and
`rumdl fmt` promoted it into a heading, caught by the gate `(au)` added, on the file that added it.
Cutting in front of a literal `+` that joined two ABI names left a continuation line beginning with
`+ `, and `docs/ko` gained a list item no author wrote; the wrapper now refuses any cut that would
place a list marker at line start. And re-flowing stranded a dash run under a paragraph, which
CommonMark reads as a setext heading, after which `rumdl fmt` answered its own `MD003` by rewriting
this entry's third paragraph into a 126-column `##` heading that also cut the entry out of the
outline. The digit-only gate missed it, and the replacement rule first tried here, flagging any
heading wider than the prose convention, was falsified on contact: twelve legitimate `### (xx)`
lead-ins exceed 100 columns. The gate now names the shape instead, that no paragraph may end on the
line above a dash or equals run. `tests/test_doc_wrapping_gate.py` (22 test functions) and
`tests/test_doc_heading_gate.py` (12 test functions) fire each invariant through an injection
rather than trusting the claim, including idempotence, structure preservation, and the fact that
`--fix` satisfies the check it reports.

The rest is editorial. The 33 `**(xx)` lead-ins became `### (xx)` headings so the ledger has an
outline; `(w)` onward moved under a `## The quality defence` heading because it had been sitting
inside a section about note placement; two dangling sections were re-levelled; and the single
21,850-character paragraph became a list of ten finding families — 3,479 words to 726, with every
crash id, mechanism and seed still named. `DOCS_STANDARDS.md` §10 states the convention, the hook is
`doc-line-width`.

### (aw) Timing floors sampled phases apart, and a red could not be attributed (2026-10-09)

Three leaderboard files asserted a structural property - "this path does less work" - from
wall-clock, each with a different estimator. `test_toml_leaderboard.py` ran every candidate block
before every reference block; `test_json_leaderboard.py` and the serialize gate took a single block
per side; `test_leaderboard.py` timed the candidate once and then each peer once. A block is only
comparable inside itself, so whichever phase a scheduler spike landed on decided the verdict, and
the message quoted two medians and nothing else. The claim that timing both sides end to end in one
process keeps the ratio honest holds only while the process owns its cores.

Measured before touching anything, because the fix has to be aimed at a real number. The parse pair
sits at 2.21-2.33x locally; across 96 trials with 23 busy workers it never fell below 2.02x and
never inverted - so CPU saturation on a 24-core box is not the mechanism, and saying so would have
been the second false premise in this ledger. The margin does collapse elsewhere: the same gate's
recorded macos-latest incident is 336us against 369us, 1.10x, on a cell with two cores where the
whole process is preempted. One red arrived in roughly six full local runs and could not be
reproduced in four more, and because the assertion text was never captured, its cause stayed
undiagnosed. The defect named here is that unattributability, not the flake rate.

`tests/timing.py` is now the single sampler. Candidate and reference are measured adjacently inside
each block, the verdict requires winning all but one of five pairs, and the failure message carries
every pair, so the next red distinguishes "one side inflated in one block" from "the two really are
close". `median_us` keeps the discarded warm-up round that the macOS flake originally traced to cold
allocator arenas. Cross-library floors keep their 5x and top-3 thresholds - measured 38x-280x
against PyYAML and 71x-77x against ruamel, so the floor is not the fragile part - and change only
their estimator, sampling the pure-Python reference with fewer repetitions per block because the
pair, not the repetition count, is what cancels drift.

The methodology is measured, not entrusted to a docstring: `scripts/quality_matrix.py` derives
`timing-floor-unpaired` for any `tests/test_*.py` that reads `perf_counter` outside CodSpeed and
does not import the sampler, with the root as a parameter so the check is fired on injected files
rather than argued about. It measures zero today. Nine tests in
`tests/test_timing_gate_discipline.py` pin the alternation as a phase-switch count (sequential
blocks give one switch, alternating give five), the tolerance bound, the fact that a genuinely
slower candidate still goes red, and that a synthetic unpaired gate is named while a benchmark file
is not.

### (ax) The instruction gate had no number for two of its five formats (2026-10-09)

`from_jsonc`, `from_json5`, `to_jsonc_text` and `to_json5_text` are public, engine-level, per-key
paths, and none of them was measured: the gate numbered strict JSON and TOML in both directions and
stopped. The comment scanner, the unquoted-key and trailing-comma grammar, and both dialect writers
were therefore outside the zero-regression instrument, which is the opposite of what a named-format
infrastructure promises.

On one and the same hub AST the three writers now read 21,232,786 / 22,176,851 / 22,974,851
instructions for JSON / JSONC / JSON5, so emitting notes costs 4.4% and the JSON5 spelling 8.2% over
strict JSON. Those three are directly comparable because they share an input, and that is the value
of putting them in the same harness: the cost of a dialect is now a number someone can decide
against, instead of a feeling about "comments are cheap".

The readers are measured from authored bytes, which breaks the rule the strict fixtures follow, and
deliberately so. `to_jsonc_text` of a comment-free AST emits no comments, so a fixture derived from
the writer would not pass through the scanner under measurement; a fixture has to *contain* the
thing whose cost is being counted. `tests/ir_fixtures.rs` therefore pins the three properties that
make the scenario mean something: the bytes keep their dialect, the reader accepts them, and they
resolve to the same names and numbers as `MEDIUM_JSON`.

Writing that parity check found a real defect and it is filed rather than absorbed: `from_json5`
writes a non-finite float into the hub as the bare word `Infinity`, which no YAML schema reads back
as a number, so `load_json5` is correct in memory while one hub round trip turns the value into a
string; `-Infinity` was emitted quoted, which is the same loss wearing better manners. The fixture
spells the exponent case plainly and pins the good behaviour separately (`3e1` survives as a number
because JSON can spell it, hex canonicalises because JSON cannot), and #312 carries the bug.

The instrument was blind in a second way, worth naming because it is the general lesson:
`ir_bridge_pairs` compares a scenario name to its twin, so it can only report a *half*-missing
bridge. A format with neither half offers no name to start from, and the matrix printed green -
which is precisely how this gap sat unnoticed while the probe that exists to catch asymmetric
coverage was running on every commit. `quality_matrix.py` now derives `ir-bridge-absent` per format,
and a test deletes both JSON5 scenarios to prove the hole appears rather than asserting that it
would.

Re-measuring moved the seventeen existing numbers by at most 0.05%, because the runner image changed
underneath (`20260927.320.1` to `20261004.327.1`). That is method and not code, so it is written
here instead of being absorbed into a silent baseline update.

### (ay) The verdict rule I shipped last week was refuted by its own CI log

The entry above is the correction; the ledger keeps the shape of the mistake, because it is a shape
that recurs. A timing gate was rebuilt once already in this ledger - phases separated, then paired
with a majority verdict - and both times the change was argued from what a burst *could* do rather
than from what a burst *does* on the runners that execute the gate. The first version's failure was
invisible because the message printed two medians; the second version's failure was legible in one
line because the message printed every block, which is the argument for spending complexity on
attribution rather than on a cleverer estimator.

Two things generalise. Adjacency is worth keeping for a different reason than the one I claimed: it
does not make per-pair signs stable, but it does guarantee both sides are sampled inside every
window, which is what makes a minimum meaningful at all. And a tolerance added to fight noise has to
be calibrated on the noisiest cell, not on the local machine - the local run this rule was checked
against showed 2.21-2.33x with no inversion at all, so no amount of local re-measurement would have
caught it.

## Note survival: the leading slot became a list (2026-10-04)

**The survival invariant is a gate now.** The decision recorded below — "landing it red would train
everyone to ignore the tier" — held for as long as inputs failed it, and they no longer do, so the
assertion is committed as `crates/pyrs-yaml-core/tests/note_survival.rs`: a deterministic replay of
the committed YAML seed corpus that requires every note the reader recorded to appear in the
emission **and** every input to reach a fixed point in one round. It runs under `cargo nextest`,
i.e. on every PR, which is where the fuzz tier's `-runs=0` replay of the same files already sits.
Measured coverage at commit time: **36** of the corpus's YAML seeds carry notes that the assertion
can act on (`former-crash-ce106ccc.seed` and `former-crash-7918272c.seed` among them), so the test
declares a floor of 30 rather than passing vacuously — a corpus that stopped carrying comments would
fail the coverage assertion, not silently satisfy it.

**Its blind spot is documented in the file, and it was found by mutation rather than by argument.**
Withdrawing the honest return value from `attach_inline_comment` — reintroducing the exact defect
that dropped `!x # note` — reddens the pinned shape tests and leaves the corpus gate **green**,
because the gate measures the notes the reader recorded, and a note lost during ingest was never
recorded. Making it instead count `#` in the raw source would red correct output: the corpus
contains `!###0 ##################################################, #######&b #`, where a tag suffix
is made of `#` characters and no comment exists at all. That is the third instance in this cycle
of "an oracle that reddes correct output is worse than none", and it is why the two halves stay
separate: the corpus test owns emission-side survival, the fuzz tier and the per-shape pins own
ingest-side survival, and neither is described as covering the other.

**One hypothesis about the relocation family has been tried and falsified, which is worth more than
a patch.** The gate that decides whether `write_mapping_pair` lifts a note or prints it in place
is "does this key already own a note", and in `crash-f8525a9e` two notes compete for one node chain:
an outer `#` reported with Right (inline) placement on the marker line, and an inner `!!"#~` that is
genuinely standalone and needs the lift. Routing by granit's placement flag — inline notes to
`meta.comment`, standalone ones to the list — *does* unblock the lift (the first emission then
already opens with `# !!"#~` at column 0, which confirms the diagnosis), but it breaks
`marker_note_settles_on_the_marker_line`, the pinned behaviour from crash-ac5d9043: a note trailing
a marker line is reported with Right placement while meaning the line **above**, so the flag cannot
be the router. The change was reverted rather than shipped with a red test, and the ingest site now
documents the rule where it lives — the stack is written flag-blind on purpose. What the next
attempt has to work with: the distinction is *which line the comment event arrived from* (its byte
offset against the marker's own line), not what placement granit labelled it, and the same source of
truth already decides `line_break_between` and `dedented_before` in the binding path.

**A separate, higher-severity defect from the same window is now closed.** `crash-68adf94c` renamed
an anchor between rounds: `bg: &b` + a comment line whose body contains `&?` + `~: ~` re-serialised
as `bg: &?`, and a renamed anchor silently orphans every `*b` alias — the class of #265 and
crash-04fddeb8. Reading the guard's actual predicate first (rather than inferring it from the
symptom, which this ledger has now paid for three times) showed why 04fddeb8's fix did not reach it:
the refusal compared only the **token immediately before** the `&`, so `# !! &?` — where the `&`
follows `!!` rather than hugging the `#` — slipped through even though a comment, once open, runs to
end of line. The guard now asks the question YAML itself asks: does a comment start anywhere earlier
on this line? A `#` embedded in a scalar still opens nothing, and the only shape that could
over-refuse (a quoted `#` earlier on a line that also carries an anchor) is unreachable, since node
properties always precede the value. Seeded as `former-crash-68adf94c.seed` — 34 bytes, as found:
the input no longer crashes, so `tmin` cannot shrink it — and pinned both ways at the predicate plus
end-to-end (`anchor_name_before_ignores_ampersand_anywhere_in_comment_text`,
`anchor_keeps_its_name_across_a_comment_line_holding_an_ampersand`, which asserts the anchor token,
the comment text and the one-step fixed point). Every pre-existing guard in `comment.rs` still
passes, so the broader refusal did not trade away the narrower cases. What remains open after it
was, at the time of writing, the **relocation** family only — `f8525a9e`, `456176be`, `e6551c75`,
`11ced252` — where nothing is lost and only the line a note sits on moves for one extra round. Two
of those four have since closed, through **two different root causes**, and the grouping that called
them one family was wrong again; see "Two more note placements closed" below.

The decision recorded above was taken and implemented: `NodeDecor.leading_comment: Option<Comment>`
is now `leading_comments: Vec<Comment>`, read through one normalised `NodeMeta::standalone_slice()`
(a slice, not a `Vec`, because `NodeMeta::eq` / `Hash` run on every `IndexMap` probe), with
`leading_comment()` / the Python `Node.leading_comment` still returning the first note and a new
`Node.leading_comments` exposing the whole list — non-breaking, as chosen. What that thread actually
contained was not one defect but five, each found by measuring rather than by reasoning, and each
silent because a text missing a note is a perfectly stable text: (1) the ingest overwrite itself —
`# alpha` + `# beta` + `key: 1` kept only `# beta`, in YAML and in JSONC (`// a` + `// b`); (2) a
document carrying nothing but comments dropped all of them, because `DocumentEnd` never fires when
there is no node (`#&l<TAB><TAB>:` → `null`, seed `former-crash-fd1938f7.seed`); (3) the null-key
fold deleted the folded entry's comment with it (seed `former-crash-00e31785.seed` already covers
the input); (4) a consumed merge key took its comments down with it — a hole the merge-identity fix
above had just made reachable, since the key only became visible to the pass at the moment it could
be consumed (seed `former-crash-953bf87a.seed`); (5) so did the mapping the merge consumed (seed
`former-crash-f453c4e5.seed`). Notes are re-homed now, never discarded. (4) opened one more: a
mapping could then hold a plain `<<` and a `<<` carrying a note side by side, which `IndexMap`
(whole-node keys) kept as two entries while the merge pass addresses one, so a pair vanished per
round — `<<` folds like `~` now, seeded as `former-crash-973bd522.seed` and pinned by
`duplicate_merge_keys_fold_to_a_fixed_point`; that the identity fix caused it is inferred from the
mechanism (before it, neither spelling was a merge key at all), not reproduced from a reverted
build. The oracle that found (2)–(5) counts note bodies in the input and requires each to appear in
the output, and it is **deliberately not committed with this change**: four inputs still fail it.
Shipping a blocking assertion that is red on day one would train everyone to ignore the tier. It
earned its keep before being shelved — putting the overwrite back made it report exactly
`output dropped ["alpha"] of the input's notes`, which is the proof that it sees the class the drift
tier cannot. Before trusting it I swept it against the committed corpus, and it produced three false
positives that each taught something: it counted a note the writer merely *re-indented* under its
parent key as lost; it split lines on `\n` while YAML also breaks on `\r`; and it compared raw text
while the receiver filters unrepresentable code points out of note bodies. An oracle that reddes
correct output is worse than none. **The CI consequence is recorded, not hidden:** with every fix in
this cycle landed, all 64 committed seeds replay CLEAN on the four targets (69 today, after
`former-crash-{11ced252,456176be,fbc8f2ae,f8525a9e,c9031de4}.seed`, and 72 after
`former-crash-{22cb5f67,e6551c75,8f7085b0}.seed` — the post-(l)(m) replay reports 5/56/6/5 clean on
the four targets), while the 60 s discovery window still lands on this family every run — the fuzz
tier is doing its job, and until the relocation is closed the weekly finding pipeline (not the PR
gate) is where those inputs belong. Re-checked on the post-list tree: `f8525a9e`, `456176be` and
`11ced252` fail the **idempotence** assertion outright, and `ce106ccc` is clean there and fails only
note survival — so "the oracle found things the drift tier cannot see" holds for `ce106ccc` alone,
and the other three were already drift failures rather than regressions introduced by the list work
(the same relocation was recorded for crash-f8525a9e before any of it).

**Filed open at that moment, with the artifacts kept as evidence — two of the three have since
closed (root causes (j) and (k) above):** `crash-f8525a9e` (a stacked note rides a marker body and
swaps position with the lifted line for two rounds before settling — closed by (k)),
`crash-ce106ccc` (a tag followed by a wall of `#` columns loses the note — the survival oracle,
stable text; still open), and `crash-e6551c75` (60 bytes): two note lines that sat under a
container's own tag header are hoisted above it as a pair on the next round — still open, and the
only drift input left (see the paragraph above for the named mechanism and the reason the candidate
fix is deferred). **Closed with it, in the TOML spoke:** `pending_leading: Option<String>` carried
the same overwrite per key line — measured before the change as `# a` + `# b` + `k = 1` → `# b` —
and so did the empty container's single inline slot, which dropped the rest of a note stack (`# d1`

- `# d2` → `{}  # d1`). Both are lists now, and the documented "only the last block survives /
  matches `toml_edit`'s decor handling" claim is gone rather than quietly kept: every own-line note
  above a pair, a section header, or an inline-table member is preserved, `to_toml` reproduces them,
  and one TOML round through the hub is stable (`TestStackedComments`). One consequence landed on a
  characterization test that had been pinning the old loss: a comment on a table-header line still
  cannot stay on that line, but it is no longer relocated to the document head either — the
  container-inline-note fix below puts a container's own note on the last line of its body, so the
  note now stays inside the table it annotates (`x = 1 # note`) and one TOML round through the hub
  is stable there, and `TestSectionHeaderCommentBoundary` was rewritten to pin survival + the
  in-table slot + that stability; the "Known engine boundaries" entry above was rewritten with it.
  **Still open in the hub:** notes inside a flow-style inline table reach the AST but are not
  emitted by the YAML flow writer at all — measured with a SINGLE note (`t = {` `# i1` `a = 1 }` →
  `t: {a: 1}`), so it is pre-existing, not a side effect of the list work, and it needs a decision
  about how a flow collection renders member comments.

The scheduled loop is proven end-to-end: each `workflow_dispatch`/cron run fuzzes the four surfaces
and, on a crash, uploads the minimized artifact for triage (findings #226/#227 were surfaced and
fixed this way). The engine is never declared "clean" — the point of the schedule is that it keeps
surfacing new edge cases to pin, one root cause per PR.

---

### (az) The width fixer rewrote page metadata, and a fold printed its own markup (2026-10-10)

Two defects shipped together, and the report came from a reader of the site rather than from a gate.
`https://759401524.github.io/pyrs-yaml/zh/changelog/` showed a released version's heading running
into its body (`pyq CLI 对齐参数#### 新增`). Reading the page for what else had gone found the
second thing: the changelog's `<meta description>` was the site's, not the page's.

The first is older than the second and was bisected to (av)'s own commit: `paragraph_blocks`
documents that front matter stays, and it skipped only the *opening* `---`, so the block's body was
a paragraph - joined, balanced, re-wrapped. `title: Changelog description: … tags:` on one line is
not a mapping, and four pages, `docs/{en,ja,ko,zh}/changelog.md`, carried that through three merged
pull requests.

What the generator does with it is not one answer, and the difference is the part worth keeping. The
locked version (`zensical` 0.0.56, what CI and the deployed site build) **accepts** the page: build
exits 0, and the `<meta name="description">` of the changelog becomes the *site's* description -
`High-performance Python YAML library with perfect round-trip support…` where the page had said what
the changelog holds - with the tags gone with it. The title is unaffected, because the title it
prints is the one the damaged line still begins with. The newer 0.0.69, which a local
`uv run --group docs` pulled in by resolving an unbounded constraint, **refuses**:
`error reading page metadata 'changelog.md'`, build fails. So the same bytes are a silent metadata
loss on the version that ships and a broken deploy on the version the lock will move to. A green
`Docs` run proved nothing in either case; the first failure mode is invisible, and it is the one
readers got.

The gate could not see it because the damage *satisfied* the gate: after the join, every metadata
line fitted inside 100 display columns. A check that measures width cannot notice a loss of
structure, and the only reader that cares - the site build - runs on push to `main` in
`.github/workflows/docs.yml`, never on a pull request. So the repair is stated in three places:
`front_matter()` is now one range shared by `offenders`, `spacing_artifacts`, `wrap_text` and
`paragraph_blocks`, so the checker and the fixer cannot disagree about which lines are data;
`scripts/check_doc_metadata.py` asks the generator's question structurally (one key per line, no
duplicate key, a `title` present) with no third-party dependency, wired as the `doc-metadata` hook
and as a `page-metadata` job in Validate; and `tests/test_doc_wrapping_gate.py` pins the block
byte-for-byte while asserting the prose in the same document still gets re-flowed, because a pass
that skipped everything would pass a metadata-only test by accident. The four blocks were restored
from the newest revision that parses (`89979030`), and each restore was proved to be a repair rather
than a substitution by comparing the whitespace-squashed text before and after - identical, so
nothing but the joining was undone.

The second defect is the fold itself. `<details>`/`<summary>` is the shape everyone uses on GitHub,
where the body is parsed as Markdown whatever the markup looks like; this site is built by a
Python-Markdown pipeline, and `md_in_html` parses the content of a raw HTML block only when the
opening tag carries the `markdown` attribute. Without it all 21 folded releases printed their `####`
headings, bullet lists and code fences as text - correct on GitHub, broken on the site, and
invisible to every source-level gate for the same reason as the first defect.
`<details markdown="1">` is inert on GitHub and load-bearing here, so the five pages now carry it
uniformly, and the checker refuses a bare `<details>` under `docs/`.

The measurement that decided each fix, not the story, and on the generator the site is built with.
The fold was tested by A/B with everything else held equal: the same zh page built with
`<details markdown="1">` renders 69 `<h4>` and no leaked markup, and built with a bare `<details>`
renders 11 while printing `#### 新增` and `#### 变更` as text inside the fold - 21 `<details>`
elements either way, so the fold *opens* and only its body is lost, which is why a glance at the
page looked merely untidy rather than broken. The metadata was tested the same way, per version, and
is the paragraph above; the numbers there are why "CI is green" is not a statement about the page.

A bug in the new test was found on the way and is worth recording, because it is the common shape of
a useless gate: `pages()` listed `docs/<locale>/**/*.md`, a git pathspec that skips a locale's own
top level, which is exactly where the four changelog pages live - 140 pages selected, every one of
them clean, and the check protecting nothing it was written for. It is now 164 pages, and the
coverage test names the four changelog paths instead of trusting a count, which is the only reason
the hole was noticed.

What this does not close: no pull request renders the site, so a property that only the *renderer*
decides is still checked here by rules about markup rather than by markup being rendered. The exit
criterion for that - a PR job that runs `scripts/build-docs.py` - is registered in
`.ci/quality-holes.json` as `docs-rendering:unbuilt-on-pr` rather than left as an impression.

### (ba) The docs toolchain was upgraded properly, and the upgrade reported that nothing uses it (2026-10-10)

`zensical` 0.0.56 - the version `uv.lock` pinned and the deployed site was built with - to 0.0.69,
and `mkdocstrings-python` to 2.0.9 (the newest stable releases of both, with `mkdocstrings` 1.0.6
and `griffelib` 2.2.0 resolved with them). Both require Python 3.11, so the docs group is now marked
`python_version >= '3.11'` instead of resolving an unusable old version for the interpreters that
cannot run it.

`maturin` was held at 1.14.1 on purpose, and that is the part of this entry worth the reading.
Getting here cost the same mistake twice: an unpinned `uv run --group docs` re-resolved the whole
project, moved `maturin` to 1.15.0, rewrote `pyproject.toml` and `uv.lock`, and a documentation pull
request arrived at `Committed type stub is derived` red - because that job regenerates the `.pyi`
with whatever the lock holds, and the reconciliation rules in `check_stub_drift.py` describe
1.14.1's output. A dependency change that rides along silently is indistinguishable from a code
change in the artefact it breaks. So `tests/test_docs_config_gate.py` now asserts the locked
`maturin`, with a message naming the route to take if it moves deliberately, and the same file
asserts the two docs tools are at least the version the manifest asks for - read from the lock, not
from whatever happens to be installed, because CI resolves the lock and a half-upgraded pair is
exactly the state that was almost shipped.

The compatibility question was answered against the installed handler rather than a changelog. Its
`PythonOptions` exposes 67 fields and every one of the fourteen keys configured in `zensical.toml`
is among them, so no rename or removal landed on this configuration. What the check is really
guarding is the other half of the measurement: adding `this_option_does_not_exist = true` leaves
`zensical build --strict` exiting 0 with byte-identical output, so an option the handler has never
heard of is *silently ignored*, and a configuration that outlives a version produces no signal at
all. (A duplicated key is different - the TOML parser rejects it and the build fails, which is why
the earlier probe of this saw `rc=1` and had to be re-read: the probe was injecting a duplicate, not
testing an unknown name.)

Then the finding that reframes the request. `mkdocstrings` is configured, installed and invoked by
the build - and renders nothing, because no page in `docs/{en,ja,ko,zh}` contains a single `:::`
directive; `docs/<locale>/api/*.md` are hand-typed signatures under `#### `name()`` headings.
Upgrading a generator cannot improve output it is never asked to produce, and the fourteen render
options are decoration until something asks for them. So the fact is registered rather than noted:
`quality_matrix.mkdocstrings_declared()` against `mkdocstrings_directives()` emits
`docs-generation:plugin-unused` while the config and the tree disagree, with the exit named in both
directions - a page that uses the handler, or the handler's configuration deleted. Closing it is a
content decision with an i18n consequence (every page under `docs/en` owes zh/ja/ko twins), which is
precisely why it is written down rather than half-started here.

Two probes were wrong before they were right, and both are recorded because a measurement that
quietly replaces itself hides the reason for the second one. The first feature-comparison script
reported zero pages for *every* variant - it rewrote a `site_dir` line the real config does not
contain, so the builds went to the normal output directory and the probe read an empty path; it
produced a uniform, tidy, completely false result. The second reported the same numbers for the
unknown option as for the real ones, which is the expected answer once the path is fixed - and is
also the reason the option-name check lives in a test rather than in the build. The third probe
tried to compare the hand-typed API signatures against the generated `.pyi` and died on a regex with
three groups and a two-tuple unpack: the drift comparison remains open, and is the substance of the
registered hole rather than something to assert about.

### (bb) The type stub shipped inside every wheel was not parseable Python (2026-10-10)

Found by asking the previous entry's question properly. If `mkdocstrings` renders nothing because no
page asks it to, then the first thing to establish is whether it *could*:
`griffe.load("pyrs_yaml", search_paths=["python"])` parsed the package, resolved the pure-Python
`pyrs_yaml.node.Node` with its docstring, and raised `AliasResolutionError` for every re-exported
extension class - `YamlDocument`, `YamlParseError`, `YAML`. The extension submodule was not
discoverable at all, which is the same thing as the generated stub being invisible.

It was invisible because it is not Python. maturin 1.14.1's stub route imports the freshly built
extension and writes each `__doc__` into a triple-quoted string *verbatim*, so a doc comment
containing a backslash lands unescaped - and a lone `\u` inside a non-raw string literal is a syntax
error. Two of this project's Rust doc comments contain one: `crates/pyrs-yaml/src/py/document.rs`
describing what `to_json` no longer does, and the JSON5 loader's note about exotic spellings.
`ast.parse` and `compile` both fail on `python/pyrs_yaml/pyrs_yaml.pyi` at the first of them, with
the error reported against a line that holds nothing but an opening delimiter - the parser blames
the `"""`, not the escape, which is why the message matters more than the fix.

The blast radius is not cosmetic. The stub is marked by `py.typed` and ships inside every wheel, so
mypy and pyright users are reading a file they cannot parse, in the one artifact that defines the
public typing contract; and no documentation generator can reach the API through it either. With the
two backslashes doubled, every layout resolves: a stub-only package gives `YamlDocument` as a class
with 55 members, and the repository's own mixed `.py` + `.pyi` layout resolves it through the alias.
That is the difference between the handler's 67 options being decoration and being a generator.

Every gate was green, which is the part worth keeping. `Committed type stub is derived` compares the
tracked file against generator output - and the generator carries the defect, so the comparison
agreed. Two copies of a broken artifact matching is not a check; it is the same shape as (az), where
the width rule was satisfied by the damage the fixer caused. Nothing anywhere asked the artifact the
one question a type checker asks: does it parse.

The repair lives in the route, not in the artifact, because the artifact may not be hand-edited. A
second declared fidelity transform escapes backslashes inside docstring bodies - counted, with
`EXPECTED_DOCSTRING_ESCAPES = 2` as the tripwire, so a third unescaped site or an upstream fix that
removes one fails the gate instead of being absorbed - and `verify_parses` then runs `ast` over the
derived text, so a generator misbehaviour that no declared fix describes yet is reported by line
number. `--fix` rewrote the stub from the pinned generator; a re-run of the route reports no drift,
and `tests/test_stub_fidelity_gate.py` (5 tests) pins both the transform on synthetic generator
output and the parsed artifact itself, including the assertion that the escaped docstring still
*says* `\uXXXX` - the escape is Python source syntax, not a change of content.

One defect is named here and deliberately not fixed. Under inspection rather than stub reading,
griffe reports `builtins.YamlDocument`: the PyO3 classes do not set `#[pyo3(module = …)]`, so
`__module__` is `builtins` for every extension type. It is harmless while the stub resolves, and it
is not harmless for `repr()`, for anything that introspects a wheel built without stubs, or for a
generator that prefers live objects. It belongs in a binding change with its own stub regeneration,
not smuggled into a repair.

### (bc) The release notes described their own first line, and an outside review said so (2026-10-10)

An external review of `ROADMAP.md` and `CHANGELOG.md` arrived as a link, and the useful response was
to measure each claim against the tree rather than accept or argue with the opinion. Four claims
held, one was false in its premise, and one was false in a way that would have re-broken the site.

- **Every fold summary is its version's first entry** - true, and the worst case is v0.16.0, whose
  summary read `JSONC block-comment hot-sample bench` over 93 entries in five sections. Measured for
  all 21 folds by comparing the `<summary>` text with the first bullet that follows it: **21 of 21
  matched**. A changelog that folds its history is a table of contents, and this one only ever
  listed line one.
- **Folding is inconsistent** - true: 24 releases, 21 folds, and the three outside the pattern are
  exactly `0.11.4`, `0.11.3` and `0.1.0`. They fold like the rest now, in all five mirrors.
- **`markdown="1"` is legacy GitHub syntax, drop it** - false here, and following it would have
  returned the defect recorded at (az). The attribute is for Python-Markdown's `md_in_html`, not for
  GitHub; measured on the deployed generator a bare `<details>` yields 11 headings and 3 literal
  `####` leaks where `markdown="1"` yields 69 and 0. `scripts/check_doc_metadata.py` refuses a bare
  `<details>`, so the advice would not even have passed this repository's own gate.
- **The research history is duplicated between `ROADMAP.md` and this file** - false in its premise:
  `9C9N` occurs eight times in `ROADMAP.md` and not once here, and `26.8`, `29.7`, `0.42` and
  `granit 1.3` are only in the roadmap. Nothing had been migrated. The *shape* it argues for -
  roadmap keeps the decision and a link, the record keeps the experiment - is the one already
  agreed, and it stays open rather than being half-done in a pull request about something else.
- **`[Unreleased]` had been compressed into slogans** - true as a reading, and also what was asked
  for at the time (40-80 characters plus a pointer). The more detailed granularity has since been
  chosen, so entries carry the behaviour, the API name and the number - `1: a` versus `a: 1` and
  what each used to load as; 21,232,786 / 22,176,851 / 22,974,851 instructions with the 4.4% and
  8.2% costs; the `139.4us vs 359.6us (2.58x)` report - and keep the pointer. The method narrative
  stays here.

Restoring the detail from history rather than rewriting it from memory matters: the block at
`89bbfbee^` holds 112 entries against the compressed 32, so the numbers and API names are the ones
written when each change landed. The pairing cannot be positional - the compression merged several
entries per line - so it was made by reading, which is also why the two shapes differ in count by
design.

Two process notes. The mirror gate compares per-section entry counts *by position* while allowing
translated section names, so four language drafts have to move in lockstep - the drafts are
generated from one table rather than hand-copied five times. And `check_cjk_localisation.py` caught
a Japanese katakana word that my own Korean repair table had injected: kana in a `ko` page, reported
with its line number. The mirror and the table that produced it were both fixed, because a repaired
artifact with an unfixed producer is not a repair.

### (bd) A documentation-only pull request could not be merged at all (2026-10-10)

Found by trying to merge one. PR #319 - changelog prose, nothing else - came back with every check
it produced passing and `gh pr merge` refusing:
`GraphQL: Required status check "Test matrix (all legs)" is expected`. The repository settings
explain it exactly: `required_status_checks.contexts == ["Test matrix (all legs)"]`, `strict: true`,
`enforce_admins.enabled == true`, `required_approving_review_count: 0`. The one required context
lives in `ci.yml`, and `ci.yml` triggered on `pull_request` with
`paths-ignore: docs/**, *.md, AGENTS.md, CHANGELOG.md, prek.toml, .rumdl.toml` - every path a prose
change touches. The workflow never ran, the check never reported, and `--admin` was refused as well:
with admin enforcement on, an administrator cannot bypass it either.

The general shape is worth naming, because it reads as a broken pipeline rather than as a rule: **a
required check that never reports is not a green light, it is a deadlock.** The four prose pull
requests before it also touched `scripts/`, `tests/` or `pyproject.toml`, so CI ran for an unrelated
reason and nothing showed.

The fix keeps the gate honest instead of routing around it. `ci.yml` triggers on every pull request;
a `changes` job (unconditional, one checkout, `git diff --name-only` with `:(exclude)*.md` and
`:(exclude)docs/**`) classifies the changeset; the heavy legs are conditioned on that
classification; and a `docs-gates` job that always runs does the work a prose change actually owes -
page metadata, headings, mirror counts, script purity, i18n, prose width, then
`scripts/build-docs.py` for all four locales under `--strict`.

`check_matrix_verdict.py` takes the classification as a second input and the tolerance is
deliberately narrow: a `skipped` leg is excused only while `CODE_CHANGED=false`, only outside
`ALWAYS_RUNNING = ("changes", "docs-gates")`, and never for a `failure` or `cancelled`. A missing or
garbled classification reads as "code changed", so the exemption cannot be obtained by not
answering. That is the whole difference between a documented exemption and a hole - the skipped leg
has to be attributable to a job that itself must succeed.

`tests/test_matrix_verdict_gate.py` grew 5 tests over exactly those shapes (prose-only excuses the
heavy legs; the classifier and the docs gates are never excused; a red leg stays red under either
classification; the CLI defaults strict and an explicit flag beats the environment), and the fan-in
test now enforces the wiring instead of trusting it: every leg reading `needs.changes.outputs.code`
must declare `changes` in its own `needs`, because GitHub otherwise resolves the reference to an
empty string and skips the leg silently - which would make the exemption self-attesting. The
classifier is asserted unconditional.

Closing this also closed the hole registered at (az) as `docs-rendering:unbuilt-on-pr`: four
changelog pages carried metadata the generator cannot parse through three merged pull requests
because nothing before merge had ever rendered them. `renders_the_site()` derives that fact from the
workflow files, finds the new job, and so the registry entry and its `QUALITY_MATRIX.md` row came
out - the registry fails in both directions, and an obsolete entry reads as an open one.

The new job went red on its first run, and the reason is the kind only a runner teaches:
`error: Failed to spawn: prek / No such file or directory`. The step read `uv run --no-sync prek`,
which assumes the tool lives in the project environment - `prek` is a tool, not a dependency, and
`hygiene.yml` provisions it with `uv tool install --from 'prek>=0.1.0' prek`. The equivalent step
passed locally, as it always will here, because this machine has `prek` on `PATH` from an unrelated
install. A gate written against a tool the job never installs is a gate that exists on one machine,
which is the sentence this file exists to repeat.

### (be) The API reference is generated now, which is the thing the toolchain upgrade was for (2026-10-10)

Two entries earlier the docs toolchain was taken to the current stable releases, and the honest
finding was that nothing could be observed improving: no page under any locale contained a single
`:::`, so the handler was configured, installed and invoked while rendering nothing. That is the
blind spot this entry closes, and the sequence matters - it could not have been closed before (bb),
because the generated stub was not parseable and griffe therefore could not see the extension's
classes at all.

Five pages per locale now carry directives - the module reference and the `YamlDocument`, `YAML`,
`Node` and `MergedView` classes - in all four locales, because the pages are one set and the site is
built four times. The measurement is the build's own output, not the exit code: `yaml-document`
renders 24 content blocks with 23 signatures, `node` 41 and 41, the module page 47 and 48, and the
control page that names no directive stays at zero - which is what makes the count evidence rather
than a number that would look the same if the generator had produced nothing. A rendered signature
reads `__contains__(key)`, `__delitem__(key)`, `__enter__()`, and the class docstring is the one
carried in the generated stub.

The member list on the module page was extracted, not written: `docs/en/api/reference.md` documents
33 symbols, 30 of which resolve as members of `pyrs_yaml` under the same loader the handler uses,
and those 30 plus the module's own classes are what the directive names. The three that do not
resolve (`dump_pydantic`, `parse_as`, `PyrsYamlConfigSettingsSource`) live in submodules with their
own pages, so putting them in a module-level `members:` list would have been a build error
manufactured out of a plausible looking list. The check runs before the pages are written, so an
unresolvable name is a refusal here rather than a mystery in CI.

The first attempt did produce exactly that mystery: the directive's `options:` block had its list
items at the same indentation as their key, and `zensical build` died inside the directive's own
YAML. A green local run of everything else was no help, because none of the other gates read a `:::`
block. Recorded because the shape of the lesson is the one this file keeps repeating - the renderer
is a gate of its own.

`exceptions.md` deliberately stays hand-written. The generated stub declares four classes and none
of the exception types, so `::: pyrs_yaml.YamlParseError` cannot resolve: maturin 1.14.1's
introspection route does not emit the PyO3 exception classes it imports, which means the public
typing contract has no exception types in it either - the same file users' `py.typed` promises to
their type checkers. That is a defect wider than documentation, and it is the subject of its own
entry rather than being quietly worked around here by hand-editing a generated artifact.

The hole `docs-generation:plugin-unused` came out of `.ci/quality-holes.json`, and its
`QUALITY_MATRIX.md` row became a closure row. `quality_matrix.py` now derives zero holes, which the
registry's own two-directional equality test accepts - an empty list is a measured state, not an
aspiration, and the probe still fires on an injected tree that declares a handler without using one.

One repeat mistake belongs here because this file has now caught it twice. Re-wrapping prose for the
width gate was run over a hand-written file list that included `QUALITY_MATRIX.md`, which the hook
does not govern, and the result was a 357-line diff that re-flowed a Markdown table. The governed
set - `^(CHANGELOG|ROADMAP)\.md$`, `docs/<locale>/changelog.md`, `docs/dev/[a-z-]+\.md` - lives in
`prek.toml`; a list typed next to a command is a copy of that rule, and copies fall behind it. The
fix is to run the fixer through the hook (`prek run doc-line-width --all-files`), the way `ci.yml`'s
new `docs-gates` job does, rather than naming files in the middle of a change.

### (bf) The shipped typing contract declared no exception type at all (2026-10-10)

Found through a docs failure. (be) left `exceptions.md` hand-written because
`::: pyrs_yaml.YamlParseError` would not resolve, and the reason turned out to be wider than the
docs: the package exports ten error types - nine `*Error` plus `YamlTagSkip` - and
`python/pyrs_yaml/pyrs_yaml.pyi` declared **none of them**. Measured before acting: the names
`__init__.py` re-exports from the extension, the `class` lines the stub contains (four, none of them
errors), and the runtime picture of each object - `__module__` `pyrs_yaml`, seven bases
`ValueError`, one `TypeError`, `YamlTagSkip` under `YamlTagError`.

That file is what `py.typed` advertises. A user running mypy or pyright was being told the library
has no exception types, so `except pyrs_yaml.YamlParseError:` - the single most likely thing anyone
writes against this package - was invisible to the tool checking their code. maturin 1.14.1 builds
stubs by introspecting the module, and a PyO3 `import_exception!` class is not the kind of object
its emitter walks, so nothing reached the output.

The repair follows the rule this file keeps repeating: fix the route, never the artifact.
`check_stub_drift.py` gained `exception_block`, which appends the declarations **derived from the
live classes** - name, single base, docstring, backslashes doubled the way a Python literal
requires. Nothing is hand-listed, so adding an exception to the bindings adds it to the contract on
the next run, and `EXPECTED_EXCEPTION_CLASSES = 10` is the tripwire that makes that change a review
rather than a silent rederivation. Two details make it correct rather than merely convenient:

- `order_by_base` places a declared base before its subclass, because the output is Python source
  and `class YamlTagSkip(YamlTagError)` does not parse with them the other way round. A base outside
  the set is fine - `ValueError` is the reader's builtin - and a cycle between declared classes
  raises `StubInputError` instead of emitting a file that cannot be read.
- If the extension cannot be imported, the derivation exits 2. It is deliberately *not* allowed to
  produce a stub without the exception block: a green gate that occasionally means "the declarations
  were skipped today" is worse than a red one, and exit 2 distinguishes "could not be derived" from
  the 1 that means "drift".

`exceptions.md` is now generated in all four locales too - ten directives, ten rendered entries per
locale in the built HTML, alongside the curated prose that explains when each error is raised, which
no stub can carry.

The first run of this on a runner went red, and it is the same lesson (bd) recorded in a different
dress: `Committed type stub is derived` failed with `No module named 'pyrs_yaml'` while the
identical command was green here. `maturin generate-stubs` builds in an isolated environment and
leaves nothing importable in `.venv`; `maturin develop` - which `docs.yml` already uses for the same
reason - does. The job installs the built extension before deriving now, and
`crates/pyrs-yaml/src/py/**` joined its path triggers because a binding change is exactly what the
derivation reads. The alternative was to hard-code the ten names in the script, which is how a
contract quietly drifts from the bindings it describes, so the precondition is stated as a step
rather than assumed. Local green said nothing about the runner's environment - the fourth time this
file has had to write that sentence, and the reason every gate here is phrased as an executable
question.

Three tests wrote themselves wrong before they wrote themselves right, and the shapes are the useful
part. One monkeypatched `extension_exceptions` to return a fixture and then asserted the ordering it
was patched away from doing - a test that passes by construction; the ordering now lives in
`order_by_base`, called end-to-end through a fake module. One asserted that an out-of-set base is
refused, which the design intentionally accepts. One called `main([...])` on a script that reads
`sys.argv`. And the regression lock for the artifact - every exported error name must appear as a
`class` in the shipped stub - reads the package by syntax tree, not by import, so it holds on a
machine where the extension is not built; on a tree that still had the gap, it fails.

### (bg) The stub parsed and type checked nothing alike, so a checker joined the gate (2026-10-10)

(bb) made the shipped stub parseable, (bf) made it declare its exception types, and after both a
type checker still reported **five errors inside our file**: `Name "Callable" is not defined`,
`Name "u32" is not defined`, and three `Invalid type comment or annotation` for `Py<PyAny>`. Every
gate in the tree was green, including the `ast.parse` check added at (bb) - which is the point.
`ast.parse` proves the file is Python; a `.pyi` is not Python, it is typing, and the only question a
user's editor asks is the second one. Those five diagnostics are attributed to this library in the
file `py.typed` advertises.

The cause is the same generator, a different symptom: maturin 1.14.1 copies Rust-side spellings into
annotations, and writes a `typing.Callable` reference without importing `Callable`. Fixed in the
route, in the shape the other two fixes already had: `rewrite_rust_spellings` maps `Py<PyAny>` to
`Any` and `u32` to `int` inside quoted annotations, counted against `EXPECTED_RUST_TYPE_SITES = 4`,
and `add_typing_imports` merges whatever an annotation references and the file never defines into
its `typing` import. A name that maps to nothing in `typing` is a failure rather than a guess,
because emitting `from typing import Whatever` trades an undefined name for an import error.

That check has its own lesson. The first version scanned *every* quoted string in the file for
names, and reported `AST`, `Accepts`, `Community` - words from docstrings - as undefined types.
`ast` distinguishes an annotation from a sentence, so the scan walks annotations and parses the
quoted ones; the earlier version would have had to be loosened until it stopped meaning anything,
which is how rules die.

`scripts/check_stub_types.py` is the gate. It builds the scratch package a user's tooling sees -
`py.typed`, the committed stub, a module importing the library - because pointing mypy at a bare
`.pyi` is a different question and crashes outright on this file. It parses output rather than
trusting exit codes, distinguishes "our file has findings" (1) from "the check could not run" (2),
and treats a run that examined no files as a failure. Both checkers run in the stub-drift job,
pinned in the invocation (`uv run --with 'mypy==2.4.0'`, `--with 'ty==0.0.85'`) rather than added as
project dependencies, the way `hygiene.yml` pins `prek`.

**Should `ty` replace `mypy`? Measured, no - and it joins as a second opinion.** Pointed at the
known-broken artifact, `ty` catches all five defects under its own rule names
(`unresolved-reference`, `invalid-syntax-in-forward-annotation`). But three things keep it from
being the sole authority, each measured rather than assumed:

- `ty check <directory>` - the invocation anyone would write - answers `All checks passed!` with
  `WARN No python files found` for a package whose only content is a `.pyi`, and exits 0. A gate
  that passes by examining nothing is the exact failure this file documents over and over, so the
  script names the file.
- Its exit code is not usable as-is: with the correctness rules selected and zero findings for our
  file, it still returns 1, because it also counted rules the gate deliberately does not enforce.
- Unscoped, it reports **32 further findings** on the repaired stub: 29 `missing-type-argument`
  (bare `dict`, `list`) and 3 `missing-override-decorator`. Those are accurate observations about
  generator output, not noise, and they are recorded here rather than silenced or turned into a red
  nobody clears. mypy is also the closer match to what users run, and the contract's audience is
  users.

`ty` paid for itself immediately, though: `missing-type-argument` fired on `def __next__(self, /) ->
dict
|None`, and that spelling is not the generator's - it is text *this* repository's fidelity fix writes. The
bare `dict` is now `dict[Any, Any]`, which is what the binding returns and what ty was asking for. A
tool found an imprecision in the tool that fixes tools.

Two workflow facts, because they cost time and would cost the next person the same:

- An edit to `check_stub_drift.py` (547 lines, five transforms) silently restored the file from a
  stale buffer, dropping ~250 lines while reporting a two-line change. `target/probe/funcprobe.py` -
  which lists the route's expected functions and constants - exists now and is run after every edit
  to that file.
- A file created inside a `jj` change that is later rewritten with `jj restore`/`jj new` disappears:
  `check_stub_types.py` was written, then gone, and had to be recreated. Untracked-in-a-change is
  not the same as safe.

Two things went wrong while writing this entry, and both are the record's, not the author's.

The width fixer was run over a hand-typed file list that included `AGENTS.md` - a file the hook does
not govern and whose 170-line re-flow buried the one line that had changed - thirty lines of ledger
after (be) wrote the sentence "a list typed next to a command is a copy of that rule, and copies
fall behind". Restoring the file fixed the diff, not the lesson: the guard is
`prek run doc-line-width --all-files`, which `docs-gates` already runs, and naming files for that
script is now known to be wrong even in the session that wrote the warning.

A created file also vanished once: `check_stub_types.py` was written into a change, the change was
rewritten during the (bf) sequencing, and the file was gone without a trace. `jj` snapshots the
working copy, so "added but not yet in a commit you keep" is not a state to rely on mid-rebase.

The gate then went red on CI while staying green at the desk, for a reason this ledger has now
written down three times in different clothes: `subprocess.run(["ty", "--version"])` on a machine
without `ty` does not return a non-zero code, it raises `FileNotFoundError`. Neither checker is a
project dependency, so every `ci.yml` leg that runs pytest met the raise, and the author's machine -
where `ty.exe` happens to sit in `~/.local/bin` - met nothing. Absence has to be an answer rather
than an exception: the gate reports 2 ("could not check"), the tests skip, and
`test_an_absent_binary_is_answered_not_raised` asks for a binary that is really missing, so the
catch cannot quietly narrow back to `returncode`. Proved by subtracting the tool's own directory
from `PATH`: before the change 2 failed with `FileNotFoundError`, after it 11 passed / 4 skipped /
rc 0.

A sweep for the same class across `scripts/`, `tests/` and `python/` found exactly one bare-tool
site - the guarded one. `git` and `cargo` are deliberately not in that category: they are
preconditions of the workflows that invoke them, and a leg without them fails loudly elsewhere.

Verified: `uv run --with mypy==2.4.0 python scripts/check_stub_types.py` and the same with ty both
report the committed stub clean; against `origin/main`'s stub both report 5 findings and exit 1 -
the negative control is a test (`tests/test_stub_types_gate.py`), not a manual step, and it refuses
to pass when neither checker is installed. With `ty` removed from `PATH` the same file skips (11
passed / 4 skipped) and the gate exits 2, so "green on the machine that authored it" is no longer
the state being shipped. Route re-derives with no drift; `pytest tests/ -q` 2334 passed / 10
skipped; changelog counts identical across five mirrors.

## Shipped milestone scoping (v0.11.3 → v0.12.0)

The planning tables `ROADMAP.md` carried after their milestones shipped. They stay because the
*reasons* are the useful part - which scope closed empty, which audit settled the argument, which
decision was deferred and against what date - and a plan silently deleted at release leaves the
next reviewer without the previous reviewer's evidence.

### v0.11.3 — "Streaming Write + Process Hardening" (target: Q3 2026)

> Complete the big-file story v0.11.2 opened (read is constant-memory, write still isn't) and close
> the two process debts flagged in the 2026-08-02 closure that caused v0.10.0-class release
> failures.

| # | Item | Layer | Priority | Notes |
|:--|:-----|:------|:--------:|:------|
| 1 | **Streaming write** — `YAML.dump_stream(file_obj, iterable)` / `dump_file(path, ...)`: serializer emits events chunk-by-chunk to a Python file object; constant memory on 100MB+ output | Rust + Python | 🔴 | ✅ Commits `061ebfd`/`11bdb80`/`7e6e821` |
| 2 | **Line-offsets cache** — carry `compute_line_offsets(source)` (src/parser/yaml/comment.rs:14) through the 5 edit primitives so an edit burst costs O(N+edit) not O(N×edit) | Rust | 🟡 | ✅ Commit `ef53ddc` |
| 3 | **publish.yml pre-release validation** — CI job on PRs touching the publish workflow (or `workflow_dispatch` dry-run) running `maturin build --release --generate-stubs` in a linux container, catching the v0.10.0-class stub failure before Release | CI | 🔴 | ✅ Commit `cb3c6fc` |
| 4 | **Changelog mirror sync check** — prek hook or CI job asserting root `CHANGELOG.md` `[Unreleased]` == `docs/{en,ja,ko,zh}` changelog mirrors | CI/Process | 🟡 | ✅ Commit `cb3c6fc` |
| 5 | **`with` context manager** for document scoping | Python | 🟡 | ✅ Commit `2bfc483` |
| 6 | **Compliance score reporting** — public `compliance_report()` surfacing the yaml-test-suite pass rate (tests gate at 75%) | Python | 🟡 | ✅ Commit `6599ee7` |

**Design decisions (2026-08-03)**: mmap-backed file streaming (read + edit without loading) stays
deferred (abi3 portability blocker). Community plugins / YAML Schema language stay in
`ROADMAP.md`'s Research & Exploration.
Line-offsets cache is an architectural optimization, not a fix (CodSpeed same-runner 3-branch showed
no real edit regression).

**Changelog mapping**: the `[0.11.3]` entries in the repository root's `CHANGELOG.md`.

---

### v0.11.5 — "Parser Robustness" (target: Q3 2026)

> Reframed from the original v0.12.0 "Compliance Improvement" items 3/4/5. The YAML Test Suite pass
> rate is saturated at **99.75%** (405/406 — only `ZYU8` fails, rejected by design), so these items
> no longer move the compliance metric. They harden rejection of invalid YAML edge cases beyond the
> suite, each bound to a strictness-audit probe corpus.

| # | Item | Layer | Fix approach | Priority | Status |
|:--|:-----|:------|:------------|:--------:|:------|
| 3 | **Indentation edge cases** — invalid indentation, wrongly indented line, block collection indentation | Rust (post-processing) | Pre-process input | 🟡 | ✅ Closed 2026-08-04 — audit found no fixable case |
| 4 | **Block mapping key detection** — did not find expected key, simple key `:` ambiguity | Rust (post-processing + granit) | Pre-process + granit patch | 🔴 | ✅ Closed 2026-08-04 — audit found no fixable case |
| 5 | **Flow context disambiguation** — mapping values not allowed in flow context, flow sequence `,`/`]` | Rust (post-processing) | Pre-process flow context | 🟡 | ✅ Closed 2026-08-04 — audit found no fixable case |

**Phase 0 strictness audit (decision gate) — result: EMPTY fix list → items close (2026-08-04)**:
these items have no in-suite target (all suite tests already pass). A 70-probe corpus (~20/bucket:
indentation, block-mapping keys, flow context) was compared against a PyYAML oracle via
`tests/test_strictness_audit.py`. The parser matched the oracle on **64/70** probes (26
reject-match, 38 accept-match). The 6 divergences are all **deliberate** and documented in the test:

- **5 accepted-by-us but rejected-by-PyYAML** — each is a YAML 1.2 spec or yaml-test-suite
  requirement where PyYAML is the outlier, not a laxness bug: empty mapping keys (`2JQS`, `CFD4`,
  `FRK4`, `UKK6` — suite requires accepting `: a`, `[ : empty key ]`), local tags (`C4HZ` — PyYAML
  fails only at constructor stage, not parse), implicit document after `...` (YAML 1.2
  `l-yaml-stream` grammar).
- **1 rejected-by-us but accepted-by-PyYAML** (`{a: 1, a: 2}`) — deliberate duplicate-key
  strictness; no suite test requires accepting duplicate non-empty keys.

Per the plan's risk note ("the audit records oracle disagreements but does not change our parser to
match PyYAML quirks"), none of these were changed. Fixing the 5 would **regress** suite compliance
below 405/406; fixing the 1 is already deliberate strictness. **No fixes shipped** — items 3/4/5
close with the audit corpus pinned as a regression test. Do not invent fixes to justify the original
~11d estimate.

**Design constraint**: granit-parser upstream may not be actively maintained; item 4 may require a
maintained fork. Unchanged — item 4 needed no fork because the audit surfaced no fixable case.

**Changelog mapping**: the `[0.11.5]` entries in the repository root's `CHANGELOG.md`.

---

### v0.11.6 — "numpy-free free-threaded wheel" (target: Q3 2026)

> Ship `cp314t` (free-threaded) wheels built with `--no-default-features` so rust-numpy is excluded
> entirely. Current free-threaded wheels compile the numpy feature (default) but runtime-probe it
> (`src/py/python_types.rs:61`) since free-threaded environments typically lack numpy; the change
> strips the dead linkage (smaller binary, no numpy capsule code, no probe needed). GIL wheels keep
> numpy enabled.

| # | Item | Layer | Status |
|:--|:-----|:------|:------|
| 1 | **`--no-default-features` wheel** — add the flag to the free-threaded wheel build steps in `publish.yml` (windows + macos `-i python3.14t`) | CI | ✅ Commit `9ad41f3` |
| 2 | **Free-threaded CI validation** — `test-freethreaded` job builds with `--no-default-features` | CI | ✅ Commit `9ad41f3` |
| 3 | **Install docs note** — `docs/{en,zh,ja,ko}`: free-threaded wheels are numpy-free (ndarray serialization unavailable on cp314t) | Docs | ✅ Commit `9ad41f3` |

**Changelog mapping**: the `[0.11.6]` entries in the repository root's `CHANGELOG.md`.

---

### v0.11.7 — "CI signal hygiene" (target: Q3 2026)

> Replace the deliberately-failing `stub-build-check` CI job with static assertions that pass when
> the repo is correct (green CI), fail only on regression. Track `rust-numpy` free-threaded support
> status for re-enabling ndarray on cp314t.

| # | Item | Layer | Status |
|:--|:-----|:------|:------|
| 1 | **stub-build-check → release-guard** — replace the always-red container build with static assertions: `grep` guards `publish.yml` against `--generate-stubs`, `git ls-files` asserts `.pyi` is tracked, `test -f` checks `py.typed` | CI | ✅ |
| 2 | **Numpy free-threaded tracking** — ROADMAP.md documents `rust-numpy` free-threaded support (PyO3/rust-numpy#476) as a tracked dependency | Docs | ✅ |

**Changelog mapping**: the `[0.11.7]` entries in the repository root's `CHANGELOG.md`.

---

### v0.12.0 — "Competitive Response" (target: Q3 2026)

> Respond to `yaml-edit` competitor features with a fast, round-trip-preserving editing story. D3
> ships the create-missing path write; D4 adds Rust-backed AST traversal.

| # | Item | Layer | Status |
|:--|:-----|:------|:------|
| D3 | **`set(create_missing=True)`** — create missing intermediate mapping keys along an edit path (`doc.set("$.a.b.c", 2)`) | Rust + Python | ✅ 2026-08-04 |
| D4 | **`doc.walk()` / `doc.scalars()`** — Rust-backed depth-first traversal yielding `Node` objects, matching `Node.walk()` semantics without per-node `to_dict()` resolution | Rust + Python | ✅ 2026-08-04 |
