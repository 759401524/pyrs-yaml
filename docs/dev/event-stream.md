# Format-neutral event stream — design and roadmap

Status: **ruled - option B, adopted 2026-10-13; step A (`pyrs-event`) is the PR this document
accompanies.** The objective lists `事件流` as a core abstraction to be split into its own crate,
alongside the AST. The AST half is done; the event-stream half is this document. Decisions 2-4 at
the bottom are still open, and each one blocks the step that needs it.

Every claim below is from reading the tree at `14196dc9` (2026-10-13). File and line numbers are
given so a reviewer can check them rather than take my word.

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
hopeful: events that cannot rebuild the AST the parsers already build are the wrong events.

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
   internal plumbing for the hub? The answer decides whether steps 2-4 need docs locales and stubs
   at all.
3. If B: does the neutral event **borrow** from the input (`&'a str`, which the replay test can
   drive from a single buffer) or keep owning `String`? Borrowing is the smaller-allocations answer
   and the harder-lifetimes one; today's `StreamReceiver` already keeps the source text alive
   alongside its events (`stream.rs:86`), which suggests it is possible.
4. If B: how are TOML dotted keys expressed — nested `MappingStart`/`MappingEnd` per level, matching
   the AST, or a flat key event with the source span? One sentence from you fixes what would
   otherwise be two implementations of the same question.
