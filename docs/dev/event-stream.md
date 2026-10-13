# Format-neutral event stream — design and roadmap

Status: **ruled - option B, adopted 2026-10-13; step A (`pyrs-event`) merged as PR #379; step 2
(JSON producer) measured, priced and shelved the same day; decision 2 answered the same day - a
multi-format `parse_stream` is promised to Python and Rust users.** The objective lists `事件流` as
a core abstraction to be split into its own crate, alongside the AST. The AST half is done; the
event-stream half is this document. Decisions 3 and 4 at the bottom are still open, and each one
blocks the step that needs it.

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
  which table levels a dotted key created implicitly (`pyrs-toml/src/parser.rs:164`, the
  `dotted_defined` paths), so a stream that only emits `MappingStart`/`MappingEnd` would replay into
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
- A second, tree-free tokenizer walk would cost the AST path nothing, and would put ~300-400 lines
  of the JSON5/JSONC dialect gates (hex integers, signed `Infinity`, leading-dot forms, identifier
  keys, comment slots) in two places, each needing its own edge-case tests. The
  fold-the-stream-and-compare-to-the-tree test catches divergence only on inputs it touches.
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
a non-goal. Decisions 3 and 4 (borrow versus own, and how TOML dotted keys are expressed) are still
open, and each blocks the step that needs it.

## What this is not for

- **Not a performance play.** `docs/dev/perf.md` prices the read path's dominant costs in hashing,
  allocation and the node model itself. An event layer adds a path and a `Vec` of events per
  document; if it is to be advertised as cheap, that must be measured on the gate, not assumed.
- **Not an AST change and not a rewrite of the binding/editing boundary** — both ruled out in
  `AGENTS.md`; `Event` would sit beside `CustomNode`, not replace it.
- **Not a Python-visible change until the last step of B**, so no `.pyi` regeneration or stub-drift
  work before then.

## Decisions this document needs from the maintainer

1. **B or C.** (A only as B's first PR.)
2. If B: is a multi-format `parse_stream` a *promised user-facing capability* (Python and Rust), or
   internal plumbing for the hub? **Answered 2026-10-13: promised, in both.** The tree-free producer
   is therefore a feature, not an internal refactor, and the Python-facing step is in scope.
3. If B: does the neutral event **borrow** from the input (`&'a str`, which the replay test can
   drive from a single buffer) or keep owning `String`? Borrowing is the smaller-allocations answer
   and the harder-lifetimes one; today's `StreamReceiver` already keeps the source text alive
   alongside its events (`stream.rs:86`), which suggests it is possible.
4. If B: how are TOML dotted keys expressed — nested `MappingStart`/`MappingEnd` per level, matching
   the AST, or a flat key event with the source span? One sentence from you fixes what would
   otherwise be two implementations of the same question.
