# Format-neutral event stream — design and roadmap

Status: **ruled - option B, adopted 2026-10-13; step A (`pyrs-event`) merged as PR #379; step 2
(JSON producer) measured, priced and shelved the same day in the shape that taxed the tree, and
shipped on 2026-10-16 in the shape the ruling asked for: a tree-free second walk
(`pyrs_json::parse_stream`), section "What the second walk answered". Step 3 (TOML) shipped the same
day as well, and it is *not* a second walk - `pyrs_toml::parse_stream` reads the intermediate
representation the AST projection already builds, because TOML's table rules are heavy enough that
re-authoring them in a second pass would be two policies; section "What the third producer
answered". Decision 2 was answered the same day - a multi-format `parse_stream` is promised to
Python and Rust users.** The objective lists `事件流` as a core abstraction to be split into its own
crate, alongside the AST. The AST half is done; the event-stream half is this document. Decisions 3
and 4 were both answered on 2026-10-14, and decision 5 on 2026-10-16. Step 4 - the replay as a gate
rather than a test - is the section "What the replay gate answered" and the change that carries it.
Step 5 - the Python-facing surface - is the section "What the Python surface answered", shipped the
same day: `parse_stream` reads JSON, JSONC, JSON5 and TOML as well as YAML. So the *sequence* is
complete, and it is worth separating that from what is not: the vocabulary's shape is still open.
Step 4's gate answers one grammar — `pyrs_json::from_events` is the only replay reader in the
workspace — and draft PR #454 puts two shape questions up for review: decision 6, splitting
`Comment::standalone` into the two facts it currently answers, and decision 7, which records that a
YAML replay cannot rebuild `chomping` or the block indentation indicator at all, because granit's
`Event::Scalar` does not carry them and only its collection events hand over a notation the adapter
discards. "The vocabulary is neutral" is therefore proven for JSON and questioned for YAML, which is
the amount this document can honestly claim.

Every claim below is from reading the tree at `14196dc9` (2026-10-13), except the measurement
section, which reads `bdc60d74`. File and line numbers are given so a reviewer can check them rather
than take my word.

## What exists today

One event vocabulary, in the YAML engine:

- `crates/pyrs-yaml-core/src/parser/stream.rs:11` —
  `struct StreamEvent { event_type, line, column }`.
- `stream.rs:23-74` — eleven `StreamEventType` variants: `StreamStart`, `StreamEnd`,
  `DocumentStart`, `DocumentEnd`, `Scalar { value, style, anchor, tag }`,
  `MappingStart { anchor, tag }`, `MappingEnd`, `SequenceStart { anchor, tag }`, `SequenceEnd`,
  `Alias { name }`, `Comment { text, standalone }`.
- `stream.rs:1-7` — it imports `granit_parser::{Event, Parser, SpannedEventReceiver}` and std
  `HashMap`/`String`/`sync::Arc`. It is an adapter over the YAML parser's own event type, not a
  neutral one, and it cannot compile without std.
- Consumers: `crates/pyrs-yaml/src/py/streaming.rs`, `py/stream_events.rs`, `py/stream_iterator.rs`.
  One producer, one consumer, both YAML.
- Two delivery shapes already differ inside that one type: `StreamReceiver` collects every event
  into a `Vec<StreamEvent>` (`stream.rs:90`, `:118`, `:210`; `parse_stream()` is documented as
  returning a vector at `:324`), while the binding's `streaming.rs:199` holds a live `SaphyrParser`
  and pulls one `(StreamEvent, Span)` at a time. So "the event stream" is one vocabulary with two
  lifetimes, and the eager one is the crate's public `parse_stream`.

For contrast, the AST half of the same objective: `pyrs-ast` is its own crate, `#![no_std]` with a
`std` feature, and all five engines build the same `CustomNode`. That is what "core abstraction
split into a crate" looks like when it is real: **a shared type with several producers.**

## The shape problem, stated plainly

Of the eleven variants, `anchor`, `tag` and `Alias` are YAML-only, `Comment` is shared-but-different
(JSONC/JSON5/TOML all have comments, but their slot rules are the three-engine double-slot
convention, not YAML's standalone/inline placement), and `style` is *not* YAML-only: the AST already
uses `ScalarStyle::SingleQuoted` for JSON5 (`pyrs-ast/src/ast.rs:657`,
`pyrs-json/src/parser.rs:82`). So the vocabulary is genuinely half shared and half one format's
grammar - which is exactly why the neutral shape has to be designed against a second producer rather
than declared by moving the first one into a new crate.

The second producer is also where the design decisions actually live, e.g.:

- A JSON event has no anchor and no tag, but JSON5 spells non-finite floats `Infinity`, `+Infinity`
  and `NaN`, and the parser projects them onto the YAML spellings `.inf` / `-.inf` / `.nan`
  (`pyrs-json/src/parser.rs:1569-1571`). A neutral scalar event has to say whether that projection
  is part of the shared vocabulary or a format detail carried in the payload - the same question the
  TOML spelling below is, one layer up.
- A TOML event has dotted keys, which are a mapping *written* in one line. The engine already tracks
  which table levels a dotted key created implicitly (the `closed` bit each `CowTable` carries in
  `pyrs-toml/src/lib.rs`), so a stream that only emits `MappingStart`/`MappingEnd` would replay into
  the same AST and silently lose a fact the parser keeps. Either the event carries it, or the
  abstraction is smaller than the engine.
- TOML's integer spelling (`0b10`, `0x1F`) survives to the hub as a tagged plain scalar (`(cp)` in
  `docs/dev/quality-ledger.md`). A neutral event must decide whether the spelling is a payload field
  or an attribute of `Scalar`, because the format-neutral reader is the only place that knows it.

## Options

**A — Move as-is.** Relocate `StreamEvent`/`StreamEventType` to `pyrs-event`, keep the YAML adapter
in `pyrs-yaml-core`, re-export. Cost: one crate, one re-export chain, no behaviour change, the
instruction-count gate untouched. Benefit: a `no_std`-compilable event vocabulary that a downstream
Rust user can name without pulling `pyrs-yaml-core` (they cannot today: the type drags granit and
std with it). Honest label: this satisfies the *wording* of the objective without making the
abstraction neutral, and designing a shared type against one implementation is the guesswork the
`AGENTS.md` no-refactor rulings name directly. It is only worth doing as step 1 of B.

**B — Core vocabulary plus per-format payload.** `pyrs-event` holds `Event<'a>` with borrowed text
(`&'a str`/`Cow` — today the fields own `String`, so this is also the allocation decision),
structural variants only, and a `FormatMeta` payload that each engine fills (YAML: anchor/tag/style;
JSON: quote and escape source; TOML: numeric spelling and comment slots). Then JSON and TOML get
producers from their existing tokenizers, and the abstraction is *validated by replay*:
`from_events(...)` must rebuild the same AST the direct parsers build, per format, as a property
test. Cost: 4-5 PRs, and one new public surface to keep honest (stubs, docs locales, the matrix).
This is the option that makes the objective's claim true.

**C — Declare it a non-goal.** Keep the event vocabulary YAML-internal, record the ruling in
`docs/dev/boundaries.md` with the reason (one producer, half-YAML fields, no consumer asking for a
second), and stop counting the item as pending. Cheapest, and defensible: nothing in the tree or the
issue list today demands a multi-format event API.

## Recommendation (adopted 2026-10-13)

**B, but only if the maintainer wants a multi-format streaming API** — for Rust users, Python users,
or both. The deciding question is not architectural, it is product: does `parse_stream` promise JSON
and TOML events to anyone? If no, C is the honest answer and A alone would be a crate built to
satisfy a phrase. The ruling is B, so the question below (2) is the one that now shapes steps 2-5.

If B: sequence is A (move, no behaviour change) → JSON producer → TOML producer → replay property
test → *only then* the Python-visible surface. Each step gates on the committed instruction baseline
with zero upward movement, and the replay test is what makes step 2-4 reviewable rather than
hopeful: events that cannot rebuild the AST the parsers already build are the wrong events. Step A
ran that way and merged; step 2 is why the sequence now reads as it does in the next section.

## What the shared traversal costs (2026-10-13, step 2 measured and shelved)

Step 2 was written, tested and gated. It failed the gate, and the reason is not the one the pull
request claimed.

The design was the one this document argued for: one traversal, two outputs. `pyrs_json::to_events`
walked through the same `value` / `object` / `array` that build the AST, handing them a sink as a
generic parameter; `from_json_with_options` passed `NoSink`, whose `ENABLED` constant is `false`, so
every emit and the `String` a scalar arm builds were to fold away. The claim was in the commit
message, the changelog and the ledger. The gate refuted it:

```text
FAIL  from_json_medium:  +0.70% over baseline (tolerance 0.50%)
FAIL  from_jsonc_medium: +0.61%
FAIL  from_json5_medium: +1.05%
```

Each shape was then measured on one host in one session (`scripts/ir_gate.py`'s callgrind recipe,
`GLIBC_TUNABLES` pinned, `from_json_medium`, loop = full minus `--setup-only`, 2000 iterations). The
baseline build is the tree at `bdc60d74`; it read 119,005,131, and a second build of the same tree
with the symbol table kept instead of stripped read 119,001,158 - 0.003%, which is the floor every
comparison below has to clear.

| Shape of the second output | Counted Ir | vs baseline |
| --- | --- | --- |
| baseline: one output, no sink at all | 119,005,131 | - |
| generic sink, emits in every scalar arm | 120,757,775 | +1.47% |
| generic sink, emits guarded at the call site, no node left in an arm | 120,811,309 | +1.52% |
| generic sink, **every emit call deleted** | 120,905,844 | +1.60% |
| no generics: `Option<StreamRecorder>` field on the parser, sites test `is_some()` | 121,771,356 | +2.33% |

The third row is the measurement that matters, and it is a control: with the emit calls removed
entirely the traversal still costs as much as with them. The events are free. **Threading a sink
through the traversal is what the tree path pays for** - and of the four shapes that carry one, the
generic sink is the cheapest, so the design was not the mistake; the price was unknown.

`callgrind_annotate` says where it goes. Between baseline and the shipped shape,
`__memcpy_avx_unaligned_erms` gains 1,320,011 counted instructions - 75% of the 1,752,644 total, and
660 per parse of a 380-byte document. The rest is `<Parser>::value` (+200,000), `attach_pending`
(+164,000) and the two `object` parts (+61,740). Nothing here is a `pyrs-json` algorithm change:
instantiating `value` / `object` / `array` per sink alters which calls LLVM is willing to inline,
and the copies it stops inlining are libc `memcpy` calls. The same attribution explains why the
field-based variant is worse still - every emit site becomes a load from `self`, and the parser's
own locals stop staying put.

The first diagnosis of this section was wrong, and it is kept here as the lesson. It read the
`+0.70%` as the price of the per-arm `let node = …` pattern, and a real second effect was found on
that route: splitting `quoted_or_plain` into a predicate shared by both outputs made LLVM outline
`needs_quotes` across the crate boundary, costing 494 instructions per parse. Fixing it moved the
total by under 0.01%. The regression was never in the code the pull request added; it was in the
shape of asking one traversal for a second output.

**Ruling (maintainer, 2026-10-13): shelve the producer.** Not re-baseline, not duplicate the
grammar:

- Re-baselining three read scenarios by 0.6-1.0% is what the gate exists to refuse: the tree path
  would carry the tax permanently, for an output that has no consumer yet. This document's "Not a
  performance play" section said the event layer must be measured, not assumed cheap - it was, and
  it is not, at this step.
- A second, tree-free tokenizer walk would cost the AST path nothing. The estimate written here
  first — "~300-400 lines of the JSON5/JSONC dialect gates in two places" — was an argument, not a
  count, and the count puts it in the wrong bucket. Sorting `crates/pyrs-json/src/parser.rs`'s
  functions by whether their signature names `CustomNode`: 18 functions and 407 lines never do
  (`ws_extended`, `string`, `single_quoted_string`, `unicode_escape` and `hex4`, `identifier_key`
  and `at_identifier_start`, the JSON5 gates, the position and error helpers), so a second walk
  calls them as they stand; 7 functions and 241 lines do (`value`, `value_inner`, `object`, `array`,
  `quoted_or_plain`, `attach_pending`, `flush_pending`), and those are what it re-writes; `number`
  (105 lines) is the one leaf returning a node, so it is the only split. The dialect gates stay
  single-authored. What a second walk duplicates is the *policy* — the dispatch is where a
  standalone note claims the key's slot rather than the value's, where `attach_pending` orders two
  notes against one node, and where a duplicate key takes the last value and the first position —
  and two walks answer those three questions twice, while the
  fold-the-stream-and-compare-to-the-tree test catches the divergence only on inputs it touches.
  That is the reason to reorder, and it does not need the line count to carry it.
- The tax exists because the traversal still builds a tree. When `to_events` stops building one -
  which is the streaming-reader half of the Python-facing step, and the only consumer that has ever
  been named for this vocabulary - there is no tree path left to tax and the producer is free by
  construction. So step 2 is not cancelled, it is reordered behind the decision that makes it cheap.

### What the closed attempt still answered

The producer was written, so the neutral vocabulary now has answers from a second format rather than
guesses about one. They stand regardless of the shelving:

- **Spans are not derivable from the tree.** Byte offsets and line/column pairs exist only while the
  tokenizer moves; a replay of `from_events` onto the AST cannot recover them. The vocabulary has to
  carry a position, which is decision 3's borrowing question made concrete.
- **A JSON `null` is a resolution, not an event.** Emitting it as a plain `null` scalar matches what
  granit hands the YAML adapter; the distinction between "the word null" and "a null node" belongs
  to the resolver, not to the producer.
- **The stream is finer than the tree.** `{"a":1,"a":2}` is two members in the stream and one in the
  hub's mapping (last value wins, first position kept - `parser.rs:1144`,
  `duplicate_keys_last_wins_first_position`). A neutral vocabulary therefore cannot promise
  tree-shaped events; a replay is a projection, not a bijection.
- **JSON5 `'k'` keys reach the hub as `Plain`.** Single quotes are a *value* style in this engine
  (PR #121), not a key style, so `style` cannot be assumed per-position - another argument for a
  payload that each format fills rather than a fixed field set.

Decision 2 is answered (maintainer, 2026-10-13): a multi-format `parse_stream` **is** promised to
users, in Python and in Rust. That is what makes the reordered step worth building - the tree-free
reader is not an optimization of an internal detail but the advertised surface - and it settles that
the Python-facing step carries stubs, the four doc locales and a matrix row rather than being cut as
a non-goal. Decision 3 (borrow versus own) was answered on 2026-10-14 and is the change this
document's last section prices, and decision 4 (how TOML dotted keys are expressed) was answered the
same day. Nothing in plan B waits on a maintainer now: what the next step waits on is the tree-free
reader that the 2026-10-13 ruling reordered it behind.

## What the second walk answered (2026-10-16, step 2 shipped tree-free)

`pyrs_json::parse_stream` / `parse_stream_with_options` are the second producer, built the way the
ruling ordered it: a walk of its own over the scanners the tree route already has, so nothing is
threaded through the traversal that produces the AST. The +1.47…+1.60% the table above measured for
a sink is not paid here because the sink does not exist here — the two walks share `string`,
`single_quoted_string`, `number_source`, `identifier_key`, `expect`, `ws` and the error helper, and
each builds its own output. `number` was split for exactly this reason: `number_source` is the
grammar and reports the source slice, `number` is the tree's wrapper over it. The tree path now
borrows the slice where it used to build a `String` per number, so the shared leaf is thinner than
before rather than decorated with a second output.

Nine tests in `crates/pyrs-json/src/parser.rs` hold the contract, and the shape of each answer is
worth recording because step 3 (TOML) inherits them:

- **`anchor`, `tag` and `Alias` are YAML's grammar, now measured rather than asserted.** A JSON
  stream leaves both fields `None` on every event and never emits an alias; the test walks a JSON5
  document with every dialect axis on so no shape is left unasked.
- **`style` is the source spelling, and the hub's storage style is a resolution.** A `"…"` string
  arrives `DoubleQuoted` even when `quoted_or_plain` later stores it plain; a JSON5 `'k'` key
  arrives `SingleQuoted` even though the hub records identifier and single-quoted keys alike as
  plain text (the closed attempt's third answer, now with the mechanism named). The fold in the test
  tier is where `quoted_or_plain` runs, so one authored rule answers the question for both routes.
- **A number's non-finite spelling belongs to the resolver.** The stream reports the token the
  document wrote (`Infinity`, `-Infinity`, `NaN`); `non_finite_spelling` / `hub_scalar` is the one
  place the `.inf` / `-.inf` / `.nan` projection lives, called by the tree walk and by the replay.
- **Positions are the producer's to carry.** Each event gets granit-convention coordinates —
  0-indexed line, column in characters from the line start, the same split `err` uses for a
  diagnostic — from one line table per document. This is the closed attempt's first finding made
  concrete: a stream knows where things are and the tree does not.
- **Both walks refuse the same input at the same place.** 32 malformed documents across the three
  dialects must produce the identical `ParseError` from either route, and the seed corpus
  (`fuzz/seeds/json_roundtrip`, 14 files × 3 dialects) is asked the same question: 33 accepted
  documents replay, the rest are refused twice over with the same diagnostic.

### The one thing the vocabulary still cannot say

`{"a": 1 /* x */, "b": 2}` and `{"a": 1, /* x */ "b": 2}` produce **identical** event shapes —
value, then the note, then the next name — while the tree puts the note on the value in the first
document and on the following key in the second. The engine decides by which side of the comma the
note sat, and neither the note's position nor its `standalone` flag records that: the run that
carried it started after the comma in one case and before it in the other, and a comma is not an
event.

So the replay is exact for comment-free documents and, once notes are in the document, it is exact
about everything the stream reports — every note's text arrives, none is invented, none is dropped —
but not about which node owns a same-line note. That is step 3's and step 4's problem to settle, and
the two candidate answers are now concrete rather than open-ended: give the note a payload field for
"the separator was crossed", or let the stream report JSON's separator. Naming the counterexample is
`a_same_line_note_between_a_value_and_the_next_member_has_two_readings`.

One belief went with this change. `NodeMeta`'s equality doc claimed `CustomNode`'s own structural
equality ignores decorations entirely; it does not — the impl compares the inline `comment` slot,
the anchor and the tag, and leaves out the leading-note list and `source_range`. The replay
discovered it by failing on two trees that differed only by a note slot, and the doc now says what
the code does.

### What was not measured

The instruction-count gate was not run here (this host has no valgrind), so the `number_source`
split's effect on `from_json_*` is CI's `Instruction-count baseline` leg to judge, not a claim made
in this file. The fold is a test-tier oracle and no public `from_events` exists yet: publishing it
before the TOML producer has answered the same questions would freeze a shape that has one
producer's assumptions in it. *(Superseded the next day: the TOML producer answered them, and step 3
published the fold - see "What the replay gate answered".)* The Python-facing surface, the stubs and
the four doc locales are the last step of plan B and were not touched.

### What the replay gate answered (2026-10-16, step 4)

`pyrs_json::from_events` is public, which is what the paragraph above asked for first: the TOML
producer had to answer the same questions before a fold froze one producer's assumptions. It is also
a validating reader now rather than a trusting one. A caller-supplied list can be empty, can close a
container nothing opened, can carry an `Alias` or an anchor this producer never emits; each of those
is a refusal with a diagnostic, not a panic and not a silent repair
(`a_malformed_stream_is_refused_rather_than_guessed`, eight shapes).

The gate is two tiers, the pairing `roundtrip_corpus.rs` already establishes: the fuzz target
`fuzz/fuzz_targets/json_events_replay.rs` explores, and
`crates/pyrs-json/tests/events_replay.rs` runs on every `cargo nextest`, so the invariant can fail a
pull request - a sampler cannot. Both ask the same two questions per dialect: one verdict at one
place (identical `ParseError` text, which carries the offset, so position is covered by the same
assertion), and the same document through the strict writer plus the same note texts as a multiset.
The corpus test measures itself: 13 seeds over 3 dialects drive 39 verdict pairs and replay 22
accepted documents, with a floor under both counts so neither half can go vacuous on its own.

What it found in its first 90 seconds of exploring, before it was merged:
`{\n "a": 0,\n //{\n "a": 0,\n // "*  "* e\n}` - both walks accepted it, the stream reported two
notes, the tree kept one. The lost note rode the key node of the repeated name, and the collapse
that keeps the first pair drops that node, so a comment disappeared from JSONC and JSON5 output that
was perfectly stable under every fixed-point oracle. PR #450 fixes it and keeps the input as a seed.

The second finding is not a bug but a limit, and it is a second thing the vocabulary cannot say.
`{"a" /* on the key */: /* on the value */ 1}` gives the tree two notes with `standalone: true` and
gives the stream the same two notes with `standalone: false`. The flags do not contradict each
other: two same-line notes cannot share the one inline slot, so #361's rule moves them to the
leading list, and a leading-list note carries `standalone: true` - so in the tree the flag answers
*which slot this note rides*, while the stream answers *whether the source gave it a line of its
own*, which is what YAML's producer reports.
`the_two_walks_answer_different_questions_with_standalone` pins both readings, and the gates compare
note text and leave the flag out. A replay that rebuilds decorations - which is what the
Python-facing step will be asked for - needs those to be two fields, not one boolean read two ways.

## What the third producer answered (2026-10-16, TOML)

`pyrs_toml::parse_stream` and `parse_stream_with_dialect` are step 3, and they take the opposite
route from step 2 on purpose. A second scanning pass was the right shape for JSON, where the whole
grammar is one nested value; here the policy worth protecting is the table rules - which header may
open which path, which dotted key closed a table against re-opening, where an array of tables gains
a tail - authored once in `parser::register_kv` and `parser::register_header` over raw pointers into
a `CowTable`. A second pass that reaches the same accept/reject verdicts would have to write that
policy a second time, which is the divergence this document has refused twice already. So the events
are read off the same intermediate representation the AST projection reads: the scanner runs once,
one verdict stands, and two readers describe it. Accept/reject parity is then a property of the
architecture, and it is still asked as a test (26 malformed documents across both grammar revisions,
plus the 16-file seed corpus).

Ruling first (maintainer, 2026-10-16, decision 5 below): pay the spans in the IR rather than
duplicate the grammar, and let the instruction-count gate judge the price.

What the plumbing cost, measured rather than estimated: `Note { text, at }` replaces a `String` per
comment, `KVAnnotations` gained `key_at` and `value_at`, `CowTable` gained `at` and `key_at`, and
`parse_key_path` reports each segment's offset because a header line names a whole path. The
projection that decides a scalar's text, style and tag is now one function,
`lib.rs::scalar_projection`, called both by `toml_value_to_node` and by the stream - TOML's twin of
`hub_scalar`, so a spelling cannot mean two things to the two readers. `LineTable` moved into
`pyrs-event` once a second producer needed it: the same conversion, one author, two producers and
three tests of it.

### What the arbitrations said (2026-10-16, decision 5's price paid in full)

The first CI run of this shape (run 38840810436) reported one scenario over the floor:
`from_toml_medium` at +1.48% against a 0.50% tolerance, 0.000% spread across its three samples.
Decision 5 makes the gate the arbiter and names the fallback - drop the spans, do not widen the
floor - so before dropping anything, the spans were priced on one host: `main` reads 159,941,362
and this shape 161,987,153, +2,045,791 (+1.28%), while `to_toml_medium` moved +0.013% and
`from_json_medium`, which TOML's IR cannot reach, +0.035%. Three numbers that agree on one thing:
the tax was in *building* the document, not in carrying a wider record or in reading it back.

Two hypotheses then died one measurement each. Narrowing the spans - `u32` offsets, `Arc<str>` for
the note text, so the two records returned to the size `main` gives them - made it *worse*:
166,341,862, +4.0%. And `ir_gate.py --profile from_toml_medium` does not show the map insertion
moving at all between `main` and this change, which is what entry size would have changed. What
does move is `__memcpy_avx_unaligned_erms`, +2,701,897 of the +2,045,791, together with the function
under it: `scalar_projection` was written to borrow `&TomlValue` and to copy the scalar's text, and
the tree route reads every scalar through it. A value the tree already owned was therefore cloned
once per scalar. That is the regression; the spans were innocent.

So the projection takes the value by value now. `toml_value_to_node` moves the text into the node
exactly as it did before this change existed, and only the event producer - which borrows the IR and
hands the text out as owned `Cow` anyway - pays a clone. Same host, same pinned ISA:
**158,750,613**, below `main`'s 159,941,362 by 0.74%. Nothing was dropped from decision 5's shape
and no floor was moved; what the gate caught was a copy the refactor should never have introduced,
and the read path is cheaper than it was before the stream existed. Whether that lands under the
recorded baseline is CI's `Instruction-count baseline` leg to say, on the image the baseline names;
a WSL reading cannot claim it, and this file does not.

The vocabulary's fields come back answered:

- **`tag` is not YAML's private field.** A date-time arrives as `!timestamp` / `!date` / `!time` and
  a binary integer as `!!int`, which is exactly what the tree stores. The field means "the type the
  document states", so a second format fills it and nothing is renamed - the generalisation step 2
  was designed to test, and it held.
- **`anchor` stays empty and `Alias` is never emitted**, the second time that answer has now been
  measured rather than declared.
- **`style` reports what the engine kept, not what the document wrote.** A literal `'…'` and a basic
  `"…"` both arrive `DoubleQuoted`, and `_`-grouped digits arrive canonicalised, because the IR does
  not store either fact. JSON's stream does report the quote it saw; TOML's cannot. The same field
  is therefore as informative as each engine's own memory, which is the honest limit of a producer
  that refuses to re-scan.

Two positions exist only in the stream, and both are shape facts about TOML:

- A close event carries the offset of the construct it closes, because TOML never wrote an end token
    - where a table ends is decided by the next header, which is not a fact the IR keeps.
- A header names its table *inside* the brackets that open it, so a table's key event sits at a
  larger offset than the `MappingStart` that follows it in structural order. The stream is in
  structural order, which is the order YAML and JSON give too; byte order would need an end offset
  the document never contained, and array-of-tables headers repeat a name that names one sequence,
  so the name is reported once (as YAML does for a key whose value is a sequence of mappings).

A third difference belongs to the AST, not the vocabulary: an own-line note that meets no key or
header has no slot in the tree - the writer emits no such text, so `from_toml` discards it - while
the stream reports it where it sits. `a_note_the_tree_has_no_slot_for_is_still_in_the_stream` pins
both halves, including the comment-only document where the tree does keep it.

Measured on this change: `cargo clippy --all --all-targets -- -D warnings` clean,
`cargo nextest run --all` 655 passed (10 of them new: 8 in `crates/pyrs-toml/tests/event_stream.rs`,
2 for `LineTable`), `cargo test --all --no-default-features` green,
`cargo check --workspace --no-default-features` green. Not measured here: the instruction count,
which this host cannot take (no valgrind), so the span plumbing's price on `from_toml_*` is CI's
`Instruction-count baseline` leg to judge - and if it moves past the tolerance the answer is to drop
the spans from the IR and report positions only where the IR already has them, not to widen the
floor. There is no `pyrs_toml::from_events`: a fold would re-derive a tree that both readers already
agree on, whereas JSON's fold exists because its stream and its tree are built by two different
walks.

## What the Python surface answered (2026-10-16, step 5)

`parse_stream(text, on_event=None, max_depth=1000, *, format="yaml")` now picks the producer:
`yaml`, `json`, `jsonc`, `json5` or `toml`. The maintainer ruling that shaped it was a choice
between widening the one verb and adding per-format verbs; the document had promised "a multi-format
`parse_stream`", and a second surface would have meant two places to keep honest, so the keyword
won.

The step is small because the vocabulary was already shared: both producers emit
`pyrs_event::StreamEvent`, the same type YAML's stream emits, so the projection to Python dicts
needed no new field and no new format's own type at the boundary. That is the neutrality claim of
step A tested by use rather than by assertion. The default is unchanged and
`tests/test_streaming.py` compares the two calls event for event, positional form included, since a
widened signature is a compatibility event whether or not it looks like one.

What users can now see for themselves is where the vocabulary stops being YAML's: `anchor` and
`alias` are empty and absent outside YAML, a TOML literal and a basic string read alike, and TOML's
tags arrive as the hub spells them (`!timestamp`, `!!int`). Two claims were corrected by measuring
instead of by writing - the docstring first said a TOML string reads `plain` and the probe answered
`double_quoted`, and the same probe showed the quoted `y` key is the hub's deliberate choice,
matching the tree. The limit the replay gate named still stands and is now user-visible: a same-line
note's owning node cannot be reconstructed from these dicts, because `Comment::standalone` answers
two different questions in the two walks. Rebuilding decorations from a stream is therefore a shape
change (a second field), not a missing feature.

`max_depth` is honoured by the two readers that carry the option and answers in two different types;
TOML's limit is the intermediate representation's own. Details: quality-ledger (ea).

## What this is not for

- **Not a performance play.** `docs/dev/perf.md` prices the read path's dominant costs in hashing,
  allocation and the node model itself. An event layer adds a path and a `Vec` of events per
  document; if it is to be advertised as cheap, that must be measured on the gate, not assumed.
- **Not an AST change and not a rewrite of the binding/editing boundary** — both ruled out in
  `AGENTS.md`; `Event` would sit beside `CustomNode`, not replace it.
- **Not a Python-visible change until the last step of B**, so no `.pyi` regeneration or stub-drift
  work before then.

## Decisions this document needed from the maintainer

All five are answered, and each answer is recorded in place with its date — a question resolved in a
conversation and left out of the document is a question that gets asked again.

1. **B or C.** (A only as B's first PR.)
2. If B: is a multi-format `parse_stream` a *promised user-facing capability* (Python and Rust), or
   internal plumbing for the hub? **Answered 2026-10-13: promised, in both.** The tree-free producer
   is therefore a feature, not an internal refactor, and the Python-facing step is in scope.
3. If B: does the neutral event **borrow** from the input (`&'a str`, which the replay test can
   drive from a single buffer) or keep owning `String`? Borrowing is the smaller-allocations answer
   and the harder-lifetimes one; today's `StreamReceiver` already keeps the source text alive
   alongside its events (`yaml_text: &'a str`, `parser/stream.rs`), which suggests it is possible.
   **Answered 2026-10-14 (maintainer): `Cow<'a, str>`.** Borrow the clean case, own the one a
   decoder produced — granit hands the scalar over as exactly that, and moving it instead of copying
   it is what the section below measures.
4. If B: how are TOML dotted keys expressed — nested `MappingStart`/`MappingEnd` per level, matching
   the AST, or a flat key event with the source span? One sentence from you fixes what would
   otherwise be two implementations of the same question. **Answered 2026-10-14 (maintainer): nested
   `MappingStart`/`MappingEnd` per level.** The vocabulary then carries one shape for "a mapping",
   so `from_events` replays a dotted key into the same tree the AST path builds, and the consumer
   never re-derives nesting out of a span. The flat answer would have made a dotted key a second
   key-spelling that every reader of the stream — the replay test and the Python-facing
   `parse_stream` both — would have had to special-case. Source fidelity stays where the vocabulary
   already puts it, on the span each event carries, not in the event's shape.
5. Which shape TOML's producer takes, once it was clear that a second scanning walk would have to
   re-author the table rules: a second walk like JSON's, spans recorded in the IR and projected from
   it like the AST is, or a stream that reports no positions at all. **Answered 2026-10-16
   (maintainer): spans in the IR, projected from it.** The read-path cost is allowed for on one
   condition - the instruction-count gate arbitrates, and movement past the tolerance is answered
   by dropping the spans, not by widening the floor. The alternative rejected on principle is the
   second walk: `register_header` / `register_kv` own a policy (closed tables, re-opened ancestors,
   array tails) that no second copy of it can be trusted to keep, and a replay test would catch such
   a drift only on the inputs it happens to touch.

## What the borrowed scalar is worth (2026-10-14, decision 3 landed)

The ruling was `Cow<'a, str>`: move granit's own `Cow` into the event instead of copying it out with
`value.to_string()`. `stream_medium` — the scenario #405 added *before* the change so that the gate
would compute the gain rather than a sentence asserting it — reads 249,928,334 counted instructions
against the committed 275,227,226: **−9.192%**, three samples identical to the instruction (spread
0.000%). The same run measured all twenty-two scenarios; the largest movement anywhere else was
+0.202% (`parse_small`), which is inside the cross-host residual `AGENTS.md` sizes — these are WSL
readings against values committed from the enforcing image, so the CI leg is the authoritative
comparison — and the AST path is not reachable from this change at all.

Two figures frame the path's size. The event stream now costs 249.9M counted instructions against
`parse_medium`'s 321.8M on the same text, so a tree-free read is about 22% cheaper than building the
node model where it used to be about 14.5%; and what the change removes is one allocation plus one
`memcpy` of the value's bytes for every scalar in the document.

Where the copy is still paid, and why that is the honest boundary rather than a leak:

- The lazy `StreamIterator` handed back to Python owns its events, because a `#[pyclass]` cannot
  carry a borrow. That branch detaches with `into_owned` at the boundary and pays what it always
  paid.
- The incremental reader (`YamlStream`) rotates the buffer a borrowed event points into, so it
  detaches inside `next_event_impl`: a borrow of `self` there would outlive exactly the call that is
  allowed to move it.
- The `on_event` callback path pays nothing new. Its events die inside the loop, so they only ever
  borrow, and the source's bytes reach Python's string without an intermediate Rust copy.

The invariant is pinned by
`parser::stream::tests::a_plain_scalar_borrows_the_source_and_a_decoded_one_owns_its_text`, which
asserts that a plain scalar's bytes lie inside the source's range and that an escaped double-quoted
value owns its own — granit delivers exactly that split, measured rather than assumed. Reintroducing
the per-scalar copy fails that test by name, and the floor for `stream_medium` now sits at the lower
number, so a regression on this path costs a red leg instead of living quietly.
