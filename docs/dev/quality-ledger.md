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
  **Closed 2026-10-15, and the decision is the maintainer's:** a flow member's note is written as a
  line-end comment, which is the single slot this engine's reader hands back to a member (measured
  over eight spellings — a note on its own line inside the braces is swallowed by the reader, so it
  cannot be preserved as its own line there), and a member carrying more than one note sends its
  container to block style, where every note keeps a line. Two residuals were measured on the way
  and are *not* claimed closed: an inline note attached to a **block** mapping key is still dropped
  (`null: " "  # A` keeps the value's note and loses the key's — the flow path writes it, the block
  path has one slot per line and no rule for which note wins), and a note whose text is only blanks
  has no spelling any reader keeps, so it is now written nowhere rather than settling on a second
  round. Both are pinned by tests: `a_note_on_a_flow_member_is_written_and_reads_back`,
  `two_notes_on_one_flow_member_send_the_container_to_block`,
  `a_blank_note_is_written_nowhere_and_leaves_the_line_alone`.

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
report the committed stub clean; against the stub that shipped in `v0.17.0` both report findings and
exit 1 - the negative control is a test (`tests/test_stub_types_gate.py`), not a manual step, and it
refuses to pass when neither checker is installed. With `ty` removed from `PATH` the same file skips
(11 passed / 4 skipped) and the gate exits 2, so "green on the machine that authored it" is no
longer the state being shipped. Route re-derives with no drift; `pytest tests/ -q` 2334 passed / 10
skipped; changelog counts identical across five mirrors.

**The control decayed the day its own fix merged, and reported a failure while doing it.** That
sentence above said the negative control asserts against `origin/main`, and for the week that was
true: main held the stub with `u32` and `Py<PyAny>` in it. #323 merged, main's stub became the clean
one, and the very next verification run failed with "ty passed on the broken stub". Nothing was
wrong with the checker; the control had silently stopped describing a broken artifact. A control
whose source is a moving ref is not a control - it decays at the moment it succeeds, which is the
worst possible time to stop meaning anything.

Two changes, both small, both necessary:

- `BROKEN_REFS = ("5ea2c0a3", "v0.17.0")` - the commit that carried the defect first, because it is
  the exact file this fix replaced, with a release tag as the fallback - and the fetched file is
  checked for the spellings it is supposed to contain before any checker is asked. If no pinned ref
  yields them, the test skips: history rewritten is a fact to report, not a pass.
- The control now runs where it can actually run. It needs a checker installed *and* the git
  history, and no job had both: on the test legs every checker is absent so the file skips, and the
  stub-drift job had the checkers but never invoked pytest. It does now, under one overlay with
  `mypy==2.4.0` and `ty==0.0.85`, with `fetch-depth: 0` on that job's checkout because a depth-1
  clone has no tags - which would have made the re-pinned control skip forever, the quietest
  possible way for a gate to stop existing.

The first CI run of that new step earned its place immediately. Pinned to `v0.17.0`, mypy exited 1
as required but reported **three** findings where the assertion demanded five: the older release's
stub carried fewer of the spelling sites, so the threshold had silently become a property of one
particular file rather than of the defect. The assertion now says what it means - the run must exit
1, and `u32` must appear among the findings blamed on our file, with the count a floor rather than
the claim. A threshold copied from whichever artifact happened to be checked first is the same
mistake as a ref that moves: both make the control about the sample instead of about the property.

Verified: with `ty` on `PATH` the file reports 13 passed / 2 skipped, and both skips name mypy - the
control bites against the pinned artifact, and CI's own run of the step is what measured mypy's
count.

### (bh) One schema chose the hub's spelling and another read it (#312, 2026-10-10)

`from_json5` returns YAML text - the hub document - and the two ends of that text disagreed about
the same three words. YAML Core resolves `.inf` / `-.inf` / `.nan` as floats and the bare word
`Infinity` as a string, so storing the source spelling turned a number into a string for every
consumer that re-read the projection: `load_json5("{a: Infinity}")` resolved in memory and was
right, `parse(from_json5(...))` handed back `'Infinity'`.

Measured first, and the report described only half of it. The quoted direction was wrong the
opposite way: `quoted_or_plain` asked `needs_quotes` - a core-schema question - whether a decoded
JSON string needed quoting. `Infinity` is a string under core and a number under JSON5, so the JSON
string `"Infinity"` was stored as a plain scalar and `load_json5('["Infinity"]')` returned `[inf]`
while `load_jsonc`, `load_json` and `json.loads` all returned `['Infinity']`. One document, two
types, decided by which loader happened to read it.

One root cause, two symptoms, both closed by asking the widest question available rather than the
one that happens to be implemented:

- `pyrs-json/src/parser.rs`: a JSON5 non-finite literal reaches the hub as `.inf` / `-.inf` /
  `.nan`; a JSON string is quoted when either the core or the JSON5 resolver would re-read it as a
  value.
- `pyrs-json/src/writer.rs`: JSON5 emits the dialect's bare token from the resolved *value*, never
  from a word. Dropping the word list from `is_json5_number` also closed a documented ambiguity that
  `test_json5_ambiguous_spellings_are_bare` had pinned as a known limitation - the test's own
  docstring named this exit criterion, so it now asserts the fidelity instead of the gap.
- `pyrs-schema/src/schema.rs`: the JSON5 resolver accepts the hub spellings the parser now feeds it,
  or the in-memory loader and the round-tripped document disagree about the type.

Strict JSON and JSONC keep quoting a non-finite float they cannot spell. JSON has no literal for
infinity, so the choice is `null`, an error, or the string - a ruling that belongs to the user, and
it stays open in `ROADMAP.md` rather than being decided quietly inside a bug fix.

Two workflow facts, both of which cost an hour:

- A parked change replays cleanly only if nothing moved underneath it. Checked rather than assumed:
  `jj diff --from <parent> --to main@origin --summary` listed the 41 paths main touched since, and
  none of them was one of the five files - so the replay was a copy of whole files, not a conflict
  to resolve.
- `maturin develop` cannot overwrite `python/pyrs_yaml/pyrs_yaml.pyd` while any process has it
  mapped, and on Windows that is `os error 32`, not a warning. The holder here was a
  `zensical serve` left running from an earlier docs preview - mkdocstrings loads the extension to
  render the API reference. The interim route (and it worked) is to stage the package elsewhere with
  the freshly built `target/maturin/pyrs_yaml.dll` swapped in and `PYTHONPATH` ahead, then print
  `pyrs_yaml.__file__` so "this run used the new binary" is checked instead of assumed; it is a
  workaround, not a habit, and it makes the compliance tests fail on a path that only exists
  relative to the real tree.

The instruction-count gate caught the cost of this fix before it could ship, which is precisely what
it is for. `from_json_medium +14.24%`, `from_jsonc_medium +13.22%`, `from_json5_medium +9.73%`
against a 0.50% tolerance, on the branch's first CI run. Reading the diff explained the surprising
half: asking "would the JSON5 resolver re-read this as a value?" was unconditional, so the strict
JSON and JSONC readers paid a second full resolve for a hazard that only the JSON5 round trip has.

The answer is not to stop asking - the question is what keeps one document from meaning two types -
but to ask only where an answer is possible. `resolve_json5_type` trims and then dispatches on `-`,
`+`, `.`, a digit, `I` or `N`; everything else is a string by construction, so
`json5_could_be_typed` tests that first byte and the resolver runs only behind it. Two tests hold
the shape: `the_guard_changes_no_strings_verdict` compares guarded against unguarded over a corpus,
because the claim is "same output, less work" and not "fewer calls";
`json5_guard_is_a_sound_superset` asserts the one direction that matters within the domain the guard
is ever asked about. That scope is not a convenience - the first version of the test failed on `""`,
which resolves to a null, and the failure is what showed where the boundary actually sits:
`quoted_or_plain` short-circuits on `needs_quotes`, so `""` and `null` never reach the guard, and an
unscoped assertion would have described a call path that does not exist.

The guard itself was measured three times before it was cheap enough, and the numbers are the
interesting part: `text.trim()` plus a byte test gave `from_json_medium +2.13%`,
`from_jsonc_medium +1.97%`, `from_json5_medium +1.45%`; decoding one `char` instead gave +0.61% /
+0.57% / +0.41%; a chain of byte compares gave +0.51% / +0.47% / +0.35%. Tolerance is 0.50%, and the
samples' own spread was 0.002% - so the residual was real, reproducible, and per-string: this runs
on every text the core schema was content to leave plain, and the medium fixture holds tens of
thousands of those. Nothing about the logic changed between the three versions; only the shape of
the test did.

The last step is a 128-entry lookup table for the first byte - the same move the serializer made for
tag emission, for the same reason - with three states: cannot begin a JSON5 value, does, or is
whitespace (including the bytes above ASCII, where JSON5's whitespace set actually lives: NBSP, the
U+2000 separators, LS/PS) and the following bytes must be consulted. A leading `é` and a padded
` Infinity ` take the same rare branch and get different answers from `str::trim`, which is what
`resolve_json5_type` itself performs; duplicating Unicode's whitespace table to avoid that branch
would be a second source of truth, the mistake this file keeps documenting.

`json5_guard_is_a_sound_superset` carries the corpus for all of it - padded, non-ASCII-leading, and
the bare `-`, `+`, `.` that a byte test must not turn into more work than it is - and
`the_guard_changes_no_strings_verdict` is unchanged, which is the point: the cost came down and the
output did not move.

A third workflow fact: the local WSL route for `scripts/ir_gate.py` cannot build the binding here
(`rust-lld: error: unable to find library -lpython3.12`), so CI is the only place this project
measures instructions. Worth stating plainly because it changes the loop: a claim about hot-path
cost is not verifiable at the desk, and pushing to be measured is the honest procedure rather than a
guess dressed up as one.

Verified: `cargo fmt --check`, `cargo clippy --all --all-targets -- -D warnings`, 525 nextest tests,
2337 Python tests (10 skipped), five tests holding the pair - two Rust in the parser, two in the
writer, one Python through the public API - plus the two guard tests above. Changelog counts
identical across the five mirrors at [11, 4, 21, 3].

### (bi) The prose fixer wrote the gap its own checker reported (2026-10-10)

Adding a Japanese changelog entry, `check_doc_wrapping.py --fix` answered with a finding and refused
to converge: `docs/ja/changelog.md:241 (1 gap(s)) [wrap residue]`, and running it again produced the
same line. The gap was not in the text that was typed. The fixer wrote it.

`join_lines` decides the glue between two physical lines of one paragraph, and the rule it used was
`"" if _is_han_kana(out[-1]) and _is_han_kana(following[0]) else " "`. U+3001 (ideographic comma) is
General Punctuation: neither Han nor kana. A line ending in `、` therefore glued to the following
kana with a space - and `CJK_WORD_SPACE` in the same file treats a space between two CJK-class
characters as the artifact it strips and reports, because its class is `\u3000-\u303f` plus
`\uff01-\uff5e`, which does include U+3001.

One rule, written twice, encoded differently. The checker's version was right and the joiner's was
not, so each pass fixed the paragraph and then re-broke it. The repair is to stop restating the
rule: `join_lines` now asks `CJK_PUNCT_SPACE` and `CJK_WORD_SPACE` whether a space at that boundary
would be an artifact, and `_is_han_kana` is gone - the function that could disagree with the
patterns it was supposed to mirror.

Two tests hold it: `test_the_joiner_never_writes_the_gap_the_checker_reports` (a `、` boundary
fuses, and neither pattern finds anything in the result) and
`test_a_boundary_touching_a_code_span_keeps_its_space` (the other half: `は `docs.md` にある` is
normal typography and must not fuse).

While the joiner was wrong, `--fix` over CJK pages was unsafe: the repair itself was creating
findings. The interim route, and the one that should be used until this lands, is to re-flow only
the English governed pages and hand-wrap the localized ones - `target/probe/fix_english_only.py`.
Hand-wrapping was also what let 104- and 108-column lines into the ledger in the first place, so the
English pages still go through the fixer.

Verified: `pytest tests/test_doc_wrapping_gate.py` 30 passed; doc gates green over all nine governed
pages; the localized pages needed exactly one manual repair (`が、 その文末` -> `が、その文末`) that
the fixed joiner now refuses to write.

### (bj) A byte-wise walk of a character-indexed string, found by feeding the API a valid schema

(2026-10-10)

The sweep that found it was the repository's own rule: no `unwrap`/`expect` in business logic.
Filtering to code before each file's `mod tests`, and dropping the parser method named `expect` that
returns a `Result`, left 29 sites - test helpers, integration harnesses, invariants that hold - and
one real defect. `unwrap` is not merely a style offence here; this one was a reachable crash.

`rule_path_to_segments` walks a schema rule path (`$.a.b[0]`) and decoded each character with
`rest[i..].chars().next()`, pushed it, then advanced `i` by **one byte**. A multi-byte key therefore
left the cursor inside the character it had just consumed, and the next slice panicked. Measured
through the public API rather than reasoned about:

```text
$ pyrs_yaml.validate_against_schema("café: 5", "- path: $.café\n  type: str")
PanicException: start byte index 4 is not a char boundary; it is inside 'é' (bytes 3..5 of string)
```

The input is a *valid* schema over an ordinary document - a French key, a Japanese key, an emoji in
an identifier

- so this was never a malformed-input guard; it was a crash on the happy path for anyone whose
  configuration keys are not ASCII. `$.emoji😀key` panicked at the same line. The fix is the
  character's own width: `i += c.len_utf8()`, with the `[n]` branch untouched because `]` is one
  byte and its index comes from `find`.

Fixing it made the second half visible. The function began with `strip_prefix('$')` then
`strip_prefix('.')`, and *after* that tested whether the remainder was empty, with a comment reading
`// path "$"`. A path consisting of just `$` cannot survive the second strip, so the branch was dead
and the root document could not be a rule target at all - an unfinished feature hiding one line
below the crash. Now a `$` with nothing after it resolves to the empty segment list (the document
itself), `$…` still requires its separator, and `$x` stays unparseable rather than being silently
reinterpreted as a key.

The same function leaves a question this change does not answer, and `ROADMAP.md` now carries it as
an open item: `mapping_of` and `sequence_of` check elements *inside*
`if let CustomNode::Mapping`/`Sequence` with no other branch, so `$.config` with `mapping_of: str`
passes when `config` is a scalar. Making the wrong container kind an error is a user-visible ruling
about whether these keys assert the container or merely describe its elements; it is not a decision
to take as a by-product of a panic fix.

One more time, in this ledger's own words: the corpus was the hole. Every existing schema test used
ASCII paths, so a defect in the character-walk could not be reached by any of them, and the third
route in - fuzzing - covers the parsers, not the schema language. The two new tests name the key and
the path they check (`a_rule_path_with_a_multibyte_key_is_walked_by_character`,
`test_rule_path_with_a_multibyte_key_fires_on_the_right_node`), and the Python one asserts a
violation *fires* as well as no panic: a path that navigates to nothing would also have looked
healthy.

Verified: 522 nextest tests, 2338 Python tests (10 skipped), clippy `-D warnings`,
`cargo fmt --check`, the `PanicException` repro now returning "ok (no violation)" / "expected str",
and doc gates green across nine governed pages.

### (bk) What a format cannot spell, it must not answer with a lie (2026-10-10)

Issue 312 fixed what a non-finite float looked like on the way into the hub. This is the half that
fix could not answer: what the strict JSON writers do with a value RFC 8259 has no literal for. It
was written into `ROADMAP.md` as a ruling to make, with three candidates and no measurement attached
to any of them.

Measured, from the running library rather than from the design:

```text
b: .inf  ->  to_json   { "b": ".inf" }      load_json gives str, not float
b: .inf  ->  to_json5  { "b": Infinity }    load_json5 gives float  ✓
```

So the strict writers were turning a number into a string on the way out - the mirror image of the
bug that was just fixed, and equally invisible: nothing in the output distinguishes a document whose
value was the text `.inf` from one whose value was the number.

Each candidate was then asked the same question - what comes back if you read it?

| emission | reads back as | verdict |
|:--|:--|:--|
| `".inf"` (before) | `str` | the value's type changes silently |
| `null` (`JSON.stringify`'s answer) | `None` | a different number, silently |
| `Infinity` (CPython's default) | rejected here | text `load_json` refuses on purpose; this loader is stricter than the stdlib by design, matching the spec |
| refuse | - | the value is not lost, and the caller chooses |

Refusal is the only row that does not assert something false about the data. `to_json5` already
carries the number losslessly, so the caller is not stranded - which is why no lenient `null` was
added: nothing has asked for a documented way to lose a value on purpose, and a default that loses
it silently is what this entry exists to remove.

The same sweep found the message was lying too. `SerializeError::Internal` carried every writer
rejection, so `to_json` on an alias, `to_toml` on a null or on a non-table root, and a malformed
TOML timestamp all reached the user as `internal-error: toml-cannot-represent-null` - wording that
tells a reader the library broke instead of telling them their document has a value the format
cannot hold. A new `UnsupportedValue` variant now carries those stable reason keys, and `Internal`
is left meaning what its name says: a violated invariant, of which the YAML serializer has exactly
one.

Verified: 526 nextest tests (including the strict-writer refusal, nested as well as at the root, and
JSON5's fixed point), 2338 Python tests (10 skipped), clippy `-D warnings`, the stub regenerated
through the declared route (`maturin generate-stubs` + `check_stub_drift.py`, which carries the new
docstring), and `to_json`'s docstring naming the reason key so the refusal is in the API reference
and not only in the source.

### (bl) A test matrix measures the surfaces it feeds, not the ones it names (2026-10-10)

The fuzz tier had six targets. Read individually, each is sound: `parse_yaml`, `yaml_roundtrip`,
`parse_json`, `json_roundtrip`, `parse_toml`, `toml_roundtrip`. Read as a matrix, they say something
no individual file says: every one of them hands the engine a *document*. The engine also parses
*schemas* - a second grammar with its own path navigator - and nothing in the tree had ever given it
one.

That is where `(bj)`'s crash lived. The observation worth keeping is not "add a target for the
schema language" but the method that found it: the matrix was described in prose ("unit / property /
fuzz / cross-library"), which is a true statement about every named surface and silent about whether
the set of surfaces is complete. `QUALITY_MATRIX.md` now carries 7 targets / 118 seeds / 60
former-crash seeds, and the row is derived from `fuzz/Cargo.toml` and the workflow matrix rather
than typed, so a declared target whose file is missing - or a file no CI job runs - reddens
`tests/test_quality_matrix.py`.

Two constraints the new target had to satisfy that a private `cargo fuzz run` would have skipped:

- one input has to carry *both* grammars, so it is split on the first NUL. A rule path is only
  navigated when the document parses, and a schema that does not parse returns early; fuzzing either
  half alone mostly explores reject paths.
- it has to fit `scripts/fuzz_rounds.sh`, whose pull-request tier replays committed seeds with
  `-runs=0` and nothing else, because that is the only fuzz mode deterministic enough to block a
  merge. The seed is the exact input from (bj), so the crash class is gated per pull request instead
  of being rediscovered weekly.

The target is then proven to bite, because "clean" is also what a broken probe reports: reverting
the `schema_language.rs` fix and replaying the seed produced `crash-59b1501e9e598d3f…`, panicking at
`schema_language.rs`; restoring the fix made the same replay clean, as did 150 s of scheduled-mode
exploration.

Two environment facts worth recording, both discovered while building rather than reading:
`cargo fuzz` needs the toolchain pinned explicitly (`RUSTUP_TOOLCHAIN=nightly` - `-Zsanitizer` is
not a stable option and the failure reads as a rustc bug otherwise), and a target directory under
`/mnt/d` dies mid-build with "Cannot allocate memory" while writing `.rmeta`. The second is a 9p
limit, not a defect in the target, and it looks exactly like one until you have hit it.

Verified: `cargo fuzz build --dev validate_schema` rc=0, PR-mode seed replay clean on the fixed tree
and crashing on the reverted one, `cargo check --manifest-path fuzz/Cargo.toml --bins`, and
`pytest tests/test_quality_matrix.py` 23 passed with the derived rows updated.

### (bm) A validator that answers "valid" about a node it never looked at (2026-10-10)

The rule-path panic of (bj) left one sentence in `ROADMAP.md`: `mapping_of` and `sequence_of` check
elements but not the container. It was written there as a choice between two user-visible outcomes,
so it waited for a ruling instead of being fixed in passing. The ruling began by asking the running
library what it does today, through the public surface:

```text
$.config  mapping_of: str    with  config: hello      PASS
$.config  mapping_of: str    with  config: [a, b]     PASS
$.numbers sequence_of: int   with  numbers: 5         PASS
$.numbers sequence_of: int   with  numbers: {a: 1}    PASS
$.port    type: int          with  port: {a: 1}       PASS
$.port    type: int          with  port: [1, 2]       PASS
$.n       sequence_of: int   with  n: [1, [2, 3]]     PASS
```

Seven shapes a schema named and the engine declined to consider. The cause was the same in all
seven: every check sat inside an `if let` for the one node kind it could describe - `if let
CustomNode::Mapping` with no other branch - so a node of any other kind was walked past without a
word. A validator that reports nothing about a document it did not examine is not stricter than one
that rejects it; it is a validator that has stopped validating.

The ruling is that **scope decides how much a rule asserts**. A rule that names a `path` is about
that node, so the node's shape is part of the claim: `$.config mapping_of: str` now fails on a
scalar, and `$.port type: int` fails on a mapping. A pathless rule cannot name a node, so it selects
the nodes it can describe and leaves the rest - which is not a loophole but the only meaning
pathless `type: str` has ever had: *every scalar is a string*, whatever else the document holds.
Turning that into an assertion would have broken the common case to honour the rare one, and a
pathless `type: map` would select nothing at all, so it is refused where it is written rather than
accepted as a rule that silently passes every document.

Members are asserted either way, which is what the seventh row above measures. `sequence_of: int`
has already said what its elements are, and a nested sequence is not one of them; skipping it was
the same false negative one level down. Leaving members lenient while tightening the named node
would have shipped half a ruling and opened a second item for the other half.

Being able to assert a shape then exposed what the language could not say: there was no spelling for
"this node must be a mapping", and none for "every value of this mapping is a sequence" - the
wildcard reaches sequence indices, never mapping keys. `type:` and the member position of
`sequence_of` / `mapping_of` now accept `map` and `seq` (`mapping`, `object`, `sequence`, `array`,
`list` too, because a schema is typed by a person and the synonyms cost nothing). A `rules:` pattern
still refuses them: a pattern resolves the text of a scalar and can never produce a container, and
the error says so instead of only saying "invalid".

Three defects surfaced while the ruling was being written, each found by touching the same code:

- **`$.rows[*]` matched `$.rows[0].a`.** The wildcard matcher compared a prefix and a suffix and
  required only that *something* sat between them, so `[*]` meant "this element and its whole
  subtree". Invisible while rules ignored shapes; a false positive on every nested value the moment
  they assert one. A wildcard now stands for exactly one index, and `$.rows[*].a` still names the
  member it reaches.
- **A rule with two checks ran one.** `type` and `mapping_of` in the same rule assigned to the same
  field, so the check written first was dropped without a trace. A rule may now carry one check, and
  `required` stays orthogonal because it combines rather than competes.
- **A located error lost the path.** `SchemaValidationError` printed `line:column: message` *or*
  `path: message`, so as soon as a node carried a source range the schema author lost the half they
  had written: `1:9: expected map but got sequence` never says which key. Both print now.

One boundary is left deliberately, pinned rather than hoped for. An alias node carries a name, not a
shape, and the validator holds no anchor table, so a rule about `$.b` in `a: &x 5\nb: *x` cannot be
decided. It passes. `an_alias_is_passed_rather_than_guessed` (Rust) and
`test_an_alias_is_passed_rather_than_guessed` (Python) record that, so making the validator
alias-aware has to break two named tests rather than quietly change what a schema means.

Verified: 541 nextest tests (the container, member, synonym, pathless-selection, wildcard-scope,
two-check and alias cases, each asserting the exact complaint text), 2350 Python tests (10 skipped)
including the same ruling through `validate_against_schema`, clippy `-D warnings`, and the four
locale guides rewritten to the new semantics.

### (bn) The document describing the defence was the one artefact outside it (2026-10-10)

Every other entry in this ledger ends by naming the gate that now holds the finding. This one is
about a gate that did not exist, and about how the repository found out: not from a measurement, but
from reading `AGENTS.md` while fixing something else. The file said `mkdocstrings` is "configured,
installed and invoked by the build - **and renders nothing**", registered the gap as
`docs-generation:plugin-unused`, and drew the obvious conclusion - API prose is a hand-typed content
decision. #321 had closed that gap: twenty pages across four locales carry `:::` directives, the
registry is empty, and the matrix prints `derived holes (0)`. The instruction was not merely out of
date, it pointed the opposite way.

The defence was not at fault. `scripts/quality_matrix.py` re-derives `QUALITY_MATRIX.md` from the
files that declare the defence, and `tests/test_quality_matrix.py` compares the measured set with
`.ci/quality-holes.json` in both directions - an unregistered blind spot fails, and a registered one
the measurement no longer reproduces fails too. Both halves are instrumented. What was missing is
the sentence *about* the instrumentation, which lives in the file an agent trusts most and reads
first, and which no probe opened. #330 rewrote that bullet and said so in its own description:
nothing compares AGENTS.md's claims about the defence with the defence.

`scripts/check_hole_claims.py` closes it with one rule: a hole id named in an instruction file must
be an id the matrix measures now. Three choices make the rule hold rather than rot in its turn:

- **The vocabulary is derived.** A token is a hole id only when its kind is one `quality_matrix.py`
  can emit, read off the probe's own `holes.append(["<kind>", ...])` sites. A list kept beside the
  probe would be a second transcription - the same mistake as a transcribed matrix - and would
  silently stop covering kinds someone adds later. The derivation is checked against the number of
  reporting sites: if an append starts with anything other than a literal, the vocabulary is
  provably shorter than the probe, and that is reported as exit 2, not as a smaller job that passes.
- **Presence is not the target; tense is.** Scope is `AGENTS.md` and `CLAUDE.md` only. The five
  release-note pages, this ledger and `QUALITY_MATRIX.md`'s own narrative name closed holes on
  purpose - a dated record is the place the evidence lives, and forbidding ids there would delete
  the history this ledger exists to keep. An instruction file is present tense, so a closed id in it
  is a false statement rather than a false annotation.
- **A true claim must stay legal.** Otherwise the only way to satisfy the gate is to stop writing
  anything, and a silenced instruction file is worse than a stale one.
  `test_an_id_the_matrix_actually_measures_is_allowed` asserts that;
  `test_a_colon_that_is_not_a_hole_claim_is_ignored` asserts that `line:column`, a URL with a port
  and `key: value` are not readings of the shape.

The checker's first finding was the file it was written for: `AGENTS.md:49`, naming the gap #330 had
just finished describing. That is the negative control this class of gate needs - a hook that
reddens on the sentence that motivated it, in the same session, before any prose was softened to
satisfy it. The remedy was not to delete the sentence but to state the obligation the closure
created (a directive added to one locale needs its three twins) and to leave the id out, since the
id now describes nothing the measurement reproduces.

Verified: 7 gate tests (the rotting sentence caught, every emittable kind covered by the derived
vocabulary, a measured id allowed, non-claims ignored, a short vocabulary exiting 2, an unreadable
file exiting 2), the quality matrix reporting no new hole for the checker itself (`orphan-gate`
would fire if it were unwired; the `prek.toml` hook `hole-claims` is what answers it),
`ruff check`/`format` clean, and the doc gates green over the five mirrors.

### (bo) The published signature and the shipped callable disagreed, and the page was the honest one (2026-10-10)

The task looked like a documentation sweep: measure how many hand-typed signature blocks still sit
on the API pages now that #321 generates members from the code. 238 of them, it turned out, across
28 pages - and the way to tell a *false* one from a *lossy* one was to ask the running package. That
question returned something nobody had expected to find:

```text
$ python -c "import pyrs_yaml; print(pyrs_yaml.read_markdown(path='note.md'))"
TypeError: read_markdown() got an unexpected keyword argument 'path'

$ python -c "import pyrs_yaml; print(pyrs_yaml.read_markdown('---\ntitle: x\n---\nbody'))"
OSError: Failed to read file: ... (os error 123)
```

The committed stub declares `read_markdown(path: str, schema, max_depth)`. The function *reads a
file at that value* - feeding it Markdown text fails on the filesystem, which is what a parameter
named `path` predicts. But `python/pyrs_yaml/__init__.py` re-exports the name through a wrapper that
had renamed the parameter to `content`, and maturin generates the stub from the extension, which
never sees the wrapper. So the published page and the stub agreed with each other, the runtime
disagreed with both, and the keyword that describes the argument is the one that raises.
`validate_against_schema` carried the same drift in the opposite direction: the stub wrote
`schema_yaml`, the callable has always taken `schema`.

Two principles came out of that, and both are now gates:

- **The authority for a published signature is the shipped object.**
  `scripts/check_doc_signatures.py` imports the package and compares parameter lists: a name, an
  order, or a dropped *required* parameter is a finding; a required prefix with its optional tail
  omitted is what a reference page legitimately looks like, so it passes; and an example call with
  real values (`register_type("!timestamp", TimestampType())`) is usage, not a claim. A checker that
  trusted only the stub would have reported the documentation as the defect here - the failure mode
  that makes a gate ignored rather than obeyed.
- **The stub must describe the callable, name by name.** `tests/test_stub_runtime_parity.py`
  compares the parameter names the committed `.pyi` declares against `inspect.signature` of the
  exported object for all 40 module functions, and asserts the inventory is non-trivial so the
  comparison cannot go vacuous. The two mislabels are fixed at their source - the wrapper now says
  `path`, the native parameter is renamed to `schema`, and the stub was regenerated through the
  declared route - and 40/40 agree.

The localized pages were not innocent either, and their defect was a different one: `zh`/`ja`/`ko`
type `register_schema(name, schema: str | dict)` and `register_type(tag, type_handler, priority)`.
Neither layer has those names, `dict` raises `TypeError`, and `priority=` raises too. The English
page was correct, so this was translation drift rather than a stale copy - the i18n mirror checker
compares *structure*, and a wrong parameter name is structurally perfect. The gate now runs over
every locale, and the pages type what the object accepts.

Three measured gaps are left open, recorded here so the next reader does not re-derive them:

- **The shipped stub omits most of the surface the documentation pages render.** `Node` (41 members)
  and `MergedView` (6) are not declared at all, and 13 members of `YamlDocument` - the whole editing
  and query set: `set`, `set_many`, `insert`, `append`, `delete`, `rename`, `sort_keys`, `walk`,
  `scalars`, `find`, `node`, `merged` - are missing, because they are attached to the native class
  from Python after import. An editor sees one class, the runtime offers another. Fixing it is an
  architectural decision about which artefact the stub describes, not a rename.
- **The tour sections still retype signatures the same page already generates.** Duplication is not
  contradiction, so the gate stays quiet; it is where the next drift will appear.
- **An inline dict schema loses its `validate` section.** `_schema_to_yaml` - the helper that turns
  the dict accepted by `safe_load(schema={...})` into text - emits `extends` and `rules` only, so a
  validation section supplied as a dict is dropped without a word, and `register_schema` refuses the
  dict entirely. The silent loss is the defect; the refusal at least raises.

Verified: 16 gate and parity tests (the published pages pass; the four false shapes caught; three
faithful ones quiet; an unreadable inventory exits 2; the parameter-name parity over 40 functions),
2380 Python tests (10 skipped), 541 nextest, clippy `-D warnings`, `check_stub_drift.py` clean after
the regeneration, the doc gates green with five mirrors agreeing at [14, 8, 26, 3], and the
four-locale site build.

### (bp) The dict a caller wrote and the schema the engine saw were not the same object (2026-10-10)

`safe_load(text, schema={"extends": "core", "rules": [...]})` is documented as equivalent to writing
that schema as YAML, and the guides show it. The equivalence held for the part of the definition the
bridge between them knew about, because `_schema_to_yaml` assembled the text with
`f"  - pattern: '{pattern}'"` - a hand-written emitter inside a package whose entire purpose is
faithful emission.

Two failures fell out of that, and both were measured before being rewritten:

- **The `validate` section was dropped.** The emitter knew `extends` and `rules`. A dict carrying
  validation rules was registered without them, and the failure was invisible in exactly the way a
  validator's failure always is: a schema with no rules accepts every document, so the caller who
  asked for enforcement got silence. The fix's own test has to be a *raising* pair - `expected int`
  and `required path is missing` - because "it did not complain" is what the bug looked like.
- **The quoting rule was one-sided.** `'...'` unless the value contains an apostrophe, then
  `"..."` - and nothing escaped the `"` a YAML pattern inevitably carries. `^["']?[0-9]+["']?$` came
  out as `pattern: "^["']?[0-9]+["']?$"`, which the parser rejects:
  `invalid trailing content after double-quoted scalar`. Reconstructing the old output and
  registering it is a test now, so the contrast stands as a measurement rather than a claim.

The remedy is not a better hand-written emitter. The dict is passed to `from_dict` - the serializer
this library uses to guarantee that anything it writes re-parses to the same value - after the same
input validation, with `extends` defaulted to `core` as before. Arbitrary pattern text needs no
quoting at all under it (`pattern: ^["']?[0-9]+["']?$` is a valid plain scalar), every key survives,
and a dict now resolves identically to the YAML text a user would have written.

The rule this entry exists to keep: **a conversion from data to text inside a round-trip library has
to go through its own writer.** Every hand-built emitter is a place where a key, a character or a
whole section can vanish without an error, and "the output is shorter than the input" is invisible
to any test that only checks whether the call raised.

Verified: 6 tests in `tests/test_inline_schema_dict.py` (the `validate` section enforced through the
registered name, including the two raising cases; every key written; a dict and its equivalent text
resolving the same; `extends` still defaulting to core; a both-quotes pattern surviving; the old
output rejected), 2386 Python tests (10 skipped), and the doc gates green with five mirrors agreeing
at [14, 8, 27, 3].

### (bq) An explicit tag is a statement of type, and the loader had been ignoring it (2026-10-10)

Asked of the shipped package and of both reference libraries, the same eight documents:

| document | PyYAML | ruamel | us, before |
|:--|:--|:--|:--|
| `!!str 1.20` | `'1.20'` | `'1.20'` | `1.2` |
| `!!float 1` | `1.0` | `1.0` | `1` |
| `!!bool yes` | `True` | `True` | `'yes'` |
| `!!binary aGk=` | `b'hi'` | `b'hi'` | `'aGk='` |
| `!!int 123456789012345678901234567890` | bigint | bigint | string |
| `!!int hello` | `ValueError` | `ValueError` | `'hello'` |
| `!!float inf` | `ValueError` | `ValueError` | `'inf'` |
| `!!binary` with an empty body | `b''` | `b''` | `''` |

One cause: the loader resolved the *text* through the document's schema and consulted the tag only
to ask a plugin registry. A tag states a type, so that ordering is backwards - and `!!str 1.20`
shows what it costs: the type changed *and* the trailing zero disappeared, in a document whose only
purpose was to keep both.

`crates/pyrs-yaml/src/py/tags.rs` now reads the standard tags before the schema is consulted, in
both spellings a document carries them in (`!!str`, and the verbatim `!<tag:yaml.org,2002:str>` that
`Node.set_tag` writes), with the tag's own lexical set - YAML 1.1's - which is why `!!bool yes` is
`True` under the Core schema: the tag is what makes it a boolean. A text that is not the tagged type
raises `YamlTypeError` rather than coming back as a string, and `!!int` past i64 is still an integer
(`int(text, 0)` on the Python side, which is what the tag's unbounded range asks for). `y`/`n` for
`!!bool` is the single place the references disagree - PyYAML refuses them, ruamel accepts them -
and the spec side is taken, with the divergence named in the file.

Unknown tags are left lenient on purpose. Both references raise on `!!weird`; we do not, because a
local tag is how the plugin system is addressed and the tag survives a round trip either way. That
choice is a test (`test_an_unknown_tag_stays_lenient_by_design`) rather than an accident someone
could reverse without noticing.

The dump side turned out to be its own defect, found by asking what `safe_dump({"d": b"hi"})`
produced:

```text
d:
  !binary aGk=
```

`!binary` is a *local* tag. The standard is `!!binary`, so every binary document this library wrote
was unreadable to the libraries whose name it had borrowed - and `{"d": b""}` wrote `!binary` with
an empty body, which re-read as `None` and raised `TypeError` inside the plugin. The plugin
registers under the standard name now (keeping the local one for documents already on disk, because
dropping it would convert a spelling fix into data loss), and the two places that built a `Tag` from
a registered name were collapsing every handle to `!`: `tag_of` keeps `!!` when the name carries it.

Two measured gaps were recorded here, both asserted so a future change would decide them on purpose:

- **`YamlDocument.set` loses a tag.** Closed - see `(bt)`, which is where the cause turned out to be
  (the editor's metadata inheritance, not the tag reading).
- **An empty tagged value emits a trailing space.** Closed - see `(bz)`, where the cause turned out
  to be a property separator left behind for scalar text that never arrives.

Verified: 6 Rust unit tests (known-answer base64 - read against text produced by Python's
`b64encode`, not against this file's own encoder; the 1.1 integer spellings; the float boundaries
including `.inf`/`.nan` and the rejection of Python-only spellings), 78 Python tests in
`tests/test_standard_tags.py` (each document through load, dump and the PyYAML oracle; empty and
long `bytes`; the local spelling still loading; both open gaps pinned), 547 nextest, 2464 Python
tests (10 skipped), clippy `-D warnings`, the doc gates green, and `check_doc_signatures.py` clean
over the pages that now name `!!binary`.

### (br) Two spellings of null are one key, and the generator had not been told (2026-10-10)

The `property tier (20k cases)` job came back red on `prop_output_always_parses`, minimised to a
serialization of

```yaml
a:
  A:
    !E null: A  # 0
    !E null: a
```

which re-reads as `DuplicateKey("null")`. Nothing in the change under review touches
`pyrs-yaml-core` - `jj diff --name-only` names no core file - so the red was a pre-existing hole in
the *generator's* model of its own writer, reached because 20 000 cases is a wider net than the
256-case default tier. It is written down rather than re-run: a flake that produces a smaller
falsifying shape than the last one is evidence, and a retry that happens to pass suppresses it.

Two AST shapes explain that text and they do not need the same fix: two `Null` variants separated
only by metadata (one carrying the note), or a `Null` variant beside a plain `Scalar` whose printed
text is the word `null`. The shipped package settled which pair the reader cannot see apart -
parsing the text under `allow_duplicate_keys=True` yields *one* entry, so the two re-read keys are
fully equal and whatever separated them in the tree is a field the reader never looks at. Both
shapes were real gaps: the rule looked at scalar text and at empty containers, and never once at a
null.

Four measurements replaced four assumptions made while writing the fix:

| assumed | measured |
|:--|:--|
| an anchor on the key separates two nulls, because it is printed | `&a null: one` + `&b null: two` re-read as **one** entry - a mapping holds one bare null key whatever rides it. An anchor separates only once a tag has taken the key out of the null fold (`!E` + `&a` vs `!E` + `&b` stay two entries) |
| a plain scalar spelled `null` is an ordinary string key | the writer's short-alphanumeric fast path emits it raw, so `null` and `Null` land on the same line and the reader folds them |
| `NULL:` and `null:` are two texts, so two keys | the reader decides on the *resolved* key, not the spelling - both fold onto the mapping's single null key |
| an empty plain key is a null spelling, as `is_null_key` reads it in source | the writer refuses to emit it: it quotes (`"": v`), so an empty-string key keeps its own entry |

The rule is now `keys_collide_in_text` (was the private `serialization_collides`) and it is public
for one reason: the model of a writer and a reader, maintained by hand in a third crate, needs its
agreement pinned against both. `pbt::tests::the_key_rule_agrees_with_the_writer_and_the_reader` runs
15 classes in both directions - a pair the rule calls distinct must still hold two entries when its
real serialization is re-read strictly, a pair it calls identical must be refused or folded - and
asserts the balance of the table (8 distinct) so a rule degraded to "everything collides" fails
instead of quietly leaving the generator with single-key mappings. Where the node map itself merges
the two keys (two identical empty mappings) the text oracle cannot run, and the guard asserts the
rule agrees with that outcome rather than skipping it.

The guard was proved to bite: with the null branch forced to `false`, the table fails on
`two tagged nulls, one carrying a note` - the exact CI shape - rather than passing silently.

What this does *not* close is the product half. A hand-built mapping holding either pair still
reaches `to_yaml`, which returns `String` and cannot object: three of the six measured classes fold
without a word, the document one line shorter than the tree. Recorded as issue #336 with the table
of measured emissions and the three options (fold like the reader, report through
`to_yaml_with_options`, or declare the invariant), and as the second open ruling in `ROADMAP.md`.

Verified: 548 nextest (547 plus this guard), `PROPTEST_CASES=20000 cargo test -p pyrs-yaml-core
--lib` at the same budget the CI tier uses - 321 tests, 0 failures, `prop_output_always_parses`
included; `cargo clippy --all -- -D warnings` and the same over `--all-targets` for the two crates;
the five changelog mirrors agreeing at [14, 8, 28, 3] for this change alone - [15, 9, 28, 3]
once the standard-tag change that was in flight lands beside it.

### (bt) The edit that promises to keep metadata was using it to overwrite the value's type (2026-10-10)

`(bq)` closed with a gap it named rather than hid: `doc.set("$.d", b"hi")` emitted `d: aGk=`. The
assignment converts `bytes` through the registry into a scalar carrying `tag: !!binary` and the text
`aGk=` - so the tag was present and then removed. The removal is `with_metadata_from`, in the
splicing copy of `crates/pyrs-yaml-core/src/editing/metadata.rs`, which took **all three** metadata
fields from the node being replaced:

```rust
meta: NodeMeta { comment: src_meta.comment.clone(),
                 anchor:   src_meta.anchor.clone(),
                 tag:      src_meta.tag.clone(), .. }
```

The function is what backs the documented claim that "replaced scalars keep their
comment/anchor/tag/quoting" (`docs/en/features.md`). For two of the three fields that inheritance is
right, and the reason is the same for both: a note is the line's, and an anchor is a name the rest
of the document already refers to - dropping it would leave a `*defaults` pointing at nothing. The
third field is not line furniture. A tag states the type of the value sitting there, and the value
is exactly what the edit changes, so the incoming tag is the truth and the old one describes a value
that is gone. `port: !!int 8080` set to `9090` still has to read the author's spelling, which is why
the rule is per-field rather than "new wins":

```rust
comment: replaced.clone().or_else(incoming)   // the document owns the note
anchor:  replaced.clone().or_else(incoming)   // the document owns the name
tag:     incoming.clone().or_else(replaced)   // the value owns its type
```

Applied to every arm that had the pattern - scalar, mapping, sequence, null - because a tagged
container assignment shares the defect; the mixed-kind cases already fell through to
`target.clone()` and keep the incoming node whole, which is the same conclusion reached by a
different route.

**A control that proved nothing, recorded rather than deleted.** The first negative control flipped
`tag: incoming.or_else(replaced)` to `tag: replaced.or_else(incoming)` and the new tests stayed
green - because each test pins one direction and its fixture leaves the other side empty, so only
the plain override of the old code exercises the difference. The control was re-run against the
*original* rule (`tag: src_meta.tag.clone()`) and `an_incoming_tag_survives_the_replacement` failed
on it, which is the evidence the tests bite. A passing mutation of a two-sided rule is not a test of
that rule.

Verified: 5 Rust unit tests for the three directions (incoming tag survives; incoming untagged keeps
the author's tag; anchor and note keep coming from the document; a tagged mapping survives),
`cargo nextest run --all` 553 tests, 80 tests in `tests/test_standard_tags.py` (the edit route now
asserts `d: !!binary aGk=` and re-reads as `b"hi"`; two new tests pin the untagged-incoming and the
note-plus-alias cases through the real `set`), 2466 Python tests (10 skipped), `cargo clippy --all
--all-targets -- -D warnings`, and the gap pointer updated in `(bq)`.

Still open from `(bq)`: an empty tagged value emits a trailing space (`n: !!null `) - since closed,
see `(bz)`. That was byte exactness of emission, not fidelity of data, and a serializer question
rather than an editor one.

### (bu) The loader began reading tags; the bridges went on reading text (2026-10-11)

PR #335 made a standard tag state the type on load. It changed nothing else, so the engine spent a
release with one AST and two answers, measured on `main` before this entry:

| document | `safe_load` | `to_json` | `to_toml` |
|:--|:--|:--|:--|
| `v: !!str 1.20` | `'1.20'` | `1.2` | `v = 1.2` |
| `v: !!bool yes` | `True` | `"yes"` | `v = "yes"` |
| `v: !!int 0b101` | `5` | `"0b101"` | `v = "0b101"` |
| `v: !!str 1e3` | `'1e3'` | `1000.0` | `v = "1e3"` |

Every row is the same line of code read twice: `write_plain` in `pyrs-json/src/writer.rs` and
`value_str` in `pyrs-toml/src/writer.rs` called `Schema::Core.resolve(value)` on the text, and the
JSON writer's scalar pattern did not even bind `meta`. A stale comment called it "the historical
serde_json projection behaviour", which is a description of an accident: serde_json is not what YAML
§6.1 says, and since #335 it was not even what our own loader did. The writer's `is_json_number`
pass-through (PR #121, added to preserve spellings like `1e3`) made the `!!str` row worse, because a
heuristic on the text beat an explicit statement in the document.

**One rule, in the crate that owns scalar types.** `pyrs-schema` gained
`standard_tag_kind(handle, suffix)` and `TagKind::resolve(text)`: the shorthand handle, the verbatim
`tag:yaml.org,2002:` URI, and YAML 1.1's lexeme sets - the tag is what makes `yes` a boolean and
`0b101` an integer, under a Core-schema document too. It is the same grammar #335 put in the
binding's `tags.rs`, so the second copy is now a delegation target rather than a permanent fork.
`pyrs-schema` stays AST-free (its `Cargo.toml` says "no AST, no parser, no regex" and CI enforces
`no_std`), which is why the entry point takes the tag *as text* and each caller passes
`handle`/`suffix`.

**A tag the text cannot satisfy is refused.** `!!int hello` now raises
`SerializeError::UnsupportedValue("json-tag-text-mismatch")` / `"toml-tag-text-mismatch"` instead of
becoming a string. That is the #328 rule - a strict format refuses what it cannot spell - and it
matches `YamlTypeError` on the load side. The alternative (retype silently) is how the table above
got written in the first place.

**What is deliberately untouched, and pinned that way.** Local and application tags (`!int 7`) still
fall back to the implicit resolution - a local tag is the plugin system's address, and #335 chose
leniency for a reason - so do unknown `!!` suffixes, and so does `!!binary`, whose base64 text still
projects as a string: JSON and TOML have no byte-string type, and deciding that mapping is a ruling,
not a type resolution. It stays open in #340, as does the question of whether `tags.rs` should now
delegate to `pyrs-schema`.

Two predictions of mine were wrong and both are recorded rather than edited out. I expected a local
`!int 7` to reach JSON as the string `"7"`; it reaches it as the number `7`, because the fallback is
the implicit resolution and the loader agrees - so those two assertions now state the *relation*
(bridge == loader) alongside the value. And I expected `1.` to be rejected as a `!!float`; YAML
1.2's own grammar allows it and both reference libraries read it as `1.0`, so the test says so with
the reason.

Also recorded, because it is a gate gap rather than a typo: the ja changelog entry first shipped
with the Cyrillic word `теперь` inside Japanese prose. `check_cjk_localisation.py` scans for scripts
that intrude on *each other's* locale (Hangul in ja, Han in ko) and passed it, because Cyrillic is
not in its class. The entry is fixed; the scanner's blind spot is not, and it is the kind of hole
the quality matrix registers rather than remembers.

The width fixer also did something worth naming: given a paragraph whose first line began with an
issue reference, `check_doc_wrapping.py --fix` rewrote it as
`## 335 made a standard tag state the type...` - a promoted heading, plus a blank line driven
through the middle of the sentence. The heading gate then caught what the fixer had broken, and CI
redded `docs gates and site build` and the prek hook on it while my local check had looked green: I
had read `prek run --all-files` through its last three lines, which hid the one hook that failed.
Two rules come out of that. A formatter that can invent a heading is a formatter whose output has to
be gated - which is why `check_doc_headings.py` exists and why the fixer is not the last word. And a
hook run is read by its verdict, not by its tail.

**Two shapes of this fix were measured on the hot path before one was kept.** Threading an
`Option<TagKind>` through `write_plain` - so every plain scalar, tagged or not, carried an extra
argument and two extra branches - measured `to_json_medium` 21,232,786 → 21,444,770: **+1.00%**
against a 0.5% line. Restructuring to dodge that - the call site matches on `meta.tag`, and both
arms then funnelled through one shared `write_resolved` helper so the emit block stayed in a single
place - measured **+7.17%** (22,754,717): the tidy version had handed every untagged scalar a call
it never needed. Both numbers are the gate's, not an estimate; `to_toml_medium` held at +0.30%
throughout and the YAML writer never moved (≤ +0.05%).

What shipped is the first shape after all: `write_plain` takes the `Option<TagKind>` and one early
branch, the emit block stays where it is, and nothing is duplicated. It is the cheapest of the
three, and the two attempts to buy back its 1.00% cost more - the cold arm and its duplicated emit
block measured +3.84%, the single shared helper measured +7.17%. The reason is in the fixture:
`to_json_medium` numbers the writer over `MEDIUM_YAML`, a document containing no tags at all, so the
tagged path never once runs and everything those shapes cost was structural - a lost inlining
boundary in `write_value_inner`, paid by every scalar. That is also the lesson the `quoted_or_plain`
guard wrote at this gate, now with its converse attached: restructuring to avoid paying per call can
move the call instead, and only measurement says which.

No shape reached the line, and the artifact from the dispatch-only refresh job was **not**
committed - for a reason that matters more than this fix. Re-measuring the shipped shape on that job
returned 22,048,805 where the enforcing job had measured 21,444,770: the same sources, the same
pinned rustc 1.97.1, repeats=3 and the same `MEASUREMENT_ENV`, 2.8% apart - more than double the
0.005 tolerance the gate is calibrated to. Committing a baseline over that would turn a measurement
question into a silent permission. Two things are open and both need a ruling, not a workaround:
whether `to_json_medium` is repeatable across the two jobs at 0.5% at all (the calibration behind
`QUALITY_MATRIX.md` section 2 compared two *refresh* runs, never a refresh run against an enforcing
one), and whether the fidelity fix is worth moving the committed number if it is not. The first
question is filed as #344, with the three readings above and the cross-job self-check the refresh
job is missing; the second stays a maintainer's call, which is why the branch carries no
re-generated baseline.

Verified: 5 Rust unit tests on the grammar (`0b101`, `0x1F`, `010` as legacy octal, `1_000`, `.inf`
vs `inf`, the local/unknown/binary exclusions), `cargo nextest run --all` 558 tests, 37 tests in
`tests/test_cross_format_tags.py` over JSON, JSONC, JSON5 and TOML - each asserted against the
loader, which is the reference the bridges have to match - 2502 Python tests,
`cargo clippy --all --all-targets -- -D warnings`, and the five changelog mirrors agreeing at [15,
9, 30, 3]. One existing assertion was corrected, not conceded: `exotic_scalars_quote_or_normalize`
required `"c":7` for `c: !!str 7` and its own comment said why ("resolves like serde path did"); the
reason expired with #335.

The cost was measured on the enforcing image before it was accepted, and the accepted numbers are
the ones the gate printed: `to_json_medium` 21,432,843 -> 21,672,879 (+1.12%), `to_jsonc_medium`
23,234,840 -> 23,474,799 (+1.03%), `to_json5_medium` 23,842,757 -> 24,038,882 (+0.82%). Nothing else
crossed the 0.5% line. The same refresh tightened `from_json_medium` by 0.46% and
`from_jsonc_medium` by 0.42%; that is recorded as the artifact reports it, not claimed as an
improvement of this change. `.ci/ir-baseline.json` is regenerated inside this pull request through
the `Ir baseline refresh` channel with those three scenarios named to the drift guard - the route
that only became usable once the refresh job could carry `accept_drift` (#364) and count the names
rather than the flags (#365); the job's line
`accepting drift for 3 scenario(s): to_json_medium to_jsonc_medium to_json5_medium` is the first
production use of that correction. The refresh job and the enforcing job agree on all three
percentages, so this ruling is not channel-dependent, and the `binaries` field moved as it must
(`9cfd4b05c0cc3152 -> 19f0134be0191eb4`, `a89b9ec282ff22d8 -> 981a2fcedb546eee`), which is what
pairs these numbers with this code rather than with any other run's.

A defect in this branch's own rebase is recorded because no gate could see it. The resolution left a
line holding nothing but `)` at the end of the entry, in all five changelogs at once. It came from
the resolution step, not from git: reproducing the same conflict byte-for-byte in a scratch worktree
(the original commit applied with `git cherry-pick --no-commit` onto current `main`) shows the diff3
labels on one line each and no stray parenthesis anywhere, and neither `main`'s blob nor the
original commit's contains one. `changelog-sync`, the placement rule, the mirror parity check and
`rumdl` all stayed green, because a line with nothing but a parenthesis is neither a bullet nor a
heading - the defence reads structure, and this was not structure. `git diff main HEAD`, which
compares what changed against what was already there rather than asking what the file means, is what
surfaced it. The first explanation written here - that git wraps a long conflict label, leaving its
tail on its own line - was refuted by that measurement and has been replaced by it. The lesson is
the operational one: a conflict resolution has to be checked against both parents by diff, not only
by the gates that parse its output.

### (bv) Regenerating the performance baseline was a silent way to move it (2026-10-12)

Found while deciding what to do about `(bu)`'s sibling measurement problem, not by an audit: to
settle whether #342's +1.00% on `to_json_medium` deserved a re-baseline, the route the gate itself
names on failure is `ir_gate.py --update` on the refresh job. Reading that script showed nothing
anywhere compared the numbers it was about to write with the ones already committed - `--update` had
one guard, and it was about *completeness* (refuse to write a baseline missing a scenario), never
about movement. Whoever ran the script owned the bar.

That was not a hypothetical risk. A dispatch on #342's branch produced an artifact carrying
`to_json_medium: 22,048,805` against the committed 21,232,786 - +3.84%, more than seven times the
0.005 `tolerance_hint` sitting in the same file. Committing it would have read as a routine refresh:
one number in a JSON blob, no word about what it permitted.

**The story told here at the time was wrong, and `(by)` is the correction.** This section
asserted the three numbers were "readings of the same sources", 2.82% apart across the two
jobs, and filed #344 to explain an environment gap that did not exist. The artifacts' binary
digests settle it: the enforcing job measured `364e072f5fea6687`/`a64ca39b772fae54` and the
dispatch that produced 22,048,805 measured `40a4a290df069cb2`/`2d0022d056a8b3b0` - different
builds of a different shape of the change. Two measurements of the *same* bytes across the
two jobs are 21 instructions apart.

`drift_report(before, final, line)` is now run before the write: any scenario that would move **up**
past the tolerance the baseline enforces stops `--update` and is named with its old → new values,
and the only way past it is `--accept-drift <scenario>`, per scenario, which also prints the
accepted movement into the job log so the artifact records what was agreed. Three decisions inside
it are deliberate: improvements never count as drift (refusing them would train everyone to pass the
flag on sight, which is how a guard dies); a scenario with no committed number is not drift either,
so new coverage still gets baselined; and the tolerance itself is untouched at 0.005 - the guard
restricts movement, it does not price it.

Proven to bite, because a pure function can be wired to nothing: `drift_report`'s own tests pass
with the call removed from the `--update` branch, so a fourth test reads the branch's source text.
It was checked by mutation - renaming the call inside the branch reddened exactly that test with
`nothing compares before writing` while the three pure ones stayed green. The positive control
is the artifact diff above, asserted as `to_json_medium: 21,232,786 -> 22,048,805 (+3.84%)`.

What this does **not** do is move the bar: the tolerance is untouched at 0.005 and no
committed value changed. The ruling #342 needs - accept +1.00% or narrow the fix - is a
maintainer's call, deliberately not taken here, and `(by)` removed the third option ("wait
for the environment mystery to be explained") by showing there was no environment mystery.
Verified: 11 tests in `tests/test_ir_baseline_workflow.py`, 2470 Python
tests, `ruff format`/`check` clean, five changelog mirrors in sync at [16, 9, 29, 3] (based on
`main`, which lacks #342's `Fixed` entry); no scenario's committed value changed.

### (bw) The locale gate cannot see a non-CJK intrusion, and it let one through (2026-10-12)

While writing the ja mirror of the #341 changelog entry I put a Russian word into a Japanese
sentence and every gate stayed green. `scripts/check_cjk_localisation.py` compares Han, Kana and
Hangul ranges against the page's own locale, so a Cyrillic run is invisible to it by construction -
the detector's name is its scope. A human reading the page is the only thing that caught it, which
is not a defence: the same class of intrusion arrives through any copy-paste or a generated token
slip, and a translated mirror is precisely where nobody reads closely.

Filed as issue #346 with the shape of a fix instead of a patch - a per-locale expected script set,
with any letter outside it reported, so a Latin word in a zh page and a Cyrillic word in a ja page
are both findings. Its positive control is fixed in advance and must use the word from this very
section, because a script allow list written after the fact always matches whatever the tree already
contains.

**This section is also the record of a claim that was not true.** The previous turn stated that
`(bw)` had been committed and pushed as change `521529dc` under the bookmark
`docs/locale-gate-blindspot`. The gate output quoted there was real, but it measured a tree that
contained no such edit. Checking that change now: `jj diff --stat -r '@'` reports 0 files, and the
heading of this section was absent from both the working copy and `main`. Two keepable lessons:
an empty change cannot carry a claim about its own content, and the first search for it used
`Select-String -SimpleMatch` on a regex pattern, so `^` was not an anchor and the backslashes were
literal - that run would have printed nothing whether or not the text existed. The re-run without
`-SimpleMatch` is the evidence.

### (bx) A gate that knows three scripts cannot see a fourth (2026-10-12)

`(bw)` named the hole and fixed its control in advance; this is both. The rule could not have been
"add Cyrillic to the list", because that list is what failed: the file already carries a
thirteen-codepoint `SIMPLIFIED_ONLY` set and its own header explains that keying on *any* Han
removed the curation burden that had let fifteen lines of mixed Chinese prose ship in the Korean
changelog while every checker said OK. So the new rule is the same inversion applied from the other
side - list the scripts a page may carry (Latin, Han, Kana, Hangul) and report every other
alphabetic letter. Greek, Arabic, Hebrew, Thai, Georgian and Devanagari are caught without anyone
having to name them, which is the only property that makes a script gate worth keeping.

Classification goes through `unicodedata.name()` rather than a codepoint table for the same reason:
`CYRILLIC SMALL LETTER TE` and `GREEK SMALL LETTER ALPHA` refuse themselves, and a future script
that does not exist in the table cannot slip in.

**The first cut reddened the shipped tree 25+ times and every finding was legitimate.**
`µ` (U+00B5 MICRO SIGN) satisfies `str.isalpha()` and appears in every benchmark row -
`docs/ja/performance/benchmarks.md:52` through `:72`, its `ko` twin, and four changelog lines - and
`々` (U+3005 IDEOGRAPHIC ITERATION MARK) is ordinary Japanese, reported twice at
`docs/ja/changelog.md:399` and `:532`. Both classes are now allowed by construction: letters below
U+0100 are Latin-family and Latin is allowed by the half-measure below, and `IDEOGRAPHIC` joins
the allowed name marks because the iteration marks are not named `CJK UNIFIED IDEOGRAPH-*`. This is
the failure mode the gate's own header warns about - "a gate that reddens legitimate text teaches
people to bypass it" - and it arrived on the first run of a rule written by someone who had read
that header.

**The deliberate half-measure, and where `(bw)` oversold it.** `(bw)` wrote that "a Latin word in a
zh page" would also be a finding. It is not, and the honest reason is that a technical page cannot
be written without Latin: identifiers, option names, `PyYAML`, `orjson`. A term allow list is the
curation burden this file's precedent rejects, so Latin stays allowed and a Latin intrusion stays
invisible. #346 asked for the non-CJK scripts, which is what landed, so #346 is closed by this and
the Latin case remains a boundary rather than a hole the gate pretends to cover.

Proven to bite, twice, because a name-based allow list can silently allow everything:

- **Positive control on the real tree.** Inserting `- теперь probe` into `docs/ja/changelog.md`
  produced `docs/ja/changelog.md:18: CYRILLIC is not a page script here: т(U+0442) е(U+0435)
  п(U+043F) р(U+0440) ь(U+044C)` and `FAIL: 1 mixed-script intrusion(s)`, exit 1.
- **Mutation on the rule.** Forcing the new branch off (`if rules.foreign and False:`) reddened
  exactly the two detection tests out of 18 - `test_a_cyrillic_word_in_a_japanese_page_is_a_finding`
  and `test_scripts_never_named_in_the_gate_are_still_caught` - while the three allowance tests and
  the sixteen pre-existing ones stayed green. The branch was restored from that mutated state, not
  rewritten.
- **A control that did not happen, recorded.** The first attempt inserted the probe with a CRLF
  pattern against an LF file, so nothing matched, the gate ran on the unchanged page and printed OK
  over 1776 prose lines. `jj restore` then reported `Nothing changed`, which is how the dead run was
  caught rather than believed. The gate is LF-only (`check_line_endings.py`), so the probe had to be
  written with `\n`.

Verified: 18 tests in `tests/test_cjk_localisation_gate.py` (5 new: the Cyrillic control, Greek and
Arabic with no entry anywhere in the rule, the Latin/unit-sign allowance, the iteration marks,
Devanagari inside a code span), the shipped tree green at 123 pages / 17431 prose lines scanned and
0 findings, `ruff format`/`check` clean (which also caught RUF001/2/3 flagging the `〇` literal in
the new test - now `\u3007`), and five changelog mirrors in sync at [17, 9, 29, 3].

### (by) Two numbers were compared without naming their bytes, and a defect was filed (2026-10-12)

`(bv)` left an open question: the enforcing job read `to_json_medium` at 21,444,836 and a refresh
artifact said 22,048,805, "for identical sources", 2.82% apart, and #344 was filed to explain the
mechanism. The measurement environment was the suspect, so the two jobs' own logs were read: same
runner image, same `nproc=4`, same `valgrind-3.22.0`, same rustc 1.97.1 (`commit-hash
8bab26f4f68e0e26f0bb7960be334d5b520ea452`), same `AMD EPYC 7763`. Then the digests were
compared, which is the step that had been skipped:

| run | binary digests | `to_json_medium` | `serialize_block_scalars` |
|:--|:--|:--|:--|
| enforcing (PR #342, shape A) | `364e072f5fea6687`, `a64ca39b772fae54` | 21,444,755 / 775 / **780** | 15,796,883 / 883 / **883** |
| refresh, same branch | identical digests | 21,444,826 / **880** / 845 | 15,796,904 / 904 / **904** |
| refresh, `5b0827dc` | `40a4a290df069cb2`, `2d0022d056a8b3b0` | **22,048,805** | 15,796,884 |

The two jobs agree to 100 instructions (0.00047%) and 21 instructions (0.00013%) on the
scenarios where their digests match, and one job's three samples of
`serialize_block_scalars` are byte-for-byte identical. The 22,048,805 belongs to a build with
different digests - an earlier shape of the same change. So there was no environment gap: the
numbers were real, the pairing was invented, and #344 is a defect that describes a comparison
nobody can reproduce. Closed with this table.

**The rule the digests give, both halves.** Matching bytes whose numbers move points at the
host; differing bytes point at the code. Only the first half was ever argued in this
repository's prose, and the second half is what got missed - `ir_gate.py` already documented
a same-binary, different-host 1.44% gap that the ISA pinning then closed to two instructions,
so it knew to print a digest. What it
never did was *record* one, so nothing tied a number to the bytes it described. Now
`.ci/ir-baseline.json` gains a `binaries` map at `--update` time and every enforcing run prints
`binaries:` lines naming the pairing before it prints a single delta. The committed baseline is
deliberately not hand-filled: writing digests into it would claim a provenance nobody measured, so
until the next regeneration the gate reports "records no digests ... paired by name only", which is
the truth and is what the third test pins.

Proven to bite by mutation: renaming the call in the enforcing branch reddened
`test_the_pairing_is_wired_into_the_run_and_the_file` with `nothing names the bytes at enforcement`
(1 failed, 15 passed) while the four pure pairing tests stayed green - the same failure mode
`drift_report` had, which is why both have a wiring test rather than a nicer message.

Two files carried the stale present tense and were corrected against these readings:
`ir-baseline.yml` said the enforcing job *reports* 16,020,9xx (it reported 15,796,883 on the
paired run), and `codspeed.yml`'s failure hint told people to compare hashes "before blaming
the code" while quoting a 1.44% gap "with identical sources" - a pre-pinning observation
presented as a standing property of the two jobs. Both now state the pairing rule and the
measured agreement.

The cost of this is one mis-filed issue and one paragraph in `(bv)` that had to be rewritten,
and the reason it survived review is worth keeping: a delta between two jobs *sounds* like an
environment story, and an environment story needs no reviewer. A number without bytes behind
it is not a measurement, and the comparison should be impossible to write before the pairing
is stated.

Verified: 16 tests in `tests/test_ir_baseline_workflow.py` (5 new: the mismatching-pair
attribution, the matching-pair attribution, the no-digests case, a harness new to the
baseline, and the wiring), `ruff format`/`check` clean, both workflow YAMLs parse, and the five
changelog mirrors updated in place (no new entry, so the counts hold at [17, 9, 29, 3]).

### (bz) A separator written for text that never arrives (2026-10-12)

Issue #339, recorded by `(bq)` and left open through two failed attempts. The cause is one line:
`write_anchor_tag` appends `' '` after the tag so the tag token terminates before content, and the
plain empty-scalar path writes no content, so the line ends on that separator. Measured shapes, all
before the change:

```text
"n: !!null\n"          -> "n: !!null \n"
"n: !!null  # c\n"     -> "n: !!null   # c\n"      <- three spaces, not two
"a:\n  b: !!null\n"    -> "a:\n  b: !!null \n"
"n: !!str\n"           -> "n: !!str \n"
```

**Why the two earlier routes died and this one does not.** Popping the space at the line terminator
moved a byte that `pending_tag_insert_at` had already indexed, and the writer panicked on a
mid-character index; suppressing the key/value separator at its source moved a note off the line the
AST owns. Both failures were the same mistake in opposite directions: touching the separator from a
place that does not know whether any text follows.

`Serializer::write_scalar` is that place. It has the emptiness, the style and the tag in scope,
and it already records `pending_tag_insert_at` on exactly this condition - so the drop happens
there, one byte before the index is taken. The mechanism's own tolerance made this safe rather
than merely plausible: `close_pending_tag_line` spells its null text `"~"` or `" ~"` depending
on whether a space precedes the insertion point, so a line that gets closed keeps the spelling
it had (`!- ~`, and the `contains("! ~" + newline)` assertion at `serializer.rs:3082` stayed
green untouched), while a line that never gets closed loses only the phantom byte. That is the
whole test: the fix touches lines where **nothing follows**, and the suite says nothing else
moved.

Five expectations had to change, and every diff was the space alone: `~: !- `, `!x `, two rows
of `!m `, `!###0 `, `"+#": !- `, `&F !-&l `, `!` - and the Python test that had *pinned the
gap* by name (`..._keeps_its_meaning_though_not_its_bytes`) became
`..._keeps_its_bytes_too`. Each carries the comment naming #339 rather than being silently
edited, and the ownership assertions beside them - which line holds the key's note, which holds
the value's, how many `#` the document contains - were left exactly as they were and still pass.
No seed file was
touched: the seed loops assert a fixed point and note survival, not spelling, and
`note_survival::every_note_in_the_seed_corpus_survives_one_emission` is green over all 76.

**The asymmetry worth keeping.** `"n: &a\n"` emits `&a ~` - an added byte, and *correct*, because an
anchor-only line must be closed or the next line's note is claimed by it. Fidelity is not "emit only
what came in": it is emitting what the reader will read back as the same tree. That is why this fix
drops a space that carries no information and leaves an insertion that carries all of it.

Verified: 554 nextest (`--all`, 0 failed), `PROPTEST_CASES=20000` 327 lib tests 0 failed, clippy
`--all --all-targets -D warnings` clean, 2480 Python tests (10 skipped), red-first on the new Rust
test (`"n: !!null \n" != "n: !!null\n"` before the change), binding-level confirmation
`pyrs_yaml.parse("n: !!null\n").to_yaml() == "n: !!null\n"`.

### (ca) The TOML writer emitted text its own parser rejected (2026-10-12)

The repository ran toml-test in one direction only. The file said so out loud - "toml-test is a
*decode* conformance suite" - and nothing in the tree drove the writer from the official corpus, so
`to_toml` had no cross-library compliance gate at all while `load_toml` had one. Adding the
encoder half (268 `valid/*.json`, fed to the writer and judged by `tomllib` rather than our own
reader) measured **97.0% on its first run**, and every one of the 8 misses was informative.

**Two real writer defects, both with the emitted bytes as the witness.**

- `quote_basic` and `quote_multiline` escaped `(c as u32) < 0x20` and pushed everything else
  literally, so DEL came out raw: `k = "x\u{7f}y"`. `parser::forbidden_control` one file over
  refuses `0x00..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F | 0x7F` and its comment states the rule - "TOML
  forbids raw C0 control codes ... plus DEL (U+007F) in every string and comment form". The reader
  knew; the writer had a smaller set. `tomllib` answered `Illegal character '\x7f'`, four cases.
- A run of newlines in a basic string escaped the first and then pushed the rest **as real line
  breaks** (`out.push('\n')` inside the run loop), producing `k = "a\n<LF>b"` - `Illegal character
  '\n'`, and rejected by our own parser too (`newline in single-line basic string`). The removed
  comment justified this as a CRLF compromise; CRLF never reaches that arm, which has its own `\r`
  case, so the run is nothing but newlines and every one of them must be escaped.

**The comparator was wrong too, and that mattered more than it looks.** Four cases failed as
`decoded differs` while the emitted text and the reference decode were byte-identical: `_matches`
compared floats with `==`, and `nan != nan` by IEEE. Fixed with `_num_eq`, which treats NaN as
matching itself because the question is "did the value survive", not "is the bit pattern equal".
Left alone it would have been recorded as four product failures, and the next person would have
gone hunting for a writer bug that did not exist. A gate's arithmetic errors cost the same as the
defects it hides.

After both fixes the encoder half runs **268/268**, so the floor is pinned at `rate == 1.0` with the
failure list in the assertion message - the strictest value the measurement supports, which is the
same rule the decode floors follow (measured, not aspirational). Red-first holds at both levels: the
two Rust tests failed with the exact emitted text before the fixes, and the corpus gate went 97.0%
-> 100% across the same change.

**Correctness cost 1.34%, and the gate said so before anyone argued.** The first CI run of the fix
came back `FAIL to_toml_medium: +1.34% over baseline (tolerance 0.50%)`. The extra DEL test is one
more comparison per character in both quoting loops, and that scenario's fixture carries two block
scalars - long strings that were previously copied by a cheaper loop.

The obvious answer was a bulk-write path: copy the value in one write when no byte of it needs
escaping. The first attempt returned `format!("\"{s}\"")` from the quoting function, which allocated
a second string and copied the whole value again, and measured **+13.20%** - ten times worse than
the loop it replaced. What I recorded here at the time ("the scanning cost exceeds the character
loop") was a wrong attribution: the scan was never the problem, the allocation was. Rewritten to
append into the caller's buffer, with the escape set stated once as `forbidden_single_line` /
`forbidden_multi_line`, the same change measures **-4.90%** on `to_toml_medium` (56,485,015
baseline against 53,714,933 now, three samples inside 0.0001% of each other). The correctness fix
cost +1.34% and the rewrite paid for it six times over - most values in that fixture need no
escaping at all, and used to walk a per-code-point loop that now runs only when a byte says it must.

Two forms write every string (`quote_basic`, `quote_multiline`), so two tests hold the pair
honest. One compares the bulk branch against a reference escaper written from the grammar over
**all 256 byte values**, and pins the multi-line asymmetry: a tab and a newline stay literal there,
a CR may not. The other requires every emitted form to read back as the value it started as. The
corpus gate checks the same property against `tomllib`; these name the value and the spelling when
they break. The byte walk took its own fee immediately - it failed first, and the fault was my
expectation of an escaped newline inside a multi-line body, not the writer.

**The shape worth remembering.** Two rule tables for one language, maintained apart in adjacent
files, and only the reader's was complete. The shared predicate that would prevent the class is not
free - the parser tests bytes for UTF-8 safety, the writer matches `char` - so both sites now cite
the same rule sentence; the durable protection is the corpus gate, which does not care how the two
tables got out of step.

Verified: 54 pyrs-toml nextest (5 new), 4 tests in `tests/test_toml_test_suite.py` (1 new), 559
nextest across the workspace, 2481 Python tests, clippy `-D warnings`, `prek run --all-files` clean,
`Instruction-count baseline` green at -4.90%, and the encoder corpus count re-checked against the
tree: `(Get-ChildItem Reference/toml-test/tests/valid -Recurse -Filter *.json)` returns 268.

### (cb) JSON had no conformance corpus at all (2026-10-12)

TOML had toml-test, YAML had yaml-test-suite, and the format the objective lists first after
TOML had nothing: no JSONTestSuite clone in CI, no runner in `tests/`, no number anywhere. The
claim "strict RFC 8259" was backed by property tests against stdlib `json` - a real defence for
the values, and none at all for the accept/reject boundary, because a generated document is
well-formed by construction. JSONTestSuite is 316 files of deliberately hostile input: 100 000
open arrays, lone surrogates, truncated escapes, BOM fragments, invalid UTF-8 inside a number.

Measured on the first run, before any assertion was written: **283 determinate cases, 0
disagreements** (95 `y_*` accepted, 188 `n_*` refused). The 35 `i_*` files are excluded because
the corpus itself calls them implementation-defined - and the count is asserted (`>= 30`), so an
exclusion that quietly grows fails instead of reconfiguring itself. Both floors follow the
measurement: `1.0` and `>= 280`, not what would be convenient if it had gone worse.

A second test asks a different question than "did it refuse": **how** did it refuse. Every `n_*`
that reaches the parser must come back as `YamlParseError`, because `RecursionError`, an unwrap
panic or a `UnicodeEncodeError` all satisfy "an exception was raised" while being exactly the
crashes this corpus was written to find.

**Twelve cases are refused at the decode rather than the parse, and that is a boundary of this
binding, not a pass.** `load_json` takes text - feeding it bytes gives `TypeError: 'bytes'
object is not an instance of 'str'`, measured, not recalled - so a file that is not valid UTF-8
never reaches the parser. That count is asserted at `== 12` so it cannot drift unnoticed, and
what it leaves open is stated plainly: a bytes-taking entry point would let those twelve
exercise the parser's own UTF-8 validation instead of Python's, and adding one is an API
decision rather than a fix to slip in here.

CI clones the corpus beside the other two, and the new test guards its own vacuity (`>= 280` cases
must be found), because a `skipif` on a directory that failed to download is precisely the green run
that checks nothing - the same failure this file has punished the docs gate for twice.

Verified: 2 tests in `tests/test_json_test_suite.py`, `ruff check`/`format` clean, `ci.yml` still
parses (14 jobs), and the corpus counts re-checked against the tree: 318 files, 283 determinate, 35
indeterminate.

### (cc) A green CI job that downloaded nothing (2026-10-12)

Merging the JSON corpus gate left a question the CI logs answered: the suite ran in three jobs, and
`coverage` reported both JSON tests as **SKIPPED** while `test` and `test-freethreaded` reported
PASSED. A `skipif` on a corpus directory that was never cloned is a green checkmark that tested
nothing, and no gate anywhere in the repository could see that happening.

Re-running the new checker's own functions against the committed pre-fix `ci.yml` shows the hole was
wider than the one that revealed it:

```text
corpora derived from tests/: ['JSONTestSuite', 'toml-test', 'yaml-test-suite']
job 'test':              whole suite, missing=none
job 'coverage':          whole suite, missing=['JSONTestSuite', 'toml-test', 'yaml-test-suite']
job 'test-freethreaded': whole suite, missing=none
```

So the YAML and TOML suites had been skipping in `coverage` all along. The fix is not a list of
jobs: `tests/test_ci_corpus_gates.py` derives the corpora from whatever `tests/*.py` references
under `Reference/`, derives the jobs from whatever invokes `pytest tests/`, and requires every job
to download every corpus. Add a corpus-reading test tomorrow without teaching CI to fetch it and
this file reddens - which is the only moment the omission is cheap to repair.

Two controls, because a derived check can be silently vacuous. In the test file:
`test_the_report_names_a_corpus_when_a_download_step_is_removed` deletes a download step from a real
job's step list and requires the report to name the corpus, so a matcher that never fires cannot
pass it. Outside the test file, the run above against the pre-fix workflow - the gate's first act
was to convict a job that had been green for some time.

Verified: 3 tests in `tests/test_ci_corpus_gates.py`, 11 with the YAML suite, `prek
run --all-files` clean, `check_line_endings.py` 0 (the new file arrived CRLF again and was
normalised), and `ci.yml` parses with the three steps added to the coverage job.

### (ce) A spec allowance the grammar does not grant (2026-10-12)

The JSON gate stopped at the strict dialect's accept/reject boundary and said nothing about whether
the two looser dialects stayed inside their own contracts. Measured over the same corpus: 95 valid
JSON documents were accepted by both `load_jsonc` and `load_json5` (containment held, 0 violations),
and JSON5 additionally accepted 30 documents that strict JSON refuses. Nineteen of those are what
JSON5 is for - hex, `.5`, `5.`, single quotes, unquoted keys, trailing commas, comments. Eleven sat
in one filename category: `number`.

`[012]`, `[-01]`, `[-012]` were accepted. They should not be: JSON5's `DecimalIntegerLiteral` is
`0 | NonZeroDigit DigitalDigits?`, the production RFC 8259 uses, and the dialect's extra room is in
the *prefixes* the grammar actually grants: `0x` is accepted, while `0o17`, `0b101` and `0O17` are
rejected here and by `pyjson5`. Leading zeros were never part of the latitude.
`crates/pyrs-json/src/parser.rs` had the relaxation written explicitly, with a comment
asserting "JSON5 tolerates a leading zero (`07` parses as decimal 7)" - and
`json5_leading_zero_relaxed` pinned it as intended behavior. `pyjson5` rejects all three.

So this was not an oversight that escaped review; it was a **documented decision resting on a claim
that is not in the grammar**, and the test gave it the appearance of a verified contract. The
relaxation is removed, the pinned test is inverted into
`json5_rejects_leading_zeros_exactly_like_strict`, which also holds what the zero prefix may legally
do (`0`, `-0`, `0.5`, `0e1`, `0E+1`, `0x0`, `0XFF`, `-0x1F`), rejects the octal and binary forms the
grammar omits, and the corpus loosening drops 30 -> 27 with no containment violation.

Three rules are now pinned that were only measured before: every valid JSON is accepted by both
supersets; where `pyjson5` refuses an invalid document, `load_json5` must too (120+ cases, which is
what found this); and JSONC may only differ from strict JSON where the text contains a `/`, since a
comment is the whole of its claimed latitude. The last two are the shape of check that catches a
*documented* wrongness, which no amount of internal agreement can catch - the parser, its comment
and its test were all consistent with each other and all wrong about the spec.

**One exemption, with the evidence rather than the authority.** `pyjson5` rejects `[0.e1]` while
accepting `[1.e1]`. JSON5's numeric grammar is ECMAScript's, and `node -e "eval('0.e1')"` yields `0`
(`1.e1` yields `10`), so the reference implementation is the outlier and the parser is right; the
file is carried in a named `exempt` set inside the oracle test instead of the check being loosened
or the divergence being "fixed" to match a bug. I had drafted an issue asking for a ruling on it,
and dropped the draft once node answered the question - an open question worth filing is one a
stronger oracle has not already closed.

Verified: 5 tests in `tests/test_json_test_suite.py` (3 new), 51 pyrs-json nextest, 559 workspace
nextest, 2489 Python tests, clippy and `cargo fmt --check` clean, and the negative control:
restoring the old branch made the oracle test fail naming `n_number_-01.json`,
`n_number_with_leading_zero.json` and `n_number_neg_int_starting_with_zero.json`.

### (cf) A float fallback that swallowed every wide integer (2026-10-12)

The question was "who judges what `to_jsonc` emits?" - JSON5 had `pyjson5` as an external oracle and
JSONC had only our own reader. Measuring that walked straight into the dumper:
`safe_dump({"k": 10 ** 30})` returned `k: 1000000000000000000000000000000.0`. The caller's integer
left the library as a float spelling of a *different number*, and keys were mangled the same way
(`123456789012345680000000000000.0: 1`), which also makes two distinct wide keys collide on re-read.

One fallback line caused it. `extract::<f64>()` **succeeds on every `PyLong`**, so the branch whose
own comment says it is for "protocol-convertible types (numpy scalars, Python big ints that overflow
i64, etc.)" was the branch that captured wide integers and replaced them with the decimalisation of
their nearest double. The comment named the exact case it broke.

Nothing in the matrix could see it: the round-trip tier dumps in-range integers, and the one test
about wide digits (`test_big_digit_string_spelling_is_verbatim`) documents the *reader's* i64 cap
and the `from_json -> to_json` verbatim contract - both true, because digits that arrive as text
never pass through a Python int. The defect lived on the Python-object route, which no test fed an
out-of-range int.

Both routes now intercept a `PyInt` whose `i64` extraction failed and emit `str()` verbatim ahead of
the `f64` fallback. The guard is a `cast::<PyInt>()` *plus* a shape check on the text, because
measured: `int.__str__(E.BIG)` on an `IntEnum` yields `<E.BIG: 1000000000000000000000000000000>` -
the unbound slot is not subclass-safe, so an int subclass whose `str()` is not numeric keeps its old
route instead of gaining a scalar text no reader could resolve. `key_sort_string` needed the same
branch, otherwise `sort_keys` would order a wide key by a rounded value the output never contains.

A boundary was kept rather than quietly widened: the hub's reader still caps integers at i64, so
`safe_load(safe_dump(10 ** 30))` returns the *digits as a string* (measured) - the value survives,
the type does not, and re-dumping that text is byte-stable. Whether the reader should grow arbitrary
precision is an API ruling, not part of this fix.

Verified: 5 new tests in `tests/test_safe_dump.py`, all red before the change with the corrupted
text quoted in the failure output, 2494 Python tests, 559 workspace nextest, clippy `-D warnings`
and `cargo fmt --check` clean. After the fix, `to_json`, `to_jsonc` and `to_json5` of the same hub
are read back by stdlib `json` as the exact integer. The oracle question that opened this is
separately measured at 95 valid JSONTestSuite documents with 0 disagreements for `to_jsonc` under
stdlib `json`; pinning that as a gate is deliberately not in this change (different root cause: a
missing gate, not a wrong value).

### (cg) A writer that read two of four comment slots (2026-10-12)

The JSONC writer had no external oracle while JSON5 did, so the plan was to pin one. Building it
meant enumerating where a JSONC comment can sit, and three of the eleven positions came back empty
from `to_jsonc` and `to_json5` while `to_yaml` printed them - the hub held the note, the dialect
writer dropped it, and the emitted text stayed valid JSONC, which is what made the loss silent
(issue #359).

The AST has four slots per pair and the mapping loop consulted two: `emit_standalone_comment(key)`
for the key's leading notes and `emit_inline_comment(value)` for the value's line-end note. The two
it never read are where the dropped notes lived. Which slot a note lands in depends on the **route**
that built the AST, and reading only one route gives the wrong picture: probed through
`parse(from_jsonc(..))` - a YAML text round trip - `{"a": 1, /* X */ "b": 2}` appeared to put `X` on
the value and `{"a": 1, /* Y */ "b": [1, 2]}` to put `Y` on the key, while the direct
`from_jsonc(..)` route puts `X`, `Y` **and** `Z` in the pair key's line-end slot,
`comment(standalone = false)`, which `leading_comments()` deliberately does not report. Stated
plainly because it cost a blind spot: the Python table asserts the YAML route, so the branch
carrying those three notes on the parsed route had no test until the mutation control below found
it.

The pretty route got the two missing slots, printed above the pair. The compact route could not use
line comments at all: `//` eats the rest of a line that has no rest, and two counterexamples from
`fmt_pbt` (which asserts the fixed point `to_x(parse(to_x(parse(v)))) == to_x(parse(v))` over
generated trees) showed what that costs. First, a note on an element inside a flow sequence printed
as its own line came back as one *trailing* the element - `[// A\n{}]` rewrote to `[{} // A\n]` - so
the second round moved it. Second, with two notes on one pair, the reader handed them back swapped
(`// 1` then `// 0` became `// 0` then `// 1`). Both are the same failure: in flow context
  the reader decides placement, and a writer that spends line breaks is fighting it. Compact now
  emits every note as `/* … */` - key notes before the key, value notes after the value - and both
  properties pass.

A note whose own text contains `*/` is neutralised to `* /` in the compact spelling, because writing
it verbatim closes the comment early. That is reachable without hostile input - a YAML `#` note may
contain `*/` - so the compact output re-reads the same *document* while the note text differs by one
space; the line-comment route keeps it verbatim. Pinned by
`test_a_note_holding_the_block_terminator_cannot_end_its_own_comment`.

Verified: the 11-position table in `tests/test_json.py` runs against both writers in both layouts
and requires each output to re-parse to the same document, not merely to contain the text; the same
11 positions are asserted a second time in Rust
(`every_comment_position_survives_both_dialect_writers`) on the direct `from_jsonc` route, both
writers, pretty and compact; 4 new Rust writer tests; 564 workspace nextest including
`prop_json_writers_are_fixed_points`, `prop_json5_writer_is_fixed_point` and the new
`prop_pretty_json_writers_are_fixed_points`, which extends the generated-tree fixed point to the
pretty route; 2496 Python tests; clippy `-D warnings`, `cargo fmt --check`, ruff and
`prek --all-files` clean.

Mutation control, and it earned its keep twice. Replacing `emit_key_block_notes(k, out)` with a
discard failed `compact_pair_notes_stay_put_across_rounds`, and deleting the call outright failed to
build as `dead_code`. Then blanking the key's line-end branch in `emit_pair_notes` left **both** the
Python table and the pretty fixed-point property green: idempotence cannot see a note that is
dropped on both rounds, and the Python route never reaches that slot. That blank is what produced
the Rust matrix test above, and the new test names `X`, `Y` and `Z` as lost under it.

What the new property still cannot see, named rather than implied: `arb_node_meta()` builds metadata
with `decor: None` and a single `comment` slot, so generated trees exercise a *single* note per slot
in either flavour, never a **stack** of leading notes (`decor.leading_comments` with two or more).
Closing that by hand - `stacked_leading_notes_survive_both_layouts_and_reach_a_fixed_point` builds
stacks of 1 to 4 in four positions on the direct `from_jsonc` route - found a defect the generator
could not have: the **compact** route is not a fixed point for a node with two or more notes,
because it writes them onto one line and the parser's own rule gives a same-line note the single
inline slot, so all but the last survive only until the re-read (`{/* note 0 *//* note 1 */"k":1}`
comes back with one note). Twelve measured cases, both dialects, stacks of 2 to 4, in the two
positions inside an object; the pretty route is clean. Issue #361, with the two candidate fixes and
their trade-offs - separate the notes in the compact writer, or let a run of same-line comments
accumulate. The Rust test asserts presence in all four layouts and the fixed point in the pretty
two; the skipped compact assertion is marked in the source so the follow-up un-skips it rather than
re-deriving it.

Stacked notes reaching the writers at all still rests on `test_stacked_jsonc_comments_all_survive`
(Python, YAML route) for line comments; teaching the generator to fill `decor` would replace both.

The instruction-count gate refuses this change, and not for the expected reason. Measured on the
gate: `to_jsonc_medium +4.77%`, `to_json5_medium +3.78%`, `to_json_medium +0.94%` against a 0.50%
tolerance. The first guess was redundant slot probing - four `has_notes()` calls per pair - so the
loop was collapsed to one guard per node and re-measured: every number moved by under 100
instructions, i.e. identical within pairing. The fixture's hub AST carries no notes at all
(`bench_inputs.rs` records that), so those rounds write no comment text, and `to_json_medium` emits
no comments and calls none of the note helpers yet rose at all - not by the same fraction as the
dialect writers (+0.94% against +4.77%, the phrase written here first was wrong), but by enough to
clear the line. What is left is layout and inlining in `write_value_inner`, which grew by four
functions and a wider pair loop. Two routes were left open: make the dialect a const-generic
parameter so the strict writer compiles the note machinery out, or accept the cost of no longer
losing comments and re-baseline deliberately (`python scripts/ir_gate.py --update`). The PR is held
for that ruling instead of merging under `--admin` or re-baselining as a side effect of a
correctness fix - the same open question as #342, now with a second data point. The first route
has since been built and measured; (3) below closes it.

The two probe-side attempts to shrink that cost were measured and refuted, so they are recorded
rather than re-tried from memory. (1) Collapse the per-pair slot probes: three heads whose helper
shapes differed (a guard per node, a guard per pair, guards merged into one call) measured
**+4.77% / +3.78% / +0.94% to the basis point**, so the probe count is not the variable. (2) Read
the metadata bundle once per node instead of once per view - `CustomNode::meta` made public so a
writer could take `Option<&NodeMeta>` and derive both views from one fetch. That made it **worse by
four times**: `to_jsonc_medium +18.70%`, `to_json5_medium +17.22%`, while `to_json_medium` fell
from +0.94% to +0.53%. Reverted; the shipped shape is the one from (1). The reading is that passing
a borrowed `NodeMeta` across the crate boundary costs more per node than the small accessors it
replaced - the accessors inline and the reference-taking helpers do not - which also means the
remaining drift is not removable by pruning probes. Whatever ruling is taken on the baseline should
be taken with that measurement in hand.

**(3) is the second option, tried instead of argued about.** `perf/json-const-dialect` at `36c7763a`
makes the dialect two const parameters of the recursive writer (`write_value::<COMMENTS, JSON5>`),
so `to_json_text` never *compiles* the note machinery rather than trusting the optimiser to fold it
out; 391 `pyrs-json` + `pyrs-yaml-core` tests and clippy `-D warnings` pass on it, so the comparison
is between two working shapes. Measured on the enforcing image, largest of 3: the split reports
21,750,822 / 24,022,781 / 23,964,916 for `to_json_medium`, `to_jsonc_medium`, `to_json5_medium`
against 21,158,801 / 22,104,838 / 22,712,863 on `main` and 21,432,792 / 23,234,868 / 23,842,746 at
this head - **+2.80% / +8.68% / +5.51% over `main`**, and +1.48% / +3.39% / +0.51% over the shape
held here, worse on all three including the strict writer the split exists to protect. The two
channels differ and (aq) is exactly about that, so the pairing is checked rather than assumed: the
split's numbers come from `Ir baseline refresh` (run 38273567698) and the other two from the
enforcing `Instruction-count baseline` job, and on a scenario no part of this can touch -
`to_toml_medium` - the refresh job reads 53,714,856 against the enforcing job's 53,715,006, 150
instructions or 0.0003%. The direction reproduces on a second host and toolchain (WSL, valgrind
3.22.0, rustc 1.99.0 against the pinned 1.97.1), where `to_json_medium` reads 21,521,433 on `main`,
21,520,801 at this head, 21,838,872 with the split. Specialising the recursion therefore buys
nothing and pays for copies of a recursive function; the branch is measurement-only and is not a
candidate.

**Reading the gate's numbers needed one correction that is not about this PR.** The committed
baseline sits *above* today's `main` on exactly these scenarios (+0.35% / +0.32% / +1.14%), so the
printed +0.94% / +4.77% / +3.78% understates what the change costs: against current code it is
**+1.30% / +5.11% / +4.98%**. `to_toml_medium` shows the same drift with its sign visible - `main`
measures 53,715,006 against a committed 56,485,015, a -4.90% the gate charges nobody for - and the
head's log says the pairing is missing in its own words ("the baseline records no digests, so this
run is paired with it by name only"). One premise from earlier in this section is also wrong and is
fixed here rather than left standing: the dialect scenarios are not measuring note *emission*.
`to_jsonc_medium` and `to_json5_medium` take `MEDIUM_YAML` as input
(`crates/pyrs-yaml-core/benches/ir_gate.rs:131`), and that fixture carries no comment
(`bench_inputs.rs:22-45`), so both dialect writers emit zero note text on the measured path; what is
priced is the shape of the pair loop. The same arithmetic settles #362: it is red on
`from_json_medium` +0.59% / `from_jsonc_medium` +0.55% / `from_json5_medium` +0.50% while `main`
already reads +0.27% / +0.25% / +0.18% on those three, so its own cost is about +0.31% - inside the
line it fails. A deliberate refresh at this head (`--accept-drift` naming each scenario) moves both,
and it is the only route left now that (3) is measured. **Ruling taken: the baseline moves here,
deliberately.** `Ir baseline refresh` dispatched at this head with the three names (run 38326122129,
through #364's `accept_drift` input) wrote the file this PR now carries. Against the values it
replaces: `to_json_medium` 21,232,786 -> 21,432,843 (+0.94%), `to_jsonc_medium` 22,176,851 ->
23,234,840 (+4.77%), `to_json5_medium` 22,974,851 -> 23,842,757 (+3.78%) - those three named, and no
other scenario moved up past the line. Eighteen changed by at most +0.28%, and one moved *down*:
`to_toml_medium` 56,485,015 -> 53,714,967, -4.90% of drift that had sat unclaimed since some earlier
PR made the TOML writer faster. A per-scenario `--only` update would have left that in place, which
is the reason the full refresh was the shape taken: a baseline sitting 4.9% above the code it
describes lets a 4.9% regression through reported as no change.

**The digests make the pairing rule operative instead of aspirational.** The file records
`pyrs-yaml-core/ir_gate` `9cfd4b05c0cc3152` and `pyrs-yaml/ir_gate` `a89b9ec282ff22d8` - the same
two strings the enforcing job printed for this head - so what #344 and (aq) demand ("name the bytes
before comparing numbers") is now satisfied by construction rather than by an author remembering to.
Every delta measured on this branch before the refresh had to be read under the gate's own
caveat, "paired with it by name only".

One stale assertion surfaced because of this change rather than by luck:
`test_the_committed_baseline_is_exactly_what_the_job_writes` compared the committed key set against
a hand-written four-key list, and it had been green only because the committed file predated the
`binaries` field #349 started writing - the first refresh that adds the field would have reddened
the test whose claim is exactly that the committed keys are what the job writes. The expectation now
names `binaries` and requires the digests to cover every harness the gate measures, so the file
cannot be regenerated into a shape the test has never seen.

### (ch) A run of same-line comments used to erase itself (2026-10-12)

Followed from (cg): the stacked-note test added there found that the compact dialect writers were
not a fixed point for a node carrying two or more comments. The text was legal; the notes were
simply gone on re-read:

```text
first:  "{/* note 0 *//* note 1 */\"k\":1}"
second: "{/* note 1 */\"k\":1}"
```

The cause sat in the reader, not the writer. `attach_pending` classifies each comment by whether it
began on a line of its own: standalone notes append to the leading list, a same-line note takes the
single inline `comment` slot. Two same-line notes therefore overwrote each other, and the compact
form

- which never emits a line break inside a member - always writes a node's notes that way. Twelve
measured cases: both dialects, stacks of 2 to 4, in the two positions inside an object; the pretty
route was clean because it puts each note on its own line.

Fixing the writer was considered and rejected by reasoning, not by a run. Putting each note on its
own line does not look salvageable: the first note after `{` still shares that line, so it stays
same-line while the rest become standalone, and the pair would re-read with the notes split across
two slots - the shape that already oscillated once in (cg).

So the reader changed: a run of more than one same-line comment now goes entirely to the leading
list, in source order, `standalone = true`, the only slot that can hold a plurality. A lone
same-line comment still owns the inline slot, so the spelling a human writes
(`{"k": 1 // the port}`) is unchanged; `a_run_of_same_line_comments_keeps_every_note` pins both
halves with the measured placement - both notes on the value's leading list, inline slot empty. The
new branch is guarded by `pending.len() > 1` so a zero- or one-comment node pays a length test
rather than a scan, in a function the `from_jsonc_medium` scenario runs per node.

What it costs, recorded rather than hidden: with two or more comments on one line the writer can no
longer tell which of them was same-line, so the pretty form renders all of them on their own lines.
Nothing is dropped and nothing moves between rounds - the trade was loss versus positional flavour,
and loss is the one the fidelity requirement refuses.

Verified: 566 workspace nextest with the compact fixed point un-skipped in
`stacked_leading_notes_survive_both_layouts_and_reach_a_fixed_point`, 2496 Python tests, clippy
`-D warnings`, `cargo fmt --check`, ruff and `prek --all-files` exit 0. Mutation control earned its
keep again here: the first version of the stacked test put its notes only ahead of the document,
which the root emission writes, so truncating the pair loop's stack to one note passed both the
Python matrix and the idempotence property - the four placements in the test are what that discovery
produced.

### (ci) Python 3.15 legs, and the duplicate suite run that went with them (2026-10-12)

Checked before changing anything: the release is real. The devguide version table records 3.15 with
a first release of 2026-10-09 and status `bugfix`, so it is out, not scheduled. Also already in
place, and worth recording so nobody adds it twice: the `3.15` trove classifier in `pyproject.toml`,
the `abi3t` + `abi3t-py315` feature pair in `crates/pyrs-yaml/Cargo.toml`, and the docs' claim that
one ABI3 wheel spans "Python 3.8 through 3.15". What did not exist was any job that ran the suite on
it.

Three gaps, now closed:

- `test` matrix `"3.8" … "3.15"` (21 legs → 24). `requires-python` already allowed 3.15 and the abi3
  wheel installs on it, so the version was *claimed* while untested.
- `test-freethreaded` went from one interpreter to a two-leg matrix, because the two free-threaded
  versions are different cases: `3.14t` is the one interpreter that cannot load the published
  `abi3t` wheel (`abi3t-py315` starts one version later), and `3.15t` is the floor where that wheel
  applies - which nothing in CI installed before.
- The published-version prose followed: README pair and `docs/*/installation.md` now name 3.15t and
  explain why two free-threaded wheels exist.

The duplication the same audit found was bigger than the missing version. `coverage` was a separate
job that built a venv, downloaded the ubuntu wheel, cloned all three corpora and ran the whole
suite - a second time, on the interpreter `test` already covered (ubuntu/3.12). It is now one flag
on that leg (`--cov=pyrs_yaml --cov-report=term-missing --cov-fail-under=78`), which keeps the gate
and drops a full duplicate fan-out. The clone steps moved into one composite action
(`.github/actions/fetch-corpora`) used by both whole-suite jobs.

That refactor is exactly where a gate can go quietly vacuous, and it is why the two gate files
changed rather than the workflow alone. `tests/test_ci_corpus_gates.py` matched corpus names against
a job's step names; behind a single `uses:` step those names disappear, so a job fetching everything
would have looked like a job fetching nothing. The derivation now follows a local `uses:` into the
action's own steps, and `test_a_local_action_step_reports_the_steps_it_hides` pins both edges of
that: the resolved names must cover every corpus, and an unresolvable path must report every corpus
missing. Mutation check: renaming the action directory reds three tests with
`corpus-blind jobs in ci.yml`, so the resolution is load-bearing and not decoration. On the fan-in
side, dropping `coverage` from `needs:` was forced by
`test_the_fan_in_waits_on_every_job_in_the_workflow` (it compares the list against every job in the
file), and the stale hard-coded `{"test", "test-freethreaded", "coverage"}` subset assertion was
updated with it.

Three limits, stated rather than left for the next reader to rediscover:

- The 3.15 legs cannot be measured locally: `uv python list` on this machine offers `3.15.0rc2`, GIL
  and free-threaded, but no final 3.15 build. CI is therefore the first real run of both, and a red
  there should be read as an untested-interpreter finding, not a flake.
- The optional cross-library dependencies stay capped at `python_version < '3.14'` (numpy, orjson,
  ujson, python-rapidjson, pyjson5), so on 3.15 those suites `importorskip` away - the same as on
  3.14. The new leg is an engine-and-conformance leg, not a cross-library one. The caps were not
  relaxed because no evidence was gathered that those projects ship 3.15 wheels; relaxing markers on
  a guess is how a matrix turns into a resolution failure.
- `push` to `main` still does not re-run the suite. The heavy legs are gated on
  `github.event_name != 'push'` deliberately (measured: a rebase merge lands the identical tree the
  PR verified, so the push run is a second identical ~1h spend), so "full suite on push" stays false
  by design while `main-gate` keeps the branch history current.

The first 3.15 run then taught two more things, both fixed in the same pass.

- The competitor packages the comparison suites use cannot be installed on 3.15 at all: `uv sync`
  died on `ryaml==0.5.1`, which ships no 3.15 wheel, and building its sdist fails in `pyo3-ffi
  v0.26.0` with `the configured Python interpreter version (3.15) is newer than PyO3's maximum
  supported version`. Our own abi3 wheel installed fine - the failure was in the optional comparison
  group, and it killed the leg before our suite could run. `ryaml`, `yaml-edit` and `yaml-rs` are
  therefore capped `python_version < '3.15'`, the treatment `numpy`, `orjson`, `ujson`,
  `python-rapidjson` and `pyjson5` already had for 3.14, so those suites `importorskip` on 3.15
  instead of breaking the install. Relax the caps when upstream ships a pyo3 that accepts 3.15.
- The matrix had `fail-fast` at its default, so that one leg's failure cancelled 22 siblings in
  flight and the run reported nothing about any other interpreter - the opposite of what a per-cell
  matrix is for. It is now `fail-fast: false`, which is also what makes a future 3.15 regression
  read as one red cell among twenty-three green ones rather than as twenty-four unknowns.

### (cj) The job that moves the baseline could not move it (2026-10-12)

The guard that stops `ir_gate.py --update` loosening the gate as a side effect of being run arrived
in #345: a scenario moving up past the tolerance has to be named with `--accept-drift`. The guard is
right, and the channel that produces baselines never learned to honour it -
`.github/workflows/ir-baseline.yml` ran a bare `python3 scripts/ir_gate.py --update`. So the one job
whose purpose is "move every number to the image that enforces them" could not do it at exactly the
moment a change was meant to pay for a move, which is the case that needs the artefact most.

Measured on the first attempt to use it that way: a dispatch at the #360 head (run 38273567698)
installed the pinned rustc, built both harnesses, measured all 21 scenarios three times each and
printed the whole sample table, then exited non-zero on `--update would move 3 scenario(s) past the
0.5% line` and wrote nothing. The refusal was the correct verdict about the numbers and a dead end
for the workflow. What was left would have been transcribing values out of a job log, the failure
mode `test_the_committed_baseline_is_exactly_what_the_job_writes` exists to prevent - "no value in
this file is transcribed".

Fixed in the channel rather than in the guard: the job now takes an optional `accept_drift` input,
space- or newline-separated scenario names forwarded one `--accept-drift` flag at a time, and the
step prints which half it ran, so a guarded refresh and an opened one are distinguishable in the log
without reading the diff. An empty input keeps the bare `--update`, so refusing stays the default,
and a name matching nothing the guard reported is simply unused - a typo reddens the job instead of
loosening it.

Three tests in `tests/test_ir_baseline_workflow.py`, in the source-reading style the neighbouring
wiring tests use because no test can afford a valgrind run, and each proven by mutation: replacing
`--accept-drift` with an inert flag reddens `test_the_input_is_actually_passed_to_the_guard` alone,
hard-coding `to_json_medium` into the loop reddens `test_an_empty_input_keeps_the_refusal` alone,
and restoring the file gives 19 passed. No baseline value and no tolerance moved in this change.

What it unblocked has since happened. The ruling on #360's measured +1.30% / +5.11% / +4.98% against
`main` was taken, the refresh ran on the enforcing image as run 38326122129, and the artefact it
produced is committed on that PR - so the numbers a change has to pay for were claimed by that
change, in its own diff, which is the ordering this file keeps asking for. **The first real use
found a defect in the channel.** The refresh produced the artefact, which was the point, and it also
logged `accepting drift for 6 scenario(s): to_json_medium to_jsonc_medium to_json5_medium` - three
names, six, because the count came from `args`, which holds a flag *and* its value per name. That
line is exactly what a reviewer reads to decide whether an artefact was opened on purpose, so a
doubled number in it is not cosmetic. The step counts names now, and
`test_the_log_counts_the_names_not_the_flags` pins both the message and the condition it branches
on: pointing either back at `args` reddens that test alone, and restoring the file gives 20 passed -
19 before this test existed, which is the same count the paragraph above reports for its own moment.

Measured while writing the fix rather than assumed: `read -ra names` over an empty here-string under
`set -euo pipefail` yields an empty array and status 0, so the default input reaches the refusal
branch with or without the `|| true` standing guard. Both halves were run - empty, and three names -
before either claim was written down.

### (ck) A ledger whose sections are not in the order its citations assume (2026-10-13)

`### (xy)` sections grow by insertion at whatever anchor a branch used, and two of them ended up
behind letters that came later: `(bh)` after `(bi)`, `(ch)` after `(cj)`. Found by reading the
heading sequence of the file rather than by following a broken link, and that is the reason nothing
was red: a citation like `(details: quality-ledger (ch))` still resolves, so the only cost is that
the file no longer matches its own numbering, and a reader scanning for the newest entries meets
them out of order.

Moved as whole blocks - 98 and 46 lines - with the multiset of the file's lines asserted unchanged
before and after. A reorder that silently drops or duplicates a paragraph is the failure mode this
kind of edit is prone to, and the check costs one line.

Two tests now pin it, in `tests/test_ledger_section_order.py`: the lettered headings run strictly
increasing, and no letter repeats. Proven by mutation, and the second mutation said something the
plan had not predicted - renaming the last section to a letter already used reddens *both* tests,
because a repeated letter is also a step backwards. The duplicate rule stays separate because it is
the one whose message names the cause. Restoring the file gives 2 passed.

Gaps are deliberately not reported, for two different reasons that are easy to conflate. `bs`, `bu`
and `cd` are letters a branch claimed and then closed by another route. `ak` through `an` are not
missing: they are in this file as bold lead-ins (`**(ak) The last registered hole closed…**`) under
the older family headings, not as `### (ak)`. Measured before this very section was appended, the
file carried 55 lettered headings, and a repo-wide scan for cited letters found 26 two-letter tokens
with no such heading - a mixture of these bold-form sections and ordinary prose collisions like "the
ledger (is" or "(of …)". A citation-resolving gate therefore has to know both section forms and must
not match a parenthesised pair in passing prose; that is a separate change, and naming the
requirement here is the point rather than shipping a check that reddens on English.

### (cl) The CI matrix was a cross product, and two gates were hidden inside it as string comparisons (2026-10-13)

Two questions arrived together: whether the 8 x 3 interpreter matrix is worth what it costs, and
whether the cost can be moved without losing coverage. Both were answered by measuring the existing
matrix rather than by reasoning about it.

**Cost.** Over six recent CI runs and 144 matrix legs, a leg's median wall time is 138 s on ubuntu,
186 s on macOS and 251 s on Windows, of which the suite itself is 116 / 141 / 183 - so 76% of the
spend is the same ~2 500 assertions re-executed per interpreter, and the same cell costs 1.8x on
Windows what it costs on ubuntu for identical content. The full grid is 78.2 minutes of leg time per
pull request, and the pull requests arrive at 70 per 30 days (~16 a week), which is ~21 hours a week
of runner time on this one job.

**Yield.** Over the last 30 pull-request runs, 642 legs executed. Not one failure was confined to a
single interpreter version across OSes, and not one was confined to a single OS across versions; the
only red groups were whole-row cancellations, which is the behaviour #363 already removed by setting
`fail-fast: false`. That is a short window and mostly documentation work in it, so it does not prove
the axis is worthless - what it locates is where the axis pays: at the boundaries. #298 was merged
over an unconsulted `test (windows-latest, 3.8)`, and the 3.15 legs earned their existence in their
first run by finding that three comparison packages refuse that interpreter.

**The defect the shape was hiding, and the reason to restructure rather than shrink.** Two gates are
selected by cells:

```yaml
${{ matrix.os == 'ubuntu-latest' && matrix.python-version == '3.12' && '--cov-fail-under=78 ...' }}
if: matrix.os == 'ubuntu-latest' && matrix.python-version == '3.14'
```

Delete or move either cell and both conditions go false. The suite still passes, the run is green,
and the coverage threshold is never evaluated. That is not a hypothesis about a possible future
edit: it is the same class the fan-in job exists for (#298, #299 - a check that never reports is not
a gate), parked in the matrix where nobody was looking. So the grid is now *planned* by
`scripts/ci_matrix_plan.py`, which reads the interpreter list out of `pyproject.toml`'s classifiers,
and the gates are *roles* carried by legs. `--check` runs in the planner job, before any leg is paid
for, and refuses a plan without exactly one coverage leg and exactly one compliance leg.

**What the pull-request tier runs now: 14 suite legs and 10 smoke legs, covering the same 24
cells.** Every declared version runs the suite on ubuntu; the floor (3.8), the top (3.15) and the
coverage carrier (3.12) run it on all three OSes. The remaining cells are not dropped -
`wheel-smoke` installs the abi3 wheel that platform built and round-trips one document through YAML,
JSON, JSONC, JSON5 and TOML, which is the claim `docs/en/installation.md` actually makes for those
cells ("a single wheel from 3.8 to 3.15 on three platforms") and costs a setup, 22 s to 68 s, rather
than a suite. The full grid runs with the suite every Sunday at 02:30 UTC, which is what makes the
demotion reviewable rather than a one-way ratchet: chosen over a nightly full grid because a nightly
would spend ~9 h a week on the cross product and cost back most of what the slim tier saves.

Measured result: 78.2 min -> ~49 min of leg time per pull request (-37%), 24 -> 14 full-suite legs
(-42%), no cell of the cross product left unexercised.

**Proven to be able to fail.** A validator that only ever sees the shape it was written against is
the same defect as the condition that cannot fire, so `check()` takes a plan as an argument and four
tests hand it doctored ones: strip the coverage role and it names the role; delete `ubuntu/3.9`'s
only leg and it names the version; delete `windows/3.8` and it names the boundary cell; put one cell
in both tiers and it names the overlap. A fifth test adds a `3.99` classifier to a copy of
`pyproject.toml` and requires the plan to grow a leg for it, because the derivation from published
metadata is the load-bearing part and a hard-coded list would pass every other test here.

**One measured limit of the new tier, found by writing it.** `scripts/wheel_smoke.py` first demanded
that a parsed document's `to_yaml()` return the input byte for byte, and on the first run it was
wrong: a standalone `# note` on its own line at the end of a short document comes back attached to
the previous member (`b: [1, 2]  # note`). Nothing is lost and the second pass is stable, so the
assertion was rewritten to hold survival plus the fixed point, which is the property a per-cell
wheel check should own, and the re-attaching behaviour is left to the round-trip tier that
characterises it deliberately rather than being reported as a break by a smoke test.

**Three stale sentences the change would otherwise have left standing**, corrected in the same
commit rather than left as prose that no longer matches the machine: `check_matrix_verdict.py` and
`test_matrix_verdict_gate.py` both quoted a leg count ("twenty-one"), and `test_docs_config_gate.py`
claimed "the CI matrix covers 3.11 through 3.14 on three OSes" - untrue twice over, since 3.15 is
declared and the mid-range versions no longer run on three OSes.

**What is not covered, named because the summary above reads as if it is.** A bug that needs a
non-boundary interpreter *and* a specific OS at the same time is now caught on the schedule rather
than on the pull request that introduced it; that is the price of the -37%, and the weekly grid is
the guarantee. Separately measured and deliberately untouched here, so it is not mistaken for fixed:
`publish.yml` smoke-tests the built wheels on 3.8 (linux, macOS) and 3.13 (windows), so the release
path never installs the artefact on the newest declared version while `installation.md` promises 3.8
through 3.15. It is not an engine ruling, so it does not belong in `ROADMAP.md`'s open table; it is
recorded here, and whoever touches the release path next should make the publish smoke run over the
same version list the planner emits.

Verified: 2533 Python tests pass and 10 skip (the count after the fixer guard in the next entry), 87
across the planner, fan-in, corpus-gate and script-import modules, `prek run --all-files` exit 0
including `check yaml`, and `ci_matrix_plan.py --check` reports the plan recorded above. (cn) raises
those two counts to 2673 and 208 by adding the floor scan; the numbers above are what this entry
measured, kept rather than overwritten so each entry reports its own run.

### (cm) The width fixer rewrote a Python file when it was pointed at one (2026-10-13)

`scripts/check_doc_wrapping.py --fix <path>` rewrites whatever path it is handed. Invoked on
`scripts/ci_matrix_plan.py` - by the author of this change, while writing it - it treated the module
as Markdown prose and re-flowed it: the docstring, and then the statements under it, joining `def`,
`if` and `raise` onto shared lines until the file no longer parsed. The damage was total enough that
the script was rewritten from scratch rather than patched, which is also why nothing was lost: the
file was uncommitted work in progress.

The hook cannot do this. `prek.toml` gives `doc-line-width` a `files:` pattern, so the script never
sees a path outside the governed pages - which is exactly the shape of the bug: the safety lived in
the caller, and the command line had none. A tool that repairs the repository's prose should refuse
to touch anything that is not prose, so the governed set moved into the script as `GOVERNED`, `main`
refuses any other path with exit 2 before it writes a byte, and the refusal names every offending
argument rather than the first.

Four tests in `tests/test_doc_wrapping_gate.py`, one of them the control this file keeps insisting
on: `test_the_guard_is_what_protects_it` patches `is_governed` to always accept and requires the
fixer to *actually* rewrite the same file. Without it, the passing test would only show that this
particular input survived, which is not the same claim as the guard working. The fourth test
compares `GOVERNED` against the hook's own `files:` regex over both the example list and every page
currently in `docs/dev/`, because two lists describing one boundary are two chances to drift.

Two traps found while writing them, both recorded because each produces a confidently wrong test:
`prek.toml` puts hook entries in inline tables that span lines, which the TOML spec forbids, so
`tomllib.loads` fails on it and the file has to be read the way `scripts/quality_matrix.py` already
reads it; and `Path.relative_to` yields `docs\dev\boundaries.md` on Windows while `prek` and `git`
hand over `docs/dev/…`, so the comparison has to normalise one side or it reports a path separator
as a disagreement about scope.

Verified: 2533 Python tests pass, 10 skip; the guard rejects `scripts/*.py`, `README.md`,
`AGENTS.md`, `QUALITY_MATRIX.md`, `prek.toml` and `.github/workflows/ci.yml` while accepting all
nine governed pages; `prek run --all-files` green, which is the hook exercising the same script
through its filtered path list. Three of this entry's own tests then died on the 3.8 and 3.9 legs
for want of a 3.10 keyword - (cn) - and the suite count there is 2673.

### (cn) An API younger than the floor raises at call time, where no import check can see it (2026-10-13)

Four legs of the first planned matrix went red: Python 3.8 on all three operating systems, and 3.9
on ubuntu only. The shape was the diagnosis before the log was - those are exactly the legs the new
plan still gives a full suite, and 3.9 keeps one only on the cheap OS.
`tests/test_doc_wrapping_gate.py` raised
`TypeError: write_text() got an unexpected keyword argument 'newline'` while building its own
fixture, three tests deep in (cm), before reaching any behaviour under test.

`Path.write_text` gained the `newline` keyword in 3.10. Nothing about the module could reveal that:
it imports on 3.8, its annotations are lazy, and the offending call sits in a function body.
`tests/test_scripts_import_on_supported_python.py` had already named this as its remaining blind
spot in its own docstring, and the named gap cost a full matrix round trip (about 75 minutes of wall
time per pull request) to find what a sub-second static scan finds.

Seven sites in the tree, not three. The other four were latent: `scripts/check_doc_wrapping.py` (the
`--fix` write path, reached by (cm)'s control test), `scripts/check_stub_drift.py` (`--fix`),
`scripts/check_stub_types.py` (the mypy overlay) and `tests/test_stub_types_gate.py` (a fixture read
from a pinned ref). Each had stayed green purely because nothing executed that path on a floor
interpreter - the two stub scripts run inside a modern Windows job, and the gate test skips on 3.8
where a shallow clone carries no known-bad tag. All seven now write through
`with path.open("w", encoding="utf-8", newline="\n")`, which has accepted the keyword since 3.5 and
produces byte-identical output, so the repair removes no intent.

The prevention is a static catalogue, `TOO_NEW_FOR_THE_FLOOR`, scanned over `scripts/`, `tests/` and
the shipped `python/` package - the last included because a 3.10-only keyword in code users run is a
crash, not a red test. `_code_only` strips docstrings, then string literals, then comments, in that
order: naming an API must not read as calling it, and stripping comments first would let a `#`
inside a string delete the code that follows it. The scan is pinned by a fixture holding one
reachable shape per entry, compared for *set equality*, so an entry whose pattern never matches is
reported the day it is added; `FLOOR` is compared against `requires-python` rather than trusted.

Two entries were rejected as written, both by measurement rather than argument. `\.walk\(\)` came
back with seven findings, every one of them this project's own `walk()` verb on a document node;
`@override\b` caught the `typing_extensions.override` shim the repository standardises on, which is
legal on 3.8 and is the reason that decorator is written the way it is. The surviving entry names
the import that actually breaks (`from typing import ... override`) instead of the identifier. The
lesson is on the list itself: a pattern that collides with the code it polices is worse than no
pattern, because the only way to make it pass is an exemption, and exemptions outlive the APIs they
were written for.

Verified on the interpreters rather than by reading version tables: real CPython 3.8.20 and 3.9.25,
`uv run --python 3.8 --with pytest --with pyyaml --no-project -- python -m pytest
tests/test_doc_wrapping_gate.py tests/test_ci_matrix_plan.py --noconftest` (the wheel is what
`--no-project` cannot provide, so the three tests that import
`pyrs_yaml` are excluded by hand). 235 of 238 on 3.8, 218 of 218 on 3.9 with those three deselected;
the previously red three tests pass on both. The scan costs 0.32 s over 182 items on every leg.

### (co) A write-side oracle whose first control passed, and what that said about the path (2026-10-13)

`tests/test_json_test_suite.py` has always been about the readers: which of the 283 determinate
JSONTestSuite cases this engine accepts, and with what error. The writers of the same four dialects
were checked against this repository's own parser and nothing else - while the measurement that
found (cf)'s wide-integer defect had been a cross-library re-read, run once and never pinned. Two
gates close that: every valid document (95) through `to_json` and `to_jsonc` re-read by stdlib
`json` (190 re-reads), and `to_json5` re-read by `pyjson5`.

Type-strictness is the load-bearing part, and it was chosen against Python's own equality:
`[1] == [1.0]` and `0 == False` both return `True`, so a comparison that reads values and stops
there accepts the substitution #358 fixed. Measured by control rather than by argument - an integer
emitted as `{i}.0` reddens 38 of 190 re-reads and 4 of 95 JSON5 re-reads, each naming the input, the
output and the emitted text.

The control passed on its first attempt, and that was the more useful result. `write_plain` returns
a number whose text already spells JSON (`is_json_number`) verbatim, before any schema resolution,
so a corpus that arrives through `from_json` never reaches the normalising arm the substitution sits
in; making the mutation observable required switching that fidelity pass off as well. Two readings
were available and the wrong one is tempting: the gate is not blind to type substitution, it is
blind to *which documents reach* the arm, which is a property of the fixture. The YAML spellings the
arm exists for (`0x1F` as `31`, `!!str 7` as `"7"`) are pinned on the Rust side by
`exotic_scalars_quote_or_normalize` in `crates/pyrs-yaml-core/src/integration/json_family.rs`, and
the docstring now says so instead of leaving the next reader to repeat the experiment.

The second control found an overlap inside the gate. Reversing the writer's pair loop reddened six
documents - reported by the *value* comparison, because `_same_value` walks object keys with
`list(a) == list(b)` and already pins order, while the pair-list check sits behind an `elif` and
never fired. The first draft of the docstring had credited that second comparison with guarding
order, which is exactly what the control disproved. What it owns is the case dict equality cannot
see, and the third control went looking for it: emitting every member twice stays valid JSON, and
the repeat collapses, so both sides are equal as dicts - 18 of 190 re-reads reddened and the value
comparison never mentioned any of them. The exemption for repeated input names is untouched -
`y_object_duplicated_key.json` and `y_object_duplicated_key_and_value.json`, pinned by assertion -
and the two arms now report value and structure separately, each with a demonstrated way to fail.

All three mutations were reverted, and the clean rebuild was confirmed by probing rather than by
trusting the build's exit code: `0x1F` -> `31`, `!!str 7` -> `"7"`, and `{"a": 1, "b": 2}` back in
order. Twice during the work a reading raced its own build, and a green gate was really a stale
`.pyd`, so the mtime of `python/pyrs_yaml/pyrs_yaml.pyd` is now checked before any control's verdict
is believed.

The same discipline caught a third thing, which was this change's own mistake rather than the
repository's. A throwaway script that rewrote the five changelog entries searched backwards for the
line starting the entry and took the *first* such line, and because the test it applied was
"does this bullet's slice down to the citation contain the citation", every bullet in the section
passed it: the script replaced 248 lines of the root changelog and 688 of the English mirror with
one entry. `git diff --stat` was what showed it, and `git checkout` repaired it, because those files
held no other uncommitted work. Inserting by section boundary (the `Changed` header, asserted) is
what the rewrite did afterwards, and the parity guard then had to be told the count had grown by one
Added entry in all five pages rather than assume it. An ad-hoc script that edits committed prose
needs the same anchor discipline as a gate: search for a boundary, not for a membership.

Verified: the file's 7 tests pass, the suite is 2713 pass / 10 skip, and `prek run --all-files`
exits
0. No Rust source changed - `git diff --stat` ends at the test file, the five changelog pages and
this section.

### (cp) A spelling the hub could not hold is now a type it can state (2026-10-13)

`(bu)` shipped steps 1-2 of #340 and left step 3 on purpose, because the two halves are ordered: the
TOML spoke could not keep `0b1010` as written until a standard tag decided the type in every format.
Measured on `main` before this change, `from_toml("b = 0b1010")` returned `b: 10` and `to_toml` of
that `b = 10`, while `0x1f` and `0o17` kept their spelling. The canonicalisation was not a defect
but design rule D1 ("only spellings YAML Core also accepts get preserved") doing its job — Core has
no `0b` lexeme, so an untagged plain `0b1010` re-reads as a string, and losing the text was the half
of the trade that kept the value.

The projection now asks the question every other consumer asks. `toml_value_to_node` resolves the
preserved spelling through `Schema::Core` and, when Core does not read it back as an integer, tags
the node `!!int`. The rejected alternative was `src.starts_with("0b")` — a second copy of a grammar,
which is precisely the shape #340 was filed against, and one that would have gone quietly wrong the
next time Core gained or lost a lexeme. `from_toml("b = 0b1010")` is now the hub scalar
`!!int 0b1010`: the loader reads `10`, `to_toml` writes `0b1010`, `to_json`/`to_jsonc`/`to_json5`
project `10`, and the TOML text is a fixed point after one round rather than after two.

Two things stay unfaithful, and neither is a spelling the writer could keep. `0b1101_0110` loses its
separators: TOML's own literal grammar rejects `_`, so `is_toml_int_literal` would refuse the text
and canonicalise it anyway — the parser strips them rather than emitting a tag that cannot pay. A
hand-built *untagged* `0b1010` still re-reads as a string, which is the point: the tag is what
carries the type, and this change moved no boundary of what Core resolves on its own. The undecided
`!!binary` mapping (#340's remaining ruling) is untouched.

The cost is not measurable on the counted scenarios, and the reason is stated rather than assumed:
`MEDIUM_TOML`, the fixture behind `from_toml_medium` and `to_toml_medium`, holds no radix-prefixed
integer (`sed -n '149,167p' crates/pyrs-yaml-core/src/bench_inputs.rs | grep -c '0x\|0o\|0b'` reads
`0`), so the new branch is guarded by `Some(src)` and never fires. `scripts/ir_gate.py`
could not confirm it on this box: the WSL build stops at the linking step of `pyrs-yaml` with
`rust-lld: error: unable to find library -lpython3.12` (no `python3-dev` there), which is an
environment limit, not a defect in this change. The enforcing image runs the gate.

Verified: `cargo nextest run --all` 573 tests, including the inverted pair `v = 0b1101_0110` →
`v = 0b11010110` in `preserves_radix_integer_source_spelling`, the new
`binary_spelling_reaches_the_hub_under_an_int_tag`, which pins tagged-binary/untagged-hex side by
side so the fence cannot be applied twice or forgotten, and
`the_fence_reaches_values_inside_a_container` for array and inline-table members.
`tests/test_toml.py::TestBinaryIntegerFidelity` (6 cases) asserts hub text byte-for-byte, the
one-round fixed point, and that the five formats agree on the *type* of the same node.
`cargo clippy --all --all-targets -- -D warnings` and `cargo fmt --all` clean, the suite is
2719 pass / 10 skip, and `prek run --all-files` exits 0 — including the mirror gate, which reads all
five changelogs at once.

That the tests carry the change rather than restate it was checked by mutation: with the tag
condition short-circuited (`if false && !matches!(…)`), two tests redden and name the cause —
`binary_spelling_reaches_the_hub_under_an_int_tag` reads `("0b1010", None)` against
`Some(("!!", "int"))`, and `preserves_radix_integer_source_spelling` emits `v = "0b101"` because the
untagged text resolves to a string. The seed-corpus fixed-point test stays green under the same
mutation, which is the expected split: it pins stability, not fidelity. The mutation was reverted
before this entry was committed.

### (cq) A digest that read what equality does not, and what fixing it priced (2026-10-13)

The attribution `docs/dev/perf.md` now carries named `Hasher::write` as the largest single self cost
of every reader. Going into `CustomNode`'s hash to see what could be removed found a defect first.
`PartialEq` compares a scalar's value, style, chomping and the three identity fields, and the
explicit block indentation indicator is not among them - but `Hash` wrote `block_indent` as well. So
`a == b` could hold while `hash(a) != hash(b)`, which is the one rule a `Hash` implementation may
not break: `IndexMap` routes by digest and calls `eq` only afterwards, so the second node is a key
that cannot be found. The crate's own documentation for `hash_custom_node_identity` already states
the rule ("hash exactly the metadata fields `CustomNode`'s structural equality compares"); it was
the scalar arm that did not follow it.

Written as a test before the fix, and it reddened on `main` with `108230566608641` against
`144115610851506687`. `equal_nodes_hash_alike_when_only_the_block_indent_differs` now sits beside
`plain_and_noted`, which guards the same contract for the `decor` slot, so the rule is pinned per
field instead of asserted in a comment.

The second half is what the profiler asked for: an undecorated node writes nothing at all for
metadata, instead of three `Option` discriminants. Equal-and-clean nodes still agree (both write
nothing), and a node that carries metadata still disagrees with one that does not, which is what
equality says. The `Hasher::write` self cost of `from_json_medium` fell from 20,954,000 to 9,954,000
instructions.

Measured before and after on the same host, same build flags, same harness, in one session:
`from_json_medium` 153,972,194 → 141,960,063 (-7.80%), `from_json5_medium` 155,787,106 → 143,778,309
(-7.71%), `from_toml_medium` 212,135,514 → 200,130,125 (-5.66%), `parse_medium` 355,652,599 →
343,644,911 (-3.38%). Key comparison did not grow - `__memcmp_avx2_movbe` 10,720,143 → 10,720,033 -
which is the evidence that the thinner digest did not buy its saving with probe quality; a hash that
turned lookups into scans would have surfaced in that column first.

Stated as what it is not: the enforcing image's `scripts/ir_gate.py` remains the arbiter and
`.ci/ir-baseline.json` was not touched here. These scenarios are expected to move *down*, which the
gate accepts without ceremony, so the change ships no re-baseline; the local numbers are a
direction, not a verdict - and the verdict arrived the same day from the image that owns it. The
`Instruction-count baseline` job on PR #372 reports against the committed baseline:
`from_json_medium` 154,041,906 → 141,699,963 (-8.01%), `from_json5_medium` 156,883,384 → 144,540,451
(-7.87%), `from_toml_medium` 218,560,766 → 206,244,157 (-5.64%), `parse_medium` 355,874,562 →
343,536,527 (-3.47%), then `OK: every scenario within tolerance`. The local and enforcing deltas
agree within 0.21 percentage points on all four scenarios, which is the first time this ledger could
check its own wall-free A/B against the channel that adjudicates it. The route measured is also the
`std` one, where `NodeMap` is `IndexMap<K, V, RandomState>` (`ast.rs:22`): the `no_std` side already
keys the same map on a fixed-seed FxHash (`ast.rs:29`), so this is not a cost embedded builds were
paying and are now rescued from - they never paid SipHash here.

Verified: `cargo nextest run --all` 574 tests including the new invariant test, `cargo test --all
--no-default-features` clean, `cargo clippy --all --all-targets -- -D warnings`, `prek run
--all-files`, `uv run pytest tests/` 2719 pass / 10 skip, five changelog mirrors at [25, 11, 38, 5].

### (cr) Three passes to decide one quote, and the control that made the two deltas count (2026-10-13)

`(cq)` took the digest off the top of the readers' cost table; the next entry down was the same kind
of work in the bridge's quoting guard. Every format asks `needs_quotes` whether a text would be
misread as a plain scalar, and the function read the value once as characters, then searched it
again for NBSP and a third time for BOM. Neither of those two can occur in ASCII text - U+00A0 and
U+FEFF are non-ASCII by definition, `is_control` over ASCII is exactly
`byte < 0x20 || byte == 0x7F`, and `is_yaml_noncharacter` needs a code point ending in
`FFFE`/`FFFF`, which seven bits cannot reach. So one byte pass now decides that case and only text
carrying a byte above `0x7F` still goes to the character rule, unchanged. UTF-8 makes the split
safe: no continuation or lead byte is ever below `0x20`, so a control byte cannot hide inside a
multi-byte sequence.

The control is exhaustive rather than sampled:
`needs_quotes_agrees_with_the_char_rule_for_every_ code_point` builds `q<c>q` for all 1,114,112 code
points a `char` can hold and compares both paths to the character rule itself. Mutating the byte
rule by one (`byte < 0x21`, which reads a space as a control) reddens it with
`first: Some((32, true, false))` - the test convicts the specific wrong answer, which is what makes
it a check on the fast path rather than a restatement of it.

What made the measurement readable was an accident that turned into the method: the two scenarios
this change cannot enter - `from_toml_medium` and `parse_medium`, which never call the quoting
guard - repeated across the runs at -0.0009% and +0.0002%, while the two it does enter moved -6.89%
(`from_json_medium`) and -1.87% (`from_json5_medium`). A same-host pair of untouched scenarios
agreeing to three decimal places is the calibration that says the other four numbers came from the
code and not from the machine; the ledger's earlier rule that cross-session readings are not
comparable stands, but this is how to check whether they are when they claim to be.

Measured on `callgrind` over the gate's own engine harness (`--features ir-gate`,
`CARGO_PROFILE_RELEASE_STRIP=false`), because the committed baseline's own image is the arbiter and
this box cannot link the binding harness. The numbers above are therefore the direction; the
enforcing image's `Instruction-count baseline` job reports the verdict against the committed
baseline, which since `(cq)` also carries that change's -8.01% on `from_json_medium`.

The enforcing image then priced what the same-host pair had called a direction. On PR #373,
`Instruction-count baseline` read `from_json_medium` 154,041,906 → 120,758,043 (-21.61% against the
committed baseline) and `from_json5_medium` -16.72%, while the two scenarios neither change enters
held at the values #372 had moved them to - `from_toml_medium` -5.64%, `parse_medium` -3.47%. Both
savings share that first column: #372 alone read -8.01%, so the quoting pass took the JSON read a
further ~-14%, twice what the local A/B estimated. The local numbers stay because the untouched
scenarios are what made them readable at all; the magnitude did not travel between the channels,
which is the reason the gate owns the verdict and not the laptop.

Verified: `cargo nextest run --all` 575 tests (the oracle included), `cargo clippy --all
--all-targets -- -D warnings` and `cargo fmt --all` clean, `prek run --all-files`,
`uv run pytest tests/` 2719 pass / 10 skip, five changelog mirrors at [25, 11, 38, 5].

### (cs) A harness that panicked on the behavior its own test was checking (2026-10-13)

`msrv-check` reddened on a branch whose diff could not have touched it
(`write stdin: Os { code: 32, kind: BrokenPipe }` at `cli.rs:35`), while the same test passed on
every other leg and on the machine of the author. The temptation is to call that a flake and press
rerun; the log said otherwise. `run_with_stdin` wrote the child's stdin and panicked on any error,
but the test that failed - `to_json_dialect_flags_conflict` - asserts that the CLI *refuses* two
mutually exclusive flags, and a refusal exits before anything reads the pipe. So the harness
panicked on exactly the behavior the test exists to pin, and whether it did depended on which side
won the race: Linux runners lose, a warm local run wins.

The fix is in the harness, not in the assertion: take the stdin handle, tolerate the failed write,
close it before waiting. Closing it is not incidental — a child that *does* read needs end-of-file,
which is what the old code left open until `wait_with_output`. And nothing is being swept up: the
exit code and stderr assertions still decide pass or fail, so a CLI that genuinely misbehaves still
reddens the test.

Made deterministic rather than left to luck:
`a_refusal_closes_stdin_before_the_harness_finishes_writing` feeds 240 KB - more than a pipe
buffer - to the same refusal, so the write cannot slip through unread. Against the old harness it
fails every run, locally with `Os { code: 109, kind: BrokenPipe }` (Windows' name for the same
condition the runner reported as 32); with the fix, all 48 CLI tests pass, and 12 consecutive runs
of the whole file stayed green.

Verified: `cargo test -p pyrs-yaml-cli --test cli` 48 passed (was 47), the same command against the
pre-fix harness FAILED with the EPIPE panic, `cargo clippy --all --all-targets -- -D warnings`,
`cargo fmt --all`, `prek run --all-files`, five changelog mirrors. No engine code changed - the
`-0.0009%`-style instruction totals are untouched by a test-harness edit, so no gate re-baseline was
asked for.

### (ct) Two bytes answer what a two-ended walk used to ask (2026-10-13)

The profile that priced this change - `callgrind` over the gate's own harness, the run recorded in
`docs/dev/perf.md` - found a cost the earlier table had not named:
`<str>::trim_matches::<is_yaml_blank>` at 1.64% of `from_json_medium`: 1,976,000 counted
instructions over 52,000 calls, 38 per call. The cost belongs to the resolver, not to the reader:
every scalar the JSON, TOML and YAML paths hand to `resolve_core_type` ("is this a number or a
word?") first pays to strip edge separation, and the keys that ask are ordinary words like `name`,
which have nothing to strip. `resolve_yaml11_type` trimmed, then delegated to the core chain, which
trimmed the same text again.

The rule that settles the question in two reads: every character `is_yaml_blank` accepts -
`SP TAB CR LF NUL` - is ASCII, and UTF-8 never places an ASCII byte inside a multi-byte sequence. So
a value begins or ends blank exactly when its first or last *byte* is one of those five. This is a
guard, not a replacement: whenever either end really is blank, the original
`trim_matches(is_yaml_blank)` still runs, so what gets removed stays defined in exactly one place.
The byte table is const-evaluated by *calling* that predicate over the 128 ASCII positions - which
is why `is_yaml_blank` became a `const fn` - because `(bi)` in this file is the record of what
happens when a rule is written twice and encoded differently.

Measured as one pair of runs of the same engine harness binary (`callgrind`, whole-process `I refs`,
2,000 iterations, WSL build): `from_json_medium` 120,167,551 to 119,281,047 (-0.74%),
`from_toml_medium` 200,129,565 to 198,295,602 (-0.92%), `to_json_medium` 22,244,247 to 21,870,305
(-1.68%), `serialize_medium` 47,556,611 to 47,386,528 (-0.36%), with `parse_medium` repeating at
+0.0014% as the control the pair needs - the two pre-change `from_json_medium` runs differed by
0.005%, so everything above that line is the change and not the machine. The write scenarios moved
because the writers resolve plain scalars through the same crate: the shared schema layer doing its
job, and the reason a reader-side cut has to be checked on both sides of the round trip.

The mechanism, not only the total: after the change the profile carries no `trim_matches` entry and
`trim_yaml_blank` costs 1,092,000, so 884,000 of the 888,571 instructions the scenario lost belong
to the function that was removed. One refinement was tried and refused on the same kind of
evidence - `#[inline]` on the guard measured 119,277,876 against 119,281,047, -0.003%, inside the
same-binary spread - so the attribute was dropped instead of kept as a preference dressed as a
measurement.

Three tests hold the correctness half, each checked against a mutant that has to redden it:
`the_blank_table_covers_exactly_the_predicate` walks all 256 byte values against the predicate, so
the table cannot be edited into disagreement with it (`byte <= 0x80` is caught there by indexing
past the 128 ASCII positions);
`trim_yaml_blank_agrees_with_str_trim_for_every_code_point_at_either_end` places every code point at
both ends of `...42` / `42...` and compares against `trim_matches` - reading the wrong end byte dies
there with `first: Some((0, "42\0"))`, which is the shape that would have shipped silently
otherwise; `trim_yaml_blank_removes_only_yaml_separation` pins the prose the resolvers promise,
including NUL, `"  a  b  "`, NBSP and NEL at either end, and an astral tail. The sweep uses stack
buffers rather than `format!` twice over 1.1M points, so it exercises the rule instead of the
allocator.

Two workflow facts, both learned the slow way:

- The edits for this change landed in the working copy of a commit that had already been pushed as
  PR #376's head, so the bookmark turned `ci-ir-rebaseline*` and `jj git push --dry-run` reported
  `bookmark: move sideways from de3b303b to 1287778a` - a rewrite of a branch under review for
  content that belonged to a different change. The repair is mechanical: copy the two touched files
  aside, `jj restore -- <path>`, `jj new`, copy them back, `jj rebase -d main@origin`, then
  *prove* the pushed commit is intact with `jj diff --from de3b303b75a4 --to 1287778ac169 --stat`
  reading `0 files changed`. The habit that avoids it is ordering: `jj new` before starting the next
  change, not after the previous one has been pushed.
- A mirrored changelog bullet does not belong to the section whose heading you found first.
  `docs/zh/changelog.md` keeps its historical entries under English `#### Performance` headings, so
  inserting at the first one wrote a new `[Unreleased]` entry into the v0.13.0 history - invisible
  to an eye reading the diff, and not invisible to `scripts/check_changelog_mirrors.py`, which
  counts entry bullets per section and reported `[25, 11, 39, 6]` against the canonical
  `[25, 11, 39, 7]`. The `[Unreleased]` headings are localized: 性能, パフォーマンス, 성능.

Verified: `cargo nextest run --all` 579 tests (the three above included),
`cargo test --all --no-default-features` clean, `cargo clippy --all --all-targets -- -D warnings`,
`cargo fmt --all`, `uv run pytest tests/` 2719 pass / 10 skip against a rebuilt extension,
`prek run --all-files`, five changelog mirrors at [25, 11, 39, 8]. The `no_std` gate's own command -
the thumbv7em build of the four foundation crates `ci.yml` runs - finishes green with the const
table in it. The gate then owned the verdict and agreed with the local pair, on the enforcing image:
`from_json_medium` -0.72%, `from_toml_medium` -0.89%, `from_jsonc_medium` -0.66%,
`from_json5_medium` -0.47%, `to_json_medium` -1.73%, `to_jsonc_medium` -1.59%,
`to_json5_medium` -1.56%, `to_toml_medium` -0.95%, `serialize_*` -0.21…-0.51%,
`to_python_*` -0.04…-0.36%, and all four `parse_*` scenarios at +0.00% - twenty-one scenarios
measured, and not one of them moved up.

### (cu) A crate boundary drawn from a fact-checked design note (2026-10-13)

The architecture pillar names two core abstractions to split out: the AST, which became
`pyrs-ast`, and `事件流` - the event stream. The second one was written as a goal, not as a measured
design, so before touching the layout the claim was checked against the tree, and the check changed
it: there is one event vocabulary (`crates/pyrs-yaml-core/src/parser/stream.rs`), one producer
(granit's `Event`), one consumer (the PyO3 stream API), and of its eleven variants `anchor`, `tag`
and `Alias` are YAML's grammar while `style` is *shared* - the AST already carries
`ScalarStyle::SingleQuoted` for JSON5. The type also has two lifetimes today: `StreamReceiver`
collects everything into a `Vec`, while the binding pulls one `(StreamEvent, Span)` at a time from a
live parser. "Split the event stream into a crate" is therefore not a refactor of shared code; the
shared code does not exist yet.

That finding was written up as an option list with the cost of each, and the maintainer ruled option
B (neutral core plus per-format payload, proven by JSON and TOML producers and a replay property
test). What shipped here is the first step of B and deliberately *not* the neutral shape: the type
moved verbatim into `pyrs-event` (`#![no_std]` + `alloc`, depending only on `pyrs-ast`), the engine
re-exports both names, and no call site changed. The crate-level documentation says in as many words
that the vocabulary is still YAML's and which fields are not a claim about JSON, so a reader cannot
mistake step 1 for the finished abstraction. An abstraction with one implementation is a guess about
the second; the guess is deferred to the step where the second producer forces an answer.

Two workflow facts, both from being wrong in place:

- A design document is only reviewable where a reviewer can open it. `docs/superpowers/` is excluded
  by `.git/info/exclude` (11th line: `/docs/superpowers/`) and `git ls-files docs/superpowers`
  returns nothing, so a document written there - as four earlier specs are - is a local artifact: a
  `cargo` crate comment citing it would point at a file that does not exist in the repository. The
  design note moved to `docs/dev/event-stream.md`, which is tracked, is English-only like its
  neighbours (`boundaries.md`, `perf.md`), and is covered by the 100-column prose gate.
- A pipeline's exit status is its last command's.
  `uv run python script.py | tail -6 && rm script.py` reported success and deleted the script even
  though the insertion had raised - because `tail` exited
  0. When a step is followed by cleanup, run the command without a pipe (or check the artifact), and
    verify the *file* rather than the message: here `check_changelog_mirrors.py` per-section counts
    are the evidence that an entry landed, not a printed "inserted".

Verified: `cargo build --workspace`, `cargo clippy --all --all-targets -- -D warnings`,
`cargo nextest run --all` 579 tests, `cargo test --all --no-default-features`, and the `no_std` gate
command extended to the new crate -
`cargo build --locked --no-default-features --target thumbv7em-none-eabi -p pyrs-event` - green. On
the Python side the extension was rebuilt (`maturin develop --release`) before
`uv run pytest tests/`, so the stream API is exercised against the moved type rather than a stale
`.pyd`. The quality-matrix and changelog-coupling gates were run by name because they are the ones
that enumerate crates; neither keys on the crate list. No behaviour changed, so no instruction-count
scenario should move - that is the claim CI's `Instruction-count baseline` job checks for this PR.

### (cv) A control that deletes the claim, not the code (2026-10-13)

The JSON producer for the event vocabulary was written the way `docs/dev/event-stream.md` argued
for: one traversal, two outputs. `from_json_with_options` passed a sink whose `ENABLED` constant is
`false`, so every emit - and the `String` a scalar event builds - was to fold out of the tree path.
The commit message, the changelog bullet and the ledger entry drafted for that change said so in
those words, and the pull request was opened on that claim. The gate answered:

```text
FAIL  from_json_medium:  +0.70%   from_jsonc_medium: +0.61%   from_json5_medium: +1.05%
```

The attribution is the part worth recording, because the obvious reading was wrong. Four shapes were
measured against the same baseline binary in one session on one host (callgrind Ir, loop = full
minus `--setup-only`, `from_json_medium`): the shipped fold +1.47%, the same emits behind call-site
guards +1.52%, **the same generic sink with every emit call deleted +1.60%**, and a non-generic
`Option<Recorder>` field tested at each site +2.33%. The third is the control, and it says the
events cost nothing at all: what the tree path pays is the *parameter threaded through* `value`,
`object` and `array`. `#[inline]` on `walk`, `value`, `value_inner`, `object` and `array` recovered
none of it - the reading moved 0.007%. `callgrind_annotate` names the mechanism: 75% of the delta is
`__memcpy_avx_unaligned_erms`, 660 counted instructions per parse of a 380-byte document - node
copies that used to be inlined into a return slot are libc calls once the functions carrying them
are instantiated per sink. Nothing in `pyrs-json`'s algorithm changed.

So "a constant folds away, therefore the path is free" is a claim about a compiler's inlining
budgets, and a budget is not a property of the code you wrote: it is a property of the whole
function, which the new parameter also changed. The discipline that follows is specific - when the
claim is that an addition costs nothing, the control is a build with the addition deleted, not a
build with the addition guarded. And the prose gets written after that measurement, not before it:
the first version of this analysis blamed the per-arm `let node = ...` pattern, and the code change
that blame produced moved the reading by 0.05% - nowhere near the 1.5% it was meant to explain. That
is how I know the first version was a story rather than a result.

What the ruling did with it (maintainer, same day): shelve the producer rather than buy it. Three
options were on the table and two lose on evidence already in this ledger. Re-baselining three read
scenarios by 0.6-1.0% taxes a path that has no consumer yet, and `AGENTS.md` makes an upward move a
separate deliberate act precisely so a floor cannot drift as a side effect of shipping a feature.
Writing a second, tree-free tokenizer walk costs the tree path nothing but puts the JSON5/JSONC
dialect gates - hex integers, signed `Infinity`, leading-dot forms, identifier keys, comment slots -
in two places, guarded by one fold-and-compare test that can only see the inputs it touches.
Reordering wins: the tax exists *because* the traversal still builds a tree, and the streaming
reader that this vocabulary was asked for in the first place drops the tree, at which point a
producer has nothing left to tax - and the same ruling settled decision 2 on the way: a multi-format
`parse_stream` is promised to users, in Python and in Rust, so that reader is a feature rather than
an excuse for stopping. `docs/dev/event-stream.md` now carries the table, the mechanism and the
reordered sequence, and the four questions the closed attempt did answer (spans are not derivable
from the tree; a JSON `null` is a resolution, not an event; the stream is finer than the tree on
duplicate members; JSON5 `'k'` keys reach the hub `Plain`) stand with it.

Verified: the CI verdict above is from the enforcing image (`rustc 1.97.1`, tolerance 0.50%). The
local pairs are `rustc 1.99.0` on WSL and read about twice the CI delta for the same commit, so they
are used here for *attribution* - which shape costs what, and where the instructions go - and never
as the gate's number; the stripped/unstripped pair of the same tree differed by 0.003%, which is the
floor they clear. No code shipped, so no test or gate ran: `cargo nextest run -p pyrs-json` (63) and
the workspace build/lint pair were green on the closed branch, and the documents in this change are
checked by `python scripts/check_doc_wrapping.py` and `prek run --all-files`.

### (cw) A control measured on the wrong compiler is a story, not a proof (2026-10-14)

`(cv)` ended with a rule: a claim that an addition costs nothing has to be tested by deleting the
addition. The node-metadata accessor pass is the same claim one level down - "an accessor inlines,
so routing 84 field reads and 16 struct literals through `NodeMeta`'s own methods cannot move the
instruction count" - and it was nearly accepted on a hash. The `ir_gate` harness binary built from
that tree and the one built from its parent came out with the **same sha256**, `c504386fe4b52efc`,
and the section wrote that identical machine code cannot differ in counted instructions.

It was true and it proved nothing. Both binaries were built by the host's rustc 1.99.0; the gate
pins **rustc 1.97.1** (`.ci/ir-baseline.json` records the toolchain and the image), and CI on the
accessor-only PR reported `serialize_small +0.61%` - inside nothing, outside the 0.50% tolerance.
Re-measured paired on 1.97.1 the accessor pass alone reads `serialize_small +5.49%`, concentrated in
`write_plain_scalar` (+81 counted instructions per iteration) and `write_scalar_for_key`, the two
places where a pattern match became a call. A byte-identical build rules out a *counted-instruction*
difference **on the compiler that produced it**; it says nothing about codegen across versions, and
the gate's verdict is the only one that counts. `docs/dev/perf.md` carries the numbers.

The step still shipped, with the layout it was a prerequisite for, because measured together on
1.97.1 the pair is net negative: `from_json_medium` −4.83%, `serialize_anchors` −3.95%,
`parse_inline_merge` −3.53%, `parse_medium` −1.51%, `serialize_small` −0.07%, and nothing above the
tolerance. That is the honest form of "a door costs nothing": not a hash, a paired run on the
enforcing toolchain.

How the pair was measured, since the method is the point: build
`cargo build --release -p pyrs-yaml-core --bench ir_gate --features ir-gate` with
`CARGO_PROFILE_RELEASE_STRIP=false` on both trees with `cargo +1.97.1`, then subtract the
`--setup-only` total from the full total per scenario. Three details bit the attempts. Cargo's
fingerprint followed `jj`'s restored mtimes, so a rebuild after switching changes silently re-used
the previous binary and both hashes matched for the wrong reason -
`cargo clean -p pyrs-ast -p pyrs-json -p pyrs-toml -p pyrs-schema -p pyrs-event -p pyrs-yaml-core`
between the two builds is what makes the comparison mean anything. WSL's `/tmp` is a tmpfs that dies
with the VM: the stashed binaries were gone before the annotation ran, so the artifacts of an A/B
belong under `target/`. And installing the pinned toolchain with `--profile minimal` left it without
`core`/`std` until `rustup toolchain install 1.97.1 --force --profile minimal` re-ran it.

The sizes that motivate the step are in `docs/dev/perf.md` as a table - `CustomNode` was 208 bytes,
`NodeMeta` 128 of them, `Tag` 48 - and they explain the two costs `(cp)`/`(cr)` priced: a `memcpy`
share of 7.88% moving 208-byte nodes, and a 4.58% teardown that asks five `Option` questions per
node. Encapsulating first also found a duplicated question: the serializer asked a pattern whether a
node carries no comment, anchor or tag in the same breath that `hash_custom_node_identity` asks it
three times, and both now call `NodeMeta::is_identity_clean()`. What the grouping then taught is
which fields may move: boxing `source_range` alongside the anchor and the tag - the YAML adapter
records a range on *every* node - cost `parse_medium +5.94%` to buy `from_json_medium −7.41%`, so
per-node-written slots stay flat and only `{anchor, tag}` went behind the pointer.
`node_layout_is_part_of_the_hot_path_contract` pins the resulting sizes - 72 / 64 and 144 for the
`std` node, 128 for the `no_std` one, because the feature split swaps the node map's hasher too - so
the claim in that file cannot rot into a comment. The first version pinned 144 alone and CI's
`cargo test --locked --no-default-features -p pyrs-ast -p pyrs-schema` job reddened on it: a node
size is a per-configuration fact, and the job that measures a configuration is the one that catches
the assumption.

Verified: the paired deltas above were measured on `rustc +1.97.1`, the compiler the baseline names,
in WSL, with `CARGO_PROFILE_RELEASE_STRIP=false` and the scenario loop taken as the full run minus
`--setup-only`. The comparison script sat under `target/ab/`, which is gitignored scratch, so the
numbers are reproducible by the recipe in this entry but not by a repo tool; CI's
`Ir instruction-count baseline` check is the verdict for the shipped tree, and `from_toml_medium`
(+0.74% local, about half that on the runners) is the one scenario likely to need
`--accept-drift`. Gates on the shipped tree: `cargo nextest run --all` 580 (the layout tripwire
included), `cargo test --all --no-default-features` 618,
`cargo clippy --all --all-targets -- -D warnings`, the extension rebuilt with `maturin develop
--release` before `uv run pytest tests/` - 2719 passed, 10 skipped - `maturin generate-stubs` plus
`check_stub_drift.py` clean, `prek run --all-files`, `check_changelog_mirrors.py` (all five mirrors
at [26, 12, 39, 8]), and `check_doc_wrapping.py --fix` over the changelogs and `docs/dev/perf.md` -
the wrapping gate reddened a first attempt at this section, which is exactly what it is for.

### (cx) Two oracle tests, two bugs in the fix for #336 (2026-10-14)

`(cv)` said the control for "costs nothing" is a build with the addition deleted. `(cw)` said the
control has to run on the compiler the gate pins. This is the same discipline applied to a
*correctness* change, and it earned its keep twice in one sitting.

The change: fold the keys the reader resolves alike, so `to_yaml` never emits text its own reader
refuses or silently shortens (#336, ruled: fold on write). The price is in `docs/dev/perf.md` -
+12-34% on the serialize scenarios, structural rather than incidental, so the branch
(`yaml-writer-key-fold-336`) is shelved with its numbers instead of re-baselining the floor.

What the tests did on the way there:

- the pinned key-rule table (`the_key_rule_agrees_with_the_writer_and_the_reader`) is an oracle
  because it writes each pair with the real writer and re-reads it with the real reader. Growing it
  from 15 classes to 19 found the hand-maintained model wrong twice (a tag does distinguish keys; a
  plain `null` spelling and its quoted twin are not one key), and then found a bug inside the
  refactor that was meant to fix it: tagged scalars routed through the null class, which claimed
  `!a k` and `!a 'k'` were one key when the measured round trip returned two. The model and the
  author disagreed, and the reader was the referee.
- the fold's own note-ordering test was written to pin determinism - re-homed notes must arrive in
  document order, because walking a `HashMap` would emit them differently per process. It failed,
  and the failure was not about order: a pair folded twice had its first entry's notes attached to
  the intermediate entry, which was itself about to be dropped, so two of three notes vanished from
  the document. A test aimed at one property caught a silent data-loss defect in the same line of
  code.

Both are recorded because the tempting shortcut was available at each step: the rule could have been
kept as prose branches that "obviously" match the reader, and the note order could have been
declared fine and left to a fuzz run to notice.

The boundary that survived is asserted, not commented: a pair whose dropped entry carries an anchor
is left out of the fold, because a node holds one anchor and a dropped value's anchor cannot be
re-pointed without changing what its aliases resolve to - folding there would trade a short line for
a dangling reference, which the engine promises never to emit. The reader solves the same problem by
inlining orphaned uses (`parser::repair_orphaned_anchors`); a writer that sees one mapping at a time
cannot.

### (cy) The stub describes the extension; somebody had to describe the package (2026-10-15)

`ROADMAP.md` row 1 asked which artefact `python/pyrs_yaml/pyrs_yaml.pyi` is the contract *of*, and
named three routes. Answering it needed a measurement rather than a preference, and the measurement
moved the question: mypy 2.4.0, run against the installed package, answered
`reveal_type(pyrs_yaml.parse("a: 1"))` with `Any`. The stub was not the first thing missing - the
public Python layer was. `__init__.py` re-exports the binding's `parse` through a wrapper annotated
`-> Any`, so no checker ever reached `YamlDocument`, and every question about its members was
unreachable through the entry point users call first.

What the two artefacts actually disagree on, counted rather than estimated: twelve public members
(`node`, `find`, `walk`, `scalars`, `merged`, `set`, `set_many`, `insert`, `append`, `delete`,
`rename`, `sort_keys`) exist on `YamlDocument` only after the package imports, because they are
assigned onto the class at lines 357-378, and maturin 1.14.1 introspects the compiled module before
that has happened. `Node` and `MergedView` were *not* in the same hole, which is where the row's
wording overreached: they are Python modules under `py.typed`, so a checker reads their members from
source. The gap is specifically the class the extension owns and the package edits.

The route taken is the declared reconciliation, plus the honest wrapper annotations it depends on.
Members are read from the compiled module's classes with `vars()` (not `dir()`), signature and
docstring included, and inserted into the class body the generator wrote;
`EXPECTED_ATTACHED_MEMBERS = 12` is the same kind of tripwire the exception block uses, an
unannotated member is an error rather than a written `Any`, and a parameter shape the route has not
measured - keyword-only, `*args`, positional-only besides `self` - stops the run instead of
inventing a spelling. Moving the members behind the extension boundary was rejected on the spot:
they return `Node` and `MergedView`, which are Python objects, so the binding would have to grow a
Python-level class to answer a typing question.

Two traps, both found by running things, worth the record because both looked like the artifact's
fault:

- `scripts/check_stub_types.py` builds a scratch package holding *only* the stub, so
  `from .node import Node` came back `import-not-found` - a finding about the directory, not about
  the wheel, and precisely the kind of red that teaches someone to widen the filter. The gate now
  copies in the sibling modules the stub itself imports, read out of its syntax tree, and a stub
  naming a module the package lacks is exit 2: the wheel would ship a contract that references
  something it does not contain.
- Matching a class by name let a synthetic `class Node` in the generator's output collect the
  *package's* Python `Node` - forty-one members injected into a class the extension never wrote.
  Candidates now come from the compiled module's namespace, which is what "the generator could have
  seen" means.

The check that the fix is load-bearing rather than cosmetic: `document.node("a")` is now an error at
check time (`Too many arguments for "node"`), which is what the runtime has always said
(`_yaml_document_node() takes 1 positional argument but 2 were given`) - the checker and the object
agree for the first time. Deleting the twelve declarations from the committed stub turns
`test_the_stub_declares_every_public_member_of_a_class_it_names` red naming all twelve, and the
wrapper predicate run over `__init__.py` at `HEAD` names `parse` and `parse_file`.

### (cz) The mismatch taxonomy said *which* round trips fail, and one row was a byte the writer invented (2026-10-15)

`(cx)` counted the property tier's rejections; `(cs)`-era work established the guard as the thing
that hides them. Classifying what the guard declines is a different instrument, and this is what it
said over 4000 trees drawn from `arb_custom_node()`, each serialized and re-read: **0 refused to
parse**, 651-626 passed the strict equality, and the rest fell into five shapes.

| count | first conjunct the equality refuses | reading |
|------:|:------------------------------------|:--------|
| ~1174 | a note the equality compares in the inline slot, text still present elsewhere in the tree | the oracle's slot model is narrower than the engine's note placement |
| ~506 | a note whose text is nowhere in the re-read tree | real loss, to be attributed before it is normalized |
| 884 | `Literal`/`Folded` re-read as `DoubleQuoted` | the writer's content-driven fallback, not yet stated as a normalization |
| 551 | a block container re-read as flow | same shape of question, container side |
| 255 | a block scalar's **value** differs, style preserved | a byte invented or dropped - not a normalization at all |

The last row is the one a taxonomy exists to catch, and it had one cause: `effective_chomping`
honored the node's claimed indicator, but `Clip` *denotes* one trailing line break. A `Clip` value
with no trailing break (`abc`) was therefore written `|\n  abc\n` and read back as `abc\n` - the
document one byte longer than the tree that produced it. Every one of the 255 was that shape: length
N read as N+1. Fixing it raised the next question, which is where the boundary belongs: `Strip`
claimed for a value that *does* end in a break is also inconsistent, but `-` is expressible and
asking for it is precisely the edit `set_chomping("strip")` means, so the correction applies only to
claims no header can spell (`Clip`/`Keep` over a breakless value, plus the two documented `Clip`
promotions). A Python-visible case settles it: `key: |-\n  line1\n  line2\n` asked for clip now
stays `|-` and re-reads as the same text, while `test_set_chomping_strip` still gets its `|-`.

Five byte-exact writer assertions had to move (`|` → `|-`, `|4` → `|-4`), which is the honest cost
of a rule that was never covered: the pinned block fixtures all ended their fixture values in `\n`,
so five generations of writer changes exercised the correct half of the table and never the
invented-byte half. Recorded for the tier discipline, because the split mattered here: the
writer-internal claim lives in `serializer.rs` tests (run by `cargo nextest`, no Python needed), and
the promise a user can reach lives in `tests/test_node_style.py` (run by `pytest`) - 526 Rust tests
and 2756 Python tests respectively, neither one standing in for the other.

Two things this measurement exposes but does not fix, so they are named rather than absorbed. The
equality compares only `NodeMeta::comment()`, so a note the reader files in the leading-comment
stack reads as lost even when its text survives; and `arb_node_meta()` never populates `decor` at
all, which is why the tier has never asserted a standalone note round-trips. Both belong to the
conditional-assert hole: an oracle that disagrees with the engine about where a note lives cannot be
made unconditional by deleting its guard.

### (da) A stated tolerance turned the blind spot into a short list, and the list named two corruptions (2026-10-15)

`(cz)` classified what the round-trip guard declines. Its finding was that almost nothing is
classifiable while the oracle itself disagrees with the engine: the guard filtered **3433 of 4000**
trees, and a note-position difference travelled with 3336 of them. So the experiment was to write
the tolerance *as a predicate* and re-measure — accept any restyling of a scalar whose value is
unchanged, accept `Null` ↔ a plain null token, accept an empty container's flow spelling, accept a
note that moves owner or sheds padding, and refuse everything else. Over the same 4000 trees:
**3004 clean, 996 with residue** — a filter rate of 25%, down from 86%, with no guard deleted. The
residue, measured before the two fixes below:

| residue, by node position | count | what it is |
|:--------------------------|------:|:-----------|
| a key's note never reaches the text | 707 | `property-tier`'s share of the open block-key-note hole |
| a note never reaches the text (value/item/flow/root) | 200 | same hole, other sites |
| a block scalar's **value** changes | 434 | not a normalization: the writer's body column |
| a note's text differs by padding | 302 | statable, and statted rather than guessed |
| an anchor name changes | 11 | rare, unattributed, `dup-in-tree=false` on every case |

Two of those rows were fixed in the same pass, and both were value corruption rather than style:

- **An indentation indicator is parent-relative.** Measured on `a: |2`, `- |2`, `? a`, and their
  comment-pushed spellings: PyYAML, ruamel and this engine's own reader all strip
  `parent + indicator`. The writer stripped `header line + indicator`, which is the same column
  until a leading note pushes the value onto a line of its own — then the body landed two columns
  deep of the number printed on the header, and the extra columns came back as content (`" "`
  re-read `"   "`). 434 of 4000 trees, all of them the same two-column shift.
- **A block body cannot hold a comment.** A simple key's inline note has no spelling before its
  `:`, so the writer rode it at "the end of the pair line"; for a block value that end is the last
  *body* line, where `#` is content — `abc` came back `abc  # n`. The note now rides the header
  line, which is the one line in that entry a reader reports a comment from, so the value keeps
  its bytes and the note survives as the value's own.

Attribution stayed in its own layer, as `(cz)` set it: the writer-internal claims are
`serializer.rs` tests under `cargo nextest` (607 now), and the promise a caller can reach — that
the text we emit decodes, in all three engines, to the value the tree held — is
`tests/test_yaml_crosslib.py::test_the_references_read_the_block_bodies_we_write` under `pytest`
(2773). Each was proven by mutation in both directions: with the parent-relative base reverted the
two own-line Rust tests and the crosslib test redden with `'     '` vs `'   '` named, and the
key-note test stays green; reverting only the note slot does the opposite.

What the predicate experiment does *not* settle is the hole itself. Re-measured with both fixes in,
the residue is **827 trees and no value corruption**: 907 note occurrences still never reach the
text, 707 of them on a block mapping key — the placement the writer has no spelling for, which is
where the tier's conditional guard still lives. So `property-tier:conditional-assert` stays open and
its next step is that note slot, not a looser oracle. The probe that produced these numbers was
throwaway by design: a `TestRunner` over `arb_custom_node()` in `pbt.rs`, counters panicked at the
end, then removed.

### (db) The slot question was answered by measuring the reader first, and 673 notes came back (2026-10-15)

`(da)` left one number as the next step: 707 note occurrences sat on a block mapping key and never
reached the emitted text. `ROADMAP.md` row 2 had named the possible rulings and insisted on the
precondition — "a placement rule is only worth having once it has been measured against the reader's
own normalisation". The measurement is three documents read into the AST: `a: 1` + `# K` +
`b: 2  # V` reports `K` in **key `b`'s** `decor.leading_comments`, while the same line above a
mapping's first pair belongs to the mapping. So the flow path's answer — spend a line break — keeps
the owner for every pair but the first, and a relocation that preserves the text beats a drop in
either case.

The rule as shipped: the value keeps the pair's trailing slot when it needs it, and the key's note
goes above the pair when the slot is taken. Nothing else changed shape, which is why the pinned
comment-on-a-pair fixtures stayed put and exactly one existing assertion moved:
`a_note_trailing_a_key_survives` had pinned `"a: b  # vv\n"` for `? a # kn` + `: b # vv` — the loss
recorded as expected text. It now pins `"# kn\na: b  # vv\n"`, plus the fixed point.

| measured over 4000 generated trees | before | after |
|:-----------------------------------|-------:|------:|
| notes that never reach the emitted text | 907 | 257 |
| … of them on a block mapping key | 707 | 34 |

Trees that still lose at least one note: 244 of 4000. The remaining 257 are spread evenly over
key/value/item/root in both block and flow spellings — the signature of several distinct causes
rather than one, so they are left unattributed here rather than closed on a guess.

The layer split moved with the claim, not with the convenience: the *ownership* assertion has to be
Rust, because no path in the bindings addresses a key node (`cargo nextest`, 610 tests now), and the
caller-visible promise — both notes present, neither duplicated, the rewrite text-stable — is
`tests/test_fidelity.py::TestFidelity::test_both_notes_of_a_pair_survive_the_rewrite` under
`pytest`. Both were red before the writer change.

### (dc) The residual was one loss per 4000 trees, and the instrument that said 257 was the thing wrong (2026-10-15)

`(db)` reported 257 notes still absent from the emitted text. That number measured the wrong
predicate: it asked whether the note's **raw** text appears in the document, so a note of `" "` was
judged lost (no comment carries padding) while a note of `"x"` sitting inside some scalar's text was
judged kept. Re-asking with "the note's trimmed text appears in the document at all, blank texts
exempt", the residual over the same 4000 trees was **3** — and shrinking found a minimal tree for
the largest of them:

```text
tree     Sequence[ Mapping[ Null(key, standalone note "2") : " " ] ]
emitted  - null: " "                 # the key's note, gone
```

The cause was a copy that stopped one line short: `write_mapping_pair` prints a simple key's leading
notes above its pair, and the compact `- key: value` loop repeats that for every pair **but the
first** — the first shares the dash line and has no line of its own, so nothing printed its key's
notes. They now go above the item's line. One Rust test per key shape (`null` and a plain scalar),
red before the fix on the exact emitted text.

After that case closed, the measured residual is **1 loss in 4000 trees**: a standalone note on a
block mapping value. It is named here rather than absorbed, and it is the last thing between
`property-tier:conditional-assert` and an unconditional round-trip assertion — with one caveat the
re-measurement made plain: the tolerance has to be stated over *trimmed* note text, because padding
around a comment marker is not content any reader can recover, and a predicate that ignores that
audits the writer for a thing writers cannot do.

| predicate over 4000 trees | (db) | this pass |
|:--------------------------|-----:|----------:|
| raw note text absent from the document | 257 | — |
| trimmed note text absent, blank texts exempt | 3 | 1 |

### (dd) The classifier refused to guess, and the guess it refused was a bug in the reader (2026-10-15)

The round-trip tier's silent filter is being replaced by a classifier: each difference between a
tree and its re-read gets a name (`NullToken`, `PlainRequoted`, `BlockRestyled`,
`EmptyContainerFlow`, `NoteRehomed`, …) or the property fails. That is `(da)`'s design put into
code, and the first thing it did was refuse to name a class it could not justify: an anchor whose
name comes back *different*. 11 per 4000 trees, all with `dup-in-tree=false`, so `(dc)` left them
open.

Refusing is what the whitelist is for. Shrinking the class produced

```text
{a: &x "x # y", b: &y 2}     ->   {a: &x "x # y", b: &x 2}
```

and the cause was in the reader's own documentation. `anchor_name_before` recovers a name by
scanning left and skips any `&` that follows a comment-opening `#` on the line, with a recorded
exception: "A quoted `#` earlier on the line is the one shape this would over-refuse, and it has
**no reachable form**: node properties (`&anchor`, `!tag`) always precede the node value, so nothing
that granit can mark as an anchor ever follows a quoted scalar on the same line." The premise is
true of a node's own properties and false of a *node after another node* — which is exactly what a
flow collection is. So `b`'s anchor was skipped over, the scan reached `&x`, and the engine
re-emitted a document where two nodes share one name and every `*y` alias points at nothing: not a
lost style bit but a corrupted object graph, and the Python-level claim is pinned as
`tests/test_node_metadata.py::test_a_flow_anchor_keeps_its_own_name_past_a_quoted_hash` (red against
the pre-fix build: `{'a': 'x', 'b': 'x'}`).

The line scan now tracks quoting (`''` in single, backslash in double), with one restraint learned
from the tests around it: a quote counts only where a scalar may begin, because granit's anchor-name
grammar lets a *name itself* hold a `"` (`&"q` is the name `"q`), and treating that as a string
opener would hide real comments. 611 Rust tests, 2775 Python.

The instrument earned its keep twice over: the same classifier named `NoteForcesBlockLayout` and
`InlineContainerFlow` for the two container relayouts the tier used to swallow, each justified by
the position that forces it (a note ends its line; a block body cannot live inside `{}`) rather than
by the fact that the old oracle happened to ignore it. Those two labels belong to the classifier
work, not to this change — they are recorded here because the *reason* each one is a normalization
is already measured, which is the test the eventual whitelist has to pass.

### (de) A slot remembered as "the last header written" is not a slot on this pair's line (2026-10-15)

`(db)`'s fix put a key's note on the block header line, so the writer began recording
`block_header_break` whenever it wrote one. The record is global to the emission - *the last header
written anywhere* - and the consumer asked only whether a body line followed. For a pair whose value
is a container that question returns yes about someone else's line: the inner pair writes its own
header after the outer one started, and the outer key's note was inserted into it.

```text
A: >  # a-5  # %note        before   (header split, two comments merged, body orphaned)
       ]$$2.RzM*yl>          ->        "misplaced bracket at line 6 column 8"
```

The emitted document stopped parsing. Nothing local saw it: `cargo nextest run --all` executes the
tier at its default case count and the shape needs a key with an inline note, a container value, a
nested pair with an inline note *and* a folded block carrying an indentation indicator. The 20k-case
`property tier (20k cases)` leg on CI walked into it on the next sample. The rule now reads: take
the header slot only while no line break separates it from the start of the pair's own line -
`note_slot_on_pair_line`, pinned by `a_key_note_never_rides_a_nested_pairs_header_line` (red on the
pre-fix build, on the fourth assertion: the note is back on `" ":` where it belongs).

The process lesson is the reason this entry exists: a writer change in a note-placement path is not
gated by an ordinary local tier run, and the elevated count is cheap to ask for -
`PROPTEST_CASES=20000 cargo test -p pyrs-yaml-core`. CI has always run it; the local loop did not.

### (df) The first candidate placement was refused by a test that already existed (2026-10-15)

`(de)` left one note family unexplained, and the tier minimized it to a sequence item carrying a
note over a `|-` body. The obvious fix was the slot `(db)` had opened for keys - ride the block
header, the last line on which a `#` is a comment - and it was written, and it made the new test
green.

It was refused by a test that predated the bug.
`parser::tests::a_block_scalar_body_never_borrows_the_containers_note` pins that a note on the
header line is read back as the *value's* comment, and its sibling at the same level pins the other
half: a note line below a block is read back as the leading note of the node that ended the block
(crash-11ced252). Taking the header slot therefore did not place the container's note anywhere near
the container - it moved it onto the wrong node and kept the text.

What the two pinned facts leave is placement by indent, and the second shrunk shape said so: the
same loss with no block scalar in sight (`- - null` under a note, the item note gone), which is why
the rule reads `indent_width > 0` and not "the tail is a block body". Below the body is kept where
the reader reports the note back to the container - column 0, where that line is the document's
tail - and moved above the container's own line everywhere else, through the same `note_line_above`
machinery `note_line_above_last_pair` measures for a pending tag line.

Both shapes are pinned with survival *and* the fixed point asserted together, because a placement
that loses the note is also stable: losing it is exactly what the equality alone would have
accepted. The anchor `body_start` is captured before the preamble line is written, so a note can
never be inserted between a container's tag line and the body it tags.

615 Rust tests at this change; the two shapes above are the last note residuals the classifier
named, and with them closed it named none in the note family - measured at `PROPTEST_CASES=20000` on
the stack that also carries the reader-side anchor fix below, where the tier is 26 passed / 0
failed.

### (dg) A quote after a blank is not necessarily a quoted node (2026-10-15)

The classifier's last named residual was an anchor coming back as another token's text -
`anchor at $."a '"#0: Some("m6") -> Some("*~=9VjQq=8")`. The name it produced is an alias spelling,
the shape of a bug rather than of a style shift, and the emitted line explains it:

```text
a ': &m6 >-4  # ":})!,"Hab*5^Z|B':^<-&*~=9VjQq=8
```

The left-scan that #421 widened ignores a `#` inside quotes, and its rule was positional: a quote at
a scalar-start position opens a region. Being after a blank is not being at a scalar start. The
apostrophe that ends the plain key `a '` is followed by a blank, so it opened a region whose close
was the apostrophe inside the comment; the real `#` between them stopped being a comment opener, and
the `&` written in the comment became the anchor name. The tier named it about once per 20 000 trees
because it takes a comment text holding both an apostrophe and an `&` - the generator's own
vocabulary, and the writer emitted the line.

The rule that survives both pinned cases is the token, not the character: a quoted node begins one,
so the scan walks back over blanks and asks what begins the token before the quote - a boundary
(`:`, `,`, `[`, `{`, `?`, `-`) or a property (`&`, `!`) - and only then treats the quote as an
opener. Two half-versions were refused by #421's own test on the way: reading the single character
before the quote lost `{a: &x "x # y", b: &y 2}`, because the token there ends in the anchor's name,
and stopping the walk only at blanks walked straight through the `[` of `[&x " #", &y 2]`. The
pinned case has to stay green while the new one turns red; that is what makes it a test rather than
a preference.

616 Rust tests, with `anchor_name_before_reads_a_lone_apostrophe_as_text` red before the change
(`left: Some("*~=9VjQq=8"), right: Some("m6")`). The class did not end there: a later 20 000-case
draw named a second shape, `A?': &x >-2  # o'k &evil`, where the apostrophe sits directly against a
`?` - and `?` is on the boundary list because it is the explicit-key indicator. A boundary opens a
node only across a blank, so the rule reads the blank as well as the character, and
`anchor_name_before_reads_a_quote_after_a_question_mark_as_text` pins the case it was blind to
(`left: Some("evil"), right: Some("x")` before that half-step).

### (dh) Replacing the filter was the debugging, not the cleanup (2026-10-15)

`property-tier:conditional-assert` was registered with an exit that named its own condition: no
`prop_*` body may guard its assertion on an `Option`-returning helper, and every mismatch is either
asserted or named. Deleting `try_roundtrip` was therefore the instrument, not the tidying. The
changes it forced are the record: a block scalar whose value was corrupted, a key note that lost its
pair's slot, a renamed anchor, a nested header that swallowed a note, a compact dash line that
printed no first key, and a container note that survived only until the second emission - #418
through #422, #424 and the two fixes stacked with them.

A named normalization has to say why the reader cannot do otherwise, or the hole is back with a
label on it. `NullToken` is a resolution fact (`~`, `null` and the empty token all read as null);
`PlainRequoted` and `BlockRestyled` are the writer choosing a spelling the value survives;
`EmptyContainerFlow` and `InlineContainerFlow` are positions with no block spelling at all;
`NoteForcesBlockLayout` and `NoteRehomed` are the measured placements of (db) and (dd). The last two
were not inherited from what the old oracle stayed silent about - each was argued from the position
that forces it, which is the only test that separates a normalization from an excuse.

The two order properties needed a different call, and it is the one worth writing down: they were
not guarding too much data, they were claiming the wrong thing. A key compared as `to_yaml(key)`
made a flow/block shift on a container key an *order* change, and collecting flow mappings by their
style bit made a block container that must read back as flow change the number of flow mappings.
Both are style facts the classifier already names. The claims are now stronger and honest - a key
compares as kind-tagged entries, and every mapping keeps its key order at its own path, root and
nested, flow and block alike - so a bit the reader decides can no longer pass for a lost order.

Measured after: 26 properties with an empty guard list, 617 Rust tests, and the elevated tier at 26
passed / 0 failed on 20 000 cases - stated as draws, because the first green draw was not the end of
it. A later 20 000-case run of the same tier named the `A?'` shape that (dg) records, and only after
the reader carried both quote rules did a draw come up empty. No seeds were committed for those
runs: `proptest-regressions/pbt.txt` is the file this change inherited, and every shape the
classifier minimized to is pinned by a named test instead - a replayed seed that no test explains is
a second oracle nobody maintains.

### (di) The last open item was closed by a price, and the price had already been paid (2026-10-15)

`ROADMAP.md` carried one row: what the writer should do with a mapping whose keys its own reader
resolves as one entry (issue #336, opened by `(br)`). Three options were written there, and two of
them turned out to be the same bill.

Folding on write was not a hypothesis but a branch, and `docs/dev/perf.md` keeps its numbers:
+33.655% `serialize_anchors`, +32.585% `serialize_medium`, +31.801% `serialize_block`, +31.481%
`serialize_small`, +12.529% `serialize_block_scalars`, paired on the gate's pinned toolchain against
the same tree with the switch off, and `parse_small` unmoved at +0.000%. The tax is per key, a key
line costs about 55 counted instructions, so the fourteen instructions any "look at every key before
writing this mapping" check spends is a third of the scenario - and the record already said no
implementation of it lands inside the 0.50% tolerance. Reporting the pair instead of folding it
needs *that same pass*: the identity of every key of every mapping, computed to compare against the
others. That is an inference from a shared premise, not a fresh measurement, and it is written here
as an inference.

The option that costs nothing is where the guarantee already sits. A key is resolved where a caller
puts it in - the edit layer - and that layer compares by `key_identity`, so the shape cannot be
built through the Python boundary. Measured on the build this change pins:

```text
parse("a: 1\n")["~"] = 1;            ["null"] = 2          ->  "a: 1\n~: 2\n"
parse("a: 1\n")["null"] = 1; ["~"] = 2; ["NULL"] = 3       ->  "a: 1\nnull: 3\n"
```

The first spelling survives, the last value wins, one line is written, and the document re-reads
with one null key. Paths reached the same conclusion in `(br)`'s follow-up; what was missing was the
sentence saying the *writer* is not where the check goes, and a test that stops someone from putting
it there on a hunch.

So the ruling is the third option, executable on both sides of the tier split.
`TestKeyIdentityAcrossSpellings::test_two_spellings_of_one_key_fold_at_the_boundary_not_in_the_writer` is the caller-visible promise. `the_write_path_keeps_both_lines_of_a_pair_its_reader_resolves_as_one` pins the unsupervised shape: a hand-built `NodeMap` still emits two lines, and the reader is the one that refuses them as a duplicate key or answers with one entry fewer. Whoever folds or reports at emission has to edit that test and say what moved the price.

Measured after: 618 Rust tests, and no code path changed - the additions are `#[cfg(test)]` and
documentation - so the instruction-count floor was left alone rather than re-claimed. The open-item
table in `ROADMAP.md` is now empty, which is what that section says it is for: an item leaves the
moment a change ships the decision.

### (dj) The root loop asked what the value was, not what the source wrote (2026-10-15)

`to_toml`'s root pass asked one question of every pair: is the value a non-empty mapping? If so it
went to the section list. `flow_style` - the bit this crate's own reader sets for `p = { a = 1 }`
and leaves clear for `[p]` - was bound in the pattern and never read.

Measured through the Python hub on the build before the fix:

```text
p = { a = 1, b = 2 } # tail   ->  [p] # tail + body        (and not a fixed point on the second round)
p = { a = 1 }  with q = 2     ->  q = 2, then [p], then a = 1
p = { a = 1 } under [s]       ->  stays inline
arr = [{ a = 1 }]             ->  stays inline
```

The move of `q` is not cosmetic. TOML wants a table's plain keys before its sub-table headers, so
promoting a root value to a header forces its siblings ahead of it, and the note that sat on the
pair line travels to the header line with it. The `[s]` case is the proof that the writer already
had the information: a nested inline table kept its form because it reached `emit_pair`, and only
the root classification ignored the bit.

Four tests had been pinning the promotion, and two of them did it by asserting the header text -
`a_note_between_two_inline_members_keeps_its_own_line` and the parser's
`the_fence_reaches_values_inside_a_container`, whose own comment called the rewrite "a writer choice
this change does not touch". A third, `to_toml_emits_multiline_inline_table_as_single_line`, stated
the single-line shape as its intent and then asserted two `contains` checks, which a section header
satisfies; the fourth carried the claim only in a comment. All four now assert the inline form, and
the one whose purpose was the shape is asserted *on* the shape.

Not claimed: a note on its own line inside the braces still does not survive the hub
(`p = { a = 1, # inner ... }` comes back as `p = {a = 1, ...}`) because the YAML writer has no slot
for a flow member's own-line leading note - the boundary the flow-note decision recorded. It is
pinned as a loss (`test_a_note_on_its_own_line_inside_the_braces_is_still_lost`) rather than left
tacit.

Measured after: 619 Rust tests, 2779 Python. The instruction floor is not re-claimed here: the
change moves one root mapping from a path that materialises the section name to one that appends, so
CI's `Instruction-count baseline` leg on `to_toml_medium` is the check, not this entry.

### (dk) The flow indicator has to begin a token as well (2026-10-15)

The 20 000-case tier named the third shape of the family (dd) and (dg) opened - and it named it
while an unrelated pull request was in CI, which is the point of an elevated draw:
`!!t /[": &Ilr |-8  # &m^fake` came back anchored as `m^/=.*/f?-6?~w`$'$z^b8`, a `&` written
*inside* the comment.

The rule #425 shipped admitted a quote as a node opener after a blank, `:`, `?`, `-`, `,`, `[` or
`{`. Only the bracket over-admits here: `/["` is a plain scalar whose own text holds a `[`, so its
`"` was read as opening a quoted node, the region never closed, the `#` that really opens the
comment was scored as region content, and the recovery took the comment's `&` as the name. Renaming
an anchor is not a style loss - every alias to the real name points at nothing in the document the
engine then re-emits.

The rule now asks a *bracket* to begin a token and leaves a *comma* alone. The first cut asked both,
and the tier caught that over-tightening inside the same change: in `0: [a, " #", &ANCH null  # n ]`
the comma sits right after a bare member (`a,`), so requiring the indicator itself to begin a token
read the member's `"` as plain text and its `#` as a real comment opener - the node's own anchor
came back `None`, and the re-emission dropped it. `prop_roundtrip` and
`prop_style_setting_roundtrip` both drew that shape at 20 000 cases on the branch carrying the
bracket fix.

A member separator always opens the node after it; only a bracket can be ordinary scalar content, so
only a bracket takes the token test. Each red check was done by deleting the condition rather than
trusting the test's name: `anchor_name_before_reads_a_bracket_inside_a_plain_key_as_text` reports
`left: Some("m^fake")` without the bracket test, and
`anchor_name_before_reads_a_quoted_hash_after_a_member_comma_as_region_text` loses `&ANCH` without
the comma clause - the two shapes pull in opposite directions, which is what a single rule has to
tell apart. 620 Rust tests, and the elevated tier at 20 000 cases is 26 passed / 0 failed on the
refined rule against three failing properties on the first cut. Nothing is left open from this
entry.

### (dl) The map keeps two container keys apart; the text never did (2026-10-15)

`KeyIdentity` routed every non-empty container and every tagged scalar to `Other`, "compared as
nodes, which is what the map itself does, so no two of these are ever the same key here". The node
map is not what decides, though: the *written text* is, and the property tier drew the shape the
model could not speak about - `? - null` twice, one of them carrying a note on the key node. Two
identical explicit-key blocks go out; `duplicate key: Sequence { .. }` comes back, and
`prop_idempotent` fails at "output re-parses".

Each candidate pair was measured rather than argued, by emitting it and reading it back:

| pair of keys | model said | reader did |
| :-- | :-- | :-- |
| two sequences of one null, one carrying a note | distinguishable | duplicate key |
| a tagged scalar and its noted twin | distinguishable | duplicate key |
| a flow sequence and its block twin | distinguishable | two entries |
| two mappings whose note rides an inner key | distinguishable | two entries |
| a sequence of one null and an empty sequence | distinguishable | two entries |

A note riding the key node reaches the line, not the key; spelling and content reach the key. So the
classes now say it: `Tagged { text, tag, style }`, `Sequence { items, flow, tag }` and
`Mapping { pairs, flow, tag }`, with equality and hashing written by hand and containers compared by
whole-node equality of what they hold - which is the reader's own comparison, not a re-derivation of
it. `Other` is what those classes do not cover.

The consequence for the tier is the reason this is one model rather than two: the generator's rule
is the same `a.key_identity() == b.key_identity()` expression, so the pair the reader refuses is the
pair the generator will not build, and the edit layer addresses by the same identity. The pin table
grew the five classes above with the reader's measured outcome on each side (14 distinguishable, 11
colliding), because every gap between the rule and the reader has so far surfaced first as a random
property-tier failure - (br) built the model for that reason and (dk) is the third time it has paid.

The pair is committed as a replay seed (`cc 05c7a606b5a5…`), taken from the run that recorded it on
TOML pull request #428 while that change sat in CI: its elevated tier failed here, not on its own
subject, so that change rebases onto this one. Measured after: 620 Rust tests, pyrs-yaml-core's 20
000-case tier 370 passed / 0 failed. The instruction floor is not re-claimed: nothing in the reader,
writer or serialiser moved, and none of the 22 committed scenarios reaches the edit layer's key
comparison, so CI's `Instruction-count baseline` leg is what would notice if that were wrong.

### (dm) The writer's bare-token question was cheaper than the reader's grammar (2026-10-15)

`is_json5_number` decided whether a plain scalar could be emitted without quotes by asking
"digits, then a dot". The JSON5 reader asks a stricter question: an integer part is `0` or
`[1-9]\d*`. The two questions disagree on `00.`, and a YAML plain scalar is free to be spelled that
way - so the writer emitted `[[00.] /* … */]` and `prop_json5_output_reparses` failed at 20 000
cases on text this library's own reader rejects.

| spelling | emitted bare before | re-parses | emitted now |
| :-- | :-- | :-- | :-- |
| `00.` | yes | no | resolved value |
| `007.` | yes | no | resolved value |
| `5.` | yes | yes | still `5.` |
| `0.` | yes | yes | still `0.` |

The fix is the integer part, not the dot: `json5_integer_part` is what the grammar says, and a
spelling the dialect cannot hold falls through to the schema resolution that every other
non-JSON notation already uses, which keeps the value and gives up only the source form. The tier
that found it was the one #428 was sitting on - a JSON5 defect, in a TOML pull request, found
because the elevated draw is orthogonal to the change it is testing. Rust owns the token rule
(`a_number_the_json5_reader_refuses_is_not_emitted_bare`, red on `00.` before the fix); the
property keeps the whole spelling space covered, and the unit test names the pair.

### (dn) The question a writer asks its own reader, asked from a table (2026-10-15)

The defect behind PR #432 was found by a draw, not by a check: `prop_json5_output_reparses` built
`00.` among 20 000

generated trees and only then did anyone ask whether `is_json5_number`'s question - "digits, then a
dot" - is the question `from_json5` answers. Random draws are the right tool for shapes nobody
thought of; they are the wrong tool for a spelling list that is already known, because the next
disagreement waits for a lucky draw too.

So the corpus is now asked directly.
`every_dialect_reads_back_what_it_writes_for_the_notation_corpus` (pyrs-json) runs 24 notations a
YAML plain scalar can carry - JSON's own forms, the JSON5-only spellings #120 admits, notations
strict JSON must normalise, and non-numbers - against all three dialect writers, and
`to_toml_reads_back_what_it_writes_for_the_notation_corpus` (pyrs-toml) asks the same of 18
spellings. A writer refusing a value passes: that is a stated reason, not a loss. What fails is text
its own reader rejects.

Measured that the tests bite rather than decorate: weakening `json5_integer_part` back to the loose
predicate fails the JSON family corpus at

```text
json5 wrote "[00.]" for the spelling "00.", which its own reader refuses
```

and the TOML corpus passes on the spellings its sweep had already shown to read back (`01`, `001`,
`+7`, `1_000`, `0o17`), so it records a boundary rather than asserting a hope.

### (do) The hub hands the note over in a slot the TOML writer never read (2026-10-15)

(dj) recorded this loss without claiming it closed: an inline table member's own-line note went
through the hub and came back gone. The reason is a slot mismatch across three components, and each
of them was right on its own:

| hop | where the note lives | what that component does with it |
| :-- | :-- | :-- |
| TOML reader | the member key's leading list (`parser.rs`, then `lib.rs`) | the crate's own writer emits it (`writer.rs:preserves_nested_inline_table_interior_comment`) |
| hub's YAML leg | written inline after the key, `{a: # lead n` | correct for YAML: a note ending the key's line is the key's |
| YAML reader, back | the key's **inline** comment | correct again, by the same rule |
| TOML writer | - | read the key's *leading* list and the value's inline slot, so nothing matched and the text fell out |

Measured at the shipped boundary, both directions: `arr = [{

## lead n

  a = 1}]` came back as
`arr = [{a = 1}]` while the trailing-note twin came back intact - the asymmetry that made the shape
worth naming. TOML has no slot that reports a key's inline note, so the writer cannot echo the slot
it received; it writes the note on its own line above the member, which is where the TOML reader
reports a note from, and the emission settles in one hop: `p = {
 a = 1, # inner
 b = 2
}` now
returns `p = {
  a = 1,

## inner

  b = 2
}` and the second round is byte-identical.

Rust owns the slot rule and the idempotence
(`a_note_on_the_key_line_of_an_inline_member_is_written`), red-checked by withdrawing the emission
alone - the multi-line form stays and the note vanishes, so the test pins the placement rather than
the layout. Python owns the caller-visible promise: the case (dj) pinned as a loss is replaced by
two that pin the survival and the one-round fixed point.

### (dp) A gate that checks the split in one direction is a half gate (2026-10-15)

The `no-std-check` job builds five foundation crates for `thumbv7em-none-eabi`, runs their tests on
the host with `--no-default-features`, and then re-checks the default (std) configuration "so
re-enabling `std` must not have been achieved by deleting the no_std path" - but that last step
listed `pyrs-ast` and `pyrs-schema` only. `pyrs-event`, `pyrs-json` and `pyrs-toml` could stop
compiling with std and the leg would stay green, which is the same class of hole the job's own
comment says it is closing: a check whose coverage silently narrowed to a subset of what it names.

Measured first, because widening a job that cannot compile is not a fix: `cargo check --locked` over
all five crates with default features finished clean on this tree. The step now names the five, and
`tests/test_no_std_gate.py` reads each step's `-p <crate>` set out of the workflow text and fails if
the std re-check stops covering what the two no_std steps cover. The guard was seen to bite: with
the pre-fix step text restored, it fails naming the three crates, and the helper that decides has
its own negative case so the reading logic is exercised on a synthetic body too.

### (dq) The array scanner skipped comments and threw the text away (2026-10-15)

PR #434 closed the note that ends an inline-table member's key line; the same question asked of
arrays had a blunter answer. `skip_array_ws` matched `Some(b'#')` and called `take_comment()`,
discarding the result, and `TomlValue::Array` was a `Vec<TomlValue>` with no place to put it.
Measured at the shipped boundary before the change, all three positions lost their note:

| source | out of TOML, before | out of TOML, now |
| :-- | :-- | :-- |
| `arr = [` + own-line note + `1]` | `arr = [1]` | note on its own line above `1` |
| `arr = [` + `1 # trail` + `]` | `arr = [1]` | `arr = [` / `1 # trail` / `]` |
| `arr = [` + `1, # between` + `2]` | `arr = [1, 2]` | `# between` above `2` |
| `arr = [1, 2]` | `arr = [1, 2]` | unchanged |

An element now carries the same `KVAnnotations` a member does - leading notes on the element's own
lines, the note that ended its line as its trailing one - and the writer's array branch mirrors the
inline table's: a decorated array goes multi-line, and a note may only finish the last element's
line, because `1 # x,` is not TOML and a note written after the comma would be reported by the
reader as the next element's leading note, which is exactly where the writer therefore puts it.
Measured: the second emission is byte-identical for all three shapes, and the hub's YAML leg needs
no change (it already had a place for a sequence item's own-line note).

One position remains open and is pinned rather than hidden: a comment line between the last
element's comma and `]` has no element to own it and no line to end, so it is still dropped
(`test_a_note_after_the_last_comma_has_no_element_to_own_it`). Closing it means giving the array
node itself a trailing note slot, which is the AST-shape question #434's slot ruling keeps out of
scope.

Tiers: Rust owns the attribution and the fixed point
(`an_array_element_note_takes_its_own_line_and_settles`,
`an_array_without_notes_keeps_the_compact_form`); Python owns the caller-visible promise
(`tests/test_toml.py::TestArrayElementNotes`).

The ask is not free, and the instruction-count gate priced it. The first version asked each element
with `leading_comments()` and `comment()` - two view lookups, each dereferencing the boxed decor -
and `to_toml_medium` measured 28,923,748 against a floor of 28,771,649: +0.53% on a 0.50% tolerance,
with the scenario's binary digest differing from the baseline build, so the movement was code and
not host. Asking `has_notes` first - two inline discriminants, no deref - for the elements that have
nothing to report brought the same scenario to 28,897,722, +0.44%, inside the line;
`from_toml_medium` moved +0.29% for the element annotations the reader now builds, which the
tolerance absorbs. Two things follow and both are recorded rather than smoothed over: the branch's
outlining and its cheap filter landed in one measurement, so the gate prices the pair, not either
half; and 0.44% of a 0.50% band is little headroom, so the next writer-side fidelity ask on this
scenario is a reason to refresh the floor on the enforcing image, never to widen the line.

### (dr) A plain scalar's comma opened a quoted region, and the fourth remedy closes it

(2026-10-16)

PR #435's `msrv-check` leg reported a new residual (`job 116457142637`, run 38745242334):
`anchor at $."$,\""#0."<-&wa #s4qr/"#0: Some("_s9_smQyB6v-5_--__t__kMKg7NbiE3xq") -> Some("wa")`,
normalizations `{PlainRequoted, BlockRestyled, NoteRehomed}`. It was reproduced locally by
rebuilding the shrunk tree field by field, and the mechanism is one branch of `opens_quoted_node`:
the test "the byte before the quote is a comma, so the token after it is a node" is unconditional.
In *block* context a comma is plain-scalar text. The writer emits the plain key `$,"` as `$,":` -
legal, since `ns-plain-char` allows `"` anywhere but the first position, and granit reads that key
back correctly - and the region opened at its quote skews every quote state after it on the line:
the `#` inside the next member's quoted key becomes a comment opener, that member's own `&_s9_…` is
rejected as comment text, and `&wa`, sitting inside the key at a `-` boundary, is returned as the
name. No value changes; the anchor name does, which is what orphans every alias to it.

The shape is `$,": {&i "<-&wa #q/": &v "x"}` written out - one line without the writer's flow
continuation is not enough, because the recovery only runs for nodes granit gives an empty span at.

Three remedies were implemented and measured, and all three were thrown away. The first two read
only the text of the line; the third asks the parser:

1. Track flow depth along the line and let a comma separate only inside it. The reported shape is
   fixed - the scanner answers `v`, the tree settles - and `PROPTEST_CASES=20000` then fails three
   properties, e.g. `anchor at $.<Sequence>#0[key][0].<Null>#2[key]: Some("_--9G51e") ->
   Some("_1-i_zY___OF-2-8FS")`. A flow opened on an *earlier* line leaves no opener on the
   continuation line, where the comma genuinely separates members.
2. Also count a comma at the line's first non-blank byte as a separator, which is how the writer
   continues a flow. Still three failures at 20k, a different shape: `anchor at $.\"a\"#0.\" #\"#1:
   Some("lmd-eL_YfH_-x2A-yU__g") -> Some("F-3-_Ksef7_-qoj_5c_u")`.

Each run recorded shrunk inputs in `proptest-regressions/pbt.txt`; those seeds went back with the
experiments, because a seed that names code no longer in the tree is a claim nothing checks.

A third remedy was then implemented, the one that asks the parser instead of the text:
`anchor_name_before` gained a sibling taking `flow: Option<bool>`, `parser/mod.rs` added
`inside_flow()` reading `ParseState`'s `flow_style` off the top of its container stack, and a comma
counted as a separator only inside a flow - including one an earlier line opened, which is precisely
the case the line cannot see. The event-stream path, which tracks no container style, was left on
`None` and so on today's permissive reading, to keep the two readers from diverging quietly. It also
failed the tier: 20 000 cases produced `anchor at $[1][0]: Some("R") -> Some("I7r-zg__T-jm_-")` and
`anchor at $.<Mapping>#0[key]." #[["#1: Some("K_3N-6__hx_t-1G-_Nzn8-v1") -> None` - the second an
anchor *lost*, through a quoted key holding `#[`. Three text or context narrowings, three different
residuals; each one trades a shape.

What survives the attempts is the diagnosis and the shape of a fix that does not guess: bound the
search by spans. granit points the recovery at a node with an event span, so the name can only be
sought between the previous event's span end and this node's span start - the position-isolation
rule this file already states for the name *run*, applied to the window instead. That removes every
line-level question about quotes, commas and comments from the recovery path, and it is the next
attempt, not a fourth narrowing.

Nothing was pinned by those three, and that was deliberate: each reproduces the defect in one shape
and breaks it in another, so a test would freeze a half-fix. The counter-examples are recorded so
the next attempt starts from them.

The next attempt did, and the fourth remedy is the one that holds: stop asking the text.
`anchor_name_within` takes a `floor` - the end of the node the builder consumed most recently in
this container, which is a mapping's pending key, else its last pair's value, else a sequence's last
item - and the recovery scans left from `byte_start` only down to there, with the comment test
clamped to the same floor. Nothing left of the window is this node's text, so no reading of a comma,
a quote or a `#` is ever asked of it. Measured: 371 lib tests, the property tier clean at
`PROPTEST_CASES=20000` where each of the three narrowings left 2-3 residuals, and
`a_nodes_own_window_names_its_anchor_past_a_plain_key_comma` pins both halves - on that document the
text-only scan still answers `w`, the windowed one answers `v`.

What it leaves open is the second reader. `stream.rs` calls the text-only entry, because its builder
tracks no container style and so has no floor to pass: the event stream keeps today's reading while
the tree path does not, and the same skew can still rename an anchor for a caller of `parse_stream`.
Closing it means carrying the same floor there - the event spans are already in hand, only the
last-child-end tally is missing - and the unit pin above is the oracle it will be judged against.

The second reader carries it now, and by a cheaper route than the tree, because it has no
`ParseState` to read a tally off: `StreamReceiver::window_floor` is one monotone `usize` - the end
of the most recent *non-empty* span granit reported - and a node's own `&name` lies between that and
its own span start. Skipping empty spans is what makes one number enough, and it is a measured
property of the events rather than a trick: dumping spans shows both a block container's own start
and its `End` event are empty spans (`map_end 23..23` sitting between `k2: v2` and `next: 3`), so an
empty span names no text that really ended, and folding one would fence the next sibling out of its
own `&` - turning a rename into a lost anchor. The tree reaches the same bound by the other route:
greatest child end for a block container, closing bracket only for a flow.
`a_block_containers_end_never_fences_the_next_anchors` pins the overshoot by naming every anchor in
a two-line block document, `the_tree_and_the_events_name_the_same_anchors` asks twelve shapes of the
two readers together, and at the Python boundary `test_flow_anchors_are_named_from_their_own_node`
reads the reproducing document through both `parse_stream` and `parse`. Measured: the event test
answers `v`, and handed back an unbounded floor it answers `w` - the mutation was run, and under it
the table fails too.

The price is on the record because four shapes of it were measured. Per-event bookkeeping put
`stream_medium` at +1.76% over its floor against the 0.50% tolerance; folding the classification
into the match that already counts depth - the cheaper-looking form - measured *worse*, +2.08%,
refuting the idea that a second dispatch was the cost; and gating the tally on the text containing a
`&` byte, which the fixture's does not, still cost +1.39%, because that gate is a whole pass over
the text per parse. What passes is one field, one index comparison, no per-event classification and
no scan: +0.48%, a marginal pass that the next floor refresh should re-measure rather than trust.

A second defence was built, measured, and thrown away. An agreement *property* - draw a tree, emit
it, require the two readers to name the same nodes - passed the tier, and it passes the mutation
too: with the event floor handed back as unbounded it reported ok at `PROPTEST_CASES=20000`. No draw
in that run made the two readers part ways, so the property asserted nothing about the case it was
written for. That is the same luck the tree defect needed - the tier found it at 20 000 cases, and
each of three narrower readings left a *different* residual - and this file's own answer to that
pattern is the notation corpus: ask the shape by name.
`the_tree_and_the_events_name_the_same_anchors` is a table for exactly that reason, and the mutation
above is what makes it a measured claim rather than a hunch.

### (ds) A container's note parked on a descendant's line costs the emission a round

(2026-10-16)

A 240-second local `cargo fuzz run yaml_roundtrip` (WSL, nightly, target dir on the Linux
filesystem) reached `<<: #*&

## &&*

 ..f!ny#*&

## &:`, which `cargo fuzz tmin` minimized to 14

bytes as `: #*&

##

 .

## &`. Both fail one assertion: `to_yaml` output must be a fixed point

Neither loses data - the emission converges on its second round and is stable from there - and that
is why no other tier sees it. The property tier compares tree to tree across one cycle, and
`note_survival` asks whether a note appears, not whether it stays put.

The mechanism is an ownership handoff the writer performs on itself. `write_pair` asks whether the
*value* owns a note before letting the key's note ride the pair line ("one line, one trailing
slot"). In the first round the value owns nothing - the document holds the trailing `# &` - so the
key's note takes the pair line and the writer parks the document's note on the body's last line. But
granit spans a node through the break that ends its own line, so the re-read reports that note as
the value's, and the same question now answers the other way: the second emission hoists the key's
note above the pair.

Two remedies were implemented and both were measured away.

1. Ask about the line instead of the value: the key's note rides the pair line whenever the value's
   body starts below it (`key_note_at.is_some()`). Both pinned shapes settle in one round, and at
   `PROPTEST_CASES=20000` six properties failed on one shape each time - a plain `<<` key carrying a
   comment - reporting `value at $."<<"#0[key]: "<<" -> "null"` and
   `entry count at $."<<"#0: 1 -> 3`: riding a merge key's line stops the reader treating
   it as a merge. Narrowed to spare a pending tag-only line as well (`pending_line_is_last()`), the
   same six failures remained.
2. Repair the handoff instead: park a container's note only on a tail line that starts at that
   container's own column. The same six properties failed, because the note written as a line of its
   own below the block is read back as the leading note of the node that ended it - crash-11ced252's
   finding, restated for a different reason.

The third remedy holds, and it is narrower than the question implied. `write_pair` still asks the
value - but only where the value cannot be the one using the slot: a scalar or null whose body
starts below (a leading note is what forces that break) leaves the pair line's trailing slot free,
so the key's note rides it in the first round and the second parse reports the same tree. Both
measured exclusions live in the predicate, not in a comment:
`matches!(value, Scalar { .. } | Null { .. })`, because a container written below turns
`<<:  # note` over a flow body into a null merge key, and `!pending_line_is_last()`, because a
tag-only line is still open. Measured: 632 Rust tests, 26 property tests at 20 000 cases, and the
fuzz harness green over both inputs - which is what let them join `fuzz/seeds/` beside the other
former crashes. The test that had recorded the drift,
`crates/pyrs-yaml-core/tests/settles_in_one_round.rs`, now requires the fixed point, and shorting
the new arm out by hand turns it red with the two-round text.

### (dt) A note-homing line test that could not see a CR

`!-\r\r\r- :\r\r\r\r- #-` - the CR-only spelling of a document the fuzz tier had already settled in
one round with `\n` breaks - reached its fixed point only on the second emission. Round one wrote
`!- \n- \n  ~: ~  # -\n- ~\n` and round two `- ~: ~  # -`, the compact dash form: the same note,
homed by a different rule, because the two parses disagreed about who owns it.

The disagreement is not in the writer. (ds) left `write_pair` asking the line's slot, and it answers
correctly for the tree it is handed; what differed was the tree.
`AstReceiver::attach_inline_comment` decides whether a `Placement::Right` note trails the node
before it or annotates the node after it by calling `line_break_between`, which scanned the gap for
`b'\n'` alone. A document broken only by `\r` therefore read as one long line, every note after a
break bound backwards onto the previous container, and the container's note went through the (ds)
route - written inline, re-read as something else's - which is exactly the two-round shape (ds) had
just closed from the other side. The blank trim two lines above already treats `\r` as blank, and
`scalar.rs` and the comment scanner already accept `\r` as a break, so this was the one place in the
note-homing path that did not.

The predicate now asks for either break, which is the vocabulary granit splits lines on. Measured as
an A/B over the three shapes in this family - `former-crash-7eb273bc.seed`,
`former-crash-9733643a.seed` and `:\t!-\r... #-o\n`, both spellings of each - every emission is
byte-identical before and after; only the ingest slot moved, from the container's inline
`meta.comment` to its `leading_comments`, which is what the `\n` spelling always produced. Two pins
recorded the CR-blind slot and were updated to the common one:
`a_containers_inline_note_after_a_text_less_value_settles_at_once` (its name stays, because the
ledger and `serializer.rs` cite it as the history of the inline reading) now also asserts that a
document and its LF spelling emit the same text, and
`a_note_above_a_pending_tag_line_keeps_its_owner` asserts the leading slot on both seeds. The
routing that (ds) shipped for a container note held inline is untouched and still serves the shapes
that genuinely arrive that way.

Two tiers cannot see this class, which is why the defence is a seed and a named test. The property
tier draws trees and the writer emits `\n`, so a CR-only byte sequence is unreachable there -
`pbt.rs` holds no `\r` at all, which is the check behind that claim; `note_survival` asks only
whether a note appears, and a migrated note appears. The fuzz harness is the tier that reads raw
bytes, so the input is archived as
`fuzz/seeds/yaml_roundtrip/cr-only-break-note-ownership.seed` and replayed by the PR gate, and
`crates/pyrs-yaml-core/tests/settles_in_one_round.rs` names it -
`a_note_after_a_cr_only_break_annotates_the_item_that_follows`, which also counts the notes so a
copy cannot pass as a move and requires the CR and LF spellings of the document to emit one text.
`tests/test_gaps.py::TestUnicodeAndSpecial::test_cr_only_line_endings` asks the same question
through the public API.

Shorting the arm out by hand (`gap.contains(&b'\n')`) reddens exactly three tests and leaves 630
green: the new pin, reporting the round-two text, and the two pins updated above, on the slot they
now share with the `\n` spelling. Gates on the change: 633 Rust tests, 26 property tests at
20 000 cases, 2790 Python tests on a fresh build, clippy across all targets, `prek run --all-files`,
and the harness replaying all 74 seeds with `-runs=0` - 5529 inputs counting the local machine
corpus, zero crashes.

### (du) The declared version arrived and was dropped one line from the source of truth

`YamlDocument.version()` answered `1.2` for every document. Measured before the change:
`%YAML 1.1`, `%YAML 1.2`, `%YAML 1.3` and a document carrying no directive all reported `1.2`, and
`parse_all_docs` reported it per document regardless of what each header said.

The information was not missing from the stream: granit 2.0 hands both the explicit-marker flag and
the declared version in `Event::DocumentStart(bool, Option<YamlVersion>)`. The tree builder matched
that event with `(..)` and used one arm only to clear a flag, and the event path mapped it to a bare
`StreamEventType::DocumentStart`. The reader therefore knew the version and threw it away one line
from the source of truth, leaving the layer above free to answer with its own.

Reporting is the scope, and the shape was ruled rather than assumed: `-> str` stays, because
widening to `Optional[str]` to spell "unset" would break every caller comparing to a shipped string,
so a document that declares nothing reports this reader's own version; and the writer does not
re-emit the directive, because carrying it through emission needs a document-level slot in the AST,
which is a structure change and was declined for now. `parser::parse_document` and
`parser::parse_all_documents` return `ParsedDocument { node, version }` beside each other - per
document, since a stream may declare a different version in each header - and `parse_with_options` /
`parse_all_with_options` are now thin wrappers over the same internals, so no existing caller's
semantics moved.

What the change deliberately does *not* mean: `%YAML 1.1` does not select the 1.1 resolver. The
schema profile does (`schema="1.1"`, the legacy-boolean lexemes and their tests), and a directive is
not a feature gate - pinned so (`test_a_directive_does_not_change_resolution`: `flag: yes` stays the
string under the Core schema however the header declares). Version-driven resolution therefore stays
an open item, and so does the emission half.

The `%TAG` half of the same question is a different mechanism and it already works: granit resolves
the handle during the parse, so `%TAG !u! tag:yaml.org,2002:` plus `!u!str x` reads as the standard
string tag and emits `u: !!str x`. Measured, and pinned as
`test_a_tag_handle_is_expanded_not_carried`: the tag's meaning survives, the local spelling of the
handle does not, because the directive line is not re-emitted either. That is the same declined
scope seen from the tag side, not a silent loss.

No tier could see the old answer: `tests/` held no `%YAML` fixture at all, the compliance suite asks
how scalars resolve rather than what the document reports about itself, and the fuzz harness is
indifferent by construction - the writer never emits a directive, so no input it round-trips carries
one. The defence is now the three core tests (`a_declared_version_is_reported_as_declared`,
`each_document_of_a_stream_reports_its_own_version`,
`a_declared_version_survives_a_document_with_no_content` - the last because a comment-only document
never fires `DocumentEnd`, which is where the per-document pairing happens) and the Python class
`tests/test_parse.py::TestYamlDirective`, which covers declaration, the undeclared default,
per-document answers, `reparse()`, the resolution boundary and the non-emission boundary.

Measured on the change: the declared version reports itself for all three declarations; a stream
declares per document (`1.1` then `1.2` across an explicit `...`); `to_yaml()` of a declared
document emits `a: 1\n` and reads back as `1.2`. Reverting the one line that records the directive -
back to dropping the `Event::DocumentStart` argument - reddens exactly those three core tests and
leaves 633 green, which is the check that the new tests measure the reporting and not the refactor.
Gates: 636 Rust tests, 26 property tests at 20 000 cases, 2799 Python tests on a fresh build,
clippy across all targets, `prek run --all-files`, and the stub route - the regenerated
`python/pyrs_yaml/pyrs_yaml.pyi` differs only in the `version()` docstring, which is what
`check_stub_drift.py` then reconciles.

### (dv) A dependency bump got a path to `main`, and the boundary where it stops having one

`.github/dependabot.yml` tracked one ecosystem, `github-actions`. Cargo and uv updates - the two
places this project pins versions that actually compile into the shipped artifact - were untracked,
and nothing at all said what happens to a bot pull request once CI is done with it.

Two facts shaped the policy more than any preference. First, a lockfile-only bump could not go
green: `scripts/check_changelog_coupling.py` names `Cargo.lock` and `uv.lock` as triggers, so a
resolution move with no release note is a violation by its own rule, and the script's header already
records what happens when the gate blocks dependabot for no reason - "a gate that blocks dependabot
gets bypassed". The fix is not an exception written in prose but rule 3, `--allow-lockfile-bumps`,
granted by CI on `github.actor == 'dependabot[bot]'` only: the commit hook stays strict, and a human
touching the same file still owes the note.

Second, the merge has to be gated by the check that can already block a human. A pull-request-scoped
workflow cannot merge the pull request whose required checks it is, so
`.github/workflows/deps-automerge.yml` triggers on `workflow_run` for the `CI` workflow and calls
`scripts/dependency_merge_gate.py`, which re-reads the conclusion of `Test matrix (all legs)` for
that head commit rather than trusting a hand-passed verdict: `skipped`, `cancelled` and
never-reported all read as refusals, the same reading `scripts/check_matrix_verdict.py` enforces for
the fan-in job.

The boundary is a resolution against a range. `Cargo.lock`, `uv.lock`, `fuzz/Cargo.lock`,
`prek.toml` and the action pins merge themselves when the verdict is green. `Cargo.toml`,
`pyproject.toml`, `rust-toolchain*` and the committed `pyrs_yaml.pyi` do not, because moving a
declared range carries obligations a bot cannot discharge: the maturin lock travels with
`scripts/check_stub_drift.py`'s reconciliation rules and a regenerated stub, a granit-parser major
travels with a measured migration of spans and comment placement, and pyo3 >= 0.30 travels with the
interpreter floor - the shipped wheels are `abi3-py38`, so its updates arrive under a `python-floor`
label the gate refuses, and a green check cannot retire Python 3.8 as a side effect of somebody
else's release. Nothing uses `--admin`: with `enforce_admins` on, that is the one road an unverified
dependency can take onto `main`.

No undocumented key is used, and the reason is the same as the reason for the verdict rule: a value
the hosted schema rejects does not fail, it stops opening pull requests for that ecosystem, so the
`versioning-strategy` that would express "resolve, do not widen" is left out and the boundary lives
in the allowlist, where a test can read it.

Measured on the change: rule 1 reddens `["Cargo.lock"]` without the flag and clears it with the flag
(`tests/test_changelog_coupling_gate.py`, rule 3, 42 cases);
`scripts/dependency_merge_gate.py --selftest` reports 15/15 fixtures as specified; the two lists -
the exemption's and the allowlist's - are asserted to name the same files, and the
`Test matrix (all legs)` string is asserted to be the same in `ci.yml`, the verdict script, the gate
and the workflow, since a rename in one of them silently un-gates the others. 78 tests across the
two files, plus the repository's Python tier.

Not verified here, and deliberately not claimed: the hosted bot's own behaviour. Its first run is
next Monday, the config is validated only by using documented keys and by Dependabot's own UI, the
security-update toggle is a repository setting outside this file, and PR #438 remains the human path
for the pyo3 floor until that decision is made.

### (dw) The event vocabulary got its second producer, and it said what a JSON note cannot say

`pyrs-event` has existed since PR #379 with exactly one producer - the YAML adapter - which is the
reason `docs/dev/event-stream.md` refused to declare the vocabulary neutral until a second format
had tried to fit it. Step 2 was attempted on 2026-10-13 in the shape of one traversal with two
outputs and shelved when the gate measured +0.61…+1.05% on the three JSON read scenarios, with the
control row (+1.60% with every emit call deleted) proving the price was threading a sink through the
tree walk rather than producing events. What ships here is the shape that ruling asked for: a walk
of its own.

`pyrs_json::parse_stream` and `parse_stream_with_options` are the new surface. They reuse the
scanners the AST route already trusts - `string`, `single_quoted_string`, `identifier_key`,
`expect`, `ws`/`ws_extended`, the error helper - and build nothing else, so the tree traversal is
untouched by the sink question. Three extractions keep one authored rule where two walks now need
it: `number_source` is the grammar and reports the source slice, with `number` as the tree's wrapper
(it allocates one fewer `String` per number than before), `hub_scalar` / `non_finite_spelling` holds
the `.inf` / `-.inf` / `.nan` projection for both routes, and `single_quoted_node` is the JSON5
style arm shared with the replay. `parser_from` builds the scanner so a dialect flag cannot reach
one entry point and not the other.

The second producer answered the questions the design doc listed, all of them by measurement rather
than by argument. `anchor` and `tag` stay `None` and `Alias` never appears - a test walks a JSON5
document with every dialect axis on to check it. `style` is the *source* spelling and the hub's
storage style is a resolution the reader applies: a JSON5 `'k'` key is `SingleQuoted` in the stream
and plain in the tree, which is the closed attempt's "style cannot be assumed per-position" finding
with the mechanism named. A non-finite token arrives verbatim; the projection lives with the
resolver. Positions are the producer's to carry - each event gets a 0-indexed line and a char column
from one line table per document, computed the way `err` computes a diagnostic, which is the first
finding made concrete: a stream knows where things are and the tree does not.

The one thing the vocabulary cannot say is named instead of patched. `{"a": 1 /* x */, "b": 2}` and
`{"a": 1, /* x */ "b": 2}` produce identical event shapes - value, note, next name - while the
engine files the note on the value in the first and on the following key in the second, deciding by
which side of the comma the note sat. Neither the note's position nor its `standalone` flag records
that: a comma is not an event. So the replay is exact for comment-free documents, and once notes
exist it is exact about every note's text (none invented, none dropped) but not about the slot. The
exit criterion for step 3 and step 4 is now concrete: either the comment payload carries "the
separator was crossed", or the stream reports the separator.

A belief went with it. `NodeMeta`'s equality doc claimed `CustomNode`'s structural equality ignores
decorations entirely; the replay reddened on two trees differing only by a `comment` slot, and the
impl does compare that slot plus anchor and tag while leaving out the leading-note list and
`source_range`. The sentence now describes the code.

Measured on this change: `cargo clippy --all --all-targets -- -D warnings` clean, `cargo nextest run
--all` 645 passed (70 in `pyrs-json`, nine of them new), `cargo check -p pyrs-json
--no-default-features` clean with `pyrs-event` added as a `no_std` edge. The refusal table is 32
inputs across the three dialects and each must yield the identical `ParseError` from both walks; the
seed corpus (`fuzz/seeds/json_roundtrip`, 14 files × 3 dialects) replays 33 accepted documents and
refuses the rest twice over with the same diagnostic.

Not verified here, and not claimed: the instruction-count gate was not run (this host has no
valgrind), so `from_json_*`'s movement from the `number_source` split is CI's
`Instruction-count baseline` leg to judge. There is no public `from_events` - the fold lives in the
test tier, and publishing it before TOML answers the same questions would freeze one producer's
assumptions into the contract. No fuzz target spans the two walks yet; the seed corpus and the
refusal table are the input such a target would use, and it is the next defence, not this one. The
Python-facing surface, the stub and the four doc locales remain the last step of plan B and were not
touched.

### (dx) TOML's producer reads the IR rather than re-scanning, and the vocabulary's fields come back answered

Step 3 of `docs/dev/event-stream.md` was written down as "a TOML producer, same question again",
with the shape problem stated as a guess: dotted keys, numeric spelling, comment slots.
`pyrs_toml::parse_stream` now answers it, and the answer on the *shape* went the other way from
JSON's on purpose.

The maintainer ruling (2026-10-16, decision 5 in that document) chose spans recorded in the IR and
projected from it over a second scanning walk, with the instruction-count gate as the arbiter of the
price. The reason is the one this ledger has kept charging against: `register_header` /
`register_kv` author TOML's table policy once - which header may open which path, which dotted key
closed a table against re-opening, where an array of tables gains a tail - and a second pass that
had to reach the same refusals would author it twice, with a replay test catching the drift only on
inputs it happens to touch. So the scanner runs once, one verdict stands for both readers, and the
accept/reject table (26 malformed documents × 2 grammar revisions) plus the seed corpus (16 files ×
2 dialects) ask the question anyway rather than being trusted to be redundant.

The plumbing is the cost of that choice, stated exactly: `Note { text, at }` replaces a bare
`String` per comment, `KVAnnotations` gained `key_at` and `value_at`, `CowTable` gained `at` and
`key_at`, and `parse_key_path` reports each segment's offset because a header line names a whole
path and a stream that reported only the line's start could not keep its events in source order. One
function, `lib.rs::scalar_projection`, now decides a scalar's text, style and tag for both readers -
TOML's twin of `hub_scalar` - and `toml_value_to_node` builds its nodes from it, so the refactor is
also the reason no scalar rule exists twice. `LineTable` moved into `pyrs-event` when a second
producer needed the same byte-offset-to-line/column conversion; `pyrs-json` now uses it too, and
three tests cover it including the multi-byte column and an offset inside a code point.

What the arbiter then said, and what it was actually pricing (same day, run 38840810436): CI's
`Instruction-count baseline` leg reported `from_toml_medium` at +1.48% against the 0.50% tolerance,
0.000% spread over its three samples, and decision 5 names the fallback - drop the spans rather
than widen the floor. Before dropping anything, the spans were measured on one host: `main`
159,941,362 against this shape's 161,987,153 (+2,045,791, +1.28%), while `to_toml_medium` moved
+0.013% and `from_json_medium`, which TOML's IR cannot reach, +0.035%. Building the document was
the taxed part - not carrying a wider record, and not reading it back. Two hypotheses then died one
measurement each. Narrowing the spans to `u32` with `Arc<str>` note text, which returns both records
to `main`'s size, read 166,341,862: +4.0%, worse. And `ir_gate.py --profile from_toml_medium` shows
the map insertion rows unmoved between `main` and this change while
`__memcpy_avx_unaligned_erms` carries +2,701,897 of the +2,045,791 total. The function under that
copy is the answer: `scalar_projection` borrowed `&TomlValue` and copied each scalar's text, and the
tree route reads *every* scalar through it, so a value the tree already owned was cloned once per
scalar. The projection takes the value by value now - `toml_value_to_node` moves the text exactly as
it did before this producer existed, and only the producer, which has to hand out owned text anyway,
clones - reading 158,750,613 on the same host, 0.74% below `main`. No span was dropped and no floor
was moved: the gate caught a copy the unification should never have introduced. Whether the shape
lands under the recorded baseline is CI's leg to judge on the image that baseline names; a WSL
reading cannot claim that, and this entry does not.

What a second format settled in the vocabulary:

- `tag` is not YAML's field. A date-time arrives as `!timestamp` / `!date` / `!time` and a binary
  integer as `!!int` - the same statements the tree carries - so the field means "the type the
  document states" and a second producer fills it without anything being renamed.
- `anchor` is empty and `Alias` is never emitted, measured twice now.
- `style` reports what the engine kept, not what the document wrote: a literal `'…'` and a basic
  `"…"` both arrive `DoubleQuoted` and `_`-grouped digits arrive canonicalised, because the IR does
  not store the quote kind. JSON's stream does keep its own spelling. A field is therefore as
  informative as each engine's memory, which is the honest price of not re-scanning.

Two positions exist only in the stream, and both say something about TOML rather than about the
vocabulary: a close event carries the offset of the construct it closes (TOML wrote no end token -
where a table ends is decided by the next header), and a header names its table inside the brackets
that open it, so a table's key and its `MappingStart` are in structural order rather than byte
order. The array-of-tables case settled the same way: a repeated header line names one sequence, so
the key is reported once before the `SequenceStart`, matching both the tree and the shape YAML's own
events give for a key whose value is a sequence of mappings. The first draft of the producer emitted
the name per element and got a shape no reader could line up with the tree - caught by the corpus
test, not by reading.

One AST defect the comparison exposed rather than caused: an own-line note that meets no key or
header has no slot in the tree, and `from_toml` discards it (the writer emits no such text), while
the document wrote it. The stream reports it.
`a_note_the_tree_has_no_slot_for_is_still_in_the_stream` pins that, together with the comment-only
document where the tree does keep the note.

Measured on this change: `cargo clippy --all --all-targets -- -D warnings` clean,
`cargo nextest run --all` 655 passed (10 new), `cargo test --all --no-default-features` green,
`cargo check --workspace --no-default-features` green. The instruction count was said here to be
beyond this host, and that was wrong in both directions: WSL can run `ir_gate.py` (valgrind plus a
`libpython3.12.so` the host's linker can find), and CI ran it whether or not this host could - the
paragraph above records what both said, and the span plumbing's price turned out to be zero once
the projection stopped cloning scalars. No public `pyrs_toml::from_events` exists: a fold would
rebuild a tree that both readers already share an answer for, unlike JSON's, whose fold is the only
place the two walks can be compared.

### (dy) A fuzz gate found a note the JSON tree threw away with a duplicate key, and what it cost to fix

`fuzz/fuzz_targets/json_events_replay.rs` asks the tree route and the event route of one JSON
document the same questions. Its first 90 seconds of exploring produced
`{\n "a": 0,\n //{\n "a": 0,\n // "*  "* e\n}`: both walks accepted it, the stream reported two
notes and the tree kept one. The lost note rode the second `"a"`'s key node, and a repeated name
collapses onto the first pair - so the incoming key and the value it replaced are both dropped, and
anything they owned used to be dropped with them. Nothing in any earlier tier could see it: the
writer forgets comment slots in strict JSON, so the fixed point held while text went missing from
JSONC and JSON5 output.

The rescue is `object()` handing the collapsed pair's notes to the pair that survived, and the shape
took four measurements to settle because the read gate prices what this host does not show
faithfully: five variants that execute near-identical instruction counts on the measured fixture
read between +0.26% and +0.85% on `from_json_medium`, same binary settings, 0.000% spread within
each run - the spread is code layout, not work. What was kept is the variant that does no work at
all unless a name actually repeats: `attach_pending` now borrows its list, so the parser still holds
the pending notes across `insert_full`, and a duplicate replays that one rule over a scratch node to
recover what the dead key owned. `from_jsonc_medium`, the scenario that carries notes, moved +0.48%
on this host; `from_json_medium` moved inside the noise band and CI's leg is the arbiter, which last
read that scenario 1.40% below its recorded floor.

Two rules this followed rather than reinvented: the placement logic is replayed, never restated, so
the #361 two-same-line-notes case and the leading-versus-inline split cannot drift from the parser;
and the regression test was mutation-checked before it was trusted - with the migration removed it
fails, with the migration in place it and all 70 peers pass
(`a_duplicate_key_keeps_the_notes_its_own_nodes_carried` in Rust,
`test_duplicate_jsonc_key_keeps_both_notes` through the Python surface). The named residual: a
rescued note reads beside the survivor, not where the input put it, because one slot cannot
say "this note came from the pair that was collapsed away". `docs/dev/event-stream.md` carries the
question as part of what the replay cannot express.

### (dz) The replay became a gate, and its first ninety seconds of exploring found a lost note

Step 4 of `docs/dev/event-stream.md` was "the replay as a gate rather than a test", and the word
that carried the weight was *gate*: a fuzz target alone is a sampler, and this repository has
already ruled that a sampler cannot block a merge. So `pyrs_json::from_events` left the test tier
and became a public reader that refuses the lists no producer of this crate can emit - eight shapes,
each with a diagnostic - and the same two questions are asked twice: once by
`fuzz/fuzz_targets/json_events_replay.rs` over whatever libFuzzer evolves, once by
`crates/pyrs-json/tests/events_replay.rs` over the 13-file corpus in `fuzz/seeds/json_events_replay`
on every `cargo nextest`. The corpus test measures itself rather than trusting its own existence: 39
verdict pairs driven and 22 documents replayed, with a floor under both counts, because a corpus of
nothing but shared refusals would drive pairs while testing the replay not at all.

The target compiled only after a real error was fixed (`match` arms must return the same type, and
an `assert_eq!` arm returns `()`), which is worth stating because a fuzz target that does not
compile rots silently - that is the reason `fuzz.yml` builds on every push to `fuzz/**`. Within
ninety seconds of a local exploration window it produced `crash-dup-key-after-line-note.seed`: the
tree and the stream agreed on the verdict and disagreed on the note count, and the reason was that a
duplicate key collapses onto the first pair, dropping the incoming key node and the value it
replaced along with any note either owned. Fixed in #450, seeded here, mutation-checked there.

The gate also turned up a limit rather than a defect, and it is recorded in both places now:
`Comment::standalone` answers which slot a note rides in the tree (the #361 migration marks the
notes it moves as standalone) and whether the source gave the note a line in the stream. One
boolean, two questions, so every two-walk comparison matches note text and leaves the flag out -
pinned by `the_two_walks_answer_different_questions_with_standalone`, which is the test that makes a
future unification visible instead of silent.

Not measured: the discovery budget is one 90-second window on this host, and the CI tier that
explores runs on the schedule, so the coverage claimed here is the corpus plus that window, not a
search. Nothing in this change touches the read path, so the instruction-count leg has no new claim
to judge.

### (ea) The Python-facing verb reads every format, and what the boundary can now be asked to prove

Step 5 of `docs/dev/event-stream.md` is the smallest of the five, and that is the point:
`parse_stream` already emitted the vocabulary's dicts for YAML, and the two newer producers emit the
very same `pyrs_event::StreamEvent`, so shipping the promise to Python was a keyword that picks a
scanner — `parse_stream(text, on_event=None, max_depth=1000, *, format="yaml")` over `yaml`, `json`,
`jsonc`, `json5`, `toml` — not a second surface to keep honest. The default is unchanged and a test
says so event for event, positional form included, because a widened signature is a compatibility
event whether or not it looks like one.

What the boundary reports is the vocabulary's own honesty, now visible to users: `anchor` and
`alias` stay YAML's; a TOML literal `'…'` reads the same as a basic `"…"` because which quote was
typed is not kept; a TOML document's tags arrive spelled the way the hub spells them (`!timestamp`,
`!!int`) rather than the way YAML would. One claim was written wrong and then measured before it
shipped: the first draft of the docstring said a TOML string reads `plain`, and the probe answered
`double_quoted` for both quote forms — the same pass also showed that a bare key named `y` is quoted
on purpose, because YAML's reader would type it, and that the tree says the same thing, so the
stream is faithful rather than surprising. The docstring now states what was measured.

`max_depth` bounds only the two readers that carry the option, and the two answer with different
types (`YamlMaxDepthError` for YAML, a parse error for JSON); TOML's nesting limit belongs to the
intermediate representation. That asymmetry is documented at the verb rather than smoothed over,
because inventing a TOML depth option would be a configurability the scanner does not have.

Measured: `cargo clippy --all --all-targets -- -D warnings` clean; `cargo nextest run --all` 659
passed; `uv run pytest tests/` 2853 passed, plus 15 new tests in `tests/test_streaming.py`'s
`TestMultiFormatStream`; the routing is mutation-checked — replacing the JSON5 arm with strict
reddened exactly `test_json5_reports_single_quoted_and_bare_forms_verbatim` and
`test_the_dialects_stay_their_own_grammars`, then was reverted. The stub came from the regeneration
route and `scripts/check_stub_drift.py` reports no drift. Not measured: the instruction count, which
this host cannot take without valgrind; the added work is a match over producers outside the
measured hot path, so CI's leg is the arbiter.

### (eb) The vocabulary crate is measured by itself, not only through the producers that use it

`pyrs-event` holds the type every producer reports, and it had two tests — both about `LineTable`.
That left the part the crate itself owns, the eleven-arm `into_owned` match, gated only by whichever
variants the JSON and TOML producers happened to emit at the time: an arm that maps to a neighbour,
or rebuilds a variant without one field, compiles, and a replay test only notices if it happens to
walk that variant. Three tests now build one event per variant in the crate and ask the three
questions the producers cannot answer for it.

Cloning and owning leave every field and both positions alone — the equality is over the whole
`StreamEvent`, so a dropped `standalone`, `anchor` or `tag` fails by naming the variant it came
from. A scalar borrowed from the source and the same text owned compare equal: `Cow` is the decision
3 ruling, and this is the sentence that makes the two-walk replays meaningful, because a borrowed
and an owned form that disagreed would let a passing replay say nothing. Owning a comment moves the
shared `Arc<str>` rather than copying it — the allocation claim is now a pointer identity, and a
change that quietly copies here fails that test alone.

Attribution was checked by mutating, not by reading: dropping `standalone` in the Comment arm
reddens exactly `owning_a_stream_keeps_every_field_of_every_variant`, and replacing the moved `Arc`
with `Arc::from(&*text)` reddens exactly
`owning_a_comment_moves_the_shared_text_rather_than_copying_it`. Both were reverted.
`cargo nextest run --all` 662 passed (3 new). Test-only: no read path is touched, so the
instruction-count leg has no new claim to judge, and this entry sits after (ea) once PR #452 lands.

The crate's own header still said "the producers do not exist yet" and listed steps 2-4 as future
work — true when step A merged, false now. It is rewritten to say what the two producers actually
answered (`anchor` and `Alias` stayed YAML's own, `tag` generalised to TOML's `!timestamp` and
`!!int`, `style` reports whatever the engine remembers, close-event offsets and structural order
belong to the stream) and what remains open (the one-boolean-two-questions note flag).
`cargo doc -p pyrs-event` builds, which checks the rewrite is at least readable.

### (ec) Decision 6: the note flag becomes two facts, and the design goes up for review before any code

The last limit the event-stream sequence kept naming is now ruled on: `Comment::standalone` answers
*which slot does this note ride* in the tree and *did the source give it a line* in the stream, and
one boolean cannot answer both. The maintainer's ruling (2026-10-16) is to split it into two fields
and to put the shape up for design review first, declining the cheaper alternative of keeping one
flag and registering the loss as an accepted hole. `docs/dev/event-stream.md` carries the review
as "The note flag split".

What was measured rather than argued is the event order a producer emits, because that is what
decides the shape. Read off the shipped boundary: `{"a" /* k */: /* v */ 1}` reports `scalar a`,
`comment k`, `comment v`, `scalar 1` — so a note between a key and its value is already readable
from position; `{"a": 1 /* after */, "b": 2}` reports `comment after` between the value and the next
key, and there position cannot decide, because the comma that closes the pair is not an event and
the engine's choice is a policy the consumer cannot see. Hence the proposal: keep `owns_line` as the
meaning the stream already reports and add an explicit owner answer, so a stream can rebuild
decorations without every consumer re-deriving a rule.

The boundary of the change is the point worth keeping: `pyrs-ast::Comment` does not move, because
the tree already *is* the placement — the information is currently compressed into one boolean at
the vocabulary and its producers, which is where it is recomputed. That makes this a `pyrs-event`
shape change (breaking for hand-constructed variants, so a minor bump and a changelog line in those
words) rather than the AST change `AGENTS.md` asks first about. Four acceptance criteria are written
down with it: the note-text-only exclusion disappears from both replay tiers, the two walks agree on
both facts, a fold demonstrating rebuildability lands in the test tier, and the cost is read from
the gate on `from_jsonc_medium`, `to_toml_medium` and `stream_medium` rather than asserted.

Not measured: nothing. This entry is a design ruling and a probe of the shipped surface; no code
path changed, so the instruction-count leg has nothing to judge until the implementation PR exists.

### (ed) The vocabulary's two generic jobs are now tested over arbitrary input, not over hand-picked cases

`pyrs-event` had a unit tier (#453) and two producer-side replay gates, but the two things it does
for *every* document — `LineTable::line_col` turning a byte offset into a position, and `into_owned`
moving an event out of the source it borrows — were pinned by chosen offsets and chosen variants. A
property tier now states them generically in the crate itself: arbitrary documents of up to five
lines of multi-width characters must answer each character's offset with that character's own
`(line, column)`, must collapse every byte inside a wide character onto its start, must be monotone
in byte order, and must clamp past the end to one answer rather than panicking; and for seven
representative variants with arbitrary text, style, anchor, note flag and positions, `into_owned`
must change nothing but the text's ownership, must be idempotent, and must preserve both
coordinates.

The tier earns its place by what mutation found. Replacing the character-boundary clamp with the
identity panics the new property at the slice it should never reach — `text[start..cut]` off a
boundary — and the hand-written table test joins it, so both tiers guard the same class. Setting
`line` to zero inside `into_owned` reddens `owning_an_event_changes_ownership_and_nothing_else`.
Both mutations were reverted, and `proptest`'s `proptest-regressions/lib.txt` was deleted after each
run rather than committed: it records the seed of a falsified build, which is not a case worth
pinning.

Mechanics worth keeping: `proptest = "1.5"` was already a workspace dependency, so this adds a
`pyrs-event → proptest` lock edge rather than a new dependency, and CI's `property tier (20k cases)`
leg runs `PROPTEST_CASES=20000 cargo test --workspace --locked`, which elevates these two properties
with no case count hard-coded. `#![no_std]` costs one line — the test module names
`proptest::prelude::*` and the crate's tests reach `std` the way `pyrs-ast`'s already do. Measured:
`cargo nextest run --all` 664 passed (2 new), clippy clean at CI's scope, and the id `(ec)` belongs
to PR #454's decision record — these letters mark the order the work was done, not the order it
landed.

### (ee) A percentage gate describes the worst leg; a named gate describes the truth

Three TOML defects landed in one session (PRs #465, #466, #467) and all three had been invisible for
the same reason: the tier that should have seen them was a *percentage*. Not a careless one — both
floors had been re-set one case below the measured value, which is exactly the discipline this
ledger asked for after the `0.85` era. The defect is that a percentage, however honestly measured,
answers a different question than "does this document get what TOML says about it".

Measured on the corpus CI clones. The decode tier stood at 262/266 with a 0.984 floor and the four
gaps were one cause: a table was *identified by the text its segments join to*, so `[a."b.c"]` — one
key whose name contains a dot — refused as a redefinition of `[a.b.c]`, and every element of
`[[a.b]]` shared the first element's identity (PR #465). The reject tier stood at 491/511 on CPython
3.8-3.13 but 488/511 on 3.14 and 3.15, with the floor pinned to the worst leg, because three
out-of-range times were refused by the host's `datetime` constructor rather than by this parser and
a newer interpreter accepts the spelling (PR #466). PR #467 then closed the last three
accepted-invalid files — `invalid/string/multiline-quotes-01` (a run of six quotes before the
closer) and `literal-multiline-quotes-01`/`-02` — and corrected the tier's own inventory, which had
been counting a fourth: a backslash-x byte escape is dialect-gated (`from_toml_v1_0` refuses it, and
`v1_0_rejects_escape_x_and_e` pins that), not unpolicied.

Each closure removed slack instead of adding it. The decode gate now asserts an empty failure list;
the reject gate asserts set equality against `DIALECT_ACCEPTED`, the seventeen `invalid/` files the
default dialect *declares*. Neither can absorb a new defect because no number is left to absorb it
into: a new hole reddens by its own path, and a dialect promise the engine quietly breaks reddens by
its own path as well. The boundary the tier now states is about the dialect, not about the engine —
and on every interpreter, since the set is identical across the 3.8-to-3.15 legs.

Two of the three also moved the instruction gate, in opposite directions, and that is the other half
of the lesson. Identifying a table by derived text cost +4.44% on `from_toml_medium` when computed
per key and +41% when a walk precomputed every prefix; carrying the same state as one bit on the
table fixed four valid documents and landed at −5.89% (159,859,652 → 150,441,866). The delimiter
bound is free: the same 150,441,866, because the bench document holds no multi-line string — which
is evidence about the priced path, not a licence to assume the rule costs nothing. The gate that
counts instructions is what turns "correct" and "cheap" into one decision instead of a trade-off,
and it is also the gate that says when a correct fix has been implemented expensively.

### (ef) Decision 7: what a YAML replay would ask of the vocabulary, and the two facts it cannot have

The fold decision 6 promises exposed that step 4's replay gate covers one grammar.
`pyrs_json::from_events` is public with a nextest tier and a fuzz tier; YAML has no such reader, and
the only `from_events` in `pyrs-yaml-core` is a local variable inside
`the_tree_and_the_events_name_the_same_anchors`. Porting it runs into three notation facts the tree
carries and the stream does not: `flow_style` on a mapping or sequence, `chomping` and
`block_indent` on a scalar. `PartialEq` compares the first two and the `Hash` mirrors them, so a
replay that leaves them out rebuilds a node the engine calls a different document; `block_indent` is
the field `eq` ignores and the writer still reads.

The split between the three is the finding. granit hands the producer a collection's notation —
`Event::MappingStart(StructureStyle, ..)` and `Event::SequenceStart(StructureStyle, ..)` — and
`event_to_stream_event` matches both with `_` in that position, so `flow` is reporting, not
re-deriving. granit's `Event::Scalar` carries no chomping and its `ScalarStyle` has five cases only,
so `|`, `|-` and `|+` arrive as one style, and the tree route's `Chomping` comes from probing the
block header in `parser/yaml/scalar.rs` — a pass over the source the stream does not make. Adding a
field to `pyrs-event` cannot close that; only a producer that reads the source can, which is what
the tree-free producer was built to stop being.

Why the gate's vehicle changes with the format: the JSON replay compares the two trees through the
strict writer, and for YAML the writer's text depends on exactly the notation the stream drops —
block against flow, clipped against stripped. So a YAML fold is either a comparison through a
normalising route that is not the shipped writer, or a content-identity check with the notation
slots emptied, which proves less than the JSON gate.

Not measured: nothing changed in code, so the instruction-count leg has no claim to judge. The
granit signatures and the five style cases are read from the vendored source and our own match arms
rather than probed at runtime; a `|` against `|-` probe needs the extension rebuilt and answers the
same question. `docs/dev/event-stream.md` carries the finding and the three-part question as "What a
YAML replay asks the vocabulary for".

### (eg) The edit layer got a fuzz tier, and it found the anchor's three doors before it was merged (2026-10-18)

The defence matrix had a cell nobody had filled: eight targets, all of them handing the engine a
*document*. Nothing drove a *change* to one - the region-splicing layer behind `doc.set()`,
`doc.delete()` and the `pyq` verbs, which is the layer the round-trip promise is about. Its tests
were named shapes plus `prop_style_setting_roundtrip` over generated trees, and a generated tree
does not contain the shapes that broke this line before (a note under a tag-only line, a CR-only
break, an alias in key position).

`fuzz/fuzz_targets/edit_yaml.rs` takes `document NUL script`, collects the paths the reader actually
reached so mutations land inside the document, applies one of twelve verbs per record, and then
re-parses the emission, and checks the verb's own claim. Three claims came back red within the first
minutes of running it, and they were the same invariant seen from three doors:

- `set_anchor_path` renamed `&x` while `use: *x` stayed behind, emitting `base: &a0 1 / use: *x` -
  refused by this library's own reader with `found unknown anchor` (`former-crash-3cc4245d.seed`);
- the same orphan through `set_path` with `preserve_metadata: false`, which is how `move_path` and
  `set_many_path` reach it: moving `$.use` onto `$.base` emitted `base: *x`;
- and through `delete_path`, moving or deleting the node that *defines* a referenced anchor emitted
  the alias alone (`former-crash-c59db74a.seed`).

The engine already owned the rule - `with_metadata_from` keeps the document's anchor name when a
value is replaced with its metadata, and `the_anchor_the_document_aliases_names_is_kept` says why -
so this is not a new policy, it is the old one applied at the doors that bypassed it. Refusals are
asked where the drop happens: one check at the head of `set_path`, one in `delete_path`, one in each
anchor verb, all sharing `anchor-is-referenced`. Seven Rust rows and one Python row pin the three
doors and the two cases that must *keep* working (a rename nothing refers to; deleting the alias
itself).

Both guards answer wider than the set of aliases that really dangle, and that is recorded rather
than hidden: an alias binds the nearest preceding anchor of its name, so a rename a later `&x` would
have kept alive, and a subtree holding both an anchor and its alias, are refused too. The trade is
stated in the code - over-refusing costs the caller one more edit, under-refusing writes a file
nothing can read. Making a rename *carry* its aliases (which is what ruamel's shared-anchor model
does, and what an author typing a new name may expect) is a decision about the verb's meaning and
stays open, as does the silent no-op of `sort_keys_path` on a non-mapping and the note that changes
owner when a container is annotated.

What the tier does not claim is in its header, and each omission has an input behind it: style
equality (the writer's fallback is correct, and the property tier owns the question), sort equality
on a scalar path, note ownership after an edit, and - the one that is a defect rather than a
boundary - the fixed point. A flow member carrying both a tag and an inline note settles only on the
second round: the first emission writes `a: !fuzz2   # fuzz 47`, the re-read hands the note to the
mapping's own line, and the second emission moves the tag after it. That is the `(ds)`/`(dq)`
species in a container the earlier fixes did not reach, and it is why
`flow-tag-note-needs-two-rounds.seed` is replayed rather than asserted: a gate that reddens on an
open defect gates nothing. Closing the flow attribution is the next change's job, and it inherits
four committed inputs instead of a re-derivation - that seed, `container-note-changes-owner.seed`,
`note-may-change-owner.seed`, and the sort case in the header.

The workflow's header comment no longer counts the targets ("eight engine targets" was one change
from being a lie about its own matrix); `tests/test_quality_matrix.py` continues to derive the list
from `fuzz/Cargo.toml` and the `fuzz.yml` matrix, and reddens if either half moves alone. Counts
moved with it: nine targets, 154 seeds of which 69 are former crashes, 712 Rust tests, 1,744 Python
test functions.

### (eh) The flow member whose note had no body to sit on, closed at the writer (2026-10-18)

(eg) left one defect open and named it: a flow member carrying both a tag and an inline note settled
only on the second round. The trigger is two verbs on one path - `k: {a: !o }`, then set the tag,
then set the note. The first emission is `k: {a: !t   # n`, the re-read hands the note to the
enclosing pair because the value's line holds nothing but its tag, and the second emission moves the
tag behind that note. No value, key or note is lost anywhere in the round, which is why the
note-survival gate, the round-trip target and the property tier all passed over it: the only claim
violated is the fixed point, and the tier that asserts it had that claim withheld.

The writer already owned the answer in block context - `close_captured_pending_line` spells a
tag-only line as `~` before a note is written, and (ds)/(dq) carry why - but nothing reached the
flow route, where `serialize_flow_value` writes an empty plain scalar as nothing at all.
`spell_empty_flow_body` now makes the same decision where a member's note is written, spelling the
empty body `''`; `{a: !t ''  # n` re-reads as the tree it came from, and the first emission is the
fixed point. `''` rather than `~` because the node is a scalar, not a null, and the quoting keeps
its text.

Measured before the remedy was written, because the risk was a body spelled twice: `{a: }` reaches
the writer as a null and still emits `{a: ~  # n`, `null` still emits `{a: null  # n`, and a string
forced to `ScalarStyle::Plain` emits `{a: ''  # n` - all three settle in one round, and none of them
grew a second body. The helper fires only where the line carried nothing, so no shape that was
already stable changed.

Three tiers hold it now. `crates/pyrs-yaml-core/tests/settles_in_one_round.rs` names the shape and
pins the text, because a bounded fuzz budget can miss a rare drift for a run. The input `(eg)`
committed as `flow-tag-note-needs-two-rounds.seed` is now `former-crash-b23eed05.seed`, so the PR
tier replays it with `-runs=0` and a regression cannot re-open it silently (measured: 12 seed
inputs replayed, `DONE`, no crash). And the claim `(eg)` took out of
`fuzz/fuzz_targets/edit_yaml.rs` is back: an accepted edit's emission must re-parse and serialize to
the same bytes. Counts moved with it - 713 Rust tests, 154 seeds of which 70 are former crashes,
nine targets. The two narrowed claims that stay out of the target - sorting a scalar path, and a
note whose owner an edit deleted - are still (eg)'s boundaries, and the `set_comment` ownership
question is still #113's ruling to make.

### (ei) The dash line took back its own note (2026-10-18)

The discovery window (420s over `edit_yaml`, run on #514's tree) found `crash-61474da6`, and the
shape under it needed no fuzzer to state: `- a: 1`, `set_comment` on `root[0]`. The first emission
was `- \n  a: 1  # in`, the re-read filed the note with the pair that ends the block, and the second
emission compacted it back to `- a: 1  # in`. Text moved; nothing was lost; the only claim broken
was the fixed point (eh) had just restored.

The cause was a permission, not a placement. `is_compact_item` asked the item mapping for
`is_identity_clean()` — no comment, no anchor, no tag — so a note the item owned pushed the whole
mapping off the dash line. And the dash line was exactly where the reader reports that note from, so
the break bought nothing and cost the round: `-` then a block is how a *pair's* note is spelled,
and the pair's note is what the re-read produced.

A note alone no longer disqualifies the compact line; an anchor or a tag still does. The compact
branch writes the item's own inline note itself, taking the last pair line's trailing slot when it
is open and, when the pair has claimed YAML's one slot per line, a note line above the dash — the
two places the reader hands a note back to this item. Five shapes were measured before and after:
`- a: 1`, `- a: 1` with the value also noted, `- a: 1 / b: 2` with the last value noted, the fuzz
input's `- :` (a null key, which is what the fuzzer minimised), and a two-item sequence with one
note each. All five now settle in one round, and the `- {}` / `- ~` controls that already settled
are untouched.

`is_compact_item` is shared with the splice layer on purpose — the manifest comment says the writer
and the splicer must never disagree — so widening it widens both together, and the check that the
widening cost nothing is the suite: 715 Rust tests pass, including
`every_note_in_the_seed_corpus_survives_one_emission` and the compact-dash note rows. No test had
pinned the old break, because the old break was only ever reachable through the edit layer, and the
note gates ask whether the text survived, not which line it sat on.

Two named rows in `crates/pyrs-yaml-core/src/serializer.rs` hold the two placements
(`a_note_on_a_block_sequence_item_rides_its_dash_line`,
`an_item_note_with_no_room_on_the_dash_line_goes_above_it`), the input is committed as
`fuzz/seeds/edit_yaml/former-crash-61474da6.seed` for the PR tier's `-runs=0` replay (measured: 13
seeds replayed against this tree, `#14 DONE`, no crash), and #514's fixed-point assertion is what
caught it.

What this does not decide is #113. `- a: 1  # in` re-reads the note as the *pair's*; the emission is
now a fixed point, but the owner still moved, and the tier therefore still withholds the ownership
claim. A note that stays with the node the call named needs a position the reader reports it back
from — that is a decision about the writer's contract, not a seed.

### (ej) An empty body is written as the null it resolves to (2026-10-18)

(eg) handed over one open defect, (eh) closed it, and the restored fixed-point claim immediately
found the next two: `crash-61474da6`, a block sequence item whose note pushed its mapping off the
dash line (ledger (ei), PR #515), then `crash-63749496` — document `- >`, set the folded style to
plain, and the emission is `-` with nothing after the dash. Re-read, that is a null; the null's own
spelling is `- ~`; the text needed a second round.

Measuring the shape rather than the input showed the hole was not the dash's. `k: `, `k:\n  a: `,
`{a: }` and `[ ]`'s empty member all write nothing for a `Scalar { value: "", style: Plain }` with
no tag and no anchor, and in every one of those positions the reader hands back a null whose text is
`~`. The scalar writer is where the answer belongs: an empty plain body is now written as `~`, at
one shared guard used by the block route (`write_scalar_node`) and the flow route
(`serialize_flow_value`).

A tagged empty is deliberately outside the rule. A tag suppresses schema resolution, so the *text*
is what has to survive and `~` would change it — that is the case (eh) settled on `''`, and the
guard is why `spell_empty_flow_body` now asks for a tag before spelling a body: with `~` written by
the shared guard, spelling `''` as well would emit `~ ''`. An anchor was left out of the rule for
the same reason one line of prose, and the window found that mistake a minute into its next run:
`crash-07b6a10e` is `- &a3 `, the same empty body with a name on it. An anchor names a node and
suppresses nothing — `&a3` and `&a3 ~` are the same null — so the guard asks only about the tag,
and the pinned test carries the anchored shape as its sixth case.

Measured across five positions before the writer changed and pinned after it:
`an_empty_plain_scalar_settles_as_the_null_text` (in `settles_in_one_round.rs`) drives `- >`,
`k: >`, `k:\n  a: >`, `k: {a: ''}` and `k: ['']` through `set_scalar_style_path(Plain)` and asserts
each emission text and its fixed point — `- ~`, `k: ~`, `k:\n  a: ~`, `k: {a: ~}`, `k: [~]`. Nothing
else in the tree changed its mind: 716 Rust tests and the whole Python tier pass, and the only
Python failure while the work was in progress was the matrix's own count assertion waiting for the
numbers below. The crash input is committed as `fuzz/seeds/edit_yaml/former-crash-63749496.seed` and
the PR tier replays the 15 seeds of `edit_yaml` with `-runs=0` (`#16 DONE`, no crash).

Counts moved with it: 716 Rust tests, 157 seeds of which 73 are former crashes, nine targets, 23
Unreleased entries per changelog mirror. The claim that found all three of these is the one (eg)
took out and (eh) put back — a withheld assertion finds nothing, which is the whole argument for
keeping the fixed point asserted and closing what it names.

The instruction-count leg caught this one, and it took a control to read. CI's own two runs — trunk
and the PR head, one baseline, the enforcing image — put the four block-style writer scenarios at
+0.62…0.66%, and the fix writes no bytes at all on those fixtures. `docs/dev/perf.md` carries the
same-session bisect: the flag hoist was not it (32 instructions of 32.5M), the branch is worth
+0.06% against a no-op control, and the single `~` write costs 0.65% when it sits before
`write_scalar` and nothing when it sits after. The reworked head then read
+0.05/+0.12/+0.33/+0.05/+0.03% on the image — each below trunk's own reading of the same scenarios —
and the leg reported success, so no floor moved. The red is recorded as priced and cleared, not
widened away.

### (ek) The TOML dialect promise got a property tier, and a mutation showed what each tier reaches (2026-10-19)

The measured matrix had TOML at two Rust properties against YAML's 25, and reading what those two
asserted showed a gap that was not about quantity: `prop_toml_writer_is_fixed_point` asks whether
the emission settles, and `prop_toml_output_reparses` asks whether it parses. Neither asks *which
dialect* the settled text belongs to, although the writer targets 1.1 and the reader splits the
dialects — `from_toml_v1_0` exists, and the 1.1 release's one visible freedom is a newline inside an
inline table, which is what lets a comment live there. Nothing tested that an undecorated tree was
handed 1.0 text.

Three tiers, because the measurement said one of them was not enough:

- `prop_toml_undecorated_output_loads_as_1_0` (`crates/pyrs-yaml-core/src/fmt_pbt.rs`) draws
  generated trees, assumes the tree carries no note in any position (`toml_undecorated`, keys
  included), and requires `from_toml_v1_0` to reload the emission. It holds at 20 000 cases.
- `an_undecorated_inline_table_writes_compact_and_loads_as_1_0` and
  `the_1_0_reader_refuses_the_1_1_inline_table_freedom` (`crates/pyrs-toml/src/writer.rs`) name the
  shapes: five spellings of an inline table, each written compact and reloaded under 1.0; and the
  oracle's own premise, that 1.0 refuses `a = {\n b = 1,\n}` while 1.1 accepts it.

The mutation is the part worth keeping. Forcing the writer's `decorated` branch to always take the
multi-line path reddened the named test immediately and left the property green at 20 000 cases: the
property's filter wants a `flow_style` mapping that carries no note anywhere, and `arb_node_meta`
attaches notes often enough that generation almost never produced one. That is a reachability fact
about the generator, not a defence, so the property's input was widened — `arb_toml_inline_shapes`
spells 1.0-shaped documents containing inline tables as text, parses them, and hands the trees to
the same claim. Measured after the widening: the identical mutation reddens the property in 0.01 s
and the clean tree passes. A green property that no mutation can redden is a number, not a guard.
Registering a property whose oracle was itself unverified would have been the same mistake in the
other direction, which is why the 1.0-refusal premise is a test and not a comment.

The third property says what the two text tiers do not: `prop_toml_roundtrip_preserves_the_tree`
compares the tree before and after one round — scalar text, mapping size and order, sequence size
and order — while the fixed point only asks that the *text* stop moving. It holds over 2 000
generated trees, so no value loss is known today. Its falsification was measured twice, and the
second reading is the one that keeps the claim honest: dropping an array element (`.take(n-1)`, then
`.take(1)`) reddened it with `value sequence size 2 -> 1`, and the text fixed point reddened in both
runs too. So this is the equality the matrix's property row names as the tier's blind spot, stated
directly — not an oracle strictly stronger than what was there, which is the sentence a later reader
would otherwise undo.

Counts moved with it: 720 Rust tests (from 716), 29 properties (from 27), TOML's unit row 88 → 90
and its property row 2 → 4. No behaviour changed — `cargo nextest run --all` and the Python tier
pass on the same emissions the gate measured before the change.

### (el) An edited document was asked what it exports, and the answer was two defects (2026-10-19)

The edit tier judged the emission of the document, never the text another spoke writes from it. The
first premise — give JSON and TOML their own edit targets — was refuted before any code: the plan
verbs are YAML-only by design (`crates/pyrs-toml/src/lib.rs` says so in its header), `YamlDocument`
carries no format field, and neither `pyrs_json` nor `pyrs-toml` exports an editing API. What *is*
reachable is the hub route: the CLI runs a plan verb and then `to_toml`
(`crates/pyrs-yaml-cli/src/lib.rs:641`), and `doc.set(...)` sits beside `doc.to_toml()` /
`doc.to_json5()` on the same object. So the claim went into `edit_yaml.rs` as `export_settles`,
which hands the settled tree to each of the seven export spellings and asks the two questions
`toml_roundtrip` and `json_roundtrip` already ask — the dialect's own reader must accept the text,
and its second emission must be the same bytes. A dialect refusing the tree is a pass, because TOML
has no top-level scalar and strict JSON refuses a non-finite float.

The first discovery window (420 s over `edit_yaml`, corpus replayed first: 16 seeds, `#17 DONE`, no
crash) found `crash-fbd364b3`. The input is `: abase: &x 1\nuse: *x` with a three-record script
whose third record is a plain `set` on the path `["~"]`; the emission was `~: v0\nuse: *x`, which
this library's reader refuses with `found unknown anchor`. The guard at `set_path` had asked only
whether the *replaced node* carried an anchor an alias named — and that node carried none, its child
did — while `preserve_metadata: true` copies exactly the replaced node's own properties, so nothing
could carry a definition from inside the subtree away. `delete_path` had already been given the
wider, subtree-shaped answer through `anchors_in`; `set_path` had not. Asking the same question at
that door refuses the edit with the same `anchor-is-referenced` reason and leaves the tree
untouched. Measured both ways: removing the new block reddens
`replacing_a_container_that_holds_a_referenced_anchor_is_refused` and nothing else, and the pair
test `replacing_a_container_whose_anchor_nothing_refers_to_is_allowed` shows the guard is about a
reference, not about the presence of an anchor.

The second window found `crash-3dd60cad`: `- !o` with a tag set to `fuzz2` and then a note, emitting
`- !fuzz2  # fuzz 47` and re-reading it back as `# fuzz 47` above `- !fuzz2`. Nothing was lost, so
no other tier saw it; the fixed point broke because a note written inline after a still-pending tag
line has no body to sit on, and the reader hands such a line's note to whatever follows. The writer
already closes that line before a *note line* (`close_pending_tag_line`, (eh)/(ej)); the inline
branch did not call it. Now it does, the item is written `- !fuzz2 ~  # fuzz 47`, and the pinned
test is the first assertion of that text. Removing the call reddens only `settles_in_one_round`'s
new case — every other pinned emission in the family, and `note_survival` over the whole seed
corpus, stayed green, so no existing spelling moved.

Then the same branch found a second one, and this one was the export claim's own. `-0` is legal TOML
text, so the integer pass-through wrote it verbatim; but TOML has no negative zero, `ist = -0`
re-reads as the integer `0`, and the second emission moved. The writer's own comment had named `-0`
as an intended pass-through case, which is how the defect survived every tier that only ever asked
value equality: the value is the same, only the text is not a fixed point. A signed zero is now
written as `0`, while the radix spellings the reader really does preserve (`0xDEAD`, `0b1`) still
pass through (`a_signed_zero_is_written_as_the_zero_the_reader_spells_back`, crash-917e103b).

A third window found the fourth defect, in YAML again and in the same family as (ei). Removing the
last item of a block sequence leaves `flow_style: false` on the now-empty container, and
`inlineable_value` required the flag for any container, so the item mapping was pushed off the dash
line. But an empty container has no block form at all: the writer always spells it `[]` on the key's
line, so the rule was vetoing a compactness the emission does not have. The reader answers `[]` with
the flag set, and the text moved: `-\n  t: []` became `- t: []`. Emptiness now counts as inlineable
(`an_emptied_block_sequence_keeps_the_dash_line`, crash-a58caa38). Four windows, four defects, and
the window that closed each one kept finding the next.

The same two questions went into the other half of the pair. `json_roundtrip` asked its three
compact writers, while `prop_pretty_json_writers_are_fixed_points` asked the pretty ones of
*generated* trees — so the text `doc.to_json(indent)` and the CLI's `--pretty` produce was the one
spelling no fuzz tier had ever read back. Three spokes joined the three. That window's first crash
stopped at the compact `json` claim (`fuzz_targets/json_roundtrip.rs:71`), and what it found was the
projection's, not a writer's: the input is a JSON5 single-quoted string of 119 digits;
`Schema::Json5` answers `Str` for a digit run that will not fit an `i64`, so `quoted_or_plain`
stored it plain; `write_plain` then asks `is_json_number`, which is JSON's grammar and puts no bound
on an integer literal, and handed the same bytes back unquoted. First emission `"12…9"`, second
`12…9` — one trip through the hub had turned a `str` into a number. The sound form of the question
is the one the writer will ask, so the projection asks it first: a byte walk over RFC 8259's number
grammar, reached only past the first-byte guard, because asking the resolver unconditionally
measured +14.24% on this path (`from_json_medium`) and that price is why the guard exists at all.

The three new spokes were then checked for the same thing, and the answer is asymmetric and written
down as it is: no pretty spelling has produced a defect, so what proves they run is a mutation of
the pretty path alone. Appending one stray comma inside `to_json_text_pretty` reddens
`json (pretty) writer output failed to re-parse` at line 77 over the committed seeds; restoring it
byte-for-byte replays all 17 `DONE`. A claim that only *its author's* defect can redden is the
number this repository distrusts, so each spoke is reported with whichever of the two it has
actually paid for.

The instruction-count leg read the same bytes as the tree that shipped #516: `serialize_small`
4,137,896, `serialize_medium` 32,360,862, `serialize_block` 14,383,162, `serialize_block_scalars`
12,302,661 on the pinned compiler, and the run reported
`OK: every scenario within tolerance of the committed baseline`. That is what the shape predicts:
the fixtures carry no tag-only line followed by a note, so the new call only ever reaches its early
return. Re-measured after the two writer rules changed, the same leg read `serialize_medium`
32,360,898 and `serialize_block` 14,383,198 - thirty-six instructions, +0.0001% against a floor of
0.50% - and again reported `OK`, because widening `inlineable_value` costs one field test on a
container that already asked for it. A third reading, taken after the JSON projection started asking
the writer's question first, moved `from_json_medium` from 80,859,913 to 80,863,858 — +0.00% against
the same floor, with all six JSON-family scenarios inside ±0.00% and the run again reporting `OK`.
The byte walk sits behind the first-byte guard the +14.24% measurement put there, so it is asked
only of text that could already have been a number.

Counts moved with it: 726 Rust tests (from 720), 162 seeds of which 78 are former crashes (from
157/73), nine targets, 30 Unreleased entries per changelog mirror. The property tier stays at the 29
that #518 raised it to, and the two new refusals join the six seeds (eg) named, because a claim that
cannot fail is a number, not a guard.

The new claim was checked for reachability before it was believed, and the first probe was wrong in
a way worth writing down. The mutation was a TOML writer that emits `a = MUTANT` for any tagged
scalar - invalid text its reader refuses - and the input `(ek)` had just closed did *not* redden it,
because that document's root is a sequence and `to_toml` has no top-level form for one, so the TOML
branch never ran. Replaying the whole `edit_yaml` corpus and seeds (1,247 inputs) under the same
mutation reddened the first claim the new code makes,
`an edited tree wrote TOML its own reader refused`, over `[k10]\na = MUTANT`. Restoring the writer
byte-for-byte and rebuilding leaves those same 1,247 inputs `DONE`, no crash.

What the widened `edit_yaml` window found and this section does *not* claim to close is the fifth
shape, `crash-08b30707`: the document `\n?ss:\n-   !a :s:\n-   !a \n` with a three-byte script. The
first emission puts the note at column 2 after the first item's line, the re-read files it inside
the second item's continuation, and the next emission writes `- \n    # fuzz 35\n    !a` — nothing
lost, the text not a fixed point. Fix 2 above covers a note on an item's *own* tag-only line; this
note sits in the gap *between* two of them, which is `pending_tag_insert_at` /
`close_pending_tag_line` / `trailing_note_slot_open` argued together rather than one of them
corrected. Task #121 holds it, and its input is deliberately not in `fuzz/seeds/`: the PR tier
replays every seed with `-runs=0`, so a still-failing seed would block every pull request until the
fix lands — the (eg) precedent, withhold the claim or fix it, never commit a red seed.

### (em) A profile that quoted what its own reader could not type (2026-10-19)

The clause that #504 put in front of every plain scalar says: if a reader following the YAML 1.1
type repository would resolve this text to a number or a bool, quote it, because a string that
leaves plain comes back typed. The clause knew `1_000`, `0b101`, `+0x1f`, `+.inf` and `1990-06-23`.
The engine's own `schema="1.1"` reader knew the legacy booleans and whatever Core already typed, and
nothing else — so the pair answered different questions about the same spelling, and the direction
of the disagreement was the one the clause exists to prevent: read `a: 0b101` under 1.1, get the
string `"0b101"`, write it back, and the clause quotes it, because a 1.1 reader would type it. The
text moved on a round trip that asked one profile the whole way. The clause's own comment named
`y`/`n` as the case where "the pair disagreed with itself"; this was the same species with four more
families in it.

`resolve_yaml11_type` now resolves them. Every row is the answer PyYAML's default loader gives the
same text, and `tests/test_yaml_crosslib.py` compares the two engines row by row rather than
trusting the spec prose: `_` between digits (`1_000` 1000, `5__2` 52, `1__` 1, `1000_` 1000, `1_0.5`
10.5, `.5_` 0.5, `_1000` still text because a token may not open with one); a binary int (`0b101` 5,
`0b1_0` 2, `+0b11` 3, and `0B11` text, since the spec and PyYAML both spell binary with a lower-case
`b`); a sign in front of a body Core types (`+0x1f` 31, `-0x1f` -31, `+.inf` infinity — the signed
word needs the dot, so `+inf` and `+nan` stay text); and underscores attached to a base prefix
(`0x_1f` 31).

The fourth family is the one this change found on the way, and it is the only place the profile
*overrides* Core instead of adding to it: YAML 1.1 reads a leading zero as octal, 1.2 Core dropped
that rule, so `017` is 15 to PyYAML and 17 to the core chain this profile delegates to. The task
that opened this line had recorded "legacy octal already resolves", and the measurement behind it
was `007` — a spelling where octal and decimal agree. Seven years of masked examples, found by
asking for a row the two readings differ on.

The gap that came out of the same work is on the quoting side, and the test that caught it is the
invariant `whatever_the_1_1_profile_types_is_never_emitted_plain`: for every text the profile types,
`needs_quotes` must agree. `0x_1f` reddened it before the clause was widened — the clause checked
underscores only on decimal tokens, so an unsigned base-prefixed spelling with an underscore went
out plain and came back a number to PyYAML. That row is now a claim in both directions: the reader
types it, the writer quotes it.

What is deliberately *not* here, each named rather than left unstated. Sexagesimal (`190:20:30`,
685230 to PyYAML) stays text, for the reason the clause already carried: a colon forces quoting one
layer below, so the family cannot reach a plain scalar anyway. Timestamps (`1990-06-23`, a `date` to
PyYAML) stay text, because the temporal-tag ruling keeps untagged text text and a read path that
invented temporal objects from plain words would reopen it. `0o17` and `+0o17` are typed here and
read as text by PyYAML — the profile is Core *plus*, and `0o` is a Core prefix 1.1 never had; the
parity test asserts that difference instead of hiding it. And the `i64` bound is unchanged:
`1_000_000_000_000_000_000_000` is a big integer to PyYAML and text here, as every too-long run
already was.

The price was measured on the same host, same pinned `rustc 1.97.1`, same session, trunk's
`schema.rs` swapped in and out: `from_toml_medium` read 135,668,882 on trunk, 137,874,882 (+1.61%)
with the first version of this change, 136,212,882 (+0.39%) at the pushed head. Two steps moved it
back. The first version asked `resolve_yaml11_number` for every text the 1.1 half of
`plain_text_is_typed` was asked about, and that function opened by scanning the token for `_` — one
pass over every key of every document, most of which start on a letter. A whitelist inversion of the
kind `resolve_core_type` already uses fixed most of it (+1.61% → +0.62%); putting the byte compare
in the *caller*, so `plain_text_is_typed` keeps getting an inlinable wrapper instead of a call into
a grown function, took the rest (+0.62% → +0.39%).

A third step, found by asking what the pair actually needs, took the scenario *below* its floor:
`plain_text_is_typed` asked core and then asked `resolve_yaml11_type`, and the 1.1 chain ends in
core, so every scalar core had already called a string ran core's whole chain a second time to reach
the same answer. Splitting the profile into "core, then the lexemes 1.1 adds" (`yaml11_lexemes`)
asks core once, and keeping the grown callee out-of-line leaves the wrapper inlinable: 136,212,882
(+0.39%) → 132,008,882 (−2.71%) → 131,612,882 (−3.00%) against the floor of 135,689,929, one
session, the file swapped between runs. What the split could break is the pair itself — the two
forms answer the same question exactly because the fallthrough is core — and that is now a swept
claim, not a reading of the source:
`the_pair_and_its_decomposition_answer_the_same_for_every_short_token` walks every token of length ≤
4 over a sixteen-character alphabet chosen to reach both chains, and it reddens when the 1.1 half is
dropped from the predicate.

`to_toml_medium` is recorded because two readings of it were wrong. It reads +0.69% on this box
against a +0.39% control, which first looked like a cost of this change and then like a placement
artifact; the per-function A/B says which. One session, symbols build, trunk's file swapped in and
out: every named function's self count is byte-identical (`push_scalar_key` 4,644,000,
`resolve_core_type` 1,628,000, `numeric_tail` 1,036,000) and the whole movement sits in
`<i64>::from_ascii_radix` (+88,000) — whose only caller chain in the run is
`write_value → Schema::resolve → resolve_core_type → numeric_tail → from_ascii_radix`, core's own
decimal parse, and `callgrind_annotate` finds no `yaml11` node anywhere in the tree. Same calls,
more executed bytes: the file grew, everything downstream of it moved, and an alignment padding in a
hot loop counts as instructions. The reading was then checked where it counts, and the enforcing
image read the same +0.69% (`FAIL to_toml_medium: +0.69% over baseline (tolerance 0.50%)` on head
`855b1916`) where the previous head had read +0.30% on the very same floor. Two hosts agreeing is
not a reason to widen a line — `docs/dev/perf.md` measures the WSL-vs-image residual at ±0.003% for
nineteen scenarios on one tree — it is a number to pay.

The payment came from the writer's own account, and it was worth taking. `to_toml` collected every
root table's header text into a `String` of its own before the section pass wrote it, while the
section list already carried the key node — the header arm appends through `push_scalar_key` exactly
as a pair line does. Dropping the writer's only materialised key takes `to_toml_medium` to
26,741,057 counted instructions against the 28,939,722 floor, −7.60%: one allocation, one grow, one
copy and one free per header, which is what the profile's `finish_grow`, `_int_free` and
`__memcpy_avx_unaligned_erms` share was made of. `from_toml_medium` holds at −3.01%,
`to_json_medium` at +0.14%, and the gate reports
`OK: every scenario within tolerance of the committed baseline`.

What the new gap costs is stated rather than glossed: a floor 7.6% below the tree is the same hole
the 2026-10-14 refresh was written about, only on the other side of the line — the band the gate
waves through has just widened by that much. The `to_toml_medium` floor needs a deliberate refresh
on the enforcing image (the `Ir baseline refresh` workflow), and that is its own change, not a line
moved beside a correctness fix.

Reachability, one mutant: asking `resolve_yaml11_number("")` — a token no family can match — reddens
`the_1_1_profile_resolves_the_numeric_families_a_1_1_reader_types` and
`whatever_the_1_1_profile_types_is_never_emitted_plain`. The negative control
(`the_1_1_profile_leaves_the_families_it_does_not_claim_as_text`) stays green under it, which is the
correct result for a test whose whole claim is that nothing happens; its own mutant is a family
added back. Counts: 730 Rust tests (from 726), 1,754 Python test functions, 33 Unreleased entries
per mirror.

### (en) A TOML root table moved a pair's key ahead of it (2026-10-19)

(#en assumes #521, written while this defect was still blocking its CI, re-letters to (eo).)

TOML's grammar puts every root key/value before any root table header, so the writer collected the
root's block mappings and emitted them after every pair - and a tree whose order did not already
have that shape came back reordered. The property tier found it at 20 000 cases, not the fuzz tier:
`prop_toml_roundtrip_preserves_the_tree` failed on `key scalar text "0" -> "a"` with the emitted
text `a = "b"` then `[0]` then `A = "a"`. Reproduced on trunk by hand: the tree
`0 -> {A -> "a"}, a -> "b"` round-trips to the order `a`, `0`. The *text* settled
(`to_toml(from_toml(t)) == t`), which is why every fixed-point claim stayed green while the tree
comparison did not.

Why only a cross-format or edited tree can carry it: a document parsed from TOML already lists its
pairs before its headers, so its order is expressible; a YAML or JSON document, or an `insert_path`
that puts a table first, is free to hold the other order, and the hub's own promise is that a
mapping is an ordered thing (`IndexMap`, and `sort-keys` exists because the order is meaning).

The remedy chosen was to stop moving the key, not to stop moving the table: a root block mapping
that has a root pair after it is written inline (`0 = {A = "a"}`), at its own position, because that
is the only TOML spelling that keeps the order. A parsed document never takes that path, so the
style promise #42 made - an inline table is not rewritten as a header, and a header is not rewritten
as inline - holds for every tree whose order TOML can spell with headers. The refusal alternative
(a new `cannot represent` error) was ruled out as a public error-surface change for a shape that has
a faithful spelling, and narrowing the property was ruled out because the oracle, not the claim, is
what found the bug.

Pinned by `a_section_before_a_pair_keeps_the_order_by_going_inline` (the exact emission text, the
key order after re-read, and the fixed point). Reachability: with the new branch replaced by `None`,
exactly that one test reddens and the other 82 in the crate pass, and
`PROPTEST_CASES=20000 cargo test -p pyrs-yaml-core --release fmt_pbt` runs the four TOML properties
back to green - the run that reddened on CI is the one the fix answers.

Counts: 731 Rust tests (from 730), 1,754 Python test functions, 34 Unreleased entries per mirror.

### (eo) A sequence's note kept its own line, not its item's dash (2026-10-19)

The placement (ds) settled, and (dq) defended, says a container's inline note goes above the line
that ended the container's block. That is right for a mapping: the line that ends a mapping's block
is a pair's own line, so the note lands where the reader reports a pair's note, and #91 measured the
alternatives - a note line below the block is re-read as the leading note of the node that ended it,
and closing a pending tag line first hands the note to the value and starts an oscillation. A
sequence has no pair line. The line that ends its block is one of its items' own dash lines, and a
note written under a dash line is reported as *that item's* leading note. So a note the edit layer
attaches to a sequence was emitted between the items and re-read inside one: `serialize(parse(x))`
still differed from `serialize(x)` for a document the writer itself had just produced.

libFuzzer's `edit_yaml` target found the shape and minimized it to 28 bytes - `?ss:` over two tagged
empty items - and that input is now `fuzz/seeds/edit_yaml/former-crash-08b30707.seed`. The branch's
first commit replays it clean (`Executed ... in 4 ms`, exit 0); with `serializer.rs` at trunk's
content the same bytes die (`SUMMARY: libFuzzer: deadly signal`, exit 77), which is the proof the
seed is the defect rather than a witness to it. The fix asks which container is speaking: a
sequence's note now goes above the sequence's own first line, the place the reader files a
sequence's leading note and the one `lift_first_item_notes` already relies on, while a mapping keeps
the pair placement it has had since (ds). Nothing pinned moved - `cargo nextest run
-p pyrs-yaml-core` is 416 passed with the settle tier's named fixed points among them - and all 21
`edit_yaml` seeds replay `DONE`.

The branch settles a second shape, one the edit verbs reach at the root: the empty path navigates to
the document node, so `set_tag_path` and `set_comment_path` both answer for a root sequence, and a
note asked there used to sit on the block's last dash line and move above it on the second
emission: `!fuzz0 \n- !a\n# fuzz 11\n- !a\n` became
`# fuzz 11\n!fuzz0 \n- !a\n- !a\n`. The fuzz tier found it
through a one-line widening of the paths it collects; the widening was reverted and the discovery is
pinned as `the_roots_own_note_settles_in_one_round` over three root shapes, because a table asks a
shape every run while a bounded budget draws one. Reverting the sequence half of this fix reddens
exactly that test and no other (416 run, 415 passed, 1 failed).

What this does not fix is named rather than folded in. A note attached to a *mapping* whose block
ends inside a nested sequence still fails to settle: `ss:\n  - !a\n  # fuzz 35\n  - !a\n` reads
back as `ss:\n  - \n    # fuzz 35\n    !a\n  - !a\n`, because the reader cannot report the note
on the mapping and hands it to an item. #113 ruled that this ownership movement stays as it is and
that no new emission position should be added, and that ruling is about a mapping's note, so the
branch stops at the sequence half. Whether the non-settling *text* is covered by the same ruling was
an open question with a twelve-byte repro; (ep) answers it below - the ownership stays, the text
settles.

Counts: 163 seeds, 79 of them `former-crash-*` (`QUALITY_MATRIX.md`'s fuzz row), 732 Rust tests on
this base (after #520 and #522), 35 Unreleased entries per changelog mirror.

### (ep) A pair restores its own line before it returns (2026-10-19)

(eo) left the text half of a mapping note open: `ss:\n  - !a\n  # fuzz 35\n  - !a\n` re-read as
`ss:\n  - \n    # fuzz 35\n    !a\n  - !a\n`, so the first emission was not a fixed point. The
note's owner moves either way, and #113 ruled that movement stays; what the round-trip contract
does not allow is the text moving with it.

The mechanism is one shared field. `note_line_above_last_pair` reads `pair_line_start` when a
*container's* own inline note is placed - the enclosing writer calls it after the body - and
`write_mapping_pair` sets that field at the pair's start. A block value then writes its own starts
into the same field: `write_sequence_item` sets it at every item, so after the last pair's nested
sequence the field names the last item's dash line, not the pair. The mapping's note went above
that dash, the reader reported it as the item's leading note, and the next emission spelled the
item's value as a continuation - the text moved.

The fix restores the field before a pair returns, so after the last pair it names the line that
ended the mapping's block. Only the mapping branch is affected: a sequence's note is placed above
its own first line (`body_start`), which is the placement (eo) measured and this change leaves
alone. `a_mapping_note_over_a_nested_sequence_settles_in_one_round` pins three shapes - the mapping
is the root's only pair, another pair follows it, and a later pair lets the note ride the last pair
line - asserting the emitted text and one round for each. Removing the store reddens it and no
other test (`cargo nextest run -p pyrs-yaml-core -E 'test(a_mapping_note_over_a_nested_sequence)'`:
1 failed, 416 skipped, the message carrying `left: "ss:\n  - !a\n  # fuzz 35\n  - !a\n"`);
restoring it is 417 passed, and the property tier's 17 properties pass at
`PROPTEST_CASES=20000`. The added store is on every mapping pair, so it was priced: on this WSL
host with the baseline's pinned `rustc 1.97.1`, `serialize_medium` reads 32,448,886 against its
32,376,895 floor, +0.22% and inside the 0.50% tolerance; the enforcing-image CI leg remains the
authority, as `binaries:` in the run names these bytes as code.

Counts: 733 Rust tests (from 732), 1,759 Python test functions, 39 Unreleased entries per mirror.

### (eq) A TOML integer keeps the spelling it was written with (2026-10-19)

(cp) kept a binary literal's `0b` text by tagging it `!!int`, but every other non-canonical integer
spelling was still folded to decimal: `1_000`, `+5` and `0o1_7` re-read as `1000`, `5` and `15`.
Underscores and an explicit `+` are legal TOML 1.0 spellings (`underscored` and `signed-int` in the
ABNF), so `boundaries.md` saying "TOML's own literal grammar rejects them" was the wrong reason for
a deliberate loss. The parser now records the whole token text whenever it differs from `i64`'s
rendering - an underscore, a `+`, or a signed zero - and the projection keeps the tag rule from
(cp): tag only where YAML Core resolves a different value, so `+5` and `0x1f` stay untagged and
`1_000`/`0o1_7`/`0b1010` carry `!!int`.

Two bridges had to learn the same grammar. The YAML emitter quotes a scalar that starts with `-`,
so a tagged `-1_000` arrives at the TOML writer as a quoted scalar; the writer now lets a standard
tag decide the type before quoting does, while an untagged `"42"` remains a string. Its integer
gate accepts exactly what the writer emits: an optional decimal sign, an unsigned `0x`/`0o`/`0b`
prefix, and `_` only between digits. A signed radix is refused by the reader (it is not a TOML
integer at all), and a signed zero still writes as `0` because TOML has no negative zero.

The first implementation cost instructions, and the gate named both halves separately:
`to_toml_medium` +3.37% and `from_toml_medium` +1.80%, while the parent tree measured +0.00% on
both scenarios with the baseline's pinned `rustc 1.97.1` (`to_toml_medium` 26,879,077 against
26,879,029; `from_toml_medium` 131,612,905 against 131,611,879) - so the move was this change, not
the host. A/B with only `writer.rs` restored to the parent isolated the writer half: +0.00% there
and +1.80% remaining. The reader cost was `value.to_string()` per integer and a `replace('_', "")`
for the leading-zero check; it now decides canonicality from the token shape and borrows the source
when it holds no underscore. The writer's `split('_')` walk and separate signed-zero scan became
one byte pass. Re-measured: `to_toml_medium` 26,827,051 (-0.19%) and `from_toml_medium`
125,581,979 (-4.58%). The reader is now 4.58% below its floor, so the gate prints FLOOR_STALE until
the enforcing-image "Ir baseline refresh" workflow rewrites the number; that refresh is a post-push
step, not a local `--update`, because `--update` would rewrite `generated_by` for the 20 scenarios
it did not measure.

Mutation evidence: recording `cleaned` instead of `raw` reddens
`decimal_spellings_reach_the_hub_word_for_word` (`left: ("1000", None)` against
`("1_000", Some(("!!", "int")))`); dropping `tag_kind.is_some()` from the writer arm reddens
`a_quoted_integer_stays_a_string_unless_a_tag_says_otherwise` (`a = "-1_000"` against
`a = -1_000`). Restoring either leaves the 94 `pyrs-toml` tests passing.

Counts: 735 Rust tests (from 733), 1,762 Python test functions, 40 Unreleased entries per mirror.

### (er) A declared version picks the profile, and the writer keeps the declaration that chose it (2026-10-19)

(du) measured the reporting half and ruled version-driven resolution plus emission out of scope.
This change supplies both behind a flag, default off, so no existing caller's resolution or output
moved: `honor_version_directive=True` on `parse`, `parse_file`, `parse_all_docs`, `safe_load`,
`safe_loads`, `YAML(...)` and `YamlDocument.reparse` lets a `%YAML 1.1` declaration select the
YAML 1.1 profile (`flag: yes` -> `True`, `012` -> `10`), while `1.2`, `1.3` and an undeclared
document stay on Core. Explicit `schema` wins; `schema` now defaults to `None` so "not given" and
"explicitly Core" are different states rather than the same string.

The core reader still ignores the schema - `parse_document` has always reported the declared
version beside the tree - so negotiation is a post-parse decision on that report, not a second
parse. The one path that had to be rerouted is P3's direct-load fast path, which converts while
reading and cannot know the declaration before conversion; it is skipped only when the flag is on.
Emission is derived from the same rule that resolved the document: emit the declared version only
when the effective profile is the one that declaration maps to, so `%YAML 1.2` is re-emitted over
Core values while a declaration is never written over values it did not govern. `with_directive`
prepends `%YAML v` and, unless the caller already asked for an explicit start, the `---` that
opens the document.

The stream writer held the one interaction no single-document test could see: `dump_iterable`
writes its own `---` between documents. Skipping that separator whenever the buffer starts with
`%YAML` is right for a leading declaration and wrong for a later one - granit requires an
explicit `...`, refusing `flag: yes\n---\n%YAML 1.1\n---\nflag: yes\n` with "missing explicit
document end marker before directive". The writer now emits `...` before a directive-bearing
document that is not first; the emitted
`flag: yes\n...\n%YAML 1.1\n---\nflag: yes\n` replays as `['yes', True]`.

Mutation evidence. Forcing the negotiation guard false (always Core) reddens 10 of the 17 tests
the class held at that point and leaves the flag-off, explicit-schema, non-1.1 and undeclared pins
green - the shape of the compatibility claim. Replacing the stream's `...` with `---` reddens
`test_a_mid_stream_directive_is_closed_with_an_explicit_end` on the exact text
(`left: "flag: yes\n---\n%YAML 1.1\n---\nflag: yes\n"`). The stub route regenerated the seven
signatures (`schema` -> `str | None`, the new flag), and both mypy and ty passed the result.

Counts: 735 Rust tests (from 735), 1,779 Python test functions, 41 Unreleased entries per mirror.

### (es) The registry could say "no measured holes" but not "what we know we left open" (2026-10-19)

`.ci/quality-holes.json` reached an empty list, and the empty list was true about what
`scripts/quality_matrix.py` measures - and false about what the tree still carries. Three
residuals were measured, understood and deliberately open, and none has a hole kind the probe
can emit: a TOML table-header note that keeps its text and its table but not its line;
`set_comment(..., standalone=False)` on a container, whose note the writer parks on the last
child's line so the re-read gives it that child as owner (the reason `edit_yaml` withholds the
ownership claim and (ei) left #113 open); and `sort_keys` on a scalar path, which answers
`None` while a missing path raises `YamlEditError: missing-path`. "No measured holes" then
read as "no residuals", the same false assurance the matrix exists to remove.

`.ci/known-residuals.json` is the second registry, and `tests/test_known_residuals.py` is its
gate. Each entry names a pytest node id, the layer that owns the fix, and a checkable exit;
the test at that id carries `@pytest.mark.known_residual(<id>)` and asserts today's
behaviour. The pair fails in three directions - an entry whose repro is gone or marked with
another id, a marked repro with no entry, and a marker pytest would never collect - and a fix
reddens the characterization test itself, which forces the entry and the marker out in the
same change. That is the both-directions discipline `.ci/quality-holes.json` applies, for
residuals the probe cannot see.

Mutation evidence: deleting the `api:sort-keys-scalar-path-noop` entry while its repro stayed
marked reddened `test_every_marked_repro_is_registered` on that exact node id; the file
restored and the gate green again. The three pins were run before they were registered:
`a: 1\nb: 2\n` and `- a\n- b\n` each emit the note on the last child's line, `sort_keys`
answers `None` for `$.a` and raises for `$.missing`, and `to_toml(from_toml(...))` still
relocates `[sec] # note`.

Counts: 735 Rust tests, 1,791 Python test functions, 42 Unreleased entries per mirror.

### (et) The MSRV job installed `stable`, so nothing tested the declared floor (2026-10-19)

Every workspace crate declares `rust-version = "1.96"`, and `ci.yml`'s `msrv-check` job exists to
hold that claim. Since f6b590b3 (2026-08-16) the job installed
`dtolnay/rust-toolchain@stable`: it compiled the workspace on whatever stable the runner image
carried, and its name described a floor no command had tested. `rust-version` documents a minimum;
Cargo does not enforce it downward, so the only way to know 1.96 still works is to build with it.
A dependency needing 1.97 would have passed the job and failed at a user's install, and no other
leg would have caught it - `Test matrix (all legs)` consumed the job's green result.

Before changing the pin, the floor was measured on this tree rather than assumed:
`cargo +1.96.0 check --workspace --locked` and `cargo +1.96.0 test --workspace --locked` both
exit 0, so the declaration is true today. The job now installs `1.96`, the version all eight
crates declare, and `tests/test_msrv_gate.py` pins both directions: one test reads the workspace
members out of the root manifest and refuses a missing or disagreeing `rust-version`, another
compares the effective toolchain named by the workflow step (action ref or `with.toolchain`) with
that declaration, and a third refuses an installer with no `cargo check`/`cargo test` behind it.
The fan-in wiring was already exact - `test_the_fan_in_waits_on_every_job_in_the_workflow` covers
`msrv-check` - so the missing piece was the toolchain, not the reachability.

Mutation evidence. Restoring `dtolnay/rust-toolchain@stable` reddened
`test_the_msrv_job_installs_the_declared_floor` with `msrv-check installs 'stable', and the crates
declare '1.96'`. Moving `crates/pyrs-ast/Cargo.toml` to `1.97` reddened both the manifest test
(naming the seven crates at 1.96 and the one at 1.97) and the pin test; both files were restored
and the three tests run green.

Counts: 735 Rust tests, 1,794 Python test functions, 43 Unreleased entries per mirror.

### (eu) Two validators, one question: the schema row moved to the standard surface (2026-10-21)

`authority-tier:schema-validation` was registered because the matrix's `schema validation` row
measured a rule language this repository invented: its unit, property and fuzz tiers asked the
engine what it believes, and parity had no second implementation to disagree with. The row beside
it, `JSON Schema validation`, measured `Document.validate()`, which hands the instance and the
schema to Python's `jsonschema` and keeps only the AST-to-instance conversion, the schema
acceptance and the `YamlValidateError` contract here.

The outside authority was measured before it was credited. `tests/test_document_validate_suite.py`
replays the official JSON-Schema-Test-Suite at commit `0b4a7fa` (46 files, 1302 cases in
`tests/draft2020-12`): on `jsonschema` 4.26.0, 1261 cases reach the corpus's verdict. The 41
exclusions are named, not hidden behind a ratio: 31 in `refRemote.json` are a contract boundary
(`Document.validate` exposes no resolver, and the 15 cases that expect an invalid instance are not
credited because a missing resolver raising is a vacuous "invalid"), and ten are engine boundaries
(`dynamicRef.json` 5, `vocabulary.json` 1, `pattern.json` and `patternProperties.json` 2+2 for
`\p{…}` Unicode property escapes Python's `re` does not implement). The same corpus replayed through
`jsonschema.validate()` directly reaches the identical verdict on all 1302 cases, so no exclusion is
attributable to the conversion. The exclusion set is keyed on the installed version - the Python 3.8
leg resolves 4.23.0 and needs one more named `unevaluatedProperties` case, fixed by 4.25.1 - and
both directions redden: a named case that starts agreeing, and an unnamed one that stops.

That measurement made the choice answerable. Keeping two validators meant two error contracts and
two answers to "is this document valid", and only one of them could be judged from outside, so the
invented one was retired rather than deprecated. `validate_against_schema()` and
`validate_against_registered_schema()` are gone, a `validate:` section is no longer read, and the
Rust types that existed only for it (`TypeSpec`, `ValidateKind`, `ValidateRule`,
`SchemaValidationError`) are deleted; `register_schema()`, `load_schema()`, `list_schemas()`,
`YAML(schema=...)` and `rules:` scalar resolution are untouched. The CLI's `validate` binds
`--schema` and `--schema-file` to one parameter and checks syntax only when neither is given. The
matrix row leaves with the surface it measured: `scripts/quality_matrix.py` derives four rows and
prints `HOLES []` (measured), `.ci/quality-holes.json` is an empty registry with both directions
still asserted, and `tests/test_quality_matrix.py` names the four rows, so a fifth cannot appear
without a measurement behind it.

The fuzz target `validate_schema` became `parse_schema` because what it fuzzes is the surviving
`rules:` grammar, not JSON Schema evaluation; `fuzz/Cargo.toml` and `fuzz.yml` moved together,
which the matrix derives from both. The shipped stub holds the removal: regenerating it and running
`scripts/check_stub_drift.py` reports no drift, and `grep -c validate_against
python/pyrs_yaml/pyrs_yaml.pyi` is 0. The documentation mirrors were checked structurally rather
than by eye - `check_changelog_mirrors.py` reports the same 61 Changed / 1 Removed / 14 Fixed
sections on all five changelog pages, and no locale guide, quick start, tutorial or API page names a
removed function.

Replacing the retired name in the signature gate's falsification control exposed a hole in that
gate. The old control was `validate_against_schema(data)`, a wrong parameter list the checker
caught; the replacement `load_toml()` drops a required parameter instead, and `typed_blocks` skipped
it because an empty argument list was classified as "not a declaration" - the page passed with
`0 typed signature block(s)`. `split_parameters` now reports `[]` for `f()`, so the control exits 1
against the live `load_toml(toml_str)`, while the four locales' pages still pass: every
zero-parameter declaration they carry names a callable that really takes none.

Counts: 748 Rust tests, 1,821 Python test functions, 9 fuzz targets over 168 seeds, 76 Unreleased
entries per mirror.

### (ev) The Ir gate could say "build failed" but not why (2026-10-21)

`scripts/ir_gate.py` builds both harnesses with `cargo bench --no-run --message-format=json`, and
on failure printed `res.stderr[-2000:]`. Under that message format rustc's diagnostics arrive on
stdout as `{"reason":"compiler-message","message":{"rendered":...}}` records and stderr carries
cargo's summary, so the gate's whole account of a compile error was one line, measured on
2026-10-21:

```text
build failed:
error: could not compile `pyrs-yaml` (lib) due to 1 previous error
```

The cause - `rust-lld: unable to find library -lpython3.12`, the binding harness linking Python,
which is exactly what a host without `libpython3.12-dev` produces - appeared only when the same
command was rerun without `--message-format=json`. A gate whose failure message stops at "could
not compile" hands every reader a reproduction to run, and that reproduction is the gate's own
question.

`build_diagnostics(stdout, stderr)` now parses the build stream, joins the `rendered` text of every
`compiler-message` record, and falls back to `stderr[-2000:]` only when stdout carries none. The
fallback is not decoration: the gate can also fail before rustc runs, and a mistyped `-p` measured
`error: cannot specify features for packages outside of workspace` with no stdout diagnostic at
all.

Mutation evidence. Returning `stderr[-2000:]` unconditionally made the pytest run for
`tests/test_ir_gate_diagnostics.py` report 2 failed / 1 passed: the two stdout tests failed and
the fallback test passed, which is the correct result for a case built with no stdout diagnostics.
Restored, the three pass. The end-to-end path was read rather than assumed: the deliberately
broken build printed a `build failed:` header followed by `error: cannot specify features for
packages outside of workspace`, and the full gate (22 scenarios, `RUSTUP_TOOLCHAIN=1.97.1` on
WSL) printed `OK: every scenario within tolerance of the committed baseline`.

Counts: 748 Rust tests, 1,824 Python test functions, 9 fuzz targets over 168 seeds, 77 Unreleased
entries per mirror.

### (ew) The YAML engine was the last crate that could not be built without an OS (2026-10-21)

`pyrs-ast`, `pyrs-event`, `pyrs-json`, `pyrs-schema` and `pyrs-toml` are `#![no_std]`; the YAML
engine was not, even though its parser dependency `granit-parser 2.0` already builds without std.
The claim was therefore false, and the `no_std` gate could not see it: `tests/test_no_std_gate.py`
derives the foundation from the crate roots that declare the attribute, and `pyrs-yaml-core` did
not declare it. A crate that never joins the group is a crate the gate never asked about.

**What `std` was actually used for.** Three surfaces, each for a reason that is not the engine's:
`i18n` (rust-i18n reads locale files through std and owns a process-global locale),
`parser::yaml::registry` (`LazyLock<RwLock<SchemaRegistry>>` — a global mutable registry) and
`parser::yaml::schema_language` (regex). The parser, the serializer, the edit layer, the splices
and the tag handling were already std-free in substance, so the change is a feature split rather
than a rewrite. The `std` feature stays on by default, which is what keeps the binding, the CLI
and the benches on the API they had.

**The hasher was the first real cost.** `alloc` carries no hasher, and the parser's duplicate-key
detection keeps the scalar keys it has already seen in a `HashSet<Arc<str>>`. The crate's own
`NodeMap` — `IndexMap` over the fixed-seed `FxHasher` the AST already uses — needs no entropy, but
it costs one allocation per mapping where a `HashSet` costs one table per mapping: `IndexMap`
allocates an entries vector *and* an index table. The first `no_std` run of the allocation gate
said so plainly: `parse_medium made 74 allocations, 7 past the committed ceiling of 67`. The fix
is a cfg-split rather than a blanket swap, because the shipped artifact is the std build: it keeps
its `HashSet` and its committed line (67 allocations / 24 512 bytes, unchanged), and the `no_std`
build carries its own measured ceiling (74 / 24 952) with the reason written next to it.

**The public API had to change with it.** `event_to_stream_event` and `resolve_anchor_name` took
`&mut std::collections::HashMap<usize, String>`, and the merge resolver's anchor table was a
`HashMap<String, _>`. A `#![no_std]` crate cannot name a `std` type, so both became `NodeMap` —
the same `IndexMap` over `FxHasher` the AST uses, which also makes an anchor's order its insertion
order. The Python binding's own anchor map was changed to match; it is the only caller outside the
crate, and the workspace suite caught the mismatch as a compile error rather than a silent shape
change.

**Measured effect, which was not the point but is the record.** The Ir gate on WSL with the
baseline's pinned `rustc 1.97.1` moved `parse_anchors` -2.36% and `parse_inline_merge` -2.32% —
the two scenarios whose anchor tables changed — with `parse_medium` -0.15%, `parse_small` -0.21%,
`parse_block` -0.16% and `stream_medium` -0.12%; the gate printed `OK: every scenario within
tolerance of the committed baseline`. The two scenarios more than 2% below their floor printed
`FLOOR_STALE`: the floor is stale in the *fast* direction, and the remedy is the dispatch job that
re-measures on the enforcing image, not a local `--update`, which would move the floor onto this
host. The local reading is on WSL rather than the enforcing image, so it is a direction, not the
verdict — the same boundary this ledger has drawn for every other Ir reading.

**The gate now covers it.** `ci.yml`'s three `no-std-check` steps — the bare-metal build for
`thumbv7em-none-eabi`, the host unit tests, and the std re-check — all name `pyrs-yaml-core`, and
the negative control in `tests/test_no_std_gate.py` grew with it: the set the guard reports as
"compiles without std but is not re-checked with it" now has four members.

**A stale number corrected while the file was open.** The 2026-10-16 review note in `ROADMAP.md`
priced the message catalog's replacement surface as "421 `t!(..)` call sites and four
locale-parity gates". Measured on this tree: one `t!` call site
(`crates/pyrs-yaml-core/src/i18n.rs:106`, the `rust_i18n::t!(key)` inside `format_message`), one
external caller of `format_message` (`crates/pyrs-yaml/src/py/convert.rs:16`), four catalogs of 32
keys each, and one locale-parity gate (`scripts/check_i18n.py`) — the only script in the tree that
reads `i18n/locales`. The decision (defer, price on record) is unchanged and so is its trigger; the
surface it named was not the real one, and a review note that names a surface three orders of
magnitude too large is worse than no note. The `no_std` work gives that row a second reason to be
revisited that the 2026-10-16 note could not have had: `rust-i18n` is now the one dependency that
keeps the message catalog out of a `no_std` build, and the catalog is the only thing the `no_std`
build of this crate loses that the std build has.

**What a `no_std` caller gets, and what it does not.** The engine's parser, serializer, edit and
splice layers work on a bare-metal target. The three std-only surfaces are absent there: there is
no translated catalog (the localized messages were never in the core's parse path — they are
produced at the Python binding boundary), no global schema registry (a caller selects a schema by
constructing the `pyrs_schema::types::Schema` variant), and no regex-backed user-schema language.

Verified: the bare-metal build for `thumbv7em-none-eabi`, `cargo nextest run -p pyrs-yaml-core
--no-default-features` 378 tests, `cargo nextest run --all` 748 tests, `cargo clippy --all
--all-targets -- -D warnings`, `cargo clippy -p pyrs-yaml-core --all-targets
--no-default-features -- -D warnings`, `uv run pytest tests/test_no_std_gate.py` 6 tests, and the
Ir gate's six YAML scenarios.

Counts: 748 Rust tests, 1,817 Python test functions, 9 fuzz targets over 168 seeds, 79 Unreleased
entries per mirror.

### (ex) The documents that describe the no_std foundation were one crate behind it (2026-10-21)

The foundation was six crates after the YAML engine joined it (PR #571) - `pyrs-ast`, `pyrs-event`,
`pyrs-schema`, `pyrs-json`, `pyrs-toml` and `pyrs-yaml-core` - and the set
`tests/test_no_std_gate.py` derives is exactly those six. Three live documents still said five:

- `docs/dev/boundaries.md` enumerated the foundation in prose ("the `no_std` job builds `pyrs-ast`,
  `pyrs-event`, `pyrs-schema`, `pyrs-json` and `pyrs-toml` for `thumbv7em-none-eabi`") and pointed
  at "`AGENTS.md`'s `no_std` story" - a story `AGENTS.md` never had. Measured on the pre-change
  file: zero occurrences of `no_std` in it at all.
- `ci.yml`'s two comments said "the same five crates" and "all five compile", while the `-p` lists
  below them named six.

Nothing measured any of it. The gate compared the sources against the workflow's `-p` lists and said
nothing about the prose, so the prose was free to drift while the code stayed correct - and a
comment is not a gate.

**The fix.** `AGENTS.md` gained the foundation story it never had; `docs/dev/boundaries.md` names
all six and now points at a paragraph that exists; the two `ci.yml` comments describe the mechanism
rather than a number ("a crate joins the group by declaring `#![no_std]` in its own root"), because
a number in a comment is exactly what went stale.

**The gate.** `tests/test_no_std_gate.py` now asks a third question: the crates the live documents
name. A document's `no_std` story is the set of blocks that mention `no_std`, and a block ends at a
blank line *or* at a markdown list item - `AGENTS.md` writes its facts as adjacent bullets with no
blank line between them, so a paragraph-only split would have fused the whole list into one block
and read every crate the file names as part of the story. The crates named in those blocks must
equal the derived foundation in both directions: a missing crate is a stale list, an extra one is a
claim nobody can check. A second test resolves "`X.md`'s `no_std` story" and requires the target to
exist and to carry a `no_std` block, so the sentence that pointed at nothing cannot point at nothing
again.

**History is exempt, and that is the point.** `CHANGELOG.md` and its four mirrors and this ledger
are dated records; they may name a five-crate foundation, because that is what was true when they
were written. This is the rule `scripts/check_hole_claims.py` already applies to a closed quality
hole: the instruction files are present tense, the dated records are history.

**Two falsification probes, because a gate that has never been seen to bite is a comment.** Putting
the stale five-crate enumeration back into `docs/dev/boundaries.md` made
`uv run pytest tests/test_no_std_gate.py` exit 1 with "docs/dev/boundaries.md does not name
['pyrs-yaml-core'] in its no_std story"; deleting the `AGENTS.md` bullet made it exit 1 on
`test_the_no_std_pointer_resolves`. Both files were restored byte-identically afterwards.

Verified: `cargo nextest run --all` 748 tests, `uv run pytest tests/ -q` 4043 passed and 11
skipped, `uv run pytest tests/test_no_std_gate.py` 12 tests, `prek run --all-files` all hooks
passed. The document gates agree too: `scripts/check_changelog_mirrors.py` five mirrors in sync,
`scripts/check_cjk_localisation.py` 123 pages in their own script,
`scripts/check_doc_wrapping.py` clean on the eight governed pages, and
`scripts/quality_matrix.py --check-rows` agreeing with the count row. The command in `ci.yml`'s
host step is this one, and it ran 658 tests - 34 / 7 / 83 / 43 / 94 / 397 by crate, doctests
included:

```text
cargo test --locked --no-default-features \
  -p pyrs-ast -p pyrs-event -p pyrs-schema -p pyrs-json -p pyrs-toml -p pyrs-yaml-core
```

Counts: 748 Rust tests, 1,830 Python test functions, 9 fuzz targets over 168 seeds, 80 Unreleased
entries per mirror.

### (ey) The rule language was retired on paper while it kept shipping (2026-10-21)

Two surfaces wore the name "schema". The *validator* this repository invented - `TypeSpec`,
`ValidateKind`, `ValidateRule`, `SchemaValidationError`, `validate_against_schema()`,
`validate_against_registered_schema()` - and the *language* it read: `parse_schema_yaml()`,
`RuleResolver`, `YamlTypeKind`, the `rules:` grammar that maps scalar patterns to YAML types.
PR #517 retired the first on 2026-10-21, and `QUALITY_MATRIX.md` recorded that retirement as
"its Rust entry points are deleted". Measured on the tree the same day, the second half is alive
and shipped: `pyrs-yaml-core` still exports `parse_schema_yaml`, `RuleResolver` and
`YamlTypeKind` (`crates/pyrs-yaml-core/src/parser/yaml/mod.rs:26`), the binding still calls the
parser (`crates/pyrs-yaml/src/py/functions.rs:741`), the Python surface still ships
`register_schema()`, `load_schema()`, `list_schemas()` and `YAML(schema=...)`, and four Python
suites, one property test and a fuzz target still exercise the grammar.

The consequence was a measurement that could not be true in both directions. The matrix's
per-format table derived exactly four surface rows while five shipped, and its schema paragraph
described the language's entry points as deleted. The blind-spot table's schema row closed its
hole with that claim, and the registry read `holes: []`. But nothing measured the language at
all, so its authority cell was never asked the question that matters: the `rules:` grammar was
invented here, so no specification defines it and no second implementation exists to disagree
with it. The unit, property and fuzz tiers all ask the engine what it believes, and the matrix
had no way to say so.

**The fix: the row is derived, not written.** `scripts/quality_matrix.py` gained per-row scoping

- `unit_scope`, `prop_scope` and a `writer` flag - because a surface that shares a crate with a
format must not be credited with that crate's tests. Measured on the tree: the language's unit
cell is the eleven `#[test]`s of `crates/pyrs-yaml-core/src/parser/yaml/schema_language.rs`, not
the 407 the crate holds; its property cell is the one function that names it,
`prop_schema_parse_no_panic`; its fuzz cell is `parse_schema`; its round-trip and parity cells
read `n/a` - it resolves scalars and owns no serializer, and no second implementation of the
grammar exists to compare against. Its authority cell reads `no`, which is the honest answer, and
the measurement derives the hole `authority-tier:schema-rule-language` from that `no` rather than
from prose.

**The registry.** `.ci/quality-holes.json` carries the hole with an exit criterion: either a
`tests/test_*_suite.py` or `tests/test_*_spec_table.py` whose name carries the surface's fragment
asks the `rules:` grammar of something this repository does not own, so the authority cell reads
`yes`, or the surface leaves the tree and the row leaves with it.
`tests/test_quality_matrix.py` fails when the measured set and the declared set differ in either
direction, so a new blind spot cannot land silently and a closed one cannot stay in the registry
as decoration.

**The document.** `QUALITY_MATRIX.md` now says there are two schema surfaces and that only one of
them is judged from outside; the blind-spot table's false clause is corrected, and the row names
the hole. The retired half keeps its own statement, because that half is true: the validator's
symbols are gone.

**The gate.** A new falsification control,
`test_the_retired_validator_is_gone_and_the_rule_language_is_not`, scans the Rust sources, the
Python package and the shipped stub and asserts both directions: `TypeSpec`, `ValidateKind`,
`ValidateRule`, `SchemaValidationError`, `validate_against_schema` and
`validate_against_registered_schema` appear nowhere, while `parse_schema_yaml`, `RuleResolver`
and `YamlTypeKind` do, and the binding still reaches the parser. The page can no longer drift
back to "deleted", and it cannot drift forward into claiming the validator is still there.

Verified: 748 nextest (`cargo nextest run --all`, none failed) and 4044 Python tests pass with 11
skipped; `tests/test_quality_matrix.py` alone is 59. `prek run --all-files` passes every hook, and
the doc gates answer for the prose: `check_changelog_mirrors.py` reports all five mirrors
structurally in sync, `check_cjk_localisation.py` 123 pages in their own script,
`check_doc_wrapping.py` clean on the six governed files, `check_hole_claims.py` names only holes the
matrix measures, and `quality_matrix.py --check-rows` finds every count in the tier table to be what
the probe measures. The `str | None` annotations that ruff's UP045 rule forced into `FormatRow`
were checked against the floor rather than assumed: under Python 3.8.20 the module imports and the
five rows are built, the annotations staying unevaluated `ForwardRef`s.

The gate was seen to bite in both directions before this entry was written, because a gate that has
never been seen to bite is a comment. A probe file carrying `pub struct TypeSpec;` made the test
fail with "'TypeSpec' is contained here"; a second probe that renamed `parse_schema_yaml` to
`retired_probe_name` in the binding made it fail with "the binding still reads a `rules:` block".
Both probes were removed afterwards and the binding was restored byte-identically - `sha256sum -c`
reports `crates/pyrs-yaml/src/py/functions.rs: OK`.

### (ez) The note flag answered two questions, so the tree and the stream could not both be right (2026-10-21)

`pyrs-event`'s `StreamEventType::Comment` carried `text` and one boolean, `standalone`. That boolean
answered two different questions at once: does the note own a line of its own in the source - a fact
about the bytes - and which slot of which node does the note live in - a fact about the tree. One
boolean cannot answer both, and the shape made the two walkers disagree about the same note.

The case that proved it is a document whose trailing note has a line of its own. The tree files it
in the root's inline slot: the writer cannot hang an inline note on a block container's last line,
so it spills the note onto its own trailing line, and the reader reports it back from there. The
stream reported "owns a line", because that was the source fact. The tree said "leading list", the
stream said "own line", and for that one note neither walker was wrong about its own question - so
no comparison could decide whether they agreed.

**The fix: the source fact and the tree's placement become two fields.** The comment now carries
`text`, `owns_line` and `owner`, and `owner` is a `NoteOwner` over the two axes the design named:
the direction in the consumer's build order (`following` is the node the consumer builds next,
`preceding` the one it built last) and the tree's two slots (`leading` is the node's leading list,
`inline` the one inline slot). `pyrs_ast::ast::Comment` did not move - the tree *is* the placement,
so the AST keeps its own `standalone` and the vocabulary gained a field rather than the tree losing
one.

All three producers report it. The YAML placement machine
(`crates/pyrs-yaml-core/src/parser/stream.rs`) names the slot it filed the note in. The JSON
producer (`crates/pyrs-json/src/parser.rs`) carries the note's offset with the record, so the source
is not re-scanned to answer "does it own a line". The TOML producer
(`crates/pyrs-toml/src/events.rs`) reads the same fact back off the bytes it already holds -
whitespace up to a line break means the note had the line to itself - which is why a note the tree
files *above* a key can still fail that test: a `# ...` written after a comma shares the line with
the value before it, and TOML files it above the *next* element.

**The one shipped value that moved.** The design said no shipped Python behaviour moves, and that
holds for YAML and for JSON - their old flag was already the source fact, so `style` reads what it
read. It does not hold for TOML, whose old flag answered the slot question. A TOML note that shares
its line with the text before it but still leads the next element used to read
`style: "standalone"`; it now reads `style: "inline"`, the source's own answer, and the placement
the old value carried is the new `owner: "following_leading"`. The Python dict keeps `style` and
gains one key, so nothing else moved, and the value that moved is pinned in
`tests/test_streaming.py` rather than described in a changelog.

**The gate.** The exclusion is gone: the two replay tiers -
`crates/pyrs-json/tests/events_replay.rs` and `fuzz/fuzz_targets/json_events_replay.rs` - compare
the note's text, its source fact and its owner instead of its text alone, and the fuzz tier adds the
coherence the JSON producer's own rule implies: a note that had a line of its own leads, and a note
that rides the inline slot shared its line with the text before it. A new tier file,
`crates/pyrs-yaml-core/tests/event_notes.rs`, walks the tree and the stream over the same documents
and asks both about both facts. `the_two_walks_answer_the_same_two_questions` is the case that split
the flag apart; `every_note_the_tree_files_is_reported_with_the_same_owner` is the table of fourteen
documents where the two walks must agree on the slot and on the direction; and
`the_notes_the_tree_drops_are_the_documented_ones` names the notes the tree itself drops - a second
trailing note, and the note an alias cannot hold - so the difference between the two walks is a
recorded policy rather than a disagreement. The JSON and TOML tiers carry the same two invariants in
their own files, and `a_jsonc_document_rebuilds_itself_from_its_own_events` folds a JSONC document's
own events back into a tree and asks whether the strict writer prints the same text the parser read
and whether the folded tree equals the parsed one, decorations included. The fold is what the split
was for: it has to know which slot each note belongs in, and `owner` is the only thing that tells
it.

**The cost, read from the gate.** The three scenarios that carry notes were measured on the
enforcing image — the pull request's own `ir` job on `ubuntu-24.04`, with the baseline's pinned
compiler (`rustc 1.97.1`, the toolchain `.ci/ir-baseline.json` records) — each the largest of three
callgrind runs:

| scenario | this tree | committed floor | move |
| --- | --- | --- | --- |
| `stream_medium` | 252,210,269 | 251,312,293 | +0.36% |
| `from_jsonc_medium` | 94,578,429 | 94,502,402 | +0.08% |
| `to_toml_medium` | 26,827,006 | 26,827,070 | -0.00% |

The job printed "OK: every scenario within tolerance of the committed baseline" and concluded
`success`. All three are inside the baseline's 0.5% tolerance, and `to_toml_medium` - the scenario
that carries the TOML producer's new work - does not move at all. A reading taken earlier on a WSL
host, against the same pinned compiler, agreed with this one to eight instructions on
`stream_medium` (252,210,277 against 252,210,269) and to 0.006% on `from_jsonc_medium` (94,572,424
against 94,578,429): the host moved the number by less than the movement the split is judged for,
which is the cross-host agreement `docs/dev/perf.md` measures. The cost of the split is not free but
it is bounded: the placement machine is still skipped on a document with no note, and that gate was
measured at 483 instructions per parse against the 1,624 the machine would cost, so the split paid
for the scan it skips.

Verified: 756 nextest (`cargo nextest run --all`, none failed, none skipped) and 4052 Python tests
pass with 11 skipped and 117 deselected; `cargo clippy --all --all-targets -- -D warnings` exits 0
and `prek run --all-files` passes every hook. The doc gates answer for the prose:
`check_changelog_mirrors.py` reports all five mirrors structurally in sync, `check_doc_wrapping.py`
is clean on the ten governed files, `check_hole_claims.py` names only holes the matrix measures,
`quality_matrix.py --check-rows` finds every count in the tier table to be what the probe measures
(the row moved from 748/1,831 to 756/1,834, written by the sanctioned `--write-rows`), and
`check_stub_drift.py` reports that `python/pyrs_yaml/pyrs_yaml.pyi` matches the regeneration route.

The gate was seen to bite before this entry was written, because a gate that has never been seen to
bite is a comment. Flipping one `NoteOwner::FollowingLeading` to `FollowingInline` in the YAML
producer made two of the three new `event_notes` tests fail with "is filed in one slot and reported
in another". The producer was restored byte-identically (`sha256sum -c` reports
`crates/pyrs-yaml-core/src/parser/stream.rs: OK`) and the three tests passed again - after the
restoration was followed by a rebuild, because restoring a file's bytes does not make cargo rebuild
it, and a green run against the mutated binary would have been a false green.

### (fa) The release was the first gate that read the wheel legs' requirement file (2026-10-22)

The v0.18.2 tag fired Publish and five wheel legs reddened before a wheel uploaded. Not a build
failure: four collection errors. `tests/test_toml_writer_suite.py` and `tests/test_toml_crosslib.py`
import `tomlkit` at module scope, `tests/test_json5_spec_table.py` and `tests/test_json_crosslib.py`
import `json5`, and `.ci/requirements-test.txt` - the only requirement file `publish.yml` and
`.ci/pytest-wheel.sh` install - named neither. The `Release` job was skipped, so nothing reached
PyPI and the tag cost a re-tag rather than a withdrawn release.

The pair arrived 2026-10-18 with `02f7bbf7`, "test(toml): judge the TOML writer by parsers we do not
own", and no gate could have seen it: the PR matrix installs the project's dev groups, and
`pyproject.toml` does declare `tomlkit>=0.12` and `json5>=0.17.3` there. Only the release reads the
thin file. A missing *test* dependency was therefore discoverable exactly where it was most
expensive, which is the definition of a defence that is a report rather than a gate.

Reproduced before fixing it, by blocking the two names in `sys.modules` and running those four
files: the same four `ModuleNotFoundError` collection errors Publish printed, in 0.41s, on this
host.

**The fix is two lines and the guard is the point.** `tomlkit` and `json5` are declared in
`.ci/requirements-test.txt`, and `scripts/check_wheel_requirements.py` now judges every module-scope
import in `tests/` against that file: standard library and this repository's own modules are
excluded, anything else must resolve to a distribution the file names. It is wired into `prek.toml`
(`wheel-requirements`, fires on `.ci/requirements-test.txt` and `tests/**/*.py`) so the drift is
caught at the edit that causes it, and into the matrix through
`tests/test_wheel_requirements_gate.py` (7 tests).

**The gate was seen to bite.** Deleting the two lines and re-running it prints the four findings by
file and name, and exits 1; an import outside the map is reported as "named nowhere" instead of
being ignored, so the vocabulary cannot narrow silently. Two shapes must stay quiet and are pinned:
an import inside `try:` is the fallback AGENTS.md asks for around optional cross-library readers,
and charging those would push every optional import toward a bare import to keep the hook green - a
wider hole than the one being closed.

**What the guard does not see.** It judges imports at module scope, not symbols a test reaches
through another module, and it charges only what the suite imports - not what a leg's `-k`
expression selects. A distribution whose import name differs from its distribution name is carried
by an explicit map in the script, so a new authority must be named there deliberately; that is the
seam an unlisted wheel-leg dependency could still slip through.

### (fb) A design document's "landed" now names the code that makes it true (2026-10-22)

`docs/dev/event-stream.md` carried "What the note flag split answered (2026-10-21, decision 6
landed)" against a trunk whose `.rs` history never contained `owns_line`
(`git log origin/main -S owns_line -- '*.rs'` returned nothing, and
`crates/pyrs-yaml-core/tests/event_notes.rs` did not exist there). The prose arrived through the
docs-only commit `0a074148`; the implementation was a local change that never opened a pull request.
Every existing gate was satisfied: `scripts/check_hole_claims.py` reads the instruction files
against the measured matrix, `tests/test_quality_matrix.py` reads the registry against the
measurement. A design document's present tense was the artefact outside all of them.

**The rule.** A heading in a live document (`docs/dev/*.md`, derived by glob) that says something
`landed` or `shipped` carries, on the next non-blank line, `<!-- landed: <path>[::<symbol>] -->`. A
path must be a file in the tree; a `::symbol` must appear in it as a whole word. Locks are accepted
under headings that do not need one, and an unresolvable lock is a finding wherever it sits - so
adding evidence is always legal and stale evidence is always caught. `docs/dev/quality-ledger.md` is
exempt by name because it is a dated record; the locale changelogs for the same reason.

**Measured on this tree.** `python scripts/check_landed_claims.py` reads 3 live documents, finds 3
claim headings and 6 locks, and exits 0. Three claim headings were wrapped
(`step 2 shipped tree-free`, `decision 6 landed`, `decision 3 landed`) and three more carry evidence
without being required to (`step 4` replay, the TOML producer, the Python surface). Scope was
calibrated before the rule was written: on the tree before the locks existed the checker reported
exactly those 3 headings and nothing else, no false hit in `boundaries.md` or `perf.md`.

**Seen to bite.** Two ways, both from the pre-#589 state. Deleting a lock leaves
`## What the note flag split answered (2026-10-21, decision 6 landed)` bare, which is exit 1 (pinned
by `tests/test_landed_claims_gate.py::test_the_heading_that_rotted_is_caught`). And a lock naming
the symbol the docs claimed resolves on today's tree but would not have resolved on `eb1e9a44`,
where `crates/pyrs-event/src/lib.rs` contains `NoteOwner` 0 times (measured with
`git show eb1e9a44:… | grep -c`); today it appears 24 times. A symbol that disappears is the same
finding, which is why the hook fires on `crates/**/*.rs` and the stub, not only on documents.

**Not covered.** The lock proves the named artefact exists, not that it behaves as the paragraph
claims - that is still the job of the test tiers each entry names. A claim written without the words
`landed` or `shipped` needs no lock, so the convention depends on design docs using the heading
shape the checker reads; that seam is named here rather than hidden, and `event-stream.md` is the
only live document that carries the shape today.

### (fc) A fold the gate never asks about (2026-10-22)

Every released section of the five release-note pages is folded - `<details markdown="1">`, a
`<summary>` of the release's themes, and a `</details>` before the next version heading - from
v0.12.x through v0.18.0. `v0.18.1` and `v0.18.3` arrived without one, on all five pages, and every
gate stayed green: `scripts/check_doc_metadata.py` asks whether a fold carries the `markdown`
attribute the renderer needs and never whether a fold exists, so the newest entries render unfolded
above twenty folded ones.

**The rule now reads the shape, not only the attributes.** A `##`/`###` heading of the form
`[vX.Y.Z]` on a release page must be followed - blank line allowed - by `<details markdown="1">`,
and the first non-blank line after that must start with `<summary>`; opens and closes are counted
per page. `CHANGELOG.md` joined the gate's default scope even though it carries no front matter by
design, because it is the canonical English page and a rule that skipped it would skip the file that
rotted.

**Measured.** Before the repair the scan reported exactly 2 unfolded release headings in each of the
five pages (`v0.18.3`, `v0.18.1`) and nothing else - no historical release needed an exemption.
After: `unwrapped=[]` in all five, with `27` folds open and `27` close per page, and
`python scripts/check_doc_metadata.py` exits 0 across 165 pages. `tests/test_doc_metadata_gate.py`
grew from 6 to 11 tests, the four new falsifications being the unfolded page, a fold with no
`<summary>`, an unclosed fold, and the root page's exemption-from-metadata that must not become an
exemption-from-folds.

**Not covered.** The rule reads headings and tags, not the renderer: a fold that parses but wraps
the wrong range (a section that swallows the next heading) is caught only by the open/close balance,
which sees counts rather than nesting. And the ceiling the release notes grew against - entry
length, median 10 lines against history's 3.5-7 - is stated in the writing contract rather than
gated, because history itself runs up to 19 lines and a number that old would have to be exempted to
pass.

### (fd) A cancelled stream is a property of the producer, not of the delivery loop (2026-10-22)

`parse_stream(..., on_event=...)` read as an incremental API and behaved as a replay: every format
collected its events into a `Vec` and only then called the callback, so a `False` at the first event
paid for the entire scan, and a syntax error after the stop was reached and raised. Ruling B asked
for one event at a time without an async runtime, which makes cancellability a producer fact: the
producer owns the loop, so only the producer can decline to read the bytes.

**The vocabulary is a `ControlFlow` answer.** `pyrs_event::EventSink` returns `ControlFlow<()>` -
`Break` means "do not emit the next event" - and `Vec<StreamEvent>` implements it by collecting, so
the existing entry points keep their signatures. The impl for
`F: FnMut(StreamEvent) -> ControlFlow<()>` is the one producers themselves take; a blanket `&mut S`
impl was not added because it conflicts with that one.

**Each producer stops what it can.** `pyrs_json::parse_stream_with_options_to` hands the event to
the sink inside the walk, so `Break` exits before the next token is read and `{"a": 1, "b": oops}`
stopped at the fifth event never reaches `oops`. `pyrs_toml::parse_stream_to` projects from the
intermediate representation its scan already built, so `Break` stops the emission while a refused
document still raises - the refusal is a fact of the parse, not of the delivery. YAML's text path
remains a push adapter over granit and cannot abort; `load_stream` is its incremental surface.

**The binding keeps the GIL released between events.** The producer runs under `py.detach(|| ...)`
and the sink re-attaches with `Python::attach` for each callback. Batching was implemented and
removed: with a batch of 64 the callback's `False` is not observed until the batch fills, which
delays the stop past the scan it was meant to avoid, so the stop is observed at the event it
answers. A callback `PyErr` is stored on the sink, the producer sees `Break`, and the callback error
is returned before the producer's parse error.

**Measured.** `tests/test_streaming.py` reports 52 passed, seven of them new: JSON stops before the
error after the stop; a refused TOML document reaches no callback and raises; a callback exception
outranks the parse error behind it; a callback that accepts everything sees exactly the iterator's
events (80 values); a stop keeps the 70-event prefix; the TOML callback sees the collected stream
(70 keys); the YAML text producer delivers no event from a document it cannot parse. Rust: 608
tests passed across `pyrs-event`, `pyrs-json`, `pyrs-toml` and `pyrs-yaml-core`, including the JSON
and TOML tests that compare the `_to` route with the collecting route event for event, and the two
`EventSink` falsifications - a `Vec` sink that broke would truncate silently, and a closure impl
that swallowed or inverted the answer would turn a cancelled stream into a complete one. The
collecting entry points delegate to the `_to` ones with a `Vec` sink, so the two routes agree by
construction and the producer tests pin the delegation rather than a hand-copied fixture. The Ir
gate reads all 22 scenarios within tolerance; the largest move is `stream_medium` +0.44% against the
0.50% line, and that scenario is YAML's stream walk, which this change does not touch — CI's
enforcing image decides it.

**Not covered.** Cancellation is not a memory bound: JSON stops its walk, but the events it already
emitted are still held by the caller, and TOML's intermediate representation is built before any
callback runs - a consumer that needs the scan itself to be constant-memory still needs a producer
that does not project a tree, which JSON's is and TOML's is not. The YAML callback path is eager, so
`False` there is a delivery stop rather than a scan stop; the docstring says so and no test claims
otherwise.

### (fe) A stream event now names its bytes, and the index that pays for it forgets what it passed (2026-10-23)

`line` and `column` locate an event in text a consumer has to keep. Ruling A asked for the byte
range instead, so an event-sourcing consumer can slice the source or resume from an offset without
holding the document. `StreamEvent` now carries `offset` and `end` on every event of every producer,
and the Python dicts of `parse_stream`, `load_stream` and `load_stream_file` gain the two keys.

**The contract is a measurement, not a definition.** The five formats disagree about what a span is,
and each disagreement is written down where it is produced rather than left for a consumer to
discover. A quoted scalar's range covers the quotes the source typed (`'y'`, not `y`); a YAML block
scalar's range is its body including the trailing newline and excluding the `|` and the indentation;
a flow delimiter is exactly one byte and a block container is an empty range, because no character
opens it; `document_start` covers `---` when the source wrote one. Where a producer records only a
position, `end == offset` is the answer and the docs say so instead of inventing a length: every
TOML event, because the IR keeps starts and a length would need a second scan of the token, plus
YAML's block containers and stream markers; a document marker covers what the source wrote and is
empty otherwise. `tests/test_streaming.py` pins the shapes by slicing the source at the reported
range and comparing the slice to the token the source typed.

**The sparse index is the whole issue on the incremental path.** granit reports char indices, and
the string path already had the conversions: it builds a char→byte table only when the source is not
ASCII. `load_stream` has no source to build a table from, so `ChunkCharIter` records one
`(chars, bytes)` mark per multi-byte character while it decodes and nothing at all for ASCII - then
`YamlStream` reads the marks under the same mutex the decode takes. Recording for the whole stream
would have made the feature's memory O(non-ASCII characters), which contradicts the bound the
streaming guide states for exactly this entry point. The marks therefore advance with the parser: a
lookup turns the two ends of the current span into bytes, then forgets everything at or before the
span's *start*, keeping a base pair that answers the dropped prefix
(`base.bytes + (q - base.chars)`). Pruning to the start and not the end is deliberate - a flow
collection's span can cover its children, and pruning past a child's position would answer the child
from the wrong base. What remains is one mark per multi-byte character in the text between the last
event and the parser's read-ahead, which is bounded by the largest token, not by the document.

**Measured.** Rust: the whole workspace's nextest run. Python: `tests/test_streaming.py` collects
69 items, seventeen of them new. The parity test feeds `load_stream` one byte at a time - the chunk
boundary that can split a wide character - and now also a second document after a flow member, since
that is where a wrong base would surface; it compares the whole event list against `parse_stream`'s.
A falsification run replaced `byte_at` with the identity and the CJK case reported `end: 3` against
`9`, so the parity test bites rather than agreeing by construction.

The index has no Rust unit test, and the reason is a gate rather than an oversight: `ByteMarks`
lives in the binding crate, which holds no Rust test tier (`scripts/check_rust_python_decoupling.py`
rule R2, the measured decision in `docs/dev/boundaries.md`), so its evidence has to come from the
Python surface. The lookup invariant is what the parity test checks - two routes over the same
multi-byte document must report the same bytes. Pruning is not testable from there: keeping every
mark answers every lookup correctly, so no observable assertion separates a bounded table from an
unbounded one.

**What it costs.** The Ir gate reads `stream_medium` +1.18% (251,020,333 → 253,988,248, three
samples identical, enforcing image) against the 0.50% line. The movement is the payload: two `usize`
fields on a struct pushed once per event, so it pays in the writes and again in every `Vec` growth
copy. The same refresh rewrote all 22 floors, and the other 21 moved by at most 0.13%
(`to_python_small`), so the line is not covering a second change of this size. This is the
intended-cost channel `ir_gate.py` describes, and the floor is moved by the enforcing-image refresh
with `accept_drift=stream_medium`, not by widening the line.

**Not covered.** `end == offset` means TOML's stream cannot report a token's extent, and no test
claims otherwise. The marks bound is the largest *token*, not a constant: one 100 MB scalar of CJK
text still costs a mark per character while that token is being reported. And the range is source
bytes, not the decoded value - a consumer that wants the decoded scalar still reads `value`, which
is the split the stream already made.

### (ff) The position index was quadratic on a document without newlines (2026-10-23)

`LineTable` answered each event's position by binary-searching the line starts and then counting
characters from that line's start. The comment above it called this "a binary search plus one char
count", which is right per event and wrong per stream: a producer reporting `k` events over a line
of `n` bytes pays `k * n`, and a minified document is one line. Both the JSON and the TOML producers
report positions in source order, and the incremental path asks for one on every event, so the cost
is real work in the shipped surface rather than a theoretical worst case.

**Found by a measurement, not by a test.** Decision C's feasibility probe timed the serial event
stream over jsonl-style input and over the same records in one array. At n = 16,000 records
(1.52 MB) the newline-per-record document streamed in 22 ms while the single-line one took 4,890 ms,
and each doubling of the single-line input quadrupled the time while the other series stayed linear.
The diff between them is the newlines: same records, same events, same walk.

**The fix is a cursor.** The table now remembers the last answered `(pos, line, column)` and walks
only the bytes between one query and the next, stepping whole UTF-8 characters and stopping short of
a position that falls inside one. A query before the cursor - out-of-order reporting is allowed by
the type even though no producer does it today - falls back to counting from that line's start,
which costs one line and moves the cursor there. A position that repeats answers from the cursor
without walking, which matters because producers emit several events at one byte.

**The mechanism is asserted, not timed.** `visited()` reports the characters the cursor has stepped
over, counted in every build (`cfg(test)`-only accounting would have measured a different program).
`a_forward_sweep_advances_the_cursor_once_per_character` queries a 200 KB single-line document every
7 bytes and requires the count to land between `len - 7` and `len + 64`: the upper bound catches a
sweep that re-walks, the lower bound catches an implementation that answers from the line start and
never advances at all. A falsification run forced the always-recompute path and the lower bound
printed `advanced only 0 characters across 200000 bytes`. The old implementation cannot be put
through this assertion (it had no counter), so its evidence is the paired probe above.
`a_cursor_answers_every_position_the_way_a_rescan_does` drives a forward sweep, a backward sweep and
a jump-around pass over a document with two-, three- and four-byte characters, comparing every
answer - including offsets inside a code point and one past the end - with an independent rescan.

**Measured.** On the development host, with the probe compiled in release and run three times, the
1.52 MB single-line document now streams in 18.8-19.7 ms and the newline-per-record twin in
17.7-18.3 ms; the 260x gap and the quadratic shape are gone. Rust: `cargo nextest run --all`, 766
tests pass (12 in `pyrs-event`, 186 across `pyrs-json` and `pyrs-toml`). `cargo clippy --all
--all-targets -D warnings` is clean.

**What it costs.** One `usize` compare per event and one add per character on the walk the cursor
already performs, plus the `starts` index, which was already built. The Ir gate's `stream_medium`
scenario is the authority on whether that moves the hot path, and on CI it reads 253,988,248 →
253,988,296: +48 instructions, +0.00%, under the 0.50% line for all 22 scenarios. The fixture is
newline-per-record, so that number bounds the compare on the already-linear path; the single-line
document this entry is about has no Ir scenario, and its evidence stays the wall clock above.

**Not covered.** The cursor is per table and per stream, so a producer that reports events in
arbitrary order pays one line-length rescan per backward query; none of the three producers does.
YAML computes its own positions from granit's char indices and never used this type. The probe that
found the defect is a throwaway crate under `target/`, not a committed benchmark: the permanent
evidence is the mechanism test above.

### Shipped milestone scoping (v0.11.3 → v0.12.0)

The planning tables `ROADMAP.md` carried after their milestones shipped. They stay because the
*reasons* are the useful part - which scope closed empty, which audit settled the argument, which
decision was deferred and against what date - and a plan silently deleted at release leaves the
next reviewer without the previous reviewer's evidence.

#### v0.11.3 — "Streaming Write + Process Hardening" (target: Q3 2026)

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

##### v0.11.5 — "Parser Robustness" (target: Q3 2026)

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

###### v0.11.6 — "numpy-free free-threaded wheel" (target: Q3 2026)

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

###### v0.11.7 — "CI signal hygiene" (target: Q3 2026)

> Replace the deliberately-failing `stub-build-check` CI job with static assertions that pass when
> the repo is correct (green CI), fail only on regression. Track `rust-numpy` free-threaded support
> status for re-enabling ndarray on cp314t.

| # | Item | Layer | Status |
|:--|:-----|:------|:------|
| 1 | **stub-build-check → release-guard** — replace the always-red container build with static assertions: `grep` guards `publish.yml` against `--generate-stubs`, `git ls-files` asserts `.pyi` is tracked, `test -f` checks `py.typed` | CI | ✅ |
| 2 | **Numpy free-threaded tracking** — ROADMAP.md documents `rust-numpy` free-threaded support (PyO3/rust-numpy#476) as a tracked dependency | Docs | ✅ |

**Changelog mapping**: the `[0.11.7]` entries in the repository root's `CHANGELOG.md`.

---

###### v0.12.0 — "Competitive Response" (target: Q3 2026)

> Respond to `yaml-edit` competitor features with a fast, round-trip-preserving editing story. D3
> ships the create-missing path write; D4 adds Rust-backed AST traversal.

| # | Item | Layer | Status |
|:--|:-----|:------|:------|
| D3 | **`set(create_missing=True)`** — create missing intermediate mapping keys along an edit path (`doc.set("$.a.b.c", 2)`) | Rust + Python | ✅ 2026-08-04 |
| D4 | **`doc.walk()` / `doc.scalars()`** — Rust-backed depth-first traversal yielding `Node` objects, matching `Node.walk()` semantics without per-node `to_dict()` resolution | Rust + Python | ✅ 2026-08-04 |
