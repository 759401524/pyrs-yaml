---
title: Changelog
description: All notable changes to pyrs-yaml, formatted per Keep a Changelog and Semantic Versioning.
tags:

- docs
status: new

---

## Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

### [Unreleased]

#### Added

- **The counterparty version caps are measured, and a gate keeps them measured** — five of
  the nine `python_version < 'X.Y'` caps in the test group still read `< '3.14'` while
  `pyjson5`, `numpy`, `ujson` and `python-rapidjson` each ship a 3.14 wheel, so the ≥3.14 legs
  install fewer oracles: the twelve green legs of PR #665 read 4,433 assertions on 3.13 against
  4,197 on 3.14 and 4,196 on 3.15, and making one counterparty unimportable at a time
  attributes 141 of the 236 to `pyjson5`, 31 to `orjson` and 54 to `numpy`, none to the others
  on an ordinary leg. `pyjson5` moves to `< '3.15'` and
  `numpy`/`ujson`/`python-rapidjson`/`yaml-rs` to `< '3.16'`, because a wheel exists on every
  leg each now covers, both free-threaded ones included; `orjson` and `rtoml` stay below 3.14
  because PEP 508 has no free-threading marker, neither publishes a 3.14t wheel, and admitting
  3.14 admits 3.14t and leaves that leg unable to install the group.
  `scripts/counterparty_wheels.py` records the availability per OS and band from real
  interpreters, and `scripts/check_counterparty_caps.py` — wired as a hook — refuses a cap that
  admits a band with no wheel *and* a cap that excludes a band with wheels on every leg of it.
  A stale cap is a red run naming the package now, not a silent skip the size of the whole
  JSON5 spec table.
- **What a shared Python object becomes when it is written is now a checked fact** — given
  `{"a": shared, "c": shared}`, both writer routes emit two copies of the subtree rather than an
  anchor and an alias, and the re-read compares equal but the two values are not one object; the
  round-trip route keeps `&x`/`*x`, so the loss belongs to the object routes.
  `tests/test_safe_dump.py::TestSharedObjectShape` pins both halves plus the
  `YamlMaxDepthError` a cyclic graph gets, PyYAML and ruamel were measured against the same graph
  (both keep the sharing), and `docs/dev/boundaries.md` names the divergence and why closing it is
  not free. No behaviour changed here.
- **The pytest tier's width is derived now, not remembered** — three contributing pages said the
  blocking Python tier runs "4 Python versions × 3 OSes" and `QUALITY_MATRIX.md` said "3 OSes × 7
  Python versions", while the declared interpreter floor is eight wide (3.8 through 3.15) and the
  tier is not a cross product at all: `scripts/ci_matrix_plan.py` runs every interpreter on ubuntu
  and only the first and last on macOS and Windows, so the product of the two factors names 24 legs
  where 12 are scheduled. The matrix probe now asks that planner - 3 OSes × 8 interpreters in 12
  cells - and rewrites the sentence itself, with a pattern wide enough to match both the old
  spelling and its own output, because a rule that only fits the prose it replaces reports "no
  sentence matches" forever after running once. `tests/test_ci_matrix_plan.py` holds the three
  locale pages to the same plan and refuses a bullet that no longer carries exactly three integers:
  a reworded sentence is a red run here, not an unchecked claim. The scheduled tier builds the full
  grid on purpose, so the derived numbers describe the pull-request plan the merge verdict consumes.
- **The line the formatter would promote is now refused before it can act** — measured on the
  pinned 0.2.78, `rumdl fmt` fixes `MD018` ("no space after a heading's hashes") by re-reading any
  line-initial run of hashes as a heading, and it does that to a wrapped paragraph and to a
  multi-line inline code span alike: the first comes back split into sentence-heading-sentence, the
  second comes back as two real headings. After the rewrite that line *is* a valid heading, so no
  rule that inspects the corpus afterwards can tell it from one the author meant — five classifiers
  were measured and all five were rejected (144, 107, 100 and 135 false hits, and a scan that read
  0 by erasing the construct it looked for). The shape *before* the rewrite is decidable: one to six
  hashes at the line start, the next character not a space, outside a fenced block.
  `scripts/check_doc_headings.py --predamage` refuses it from a `format`-priority hook slot declared
  ahead of the formatter, because a `checks`-priority slot would only judge the formatter's own
  output. Over the 177 tracked pages it reads 2 hits, both live damage-in-waiting, and no false
  positive. The boundary is stated plainly: it blocks the commit, not the rewrite — with the shape
  injected, the preflight names both lines and the run refuses, while the formatter rewrites the
  working copy in the same pass.
- **The ledger's own citation rule is now a hook** — `docs/dev/quality-ledger.md` numbers its
  sections `### (xy)`, and a pointer like `(details: quality-ledger (bf))` names one section only
  while those letters run increasing and unique. Until today the Python tier judged that alone: a
  change whose only defect was a section placed one letter early passed the hook set, the docs
  gates, `rust-lint` and the Ir leg, then reddened all twelve `test` legs across three OSes.
  `scripts/check_ledger_order.py` owns the rule now — exit 1 names the section sitting wrong, exit 2
  refuses a file with no lettered section at all, which is how a renamed heading form would
  otherwise buy a permanent pass — and the `ledger-order` hook fires on the ledger itself. Control
  measured in the working copy: swapping `(gq)` and `(gr)` reddens the hook with
  `(gq) follows (gr)`, and restoring it gives `9 passed`. `tests/test_ledger_section_order.py`
  guards the checker and its `files:` pattern instead of holding a second copy of the rule.
- **A quoted Ir floor is checked against the file it quotes** — a refresh rewrites all forty-seven
  readings in `.ci/ir-baseline.json`, and every copy left in undated prose becomes history nobody
  marked: nine minutes after the last one, `QUALITY_MATRIX.md` disagreed with the file on nine of
  the eleven floors it quotes (15 to 31,977 instructions) with every gate green. The page now
  declares which readings it means as current, each followed by a comment naming its scenario, and
  `scripts/check_ir_floor_quotes.py` compares every declaration with the baseline — refusing a stale
  digit, an unknown scenario, an orphan marker whose number was deleted, and a page that declares
  nothing (exit 2, never a pass). Declaration rather than parsing, because the same page keeps
  refuted measurements, event counts and fixture widths, and a proximity heuristic for that question
  produced ~110 false hits in `docs/dev/perf.md`. `tests/test_ir_floor_quote_gate.py` forces each
  refusal and proves the hook's `files:` pattern reaches the page and the baseline. An *unmarked*
  hand-copy stays invisible: absence of a marker reads as history, which only the author can claim
  wrongly.
- **A keyed Ir floor is quoted with the image that measured it** — seven scenarios carry one floor
  per runner image, and an enforcing leg compares against its own image's row, never the flat
  `scenarios` column. The marks compared against that column, so a sentence about the leg on
  `20261023.334.3` could pass the gate while quoting `20261026.354.1`'s number: the committed file
  holds 144,902,515 and 142,467,241 for `load_toml_medium`, 1.71 % apart, and which one the flat
  column carries depends on whichever image the last refresh happened to land on.
  `scripts/check_ir_floor_quotes.py` now requires a keyed mark to name its row, written
  `load_toml_medium @ 20261023.334.3`, and refuses four shapes: a keyed mark naming no image, digits
  taken from the flat column under a named image, an image the file has no row for, and an image on
  a scenario the file does not key. Four of the page's thirteen readings are keyed; the nine
  engine-only readings keep the bare mark, because for those the flat column is what every leg
  compares against. The checker refused the un-repointed page four times,
  `uv run pytest tests/test_ir_floor_quote_gate.py` reads 22 passed, and four mutations — each
  deleting one branch of the new rule — reddened exactly the owning test.
- **The dumpers accept the tuple the loader hands back** — `? [a, b]` projects to the Python key
  `("a", "b")`, and both writers refused a tuple in any position with `Unsupported Python type`, so
  `safe_dump(safe_load(text))` raised on a document `safe_load` had accepted, and `from_dict` and
  `_set_path` refused the same value. A tuple is now a sequence on both routes; a sequence used as a
  key is written in the explicit-key shape the node serializer owns, with the direct writer handing
  that one pair over rather than mirroring the spelling, which is how the two routes stay
  byte-identical. One shape stays refused and the reason is the reader's: only the *outer* sequence
  of a key is projected, so `? [[1], [2]]` raises Python's own `unhashable type: 'list'` — measured
  against ruamel — and a key holding another sequence has no text that re-reads. Nine rows joined
  `tests/test_route_parity.py`, each asserted against both writers and as a fixed point (182
  passed), and `tests/test_edit.py::test_set_tuple_value_raises` became
  `test_set_tuple_value_writes_a_sequence`.
- **`load_json_lines` reads a `.jsonl` stream as values, one line at a time** — the engine has
  described line-delimited JSON since #599 (`pyrs_json::parse_documents`), but nothing at the Python
  layer could read it: a caller split the text and called `load_json` per line, paying for a list of
  lines plus a tree per record on top of the parse. `pyrs_yaml.load_json_lines(text)` returns a
  `JsonLines` iterator whose `__next__` resolves one line: canonical JSON goes straight to Python
  objects through the AST-free route `load_json` uses, and anything else through
  `pyrs_json::document_from_line`, so one document is live at a time and a caller that stops early
  never reaches the next record. The input is copied once into the iterator — the saving is the
  tree, not the buffer. Refusals keep the engine's contract: a blank line, a note-only line, or a
  value needing the next line raises naming its line in the whole input, and asking again reports
  the same record rather than skipping it. Measured here (release build, best of 15, 10,000 records
  / 576.5 KiB): whole stream 3.861 ms, first record 0.343 ms, per-line `load_json` 8.440 ms. Three
  Rust tests pin the value surface to the event surface — same lines, same trees, same refusal and
  wording — and 16 Python tests pin the surface; two CodSpeed scenarios carry the number.
- **The refresh job can answer "where" as well as "how much"** — `scripts/ir_gate.py --profile`
  has ranked a scenario's functions by self instructions since #401 registered it, but only a hand
  run ever called it, and a hand run cannot pair with a committed floor: the local build and the
  image's are different bytes. That is how the -21.23% reading on `documents_short_strings` stayed
  unattributed (`docs/dev/quality-ledger.md` (gk)). "Ir baseline refresh" takes `profile` and
  `profile_top` inputs now, one `--profile` flag per named scenario, on the image that enforces
  these numbers, and the step sits behind `if: inputs.profile != ''` so a refresh that names nobody
  still costs one measurement. Attribution rebuilds with symbols and returns before the
  measurement, so it cannot move a floor. Two tests in `tests/test_ir_baseline_workflow.py` pin the
  input reaching the command and the empty default keeping the refusal. No shipped behaviour and no
  measured floor moved.
- **The scan the aarch64 wheels ship is tested as the whole path, not as a tail** — every SSE2
  string scan dispatches by `target_arch`, so on an x86 CI the scalar loop only ever answers for the
  last partial vector, while the aarch64 wheels ship it as the entire answer with no test asking it
  a full-length question. Four tests - JSON reader, JSON writer, TOML reader, TOML writer - now
  require the scalar loop and the vector loop to return the same index for an 80-byte body carrying
  each of the 256 byte values at each of thirteen offsets, read from twelve start positions that
  begin, end and straddle a vector, plus a three-byte character moved across every boundary.
  Mutating one scalar loop - the C0 bound, JSON5's `\r`, TOML's multi-line arm, the writer's DEL
  case - reddens its test, and the comparison is arch-gated rather than self-compared: on a non-x86
  leg the test is compiled out, because asking the scalar path to agree with itself proves nothing.
- **The JSON and TOML event streams stop where the consumer stops them** — `parse_stream(...,
  on_event=...)` collected every event before the first callback ran, for every format, so a `False`
  at the first event still paid for the whole scan and a syntax error later in the document was
  reached and raised. `pyrs_event::EventSink` now returns `ControlFlow`, and
  `pyrs_json::parse_stream_with_options_to` / `pyrs_toml::parse_stream_to` hand each event to a sink
  as they produce it: the callback runs under `Python::attach`, the producer under `py.detach`. What
  `False` can stop is the format's own fact — JSON/JSON5 halt the walk before the bytes of the next
  event, so an error past the stop is never reached, TOML halts emission from the intermediate
  representation its scan already built, so a refused document still raises before any event is
  delivered, and YAML's push adapter over granit stops delivery after the (unabortable) parse,
  leaving `load_stream` as YAML's incremental surface. Collecting entry points now delegate to the
  `_to` ones with a `Vec` sink, so the iterator and the callback see the same stream by
  construction. A callback exception outranks the parse error behind it, and `False` returns `None`.
  Seven tests in `tests/test_streaming.py` pin the stop prefix, the per-format refusal facts and the
  error precedence.
- **Every stream event names the source bytes it came from** — an event carried a line and a column,
  so a consumer that wanted to resume from it could only count lines back through text it had to
  keep. `StreamEvent` now carries `offset` and `end` on every event of every producer: `offset` is
  the first source byte the event names, `end` one past the last, both character boundaries inside
  the source's byte length. The range is the token, not the subtree — a quoted scalar includes its
  quotes, a flow delimiter is one byte, a block container is empty — and `end == offset` where a
  producer records a position and no extent: every TOML event (its IR keeps starts), a YAML stream
  marker and a block container. A document marker covers `---` or `...` when the source wrote one
  and is empty when it did not. granit hands YAML char indices, so the text route
  converts them through the offsets it already computes and the incremental reader keeps one
  `(chars, bytes)` mark per non-ASCII character and none for ASCII, which is what keeps
  `load_stream`'s chunked memory flat. `parse_stream` and `load_stream` event dicts gain the two
  keys. Seventeen tests in `tests/test_streaming.py` pin the ranges across all five formats and both
  YAML routes, including a reader fed one byte at a time.
- **Line-delimited JSON: one document per line, optionally one sink per shard** —
  `pyrs_json::parse_documents*` describes NDJSON/JSON Lines through the same one-line walk as
  `parse_stream`, with every event and every error positioned against the whole input. A blank line,
  a note-only line and a value that needs the next line are refused rather than skipped, the chosen
  dialect (JSONC or JSON5) applies to every line, and the stream keeps the shape the YAML engine
  uses: one `StreamStart`, per-line `DocumentStart…DocumentEnd`, one `StreamEnd`. The opt-in
  `parallel` feature divides the lines into consecutive shards, one sink and one thread each, so
  nothing is buffered between a producer and its sink; shards merge by `offset` because every
  position is global. Measured over 200,000 lines (20.5 MiB, 24 threads, interleaved medians):
  112.3 ms through one sink, 18.9 ms across 24 shards (5.96x). A single shared sink does not
  parallelize — 142.1 ms, and a bounded ordered pipeline 35.6 ms — because a `StreamEvent` is 136
  bytes and a 107-byte line produces about 15 of them, so buffering costs roughly 19x the input.
- **Opt-in `mmap` read path** — `parse_file` parses a regular non-empty file straight from a memory
  map instead of a `read_to_string` copy, dropping one whole-file heap allocation; off by default
  (`default = ["numpy"]` unchanged). Empty, directory, non-regular and non-UTF-8 inputs keep their
  existing errors; see `docs/dev/boundaries.md`.

#### Fixed

- **A registered `CustomType` answers for the value it claims** — `register_type("!money", ...)`
  whose `python_type` is a class carrying `__index__` was bypassed by both YAML writers:
  `safe_dump({"m": Money(500)})` emitted `m: 500`, the integer Python's number protocol hands back,
  and the handler's text appeared only for a class the `f64` extraction happened to catch. That is
  why `Decimal` was honoured and an integer-shaped value type never was — the one registry check
  each writer carried had been written for the float shape alone, the same ordering the array route
  needed in the row above. `crates/pyrs-yaml/src/py/direct_dump.rs` and
  `crates/pyrs-yaml/src/py/python_types.rs` now ask the registry before the protocol, in value, key
  and sequence-item position, and the one exclusion is named the same way on both sides: an `int` or
  `int` subclass — and a `bool`, which is one — keeps the integer spelling a reader resolves.
  Measured on this build, all three positions emit `!money 500c` on both writers and
  `{Money(500): 1}` round-trips through `from_yaml`. The direct writer's `sort_keys` text moved with
  the spelling, because the serializer's comparator reads the key node's scalar value: a
  `Money(5)`/`Money(50)` pair sorts on `5c` and `50c` there, so keeping the protocol's `5` and `50`
  would have ordered the same mapping the other way on one writer. Eight test functions (nine cases)
  joined `tests/test_community_plugins.py`, and four falsification controls — one per registry call
  site — redden 6, 5, 4 and 1 of them; the `direct_dump` value site also carries the two
  pre-existing float rows, which is the measurement that the new question is a superset of the one
  it replaced. `docs/en/guides/plugin-development.md` states the rule. No instruction floor reaches
  this route, so the extra type compare per scalar is unpriced by the gate
  (`docs/dev/quality-ledger.md` (hq)).
- **Dumping a NumPy array no longer ends the process on Python 3.15** — the array route cast the
  object through `rust-numpy` and read its memory, which the `cp315-abi3t` wheel cannot do soundly.
  Measured on the published 0.18.3: `safe_dump(numpy.arange(8))` under 3.15 GIL died with
  `Fatal Python error: Segmentation fault` (exit 139; `Windows fatal exception: access violation` on
  the Windows leg), the same bytes under 3.15t answered `YamlTypeError`, and 3.12, 3.14 and 3.14t
  all dumped it correctly. It was deterministic and single-threaded and `numpy.zeros((0,))` crashed
  too, so the fault sat at the first read of array internals rather than in any race.
  `crates/pyrs-yaml/src/py/ndarray.rs` now asks the array for its own `dtype`, `shape` and
  `tobytes()` and decodes that buffer in Rust: no numpy C API is reached in any artifact, and the
  `numpy` Cargo feature gates the route instead of a dependency. Two more defects fell with it. The
  first is visible on every interpreter: the old route read memory order and nested by logical
  shape, so `safe_dump(numpy.arange(4).reshape(2, 2).T)` wrote `[[0, 1], [2, 3]]` where the array
  says `[[0, 2], [1, 3]]` - Fortran-ordered and strided arrays now emit what they mean, and strided
  ones are dumped where they were refused. The second was decided by the installed numpy, because
  both writer routes asked Python's number protocol before asking the array route, and that protocol
  answers *for* a one-element array: `float(numpy.array([1e30], dtype="float32"))` is the widened
  double `1000000015047466200000000000000.0` with the dimension dropped, and numpy only turned that
  into an error in 2.3. One install therefore wrote `1.5` where another wrote `- 1.5`, and the
  spelling `docs/en/guides/numpy.md` recommends for an explicit boolean round-trip,
  `numpy.array([True])`, answered `1`. An array carrying a dimension is now refused that protocol, a
  0-D array keeps the scalar spelling the same page states, and the accepted dtypes and every scalar
  spelling are unchanged. `tests/test_numpy.py::TestNumpyBufferRoute` pins eleven rows, three of
  which fail on the published wheel with numpy 2.2.6 and pass on this tree; one rebuilt wheel then
  carries the whole 54-test module - on 3.8 with numpy 1.24.4, on 3.10 with 2.2.6, on 3.12 and 3.15
  with 2.5.4, and on 3.15t as 53 passed with the one GIL-contention row skipped by design - which
  are the two interpreters on which that module could not run at all before.
- **The memory bound on `load_stream_file` is measured now instead of skipped forever** — the test
  that guarded it asked for `psutil` through `pytest.importorskip`, and no requirement file names
  that package: neither `pyproject.toml` nor `.ci/requirements-test.txt`, so on every interpreter of
  every leg the assertion reported a skip, which reads like coverage and is none. It is replaced by
  a scaling invariant on the stdlib `tracemalloc`: consuming a 2,000-line document and a
  200,000-line one through the same route must leave the same traced footprint, and today it leaves
  3,168 bytes at both sizes under 3.12 and 3,200 under 3.14t. The boundary is stated, not assumed —
  `tracemalloc` counts the interpreter's own domain, so the engine's read buffer (a fixed
  `DEFAULT_CHUNK_SIZE`, never accumulated here) is outside the measurement, which is why the old
  resident-set figure has no replacement; the bound on how much of the file a route reads stays with
  the `read_calls` guard beside it. The bound is one hundredth of the accumulating route's own
  growth, measured in the same run, and both halves are falsifiable in the same file: the streaming
  readings stay 0 bytes apart while the buffering walk grows by 199,854,440 bytes on 3.12 and
  242,617,688 on 3.14t, so an accumulating reader fails the guard. A fixed byte allowance was the
  first shape and the `test-freethreaded (3.14t)` leg rejected it - that runner holds 292,544 bytes
  for the small document where this host holds 3,168, which is a difference in how a build counts
  memory, not in what the route reads.
- **`to_toml` and the JSON writers now refuse a key pair only YAML can tell apart** — a tag is part
  of a key's identity, so the reader accepts `c: 0` beside `!A c: "!"` as two keys, while TOML and
  JSON key by text alone. The projection used to emit `c = 0` plus `c = "!"`, which this project's
  own TOML reader rejects as a duplicate key, and `{"c": 0, "c": "!"}`, which `json.loads` resolves
  to `{'c': '!'}` — one entry gone without a sound. Both routes now raise `YamlSerializeError` with
  the reason keys `toml-keys-collide-without-metadata` and
  `json-object-keys-collide-without-metadata`. The check runs only for a mapping whose keys carry a
  tag, an anchor or a note, so an undecorated document pays one boolean per key. Found by
  `prop_toml_writer_is_fixed_point` drawing the shape at random in the property tier.
- **`to_dict` and `safe_load` refuse a key pair only YAML can tell apart** — the pair the writers
  now reject had a third route, and the quietest one: a Python dict keys by text, so `c: 0` beside
  `!A c: "!"` answered `{'c': '!'}` with the first entry gone and nothing reported. Both now raise
  `YamlTypeError` naming the colliding text, the same shape `to_toml()` and `to_json*()` already
  refuse with. The tree still holds both keys and `to_yaml()` is unchanged, and a decoration that
  does not collide still projects. Against the counterparties: ruamel keeps both entries, PyYAML
  raises at this point too.
- **The edit floors no longer price a scan the shipped caller does not run** — the three `edit_*`
  scenarios handed the editing layer `line_offsets: None`, and that layer answers `None` by
  rebuilding the document's line table, which is `scan_yaml(text).line_offsets`: a full pass over
  the input that also decides `has_hash`, `has_amp` and `is_ascii`. Measured on the baseline's own
  `rustc 1.97.1` the scan spent 10,104,000 counted instructions in every one of the three loops, and
  the three counts are byte-identical because the work cannot depend on the edit. The binding
  memoizes that table per document (`Splice::line_offsets`), so the harness was pricing the
  tree-only route while its fixture parses a document and then edits it. The harness now builds the
  table beside the parse, hands `Some(&offsets)` to all three verbs, and lets `--setup-only`
  subtract it exactly as it subtracts the parse. Refresh run 81 on the enforcing image read −24.45%
  (`edit_set_nested` 48,288,771 → 36,483,718), −27.48% (`edit_insert_sequence` 42,462,454 →
  30,793,506) and −27.86% (`edit_delete_key` 42,392,088 → 30,583,030); the excess over the scan's
  own 20.9% share is that `Vec`'s allocation, append and free traffic. The floors are that
  run's numbers, not a local re-tune, and what the three now assert is the first edit
  through a parsed document plus the per-iteration tree clone — the clone stays, because an edit
  applied twice to the same tree is not the same edit.
- **A note inside a complex key can no longer be lifted out of it** — `hoist_marker_note` writes a
  standalone note owned by a node inside a `? … :` block above the outermost `?`, because that is
  where a re-read reports it. Two keys that differed only by such a note therefore emitted as two
  pairs whose keys the reader saw as one, and it refused the second —
  `DuplicateKey("Sequence { items: [Scalar { value: \"null\", …")`, drawn by the property tier
  (seed `de78e195…`; the note text holds no break, so the shape predates #645's wider alphabet).
  `keys_collide_in_text` now answers on a copy with every standalone slot dropped, which keeps the
  unemittable pair out of a generated tree; an *inline* note still separates two keys, so both
  directions are named — the shrunk case in `proptest-regressions/pbt.txt` and the boundary in
  `the_key_rule_agrees_with_the_writer_and_the_reader`.
- **A note can no longer break the line it is written on** — `arb_comment` drew note text from
  `[ -~]+`, so no property test could reach the flattening a one-line comment carrier owes. Widening
  it to also draw `\n` and `\r` reddened four properties at once: the YAML and TOML writers put the
  text after the break into the document, so TOML refused its own output
  (`bare carriage return (must be part of a CRLF pair)`) and an array item's note `"<\n&"` came back
  as a bare `&` value. Both writers now flatten a break to a space in a single `push_note_text`
  that writes into the document: the `Cow` that shape first returned allocated nothing per note and
  still cost `serialize_note_flow` +8.80% on the enforcing image. The shape that shipped is priced
  by the floor this change moves: 62,066,033 → 64,411,980 (+3.78%), what it costs to ask every note
  whether it holds a break. Deleting a writer's branch reddens its own two properties again;
  `proptest-regressions/fmt_pbt.txt` keeps the shrunk cases.
- **The dash and the colon opened for a child that had already decided to go block** —
  `write_mapping_node` and `write_sequence_node` demote a flow-marked container when a member
  carries more notes than the braces can hold, but seven decisions trusted the `flow_style` flag
  alone: `is_compact_item`, the sequence item's line, the mapping value's line, the `?` marker's
  lift branch, `hoist_marker_note`'s two arms and its read-only twin `spine_has_notes`. Each wrote
  its indicator and let the demoted child open a block body at the parent's column, so the emission
  read `- # first` / `# second` / `"": v` — and a demoted value *sequence* is worse still, since `-`
  cannot start on a `key:` line for any reader. The property tier hit the item case at 20,000 cases
  (`prop_style_setting_roundtrip`), and only through a `<<` pair, because merge resolution is what
  stacks two notes onto one key. All seven now ask the demotion's own question, and
  `spine_has_notes` gained the sequence arm its twin already had — without it a note riding a
  complex *sequence* key stayed under the container's own `&anchor` header line. A demoting complex
  key then keeps its braces: lifting the overflowing note above the marker removes the reason to
  demote, so the flow spelling the source asked for survives. Two table tests pin nineteen shapes
  beside the reported seed: each must re-parse, keep its note and scalar text, be spelled the way it
  settles — a demoted container loses its braces, one whose notes fit keeps them — and be canonical
  on its first emission. Reverting each of the seven guards on its own reddens a named row, which is
  what says all seven are load-bearing rather than five of six guessed. The tables are not the whole
  guard: `arb_node_meta` carries at most one note per node, so no draw of `arb_custom_node` can
  build the stack that triggers the demotion — the tier that reported this class could not have
  reported it again. A new strategy builds the overflow by construction in eight roles (sequence
  item, mapping value, complex key, complex key under an `&anchor` header; mapping body or sequence
  body; anchored or tagged) and the property over it reddens on six of the seven guard reversions at
  256 cases; the seventh, `is_compact_item`, is held by the tables. The enforcing image priced the
  extra question: the writer family moved +0.18% to +0.72% (`serialize_small` the worst) and no
  reader scenario moved by more than 0.19%, so both floors are re-measured on that image rather than
  widened. Four reader-accepted spellings of the shape went into `fuzz/seeds/yaml_roundtrip/`, which
  puts them under `note_survival.rs` and the two-reader parity gate on every `cargo nextest` - and
  that gate had to learn to render a complex key at all, since `json.dumps` refuses a tuple as a
  key. The 20,000-case sweep re-runs clean at 809/809; no new scenario, stub or public API.
- **The refresh job stopped dropping every scenario name after the first** — `profile` and
  `accept_drift` both split their input with `read -ra <<< "$VAR"`, and `read` consumes one line:
  refresh run 40221999337 took a `profile` input carrying two names on two lines - the shape its own
  description invites - and logged "attributing 1 scenario(s)", so the artefact arrived with half
  the attribution asked for. A text assertion cannot see that, because the flag is in the script
  either way. Both steps flatten the separators before splitting now, and
  `tests/test_ir_baseline_workflow.py` executes each step body with `python3` replaced by an echo
  and requires both names in the printed command and the right count in the log; the control puts
  the one-line read back and has to reproduce the dropped name.
- **The merge verdict could be green while the hook set was red** — `Test matrix (all legs)` is
  computed from `ci.yml`'s jobs, and `toJSON(needs)` cannot carry a job from another workflow, so
  `prek hook set on the full tree` — which lived in `hygiene.yml` — was never a name the verdict
  read. Measured on PR #632's first head: that check-run reported `failure`, refused by
  `check_changelog_coupling.py` over a changeset that moved the product with no release note, while
  the required name on the same sha reported `success`. The job runs in `ci.yml` now, once and
  unconditionally, `hygiene.yml` is gone, and `quality_matrix.py` reads the skip list from the job
  the verdict waits on. Two guards in opposite directions: the fan-in's exact-set test reddens if
  the job is added without its `needs` entry, and
  `tests/test_matrix_verdict_gate.py::test_the_hook_set_is_a_leg_and_exists_once` reddens if the job
  leaves `ci.yml` or a second copy of `prek run --all-files` reappears elsewhere — the duplication
  being how this hole reopens quietly. No engine code and no shipped behaviour move.
- **The JSON5 key refusal names a spelling its grammar allows** — in key position the message read
  `expected a quoted object key`, while JSON5 also takes a bare identifier, so the error asked the
  author for a form the dialect never required. The wording now comes from the dialect's own flag:
  `expected an object key (a quoted string or a bare identifier)` where unquoted keys are allowed,
  and the old sentence where they are not. One `Parser` serves all three dialects and both walks
  read the same flags, so the shared helper covers the tree route and the stream route alike, and
  strict JSON and JSONC keep their exact wording.
  `the_key_position_error_names_what_the_dialect_accepts` asks all five dialect-and-walk
  combinations and was red before the helper existed;
  `test_json5_key_refusal_names_the_bare_identifier` asks the same question through the shipped
  Python module.
- **A one-line document streams in linear time again** — `LineTable::line_col` counted characters
  from the line's start for every event, so a producer whose source has no newlines paid O(n) per
  event and the stream was quadratic: a 1.5 MB minified JSON array spent 4.9 s reporting its
  224,006 events, four times longer for each doubling of the input. The table now keeps a cursor at
  the last answered position and walks only the bytes between one event and the next; a position
  behind the cursor - a producer reporting a note it held back - still counts from its line's
  start. The same document now streams in 19 ms, against 18 ms for its newline-per-record twin
  (development host). `visited()` reports the characters the cursor stepped over, so
  `a_forward_sweep_advances_the_cursor_once_per_character` asserts the sweep is one pass over a
  200 KB single line without a clock, and a second test compares both directions and mid-code-point
  offsets with a rescan. JSON and TOML share the table; YAML computes its own positions and never
  had this cost.
- **The deployed site's table of contents lists each section beside its version** — the four
  localized changelog pages carried the `[Unreleased]`, `[v0.18.3]` and `[v0.18.1]` sections at the
  version heading's own level, so the site rendered each section as a sibling of its version rather
  than under it; root was correct all along, which is why the defect showed only on the localized
  pages. `scripts/check_changelog_mirrors.py` now checks the level relationship (`level_errors`),
  and `tests/test_changelog_placement_gate.py` pins both the clean tree and the injected shape.
- **The published benchmark tables cover four formats, not only YAML** — the measurement surface
  already covered five (`tests/test_benchmark_crosslib.py` races `json`, `orjson`, `tomllib`,
  `tomlkit` and `pyjson5`, and the divan set carries twelve non-YAML benchmarks), but
  `.ci/perf-snapshot.json` recorded only PyYAML and ruamel.yaml and every table heading named a YAML
  document. `scripts/bench_snapshot.py` now measures a format axis - YAML on parse, serialize and
  round-trip, JSON, TOML and JSON5 on parse - and each generated table is bound to its measurement
  by a `<!-- bench: ... -->` marker naming format, operation, fixtures and speedup baseline rather
  than by counting columns. `tests/test_perf_docs.py` asks both directions: a case measured and
  never published fails, and so does a marker naming nothing measured. Two of the new rows are
  losses and are printed as losses - `orjson` reads the 1.9 KB document 1.5x faster, `pyjson5` reads
  the JSON5 config 2.4x faster - and TOML is 5.5x to 5.9x faster than `tomllib`. The write halves of
  JSON, TOML and JSON5 stay unpublished for a reason the snapshot records: those are emitted from
  the engine's own AST rather than from a Python object, so `json.dumps` and `tomli_w.dumps` are a
  different job, and the instruction floors price them instead.
- **The Rust-backed readers are in the published tables, and two of them win** — the YAML tables
  raced PyYAML and ruamel.yaml, both pure Python, and the TOML table raced only `tomllib`, so the
  readers a user weighing a Rust engine would compare against were measured nowhere in the published
  record. `scripts/bench_snapshot.py` now carries `yaml-rs` and `ryaml` on the YAML axis and `rtoml`
  on the TOML one, and the re-measured snapshot (2026-10-25) says `yaml-rs` parses 2.4–2.6x faster
  and round-trips 1.6–1.9x faster than pyrs-yaml, and `rtoml` reads 1.9x faster than `load_toml`,
  while pyrs-yaml serializes 2.2–4.1x faster than `yaml-rs` and 198–339x faster than PyYAML. The
  losses print as losses with the reason beside them: those readers take and hand back plain Python
  objects - their round-trip is a value's, not a document's - where the pyrs-yaml row builds the
  tree a later edit format-preserves, and `load_toml` walks that tree before returning a dict. The
  gate moved with the tables instead of describing them: `tests/test_perf_docs.py` takes its library
  list from the snapshot rather than a hardcoded five-column shape, checks a peer's
  `faster`/`slower` word against the ratio printed beside it, requires each row label to name the
  fixture its numbers came from and the ratio column the library it divides by, and fails any
  library comparison carrying no `bench:` marker. Six controls were run - a stale cell, a flipped
  direction word, a dropped row, a removed marker, a swapped label, a header that stopped naming
  its baseline - and each reddens by naming the page and the row.
- **A release note that calls a closed gap open is a red run now** — `scripts/check_hole_claims.py`
  read only `AGENTS.md` and `CLAUDE.md`, which is how one claim about a measurement this tree no
  longer reproduces survived in eight places: the five changelog mirrors,
  `docs/dev/quality-ledger.md`, a test's own reason for existing, and the Rust doc comment on the
  `ir-gate` code that had already closed the gap it described. It then rode across two pull requests
  (#613, #614) with every gate green. Scope is present tense now — the two instruction files plus
  the `[Unreleased]` block of each mirror, sliced by a new `unreleased_lines` in
  `scripts/check_changelog_mirrors.py`, so one parser owns "what is `[Unreleased]`" for both rules.
  Released sections and the ledger stay dated records, and there a hole that has since closed may
  still be named. A mirror yielding no `[Unreleased]` lines exits 2 rather than passing, because a
  heading spelled without its brackets would otherwise stop the gate in silence. Four controls in
  `tests/test_hole_claims_gate.py`: an id inside `[Unreleased]` fails by naming the path and line,
  the same id under a released heading passes, the same id passes while the matrix measures it, and
  a default scope that drops any mirror is red. The predicate ran on the real tree too — injecting
  the id at `CHANGELOG.md:9` returned 1, restoring it returned 0.
- **A note on a YAML key line no longer vanishes on the way to TOML** —
  `to_toml("sec: # note\n  x: 1\n")` returned `[sec]\nx = 1\n`: the comment was neither kept on the
  header nor moved anywhere, it was dropped. The section pass of the TOML writer read only the
  mapping's own inline slot, while the hub hands over a YAML document's key-line note on the **key
  node** — the same slot #146 taught the writer to read one level down, for an inline member. The
  measurement also retires two explanations this repository had been giving for a related gap:
  `pyrs_toml`'s own scanner does capture a `[sec] # note` inline note, granit is not in that path at
  all, and a YAML-origin `sec: # note` does keep its position through the hub. What is left open is
  the placement rule the `api:set-comment-container-owner` residual records — a TOML-origin header
  note still comes back on its table's last body line — and that behaviour is unchanged and still
  pinned. Pinned by
  `writer::tests::a_note_on_the_key_line_of_a_section_header_is_written_on_the_header` and
  `tests/test_toml.py::TestSectionHeaderCommentBoundary::test_a_note_on_a_yaml_key_line_reaches_the_toml_header`; reading only the mapping's slot again reproduces the drop verbatim, `"[sec]\nx = 1\n"` against `"[sec] # note\nx = 1\n"`.
- **A key-line note on an entry inside a section body no longer vanishes** — the slot the previous
  release closed for header lines was still unread by the line that writes `key = value`:
  `to_toml("a:\n  b: # note\n    x: 1\n")` emitted `[a]\nb = {x = 1}` and the array shape emitted
  `b = [{x = 1}]`, both without the note, while the YAML route re-emits it and a TOML-origin
  `b = {x = 1} # note` survives two rounds. `emit_pair` now falls back to the key node's inline
  slot. A header is rare; every pair pays. On the enforcing image, with the compiler the baseline
  pins and three repeats keeping the largest, both TOML writer floors move - `to_toml_medium`
  26,604,776 to 27,024,692 (+1.58 %) and `to_toml_long_strings` 28,255,071 to 28,607,004
  (+1.25 %) - and no other scenario of the 37 moves by more than 0.32 %, so the refresh that raises
  them names both. The inline-table spelling settles in one round; an array expands and carries the
  note onto its last element, settling there - relocation, never loss.
- **Six gates could answer "held" over a measurement that read nothing** - the class `(fz)` met
  once is now swept. `check_changelog_coupling.py` exited 0 with `OK: … listed no files` on an
  empty path list; `check_doc_headings.py` counted the paths handed in, not the files opened, so
  one missing path read as a clean tree; `check_line_endings.py` never checked `git ls-files`'s
  exit code; `check_doc_signatures.py` and `check_i18n.py` printed their OK lines over an empty
  glob; and `ir_gate.py --only a,b` - `--only` appends, so that is one name matching nothing -
  compared no scenario and reported the floor as held. All six exit 2 now, the verdict the
  landed-claim and stub-type gates already state, each forced by a test. A prose-only changeset and
  a page of untyped examples stay green, and that boundary is pinned too - a first draft of the
  refusal reddened the signature gate's own suite. `check_i18n.py` had no self-test of any kind, so
  `tests/test_i18n_gate.py` is new. Recorded as `(gc)`.
- **Ten assertions that could report green without running one** - `(gc)` closed the class in the
  checker scripts; the same shape sits in the suite. A test whose only `assert`s are inside a `for`
  body passes on an empty iterable, and `all(x for x in [])` is `True`. Sweeping `tests/` by AST
  named 41 candidates; the ten whose iterable arrives from outside the test now carry a floor that
  states the measured number. `tests/test_dependency_merge_gate.py` reads `.github/dependabot.yml`
  and refuses below 3 entries (measured 4), which closes its three per-entry loops plus
  `test_every_entry_targets_main` - an `all(...)` the sweep's predicate cannot see. The mkdocstrings
  `paths` list (measured `['python']`) refuses the empty case, where a handler told to read no
  package still passes the loop that checks those directories exist. The API-page glob refuses two
  vacuous shapes - no page at all, and a page published in one locale only, where comparing a page
  with itself always agrees (measured 28 pages = 7 names x 4 locales). `load_stream`/`parse_stream`
  parity now states both sides hold 13 events and that the lengths are equal, because `zip()`
  truncates to the shorter side, and its per-event key check names the 8 events one pair emits. A
  `walk()` of `a: 1` refuses fewer than its 2 nodes. Seven mutations - empty the source, truncate
  the producer - each turned its floor red and every file came back byte-identical; that run is also
  what showed `YamlStream` has no `__len__`, so the length equality was not expressible until the
  stream was materialised. The other 32 candidates were read and left: literal iterables that cannot
  empty without an edit in the same function, constants another test already floors, `@given` loops
  whose concrete siblings pin the shape, and the two registry tests where an empty registry *is* the
  assertion. Nothing yet stops a new vacuous-loop test: the sweep is a one-off script, not a
  checker, and as a rule it would reject the literal and `@given` cases too. Recorded as `(ge)`.
- **A cross-library row named a competitor it never timed** — `tests/test_benchmark_crosslib.py`
  skips a competitor that cannot ingest a fixture, and the comment on that branch for `yaml_edit`
  attributed the refusal to "newer yaml_edit releases". Measured against the pinned 0.1.3 the
  refusal was not newer: all three fixtures raised `Document::from_str: Input contains stream-level
  comments`, so the row was three skips beside an installed package, and a suite that counts passes
  cannot see a benchmark that never ran. 0.2.0 — taken with a scoped `uv lock --upgrade-package`,
  not as drift from a full re-resolve — parses `small` and `medium` and still refuses `large`, so
  the row times two fixtures now. Its `YamlError` subclasses `ValueError`, so the existing `except`
  covers both releases and only which branch is taken changes. The durable half is the floor
  `tests/test_leaderboard.py::test_yaml_edit_row_measures_something`: installed has to mean ingests
  at least one fixture. It lives in the leaderboard file because the benchmark file is deselected
  from normal runs and `--codspeed` deselects every test without a `benchmark` marker (measured:
  `50 deselected`), so the floor placed there would have run in neither leg. Control: putting the
  venv back on 0.1.3 turns it red with the release in the message; on 0.2.0 it is green. Recorded
  as `(gg)`.

#### Changed

- **The Ir floor for a scenario that counts an interpreter is keyed by runner image** —
  `.ci/ir-baseline.json` gains an `image_floors` table, and `scripts/ir_gate.py` now compares an
  enforcing run against the row for the image it is on, instead of asking one number to settle the
  question on every host. Which scenarios get a key is derived from the harness map the measurement
  builds, so only the ones that count CPython are keyed: at one tree with both harness digests
  equal, `load_toml_medium` read 142,467,241 and 144,904,012 on two images, which is 1.71 % a single
  floor cannot hold. An image with no row is refused (exit 2) rather than passed or failed; a
  baseline with no table behaves as before.
- **The `no_std` host tests now run on Windows as well as Ubuntu** — the job was pinned to one
  runner, so the evidence that a `#![no_std]` crate's logic survives without std came from one
  operating system while wheels are published for three. The bare-metal build is host-independent
  and stays on one (`if: matrix.os == 'ubuntu-latest'`), so the matrix does not pay a second
  dependency build for a check that cannot disagree; the unit tests, which can, now spread. Measured
  before the leg was added rather than after: `cargo test --locked --no-default-features` over the
  six foundation crates on a Windows host runs 724 tests over 23 targets with 0 failures, and that
  holds only because the same change set stops three dev-dependency edges from re-enabling `std`
  through feature unification. `tests/test_no_std_gate.py` refuses a job that drops back to one host
  and states the asymmetry it protects.
- **The `no_std` unit-test leg now runs the configuration its name states** — `no-std-check` runs
  `cargo test --locked --no-default-features` over the six foundation crates in one invocation, and
  its comment recorded that widening the list "was worth 112 host tests that had never run without
  std". Read off the actual rustc command lines with `cargo test -v --no-run`, that was not what
  happened: `cargo test --no-default-features -p pyrs-toml` alone compiles `pyrs_toml`'s test unit
  with no feature cfg at all, while the same command naming all six carries
  `--cfg feature="default"` and `--cfg feature="std"` for it, for `pyrs_json` and for `pyrs_ast`.
  Cargo unifies features across the packages selected in ONE invocation, and `pyrs-yaml-core`'s
  dev-dependency edges on those three siblings did not say `default-features = false`. Nothing
  shipped was affected — dev-dependencies are in no downstream graph, and the bare-metal build had
  been compiling every lib unit with `features=[]` all along — but the behaviour-under-no-std claim
  rested on a step that was measuring the std build. Three manifest edges close it, and the gate is
  widened to read the manifests rather than trust a `-p` list: `tests/test_no_std_gate.py` now
  refuses any foundation path edge into a sibling that declares `default = ["std"]` without
  `default-features = false`, with the pre-fix manifest as its control (it reports exactly the three
  edges) and a second control pinning the reader's section walk. The CI shape then runs 724 tests
  over 23 targets std-free and all pass; the default shape still runs 815, and
  `cargo clippy --all --all-targets` stays clean under `-D warnings`.
- **The two YAML readers now carry a floor for a note, not only the writer** — `serialize_note_flow`
  prices what the writer does with a note it has to emit, but nothing priced what a reader does to
  resolve one: the five `parse_*` scenarios read `SMALL_YAML`, `MEDIUM_YAML`, `BLOCK_STYLE_YAML`,
  `BLOCK_SCALAR_YAML`, `ANCHOR_YAML` and `MERGE_INLINE_YAML` and the two document-carrying
  `stream_*` rows read `MEDIUM_YAML` and the generated long-record document, all of which hold zero
  `#`, so the byte scan in `parser/yaml/comment.rs`, the receiver's decision about which node owns
  an incoming note and the pending-slot carriage ran 2,000 iterations per floor on the cheapest
  answer available. `parse_notes` 701,017,318 and `stream_notes` 579,067,612 read the one fixture
  the writer row already writes, so what separates the two new numbers is the route and nothing
  else; `crates/pyrs-yaml-core/tests/ir_fixtures.rs` pins eleven notes through both readers and zero
  through each fixture the older rows use. Against the same 434 bytes with every note run blanked to
  the same number of spaces — both anchors, both tags and every brace untouched — the eleven notes
  cost +16.30 % on the tree route and +19.37 % on the event route, 71 % and 74 % of it inside the
  locked parser (`docs/dev/perf.md` carries the row-level attribution). The refresh that wrote the
  pair refused itself first, at `from_json5_medium` +0.55 % with `pyrs-json` untouched and the
  binding binary's digest unchanged: taking the set from forty-nine scenarios to fifty-one is what
  moved it, so the name went into `accept_drift` and `tolerance_hint` stayed at 0.5 %.
- **A stream asked of a single-document route is refused, not resolved** — `parse`, `parse_file`,
  `safe_load`, `to_toml` and the `YAML()` instance's equivalents read one document, and until now an
  input carrying two answered whichever document the event walk ended on: `a: 1\n---\nb: 2\n` came
  back as `b: 2` and `a: 1\n---\n` as `null`, with nothing saying a document had been discarded.
  They now raise `YamlParseError` with a message that opens `multi-document-input`, names the second
  document's line and column, and points at `parse_all_docs`, `safe_loads` and `parse_stream` — the
  three routes that still accept separators, each measured doing so over the same ten shapes. PyYAML
  and ruamel.yaml raise on all ten and still accept a leading `---`, a trailing `...`, a bare `---`
  and an empty input, which are pinned here as the controls the guard must not over-fire on; the CLI
  says the same thing with `-A/--all-docs`, whose own behaviour is unchanged. What is not claimed:
  that the position matches the reference libraries, which mark the second document's directive
  where this marks its separator. Recorded as `(hg)` in `docs/dev/quality-ledger.md`, together with
  the compliance-harness mis-scoping this exposed.
- **The Ir gate gained two floors that price metadata carriage on an edit** — every replacement asks
  `editing::metadata::merged_meta` whose note, whose anchor and whose tag to carry, and
  `delete_path` asks `extend_delete_over_comments` to widen its range, yet no floor reached any of
  it: the three `edit_*` scenarios run over `MEDIUM_YAML`, which holds zero `#`, zero `&` and zero
  `!`, so all three branches missed on each of the 2,000 iterations. `METADATA_EDIT_YAML` is the
  same shape with an anchored, tagged and annotated `server.port`, an alias that makes that anchor
  load-bearing and a standalone comment above the key being deleted; `edit_set_metadata` and
  `edit_delete_notes` run the existing two verbs over it, so making carriage dearer now moves a
  number. `crates/pyrs-yaml-core/tests/ir_fixtures.rs` pins each answer and reddens if the shared
  fixture is ever decorated, which would leave the pair with nothing to contrast against. The
  enforcing image writes `edit_set_metadata` 42,838,349 against `edit_set_nested` 36,483,730 on a
  document 26 characters *smaller*, so what separates the two is the carriage and not the clone of a
  bigger tree. That same run moved two floors it never enters — `documents_short_strings` −10.46%
  and `documents_long_strings` −7.00% — and the gate's own profile puts the whole move in one glibc
  copy row and its malloc bookkeeping, with every engine row identical to the instruction on both
  hosts: a floor is a count of what the build does, not only of what the engine does.
- **A blank line between two YAML entries is now a registered boundary, with a test that pins it** —
  measured through the shipped extension at eight positions (between root entries, between nested
  keys, between sequence items, before a standalone note, consecutive blanks, after a block scalar,
  leading, trailing) and through the edit route: the line survives none of them, while values, key
  order, anchors and the note do, and what comes back is already a fixed point.
  `tests/test_roundtrip.py` now asserts that behaviour as it is today, so closing the gap reddens
  the pin and forces the registry entry and the boundary note out in the same change. It stays open
  because restoring the layout means per-container state carried through the serialization pass,
  which the instruction floors price on every `serialize_*` scenario. No shipped behaviour changes.
- **Both lockfiles moved to the newest version their declared ranges allow** — `similar`
  3.2.0 → 3.2.1 on the Rust side, the only step left under the eight crates' MSRV 1.96 floor, and
  nine Python packages: `zensical` 0.0.71 → 0.0.72, `mkdocstrings` 1.0.7 → 1.0.8,
  `mkdocstrings-python` 2.0.9 → 2.0.10, `python-dateutil` 2.9.0.post0 → 3.0.0, `deepmerge`
  3.0.1 → 4.0.0, plus griffelib, markdown and two patch rows. The deliberate pins held: `maturin`
  stays at 1.15.0, which is current, and `pyo3` stays on the 0.29 line; the newest `pytest`,
  `hypothesis`, `pydantic` and `numpy` ask for an interpreter above the 3.8 floor this release
  still supports. Measured here: 806 Rust tests, 4,306 Python tests,
  `cargo clippy --all --all-targets -- -D warnings`, the whole hook set, and a strict four-locale
  docs build.
- **Notes have a floor on both sides of the bridge** — attributing PR #641's writer-floor move meant
  counting what the benchmark fixtures actually hold, and the count refuted the explanation written
  beside it: every YAML input the serialize scenarios ran over at that point carried zero `#`, and
  the five writer inputs carried zero `{`, `[`, `]` or `}` - so `notes_have_flow_spelling`, the
  flow-to-block demotion, `hoist_marker_note`, `spine_has_notes` and the two `lift_first_*_notes`
  helpers answered no measured question, and making any of them dearer would have moved no number.
  The same count reddened the outbound JSON half: `to_jsonc_medium`, `to_json5_medium` and their two
  long-string twins are handed the AST of a document holding zero `#`, so a dialect writer that
  dropped every note it was given would have stayed green. Three documents now carry the shape their
  halves must answer for. `serialize_note_flow` (62,066,048 counted instructions on the enforcing
  image) runs one document through the five answers that walk can give - braces kept, braces
  abandoned for a member holding two notes, a `?` marker with its note on the marker line, a marker
  whose own note slot is empty but whose spine carries one three levels down, and an anchored header
  whose first entry is annotated - and the image's own attribution puts that walk at 12.61% of the
  scenario (7,824,000 across the predicate's two instantiations). `to_jsonc_notes` (17,002,806) and
  `to_json5_notes` (17,120,806) write one six-note document through the two dialect writers, and run
  75's profile named where its money went, before PR #643 took the block half in one pass:
  `<str>::replace` - the flattening every note passes through before it is written as a comment, and
  reachable from note text alone - is the largest self term in both, 16.08% and 16.01%, with
  `push_block_note` and `emit_value_block_notes` behind it. Their reader twins `from_jsonc_notes`
  (50,684,157) and `from_json5_notes` (50,864,157) exist because the defence matrix pairs a
  `to_<format>_<shape>` with its `from_<format>_<shape>` and asserts that pair on the committed
  harness; they resolve a document of seven comments - five line-form, one block-form, one before
  the document - and name `<pyrs_json::parser::Parser>::ws_extended` as the largest term in both
  (19.86% and 19.79%), with `attach_pending`, where a comment finds its node, at 1,484,000.
  `crates/pyrs-yaml-core/tests/ir_fixtures.rs` pins the five YAML answers, the six written notes and
  the seven carried notes against the text the engine emits, and reddens by name when a note is
  deleted from any of the three fixtures - so a floor cannot quietly come to guard a note-free
  document. Growing the harness moved 15, then 13, then 18 floors past 0.005%, and the set that
  crossed the 0.50% line was not the same name each time: `from_json_medium` 78,671,707 → 79,283,878
  → 79,904,977 crossed twice at +0.78% a step, then the third growth sent it 0.41% the other way
  while four readers it does not enter crossed instead (`from_json5_medium` +1.05%,
  `from_jsonc_medium` +0.72%, `parse_inline_merge` +0.67%, `from_json5_long_strings` +0.50%). That
  is why the names come from a dispatch's refusal rather than from a prediction: run 74 refused with
  those four while `from_json_medium` sat pre-named from the two earlier crossings, and run 75 named
  the four and wrote. `tolerance_hint` stays 0.005. All forty-seven floors come from those runs,
  re-measured by run 79, and `QUALITY_MATRIX.md` §2 counts them out loud.
- **The assessment's blind-spot count is derived from the list, not trusted** — the sentence in
  `QUALITY_MATRIX.md` said `Seven blind spots were found by measuring` while the list under it
  already carried eight entries, and no gate could see the difference: the derived tier row counts
  tests and scenarios, never paragraphs. One test now re-derives the number from the list that
  sentence introduces, and both ways of breaking it were proven - the digit moved to `Ten` reddened
  it, an injected extra entry reddened it - so a hole recorded without its count is a red run
  rather than a sentence that used to be true. The count reads nine now because the hook-set leg
  joined the list. No measured floor and no shipped behaviour moved.
- **The instruction gate's prose now says what was measured** — three sentences of
  `docs/dev/perf.md` credited `AGENTS.md` with calling the binding channel noisy, a claim that file
  does not contain (`grep -c noisy AGENTS.md` is 0), and the attribution they carried was wrong: the
  wide reading was CPython's per-process hash seed, since pinned. The pointers name the term now,
  and `QUALITY_MATRIX.md` section 2's cross-host ceiling is marked as the dated reading it is —
  re-measured on the pinned tree, the binding channel sits +0.15% worst against the image's floors
  while three scanner scenarios diverge by up to -21.23%, recorded in `docs/dev/quality-ledger.md`
  (gk) together with the two explanations that were tested and refuted. One guard test, so a pointer
  at an absent claim reddens. No measured floor and no shipped behaviour moved.
- **The line route has a floor, and a long body costs more there than on the event path** —
  the record that added the event stream's long-value floor named the clause it left standing: the
  NDJSON path had no number of its own. `Work::Documents` now reads eight one-line records into
  events through `pyrs_json::parse_documents`, and the pair differs only in the width of the value -
  9 characters against 256. On the image that enforces these numbers that is 95,815,186 against
  328,951,144 counted instructions, **+243.3%**, where the same single-dimension pair on the event
  path reads +8.16%. All thirty-three floors come from one run, which needed exactly one
  `accept_drift` name: growing the harness moved `from_json5_long_strings` from 100,340,744 to
  101,160,437 (+0.82%), a scenario this change does not enter, while `from_json5_medium` (+0.479%)
  and `from_json_medium` (-0.905%) stayed inside the line on their own. Re-measuring also moved a
  published ratio: the event pair the previous record wrote down at 8.33% reads 8.16% on this run,
  and `QUALITY_MATRIX.md` §2 now carries the newer number - which
  `tests/test_ir_baseline_workflow.py` checks against the file rather than against memory.
- **Each Rust benchmark runs in its own process** — `codspeed-divan-compat` 5.0 → 6.0. Upstream's
  6.0.0 forks every Divan benchmark, so allocator state left behind by one benchmark no longer
  colours the next, and it reports roughly 70% fewer benchmarks varying by more than 1 percent;
  `CODSPEED_PROCESS_ISOLATION=false` reverts it. Nothing that decides a merge moves: the verdict is
  `Test matrix (all legs)`, the instruction floor is the committed `.ci/ir-baseline.json`, and
  documented wall-clock numbers still come from `.ci/perf-snapshot.json`. What does move is the
  CodSpeed dashboard, because 6.0 declares that some values shift on the way up - so a trend read
  across this bump is not one measurement, and the run-to-run spread is being measured on an
  identical head before anything is claimed about it. `cargo codspeed build --workspace --exclude
  pyrs-yaml` built both suites with CLI 5.0.3 against the 6.0 runtime; CI installs the CLI unpinned
  and 6.0.0 was released the same day as the runtime.
- **The event stream now has a floor at the long-value shape** — the record that added the four
  long-value reader floors named the gap it left in one clause: "`stream_medium` and the NDJSON path
  still carry short values, and the event path has no long-string scenario of its own". One scenario
  closes the event half: `stream_long_strings` runs the same `Work::Stream` over `LONG_YAML`, the
  same eight 256-character records the four writer floors use, so those numbers differ only in the
  shape of the strings they carry. On the image that enforces them the long shape costs
  276,633,813 counted instructions against `stream_medium`'s 255,359,778 - **+8.33%** - the first
  reading this gate can give for a long body on the event path. The refresh needed exactly one
  `accept_drift` name: growing the harness moved `parse_inline_merge` from 355,404,524 to
  359,069,238 (+1.03%), a scenario this change does not enter, while `stream_medium` itself came up
  +0.485% - inside the line, so nothing else was excused. All thirty-one floors now come from that
  run, `QUALITY_MATRIX.md` §2 counts them out loud, and
  `tests/test_ir_baseline_workflow.py` fails if the sentence and the file disagree.
- **The long-string measurement surface now covers all five formats** — the vectorized readers
  shipped floors for the long-value shape at strict JSON and TOML only, because the gate pairs a
  bridge's reader with its writer and the two dialect bridges had neither. Four scenarios carry it
  end to end now: `from_jsonc_long_strings` at 141,062,599 counted instructions,
  `from_json5_long_strings` at 99,998,599, `to_jsonc_long_strings` at 60,056,647 and
  `to_json5_long_strings` at 60,480,707 - each the largest of three samples at 0.000% spread, all
  thirty numbers written by the refresh dispatch on the image that enforces them. That run needed no
  `accept_drift` name at all: growing the set from 26 to 30 crossed the 0.50% line for nobody, the
  largest upward move being `from_toml_medium` at +0.363%, so the same dispatch also re-measured the
  four floors the previous change recorded - `from_json_long_strings` 112,664,928 → 112,987,111,
  `from_toml_long_strings` 109,254,466 → 109,535,451, `to_json_long_strings` 58,550,997 →
  58,550,735, `to_toml_long_strings` 72,438,982 → 72,438,675. The fixtures are
  `bench_inputs::long_string_jsonc` (2,557 bytes: the same eight records, a line comment and a block
  comment between members) and
  `long_string_json5` (2,550 bytes: single-quoted values - the dialect whose string body the
  single-quote instantiation of the vector scans - bare keys, and a trailing comma on every member).
  `crates/pyrs-yaml-core/tests/ir_fixtures.rs` pins both to the record list the strict JSON fixture
  renders, to a widest run of exactly `LONG_VALUE`, and to carrying both comment forms, so a fixture
  that loses its comments cannot keep the scenario honest unnoticed; one already caught a fixture of
  this author's that emitted a double comma. The two writer floors also record what the outbound
  scan costs today, before anything proposes to vectorize it.
- **The long-string shape now has a committed measurement on both channels** — every benchmark in
  this tree reads short values (`MEDIUM_JSON`'s longest is 21 characters, and both leaderboard
  documents build `item_N` names), so the bulk scan this release vectorized could only be measured
  from a throwaway probe. `bench_inputs::long_string_json` / `long_string_toml` / `long_string_yaml`
  generate the same record list with 256-character values - letters only, no byte a scanner stops
  on, pinned by `crates/pyrs-yaml-core/tests/ir_fixtures.rs` - and both harnesses read them at both
  ends of the two cross-format bridges: four benches for the ranking channel (2.599 µs and 3.499 µs
  a call to read, 950 ns and 1.174 µs to write, one run on this box) and four floors for the channel
  that enforces - 112,664,928 and 109,254,466 read, 58,550,997 and 72,438,982 written. Per source
  byte of that one image run the long documents cost a fifth of `from_json_medium` and an eighth of
  `from_toml_medium`, their writers two fifths of the medium twins: the scan, not the node, is the
  cost centre there. The writer halves are not decoration - `scripts/quality_matrix.py` reports a
  bridge numbered on one side only as `ir-bridge-unidirectional`, which is what the reader-only
  first version was. Growing the harness also moved a scenario it never enters: the head before this
  one read `stream_medium` at +0.01% on the enforcing image and this one reads it at +0.63%, one
  variable apart, which failed the leg until the refresh dispatch recorded all 26 numbers on that
  image with `accept_drift stream_medium` - so every other move still refuses itself.
- **The bump bot may no longer offer the compiler that defines the tested floor** - the weekly
  `github-actions` group opened #620 to raise two actions and rewrote
  `dtolnay/rust-toolchain@1.96` to `@1.120` inside `ci.yml`, where for the `msrv-check` leg that ref
  is not a version but the compiler it installs. The defence caught it - five tests in
  `tests/test_msrv_gate.py` and `tests/test_ci_rust_leg_exclusion.py` went red on that head - but a
  pull request that is red every Monday gets closed and re-offered, and the config guaranteed the
  re-offer: the `actions` group carried `patterns: ["*"]` and no `ignore`. The name is excluded in
  `.github/dependabot.yml` now, with the measurement written beside the reason, and
  `test_the_bump_bot_is_told_never_to_offer_the_floor_selector` reddens if the exclusion is edited
  away. Moving the floor stays a human act.
- **The TOML crate documented a parser that builds no intermediate tree; it builds one** —
  `crates/pyrs-toml/src/lib.rs` opened with "no intermediate value tree", a sentence the three
  records `(fs)`, `(ft)` and `(fv)` each left owed to the change that removes the tree. `(fw)`
  measured it: the two table-insertion rows are 8.10 % of `load_toml_medium` and 9.68 % of
  `from_toml_medium`, and the larger rows in that profile are scanning work a tree-free parser
  still performs. The rewrite is therefore not scheduled and the sentence now describes the build
  it ships. Documentation only - no behaviour, output or counted number moves.
- **The bump bot can read its own config again** - three `groups` entries in
  `.github/dependabot.yml` carried `version-update:semver-patch` / `-minor` / `-major`, which is the
  `ignore` and `allow` vocabulary: `groups.*.update-types` accepts only `major` / `minor` / `patch`,
  so Dependabot rejected the file - nine days after it landed, as a check-run no merge gate reads,
  with every workflow still passing. The five values now use the short forms, the one `ignore` rule
  that declares types keeps the prefixed form where it belongs, and
  `tests/test_dependabot_config.py` pins the two vocabularies separately - and fails if the sites
  either one polices disappear - so an edit that harmonises them reddens a test instead of quietly
  stopping the bumps.

#### Performance

- **A block note is written in one pass** — `to_jsonc_notes` and `to_json5_notes` price the
  flattening a note passes through before it is written as a comment, and the block half of it
  allocated twice: `push_block_note` did `text.replace(['\\n', '\\r'], " ").trim()
  .replace("*/", "* /")`, so a compact note cost two intermediate `String`s before being written
  into a buffer that was a `String` already. One pass writes it now: trim, map interior breaks to
  spaces, split `*/` only where it could close the comment early. Trimming first is equivalent and
  not assumed — `\\n` and `\\r` are whitespace and so is the space that replaces them, so
  flatten-then-trim drops the same prefix and suffix. On the enforcing image `to_jsonc_notes` reads
  17,002,806 and `to_json5_notes` 17,120,806, down 38.27% and 38.11% from the floors PR #642
  committed, with the other 45 scenarios inside ±0.16% (`to_json_medium` −0.0004%) — the refresh
  wrote all forty-seven without naming a single drift. The six line-form sites went through the same
  pass in a first version and were taken back out: they bought a further 2.5 points on the two notes
  floors but moved six note-free writers past the tolerance `ir_gate.py` enforces (`to_jsonc_medium`
  +1.63%, `to_json5_medium` +1.59%, `to_jsonc_long_strings` +1.44%, `to_json5_long_strings` +1.42%,
  `to_json_medium` +1.02%, `to_json_long_strings` +0.96%), reproduced on a second run of that same
  tree, and `#[inline(never)]` on the new helper changed none of the six — so the price is that edit
  itself, not the pass being inlined, and 2.5 points did not repay 1% on paths that never reach a
  note. What shipped stays guarded by the two floors and by
  `a_note_is_flattened_and_a_block_carrier_splits_its_own_terminator`, which pins the line spelling
  as `// first  second` for a note carrying `"  first\\r\\nsecond  "`, the block spelling as
  `/* ends * / here */`, and reddens on the second assertion when the terminator split is removed.
- **A line of JSON is now indexed eight bytes at a time** — `pyrs_event::LineTable` counted one
  UTF-8 character per step, both to find the line starts and to carry the cursor between events, and
  on the image that enforces the instruction floors that walk read 51.79% of
  `documents_long_strings` and 21.09% of `documents_short_strings` (`docs/dev/quality-ledger.md`
  (gl)). A `u64` now answers eight bytes at once: `zero_lanes` finds the breaks, and each candidate
  lane is checked against the byte it claims, because the borrow out of a zero lane makes a
  following `0x01` look like a break too - the new property test caught that before it shipped. The
  cursor's column advances by the byte count while a word holds no byte at or above 0x80, and a word
  that does goes back to the character walk. Measured twice on one host with the compiler the
  baseline names: `documents_long_strings` 308,642,512 counted instructions against 126,840,512
  (**-58.84%**), `line_col` down 82.6% and `LineTable::new` down 66.0%; `documents_short_strings`
  75,506,559 against 64,952,559 (**-13.98%**). Positions are unchanged: the property asserts
  `starts` and `line_col` against a rescan at every byte of an arbitrary document, and a lane table
  covers a break sitting in any of the eight positions and eight breaks in one word. Core only - no
  dependency, no unsafe, so the `no_std` set is unchanged. The floors moved with the code,
  re-measured on the image that enforces them: `documents_long_strings` 328,990,934 to 147,188,912
  (**-55.26%**) and `documents_short_strings` 95,854,968 to 85,300,932 (**-11.01%**), and two
  scenarios no local profile had named moved with them - `stream_json_medium` **-14.90%** and
  `stream_json_stop` **-27.17%** - because the same index serves the whole JSON stream route. The
  other 38 moved by 0.0051% or less.
- **The gate's widest reading was the interpreter, and it is pinned now** - three refresh runs
  record the binding harness under one binary digest (`7adbeac719edacd6`) and spread
  `load_json_long_strings` 1.121% across their nine samples, while the engine scenarios on those
  same bytes agree inside 0.00001%. Only one harness links CPython, and the term is the hash seed
  it draws per process: under `PYTHONHASHSEED=0` that scenario reads 95,136,157 three times over
  and all seven binding scenarios collapse (0.2811%, 0.2550%, 0.1249%, 0.0787% down to
  0.0000-0.0004%). `scripts/ir_gate.py` pins the seed beside the glibc caps - the same remedy that
  closed the 1.44% cross-host term - and the refresh run on this change's head reports all 42
  scenarios within 0.0010% of themselves. The floors moved only where the seed had been doing the
  work: `load_json_long_strings` -0.526%, five other loaders -0.166% to -0.055%,
  `to_python_medium` +0.051%, and the 35 engine scenarios 0.001% or less, nothing past the line
  upward. The line stays at 0.5% because the instrument no longer needs the room;
  `tests/test_ir_gate_provenance.py` names the key if it goes missing, and no shipped code moved.
- **The TOML loader's route has an instruction floor now** - `(fs)` moved `load_toml` onto
  `pyrs_toml::read_toml`, and neither Ir harness entered it: the engine channel's `from_toml_*`
  stops at the tree, the binding channel's `to_python_*` measures the YAML AST-to-Python walk.
  Two binding scenarios, `load_toml_medium` and `load_toml_long_strings`, drive `load_toml` itself
  through a feature-gated seam, and the harness asks that loader's own route guard before the
  counted loop rather than price the node model under a reader's name. On the image that enforces
  the floors, largest of three samples: 144,803,683 against `from_toml_medium`'s 120,986,802, and
  140,182,211 against `from_toml_long_strings`' 109,012,890 - the loader is 1.20x and 1.29x the
  tree builder, and that gap is the Python half no counted number described. The baseline goes
  from 33 scenarios to 35, and growing it moved its siblings by +0.212%, -0.357% and -0.111% on
  the three `to_python_*` rows - inside the 0.5% line, so the refresh was written naming nobody.
  The JSON loaders' fast paths stay wall-clock-gated; floors of the same shape are the follow-up.
- **`load_json5` takes the AST-free route its two siblings already had** — the family's fast path
  (`json_fast::try_load`, one pass straight into `dict` / `list` / scalars) was wired into
  `load_json` and `load_jsonc` only, so the loader advertising the same family paid the whole
  `CustomNode` round-trip on every document: 2.063 µs against 0.473 µs for `load_json` over the
  same 123 B of bytes. JSON5 is a superset of strict JSON, so what the fast path accepts is a legal
  JSON5 document with the same resolved value, and everything the dialect adds - single quotes,
  unquoted keys, `.5`, `0x…`, `+7`, `Infinity`, comments, trailing commas - bails the whole
  document onto the AST path that owns those semantics. Values are unchanged, judged against
  `json.loads` and `pyjson5` by 712 tests across `test_json.py`, `test_json5_spec_table.py`,
  `test_json_crosslib.py`, `test_key_resolution_parity.py` and `test_property_dialects.py`.
  Measured in one process against the same loader's own dialect route - the only way Python can
  see both - the fast path wins 6.25x on a 300-item document and 9.34x on 1,200 items, 5/5 pairs
  each, and `tests/test_json_leaderboard.py` now gates that ratio because neither of this
  loader's routes is priced by an instruction count - CodSpeed builds the workspace with
  `--exclude pyrs-yaml`, and the Ir channel's binding harness floors one route only,
  `bench_to_python`, the YAML AST-to-Python walk. With the probe disabled by hand both sides sit
  at 187.8 µs and the gate reddens naming every pair. The published JSON5 row is a
  dialect-shaped
  fixture, so it still takes the AST route and stays behind `pyjson5`; that gap needs a
  JSON5-aware direct builder, not this change.
- **The outbound escape scan runs sixteen bytes at a time in the JSON and TOML writers** — the
  readers were vectorized first because their share was bigger; the writers' turn came once the
  long-value shape had committed floors (#600), and the question "does this value need escaping?"
  measured 75.12% of `to_json_long_strings` and 69.01% of `to_toml_long_strings`.
  `write_json_string` and `write_single_quoted` now take the first byte needing a decision from one
  SSE2 pass over {delimiter, `\`, `b < 0x20`} - const-generic, so strict JSON, JSONC and JSON5 each
  monomorphize their own delimiter - and copy the clean prefix in one `push_str` before the char
  loop runs verbatim. `write_basic_body` had asked `any(forbidden_single_line)` and then re-walked
  the whole value; it now gets a position over {`"`, `\`, C0, DEL`}, with DEL its own equality mask
  because the saturating subtract that catches the C0 range cannot see it. Measured on the
  enforcing image by the refresh dispatch this change carries, largest of three samples:
  58,550,735 → 22,038,711 (−62.36%), 72,438,675 → 28,254,711 (−60.99%), 60,056,647 → 23,544,666
  (−60.80%) and 60,480,707 → 23,968,721 (−60.37%). The same-session wall-clock pair, with only the
  dispatch swapped, is 1.199 µs → 399.9 ns on TOML, 943.5 ns → 431.1 ns on JSONC and 987.2 ns →
  474.9 ns on JSON5, while `to_json`'s pair does not separate at the timer's 100 ns precision - so
  its claim is the floor, not the clock. Four new tests ask every byte value at every offset of a
  corpus crossing the vector boundary, and the output pair at lengths 0, 1, 15, 16, 17, 31, 33 and
  256: mutating TOML's DEL mask from `0x7f` to `0x7e` reddens both of them, and compiling the
  vector out leaves all 187 unit tests green on the scalar path the aarch64 wheels take. Short
  values are a different shape and no win is claimed there - the same scan is 25.95-26.61% of those
  scenarios - and one of them paid: `to_json_medium` rose to 21,492,584, +0.52%, the price of
  asking a 16-byte question of a 21-byte value, named in that dispatch so every other move still
  refuses itself. `to_jsonc_medium` (+0.48%) and `to_json5_medium` (+0.47%) stayed inside the line,
  `to_toml_medium` came down 0.84%, and `parse_inline_merge` - a YAML scenario, +0.55% on the
  development host - read +0.00% on the image, which is why the move is decided there. The YAML
  writer is untouched, its quoting answer is already one byte pass, and the only lead left there is
  `serialize_block_scalars`' line-break search inside `std`'s pattern machinery.
- **String bodies and the TOML line are searched sixteen bytes at a time** — the JSON and TOML
  readers asked "closing quote, escape, or a byte this dialect refuses?" one byte at a time, and on
  a document whose strings are long that question is between a quarter and two fifths of the whole
  parse. TOML also walked the entire source once per call to reject a bare carriage return, which
  every input over a megabyte paid a fifth of its time for and which has nothing to do with string
  length. Both searches now run on SSE2 - part of the x86_64 baseline, so this added no dependency,
  no runtime detection and no `std` - with the byte loop kept as the vector's tail and as the whole
  path on other architectures. Measured on the development host (release build, median of eleven
  interleaved runs): 2 MB of JSON with 5000-byte strings 2.47 → 1.97 ms, the same records as 6 MB of
  CJK 6.17 → 5.27, JSON5 single-quoted 1.53 → 1.11, 2 MB of TOML basic strings 2.36 → 0.99, TOML
  literal 2.41 → 0.96, a TOML multiline 3.37 → 1.01, and a 0.45 MB config of 100-byte values 2.98 →
  2.54. Each stop set is pinned by a differential test that asks every offset of a corpus holding
  every byte value against an inline restatement of the rule, and the same 200 tests pass with the
  vector path compiled out, which is the path the aarch64 wheels take. The instruction-count gate
  prices the same change on the committed fixtures, on the image that enforces them, and it moves
  down rather than holding: `from_toml_medium` −4.02% (125,580,997 → 120,534,055),
  `from_jsonc_medium` −3.69%, `from_json_medium` −3.55%, `from_json5_medium` −1.09%. No floor sits
  above the tree it now describes - the refresh dispatch recorded all 26 numbers there, which is its
  job and not a local `--update`. YAML has no
  search of its own to move: granit owns its tokenizer, and our side of a long-scalar parse is the
  scalar copy (two thirds of the share, with a whole-source memcpy as its floor) and of a
  small-scalar one the per-node build (24 ms of 39 ms for 50,000 nodes).
- **`load_toml` stopped building the tree it cannot keep** — the loader advertises the same AST-free
  route its JSON siblings take and did not take it: TOML text went scan, intermediate table tree,
  `CustomNode`, Python, so a plain dict paid for a node model that exists for format-preserving
  edits. `pyrs_toml::read_toml` now walks the IR into a caller's `TomlSink`, and the binding's sink
  builds `dict` / `list` / scalars through the same `scalar_projection` the AST and event routes
  already share - one function states what `0b1101`, a multi-line string and a date mean, so a
  third reader cannot restate them differently. The guard is the tag registry: a registered handler
  rewrites a tagged scalar on its node, and only the node model has nodes, so a document read while
  any handler is installed takes the AST route. Measured against `rtoml` in the same run - the host
  drifted about 20% between the two snapshots, so absolute readings are not the evidence - the
  published row went from 1.92× and 1.91× behind to 1.47× and 1.46×, about 1.31× faster on both
  fixtures; a paired in-process comparison against the route the guard selects reads 1.15-1.34× and
  1.31-1.35×. The alternative that was measured and rejected: the event stream reaches the same dict
  and costs the same (slower where the document has notes, a wash once its positions and notes are
  stripped), which is why this reader carries no notes, no offsets and no anchors.
  `crates/pyrs-toml/tests/value_reader.rs` flattens the tree and the new reader to the token list
  the stream comparison uses and requires them equal over the seed corpus in both dialects, with a
  named floor of 16 documents and a refused document reaching no value; `tests/test_toml.py`'s
  `TestValueReader` compares eight documents against the hub's own export and reimport, and proves
  the guard by a value only the AST route can change. What is left of the row is the intermediate
  tree, which stays.
- **The JSON loaders' fast route has an instruction floor now** — `(ft)` floored `load_toml`'s route
  and named what it left open: `load_json`, `load_jsonc` and `load_json5` each ask
  `json_fast::try_load` first, and neither harness entered that route. The engine's `from_json_*`
  rows stop at the node model the fast reader skips, and the binding channel measured only
  `bench_to_python` and `bench_load_toml`, so a per-key allocation inside the JSON fast path would
  have moved no committed number. `bench_load_json` now drives `load_json` end to end and
  `load_json_medium` / `load_json_long_strings` floor it over the same bytes the engine reads
  (`MEDIUM_JSON`, `long_string_json()`) at 57,286,388 and 95,502,189 counted instructions from the
  enforcing image - 0.74x and 0.86x what `from_json_medium` and `from_json_long_strings` pay, the
  opposite direction to TOML's 1.20x because this route replaces the tree and the YAML text rather
  than adding to them. `load_jsonc` and `load_json5` share the row by design: for bytes the fast
  reader accepts all three call exactly the same code, and what differs is the fallback the engine
  already prices. The premise is per-document rather than per-process, because `try_load` bails on
  the whole document at any non-canonical spelling, so the harness asks `json_route_is_fast` of the
  fixture's own bytes before the counted loop and exits 4 instead of pricing the fallback under
  this name; forcing that guard to `false` made `load_json_medium` exit 4 while
  `load_toml_medium` stayed green. Growing the scenario set moves its siblings, so the refresh ran
  on this head: `to_python_anchors` read +0.1343% and `load_toml_long_strings` -0.2175%, both
  inside the 0.5% line, so the run was written naming nobody.
- **The edit layer has an instruction floor now** — `AGENTS.md` names parse, edit and AST serialize
  as the three expensive pure-Rust computations, and the gate priced two of them: no scenario
  entered `set_path`, `insert_path` or `delete_path`, so an edit that began doing more work moved no
  committed number. Three scenarios join, one per verb, over the `MEDIUM_YAML` the readers already
  use, with the tree cloned inside the measured span and `--setup-only` subtracting that clone along
  with the parse, so what is left is the verb; `move_path` needs no row because its halves are
  priced. Named gate-only rather than after divan's `edit_set_medium`, which replaces a whole
  `database` section — a different edit, and a shared name would make two reports of different work
  read as one measurement. Both premise guards were observed against a single misspelling: the
  harness exits 3 on `missing-path`, and the test-time pin fails on the same scenario for the same
  reason. Growing the set was priced by the refresh instead of predicted — six JSON readers moved
  +0.73%…+1.91% on the enforcing image with no engine file in the diff, were named in
  `accept_drift`, and left `tolerance_hint` at 0.5%. Recorded as `(gf)`.
- **The event stream's cancellation half now costs a number, and it is a quarter of the walk** —
  `parse_stream(..., on_event=...)` stops a producer mid-document and nothing priced the stop: the
  JSON producer had no floor in any shape (`stream_medium` and `stream_long_strings` are YAML's path
  over granit), so a change to the check that ends the walk, to the state dropped on the way out, or
  to an event built for a consumer that had already refused could land with every gate green. Two
  paired scenarios on `MEDIUM_JSON` - `stream_json_medium` and `stream_json_stop` - and the run that
  wrote the file read 50,237,295 against 12,603,257: the stop costs **25.09 %** of the walk, which
  is the evidence that `Break` ends the scan and not merely the delivery. That document emits 49
  events, so the stop sits at twelve; the limit is derived from the stream's own length in setup
  rather than written down, and both premise guards were observed against a mutation - asking for
  the whole stream exits 3 ("so a quarter asks for 49"), and a sink that answers `Break` one event
  late exits 3 ("the sink saw 13 events where the stop asks for 12 of 49"). Growing the set was
  again priced by the refresh instead of predicted: the first dispatch refused over one upward move,
  `load_json_long_strings` +0.76 %, measured from a binding binary whose digest the committed file
  already records, so the runner moved and the code did not - named in `accept_drift`, and
  `tolerance_hint` stayed at 0.5 %. TOML deliberately gets no stop scenario: its `Break` ends the
  emission, not the scan, so the number would price the intermediate representation under a name
  that promises cancellation. Recorded as `(gi)`, with `(gh)`: the harness had copied a floor into a
  comment (`parse_medium` "321,836,091" against a committed 321,580,801), and a refresh rewrites the
  JSON without ever touching a comment, so those citations state the relationship now and leave the
  numbers to `.ci/ir-baseline.json`.

### [v0.18.3] — 2026-10-22

<details markdown="1">
<summary>gates that read the tree · crate matrix · note ownership · wider wheels</summary>

#### Added

- **A landed claim in a live document names its code** - `docs/dev/event-stream.md` said "decision 6
  landed" against a trunk that carried only the docs-only commit `0a074148`, and nothing compared a
  design doc's present tense with the tree. `scripts/check_landed_claims.py` now requires a heading
  saying landed or shipped to carry, on the line below it, the file - optionally `::symbol` - that
  makes the sentence true; it runs as the `landed-claims` prek hook and through
  `tests/test_landed_claims_gate.py`.

- **A cancelled CI leg re-runs itself** — `.github/workflows/ci-rerun-cancelled.yml` fires when the
  CI workflow finishes on a pull request and, through `scripts/ci_rerun_gate.py`, re-runs that run
  only when a leg was `cancelled` on a run whose other legs went green and none failed, and only
  below an attempt cap so it cannot loop. A leg cancelled by `cancel-in-progress` supersession or a
  reclaimed runner used to turn the required `Test matrix (all legs)` verdict red for reasons that
  were not the change's, and a human had to press Re-run before anything could merge. The verdict's
  accept-set is untouched — a `cancelled` leg still refuses a merge — so this adds a retry, not an
  exception. `tests/test_ci_rerun_gate.py` pins the decision in both directions and the wiring as a
  contract.

- **The YAML engine builds for a target that has no standard library** — `pyrs-yaml-core` is now
  `#![no_std]`: the parser, the round-trip serializer, the comment-preserving edit layer and the
  splices run on `core` + `alloc` alone, on top of a `granit-parser` that was already built without
  std. The `std` feature stays on by default, so the Python binding, the CLI and the benches keep
  the API they had; it is what adds the three surfaces that need an OS rather than the engine — the
  translated error-message catalog (rust-i18n owns a process-global locale), the runtime schema
  registry (a global `RwLock`) and the regex-backed user-schema language. A `no_std` caller selects
  a schema by constructing the `pyrs_schema::types::Schema` variant directly. The three
  `no-std-check` steps in `ci.yml` now build the crate for `thumbv7em-none-eabi` and run its unit
  tests on the host, and `tests/test_no_std_gate.py` reads the foundation set out of the crate roots
  that declare `#![no_std]`, so a crate that joins the group without joining the gate is a red run
  rather than a silent gap. The `no_std` build tracks the scalar keys it has already seen in the
  crate's ordered node map rather than in a `HashSet`, because `alloc` carries no hasher: the
  `parse_medium` allocation ceiling is 74 allocations against the std build's 67 — one more per
  mapping — and `tests/alloc_gate.rs` records a ceiling per feature set.

- **The no_std story is checked against the crates the sources actually declare** — the foundation
  set is read out of the crate roots that write `#![no_std]`, and `tests/test_no_std_gate.py` now
  compares it with two more things: the three `no-std-check` steps in `ci.yml`, and the crates the
  live documents name. `AGENTS.md` gained the `no_std` foundation story it never had, and the
  paragraph in `docs/dev/boundaries.md` that pointed at it named five crates after the YAML engine
  had joined the group — the gate that would have said so was a comment. The dated records are
  exempt: `CHANGELOG.md`, its four mirrors and `docs/dev/quality-ledger.md` are history, and history
  may name a five-crate foundation, because that is what was true when it was written.

- **The crate matrix is measured, not described** — `scripts/check_rust_python_decoupling.py`
  derives the matrix from the manifests themselves and enforces four rules: exactly one crate is a
  cdylib (the extension), no other crate's dependency closure carries a Python-runtime crate, the
  extension holds no Rust test tier and declares `[lib] test = false`, and every `required-features`
  entry names a declared feature that is not a default. The gate carries its own strict TOML reader,
  so it needs neither `cargo` nor `tomllib` and runs on the same Python 3.8 floor as the package -
  including in the Python test legs, where cargo is absent.
  `tests/test_rust_python_decoupling_gate.py` is the falsification control: every rule is broken on
  a synthetic tree and required to flip the verdict, the reader is compared against `tomllib` on
  every manifest in this repository, and a manifest the reader cannot understand must exit 2 rather
  than pass. `crates/pyrs-yaml/Cargo.toml` now declares `test = false`, so cargo stops building a
  test target for the extension, and the hook in `prek.toml` runs the gate on every change that
  touches a manifest or a crate.

#### Changed

- **A note in an event stream says which node it belongs to** — `pyrs-event`'s
  `StreamEventType::Comment` carried one flag, `standalone`, and it answered two questions at
  once: did the note have a line to itself, which is a fact about the source, and which node is
  the note filed under, which is a fact about the tree. A consumer that wanted to put the note
  back where the tree had it could not ask the second question, so the flag is two now.
  `owns_line` keeps the source's own answer, and a new `NoteOwner` enum carries the tree's:
  `FollowingLeading`, `FollowingInline`, `PrecedingLeading` and `PrecedingInline`, named in the
  consumer's build order, so `following_*` is the node the consumer builds next and `preceding_*`
  the one it built last. Every Rust caller in this workspace that constructed
  `StreamEventType::Comment` was updated in the same change, and these crates are not published,
  so no shipped API moves; Python callers of `parse_stream` see the old `style` key
  (`standalone` / `inline`) unchanged and gain an `owner` key naming the same four placements.
  The three producers answer the new question from the node they filed the note under, so the
  stream and the tree cannot disagree: the YAML producer files a note that trails a node in that
  node's inline slot and a note that leads the next node in its leading list; the TOML producer
  reports a table's own notes after the table's name, because the notes above `[a]` lead the
  mapping `a` holds and not the name; and the JSON producer reports each group of notes with the
  side and the slot the tree's own rule gives it. Two visible details follow: a TOML note written
  after a comma now reports `style: "inline"` where it reported `standalone`, because the note did
  not own its line and `owner` says where it belongs instead; and a TOML table's notes now arrive
  after its key event rather than before it. The JSON producer also stopped re-reading the source
  for the notes' offsets - the offset travels with the record - so the stream walk no longer
  scans the text a second time. The split is proved by tiers that ask the tree and the stream the
  same question: `crates/pyrs-yaml-core/tests/event_notes.rs` walks fourteen documents twice and
  asserts that every note the tree files is reported in the same slot with the same direction and
  the same own-line fact, the JSON and TOML replay suites compare the whole note instead of
  skipping one of the two fields, and a JSONC document folded back from its own events
  round-trips through the strict writer to the same text.

- **The `rules:` schema language is judged from outside** — `tests/test_parse_schema_spec_table.py`
  asks each `type:` name the grammar accepts a question this repository does not own: whether the
  value a rule resolves is really an instance of the JSON Schema type that name denotes.
  `jsonschema` owns that membership, and its disjointness of `boolean` and `integer` — even though
  Python's `bool` subclasses `int` — means a mistyped rule is a red run, not an agreement; each row
  carries a decoy type the same value must not satisfy, so the confirmation cannot be a rubber
  stamp. The `external authority` cell of the `Schema rule language` row moves from `no` to `yes`,
  the `authority-tier:schema-rule-language` hole is deregistered, and `.ci/quality-holes.json` is
  empty again. `scripts/quality_matrix.py` gained `jsonschema` in its oracle set so the credit comes
  from the reader the test calls, not from the file's suffix.

- **The decoupling gate learned to guard the bench leg** — `scripts/check_rust_python_decoupling.py`
  added rule R5: the extension crate may not hold an `[[bench]]` target without `required-features`,
  because such a target is selected by every workspace-wide bench command and would rebuild the
  `pyo3-build-config` interpreter probe the CI bench legs only just excluded. The gate had four
  rules; this fifth locks in what the bench-leg scoping bought, so a future unconditional bench in
  `crates/pyrs-yaml` is a red run rather than a quiet return of the coupling.
  `tests/test_rust_python_decoupling_gate.py` breaks R5 on the synthetic tree and requires the
  verdict to flip - 13 negative controls now - and the clean workspace still passes.

- **The CI Rust benchmark leg builds without an interpreter too** - CodSpeed's `Rust benchmarks` was
  the last leg to pull the Python runtime, for the same reason the test legs did: workspace
  *membership*, not a dependency edge. `cargo codspeed build` selects every member, so it compiled
  the `pyrs-yaml` cdylib and the `pyo3-build-config` build script that probes for an interpreter,
  even though neither `yaml_bench` (in `pyrs-yaml-core`) nor `cli_bench` (in `pyrs-yaml-cli`)
  depends on the binding - `cargo tree -i pyrs-yaml` names no workspace member. The leg now runs
  `cargo codspeed build --workspace --exclude pyrs-yaml`, the exclusion the test legs already used,
  and drops its `setup-python` step because nothing in the job probes for an interpreter. Measured
  on this tree: the scoped build selects `yaml_bench` and `cli_bench` and compiles zero
  `pyo3`/`numpy`. The `ir_gate` target is untouched - it is gated on `ir-gate`, was never a CodSpeed
  benchmark, and `scripts/ir_gate.py` builds it deliberately when it means to start CPython.

- **The Rust benchmarks live in the crate that measures them** — the divan benchmark set moved from
  `crates/pyrs-yaml/benches/yaml_bench.rs` to `crates/pyrs-yaml-core/benches/yaml_bench.rs`. The
  forty-seven benchmarks measure the same code as before: the binding crate only re-exported the
  engine's API, so every import changed name and nothing else did. The bench's package closure
  drops the six Python-runtime crates it inherited from the binding crate (`pyo3`, `pyo3-ffi`,
  `pyo3-macros`, `pyo3-macros-backend`, `pyo3-build-config` and `numpy`) and carries none. The bench
  binary was already free of a Python runtime — Linux `DT_NEEDED` lists no `libpython`, and
  Windows' import table has no `python3.dll` — but only because the linker pruned a dependency that
  no symbol referenced; the dependency is gone now, so the independence is structural and a gate can
  assert it. The extension module keeps its own link to `libpython3.12.so.1.0`, which is where a
  Python extension belongs. The measured code is unchanged, so the only visible difference is the
  package segment of the CodSpeed identity: the benchmark entries are re-baselined under
  `pyrs-yaml-core`.

- **The event stream and the merge resolver keep their anchors in the crate's node map** — a
  `#![no_std]` crate cannot name `std::collections::HashMap`, so `event_to_stream_event` and
  `resolve_anchor_name` now take `&mut NodeMap<usize, String>` and the merge resolver's anchor
  table is a `NodeMap<String, _>`: the same `IndexMap` over `FxHasher` the AST already uses, so an
  anchor's order is its insertion order and the keyed lookups are measurably cheaper. Over the
  instruction-count gate's YAML scenarios the tree moved `parse_anchors` -2.36% and
  `parse_inline_merge` -2.32%, with `parse_medium` -0.15% and `parse_small` -0.21%. The committed
  floor was not touched, so the slack the gate now accepts on those two scenarios is stale in the
  fast direction and waits for a refresh on the enforcing image.

- **The Ir gate prints the compiler's diagnostic instead of cargo's summary** — a failing harness
  build reported only `error: could not compile pyrs-yaml (lib) due to 1 previous error` and
  nothing else. The gate builds with `--message-format=json`, which moves rustc's diagnostics onto
  stdout as `compiler-message` records, but it printed `res.stderr` alone, so the cause it was
  hiding — `rust-lld: unable to find library -lpython3.12` — was only visible by rerunning cargo by
  hand. `build_diagnostics()` now joins every rendered diagnostic from the build stream and falls
  back to the stderr tail only when stdout carries none, which is the missing-toolchain and
  bad-manifest case rather than a compile error. `tests/test_ir_gate_diagnostics.py` pins both
  streams and the fallback; the mutation that returned stderr alone reddened two of its three
  tests.

- **The standard JSON Schema surface is judged by the official corpus, in a row of its own** —
  `Document.validate()` hands evaluation to Python's `jsonschema` and keeps only the AST-to-instance
  conversion, the schema acceptance and the `YamlValidateError` contract here, so the matrix's
  existing `schema validation` row was measuring the *other* surface: the rule language
  `validate_against_schema()` evaluates, which this repository invented. A new row,
  `JSON Schema validation`, is credited by `tests/test_document_validate_suite.py` replaying the
  official JSON-Schema-Test-Suite at commit `0b4a7fa` (46 files, 1302 cases). Measured on
  `jsonschema` 4.26.0: 1261 cases reach the corpus's verdict - `refRemote.json` is excluded whole as
  a contract boundary (no resolver is exposed, and one representative case is probed to keep that
  exclusion live), and ten `dynamicRef`/`vocabulary`/`\p{...}` cases are named as boundaries of the
  delegated engine. The same corpus replayed through `jsonschema.validate()` directly reaches the
  same verdict on all 1302 cases, so no exclusion is attributable to the conversion. The exclusion
  set is keyed on the installed version - the Python 3.8 leg resolves 4.23.0, which needs one more
  named `unevaluatedProperties` case, fixed by 4.25.1 - and both directions are asserted: a named
  case that starts agreeing reddens the run, and so does an unnamed one that stops agreeing. The
  invented language keeps its registered hole `authority-tier:schema-validation` rather than
  borrowing this corpus's credit; its exit now says that retiring it in favour of
  `Document.validate()` closes it.

- **The `rules:` schema language keeps a row of its own, and its blind spot keeps a name** — the
  matrix's schema paragraph said the rule language's "Rust entry points are deleted" after the
  validator was retired on 2026-10-21, but the *language* is still here: `parse_schema_yaml()`,
  `RuleResolver` and `YamlTypeKind` are exported by `pyrs-yaml-core`, and `register_schema()`,
  `load_schema()`, `list_schemas()` and `YAML(schema=...)` still read a `rules:` block through it.
  It had no row in the table at all, so the page claimed four surface rows where five shipped, and
  the registry read `holes: []` while a grammar no outside reader can contradict was measured only
  by its own author's tests. The row is derived now rather than written: `scripts/quality_matrix.py`
  scopes a row's unit and property tiers to the file that declares the surface, so the language is
  credited with the eleven `#[test]`s of
  `crates/pyrs-yaml-core/src/parser/yaml/schema_language.rs` and the one property that names it,
  `prop_schema_parse_no_panic`, instead of the YAML engine's crate-wide counts. Its round-trip and
  parity cells read `n/a` — it resolves scalar types and owns no serializer, and no second
  implementation of the grammar exists to compare against. The authority cell reads `no`, which is
  the honest answer: the grammar was invented here, so the unit, property and fuzz tiers all ask the
  engine what it believes. The measurement derives the hole `authority-tier:schema-rule-language`,
  the registry carries it with a checkable exit, and `QUALITY_MATRIX.md` names it in the blind-spot
  table — so the registry holds an entry again, and a second falsification control asserts that the
  retired validator's symbols are gone *and* that the surviving language is present, which keeps the
  page from drifting back to "deleted" in either direction.

- **`sort_keys` on a scalar path is a documented no-op, not a silent one** — `sort_keys("$.a")`
  returned `None` and changed nothing while `sort_keys("$.missing")` raised `YamlEditError`, and
  nothing the package shipped said so; the registry entry recorded the mismatch with the exit
  "answer the way a missing path does, or state the no-op in the shipped stub". The stub now states
  it: a path that resolves to a scalar has no keys to sort, so the call answers `None`, while a
  path that does not resolve still refuses. The test that pinned the shape is named for the
  contract and asserts both sides, `api:sort-keys-scalar-path-noop` leaves
  `.ci/known-residuals.json`, and the matrix's Known residuals table now names only
  `api:set-comment-container-owner`.

- **The parity column reads a unit even when no reader is bounded** — `crosslib_parity` returned
  `yes` on the bounded-reader check before reading a single test, so a family whose readers all
  reach every leg was credited by its import list alone: a `test_*_crosslib.py` whose every
  comparison carried `@pytest.mark.skip`, and a file with no test at all, measured `yes`. The unit
  rule now runs for every module. YAML keeps `yes` because its guards cannot fire - every comparison
  carries `skipif(pyyaml is None)` or `skipif(not _ruamel_available())`, both readers are declared
  without a marker, and the helper is resolved by its body (`return _ruamel is not None`) - while
  `skipif(sys.version_info < ...)` still denies. Reading YAML's comparisons off `safe_load`,
  `safe_dump` and `parse` needed the dialect regex to know the unsuffixed default verbs. Controls
  pin all four no-live-comparison shapes (`@pytest.mark.skip`, no test, a module-level `pytestmark`,
  an interpreter gate) to `partial`, and the dead availability guard to `yes`; the measured tree is
  unchanged - YAML, JSON and TOML parity still read `yes`, and the derived hole list is still the
  single `authority-tier:schema-validation`.

- **A corpus suite is credited by the corpus it names** — the external-authority column's no-reader
  branch credited every `tests/test_*_suite.py` or `*_spec_table.py` that imported no reader, so a
  file with the right suffix and a placeholder test measured `yes` while the prose said a corpus
  file counts by the suite it replays. The branch now requires the module to name its corpus in a
  string constant - the `Reference/` root all three suites hand to their loader - and a mention in a
  comment is not a load. Controls pin both sides: a suffix-only file and one naming the corpus only
  in a comment read `no`, the real corpus suite still reads `yes`, and the measured tree is
  unchanged - `test_yaml_suite.py` is the only file that reaches the branch, all seven authority
  files read `yes`, the four format rows keep `authority=yes`, and the derived hole list is still
  the single `authority-tier:schema-validation`.

- **The external-authority column stops crediting an import** — a `tests/test_*_suite.py` or
  `*_spec_table.py` file read `yes` when it imported a reader no marker bounds, whether or not
  anything in the file still reached it. The cell asked about dependencies rather than about tests,
  so a comparison deleted beside a live-looking import would have left the column green. A file now
  earns the reader credit only by reaching into that reader: a call in a test, or a callable wired
  into a module-level table before any test runs - the shape the writer suites use. `json is None`
  is a probe, not a comparison, and an import with no use is neither. The bounded branch keeps its
  stricter rule, because a skip there can be the one that removes the reader from a leg; a corpus
  suite still needs no reader, since a corpus has no reader to be absent. `tests/test_yaml_suite.py`
  was the one file that read `yes` through a bare import: `import yaml` existed only to derive the
  `HAS_PYYAML` flag its gates use, while PyYAML loads the corpus inside `pyrs_yaml.compliance`, so
  the flag now comes from that module and the file is credited by the suite it runs. Controls pin
  all four shapes - import-only and a `json is None` probe read `no`, a module-level reader table
  and a corpus suite read `yes` - and the measured tree is unchanged: all seven authority files read
  `yes`, the four format rows still read `authority=yes`, and the derived hole list is still the
  single `authority-tier:schema-validation`.

- **The last parity hole closes: JSON5 and JSONC have a reference on every leg** — the registered
  entry named the two dialects whose only outside reader was `pyjson5`, declared
  `python_version >= '3.9' and python_version < '3.14'` with armv7l/s390x/ppc64le excluded, so the
  3.8 floor this package promises and 3.14+ compared both against nothing while the leg reported
  green. The pure-Python `json5` package (`>= 0.17.3`, `requires-python >= 3.8`, a `py3-none-any`
  wheel) is declared in the test group and imported unconditionally by the two files that judge the
  dialect; the comparisons that judge it carry no skip marker, and `pyjson5` stays as a second
  opinion where its wheel installs. The two references disagree on two constructs, and the tests pin
  this engine's reading instead of hiding the difference: a surrogate pair is combined into one
  character here where the pure reader keeps the two UTF-16 code units, and a lone surrogate is
  refused here where that reader accepts it. `tests/test_json5_spec_table.py` no longer gates its
  whole module on `pyjson5`, so the spec table's rows stop skipping on the floor. Measured:
  `cross_library.restricted` is empty, the JSON row's parity cell reads `yes`,
  `.ci/quality-holes.json` holds only the schema row, and the external-authority column now yields a
  verdict per file rather than crediting a name - a spec table that regresses behind a bounded
  reader alone is emitted as a hole of the `authority-tier` kind, named for the file.

- **The TOML three-way agreement now runs on every leg, not only above 3.11** — the cross-library
  module skipped its eleven pyrs/tomlkit/tomllib comparisons on 3.8 through 3.10, because `tomllib`
  arrives with 3.11 and nothing installed a reference reader below it. The backport `tomli>=2.0` is
  now declared for `python_version < '3.11'` in the test group and imported under the `tomllib`
  name, so every leg runs all 24 comparisons; the row names `tomli` and `tomllib` separately with
  their own bounds, and `tomlkit` still judges the dialect everywhere, which keeps the verdict at
  `yes`. The reader search also stops at the end of a distribution name:
  `_dependency_marker("tomli")` used to answer the docs group's `tomli-w` requirement and report a
  marker that belongs to a package no test imports. A control now pins `tomli` to its own marker and
  `tomli-w` to none. The official-corpus encoder half, skipped on the same legs by the same
  `tomllib` probe, also runs there now; its three `spec-1.1.0` rows spell a fractional second with
  one digit, which CPython < 3.11's `fromisoformat` refuses, so the test's own converter pads the
  fraction to six digits.

- **A `partial` parity cell is a registered hole, not a footnote** — the parity column already
  derived its verdict from the oracle imports in each `test_*_crosslib.py` and the marker
  `pyproject.toml` gives them, and it read `partial` for the JSON family because `orjson` and
  `pyjson5` are both declared `python_version >= '3.9' and python_version < '3.14'` with
  armv7l/s390x/ppc64le excluded. The page explained that in prose while the registry stayed silent,
  so the sentence "the JSON family has a cross-library tier" was quotable about the 3.8 leg this
  package promises, where the module imports nothing, skips, and reports the same green as a leg
  that ran all 43 of its comparisons. `parity_gaps()` emits `parity-tier:json-jsonc-json5` from the
  same derivation, `.ci/quality-holes.json` carries it with the exit that removes it (an empty
  `restricted` list, measured the same way), and the hole-id slug is now derived from a label's
  alphanumerics so `JSON / JSONC / JSON5` cannot spell itself into an id nobody can read.

- **RFC 8259 is now judged on the legs that cannot install a JSON reader** - the JSON cross-library
  module gated itself as a whole file, so on the 3.8 floor this package promises, and on 3.14+, all
  43 of its comparisons skipped, including the ones that need no third-party reader. stdlib `json`
  is one of them now: `parse_constant` refuses the three literals the stdlib accepts by default,
  which makes it an RFC 8259 oracle that no dependency declaration bounds and that every leg has.
  Measured on a simulated excluded leg (`orjson` and `pyjson5` both unimportable): 29 comparisons
  run and 44 skip, where the same leg ran 0 of 43 before; 73 run where both readers install. The
  parity cell had to become per dialect for that to be reportable - `crosslib_parity()` credits a
  dialect only when a test calls an unbounded reader itself, in a test nothing can skip, so
  `@needs_pyjson5`, a module `pytestmark`, a docstring mention, or a helper defined after the test
  all earn nothing - and the hole stays registered, naming `json5` and `jsonc` rather than the whole
  family.

- **A bound the interpreter draws is now a bound the row reports** - the parity column asked only
  `pyproject.toml` whether a reader could be missing from a leg, and it looked for readers only in
  module-level imports. `tomllib` is invisible to both halves: no file declares it, and
  `tests/test_toml_crosslib.py` binds it with `pytest.importorskip` inside the one test that wants a
  second opinion. The leg's own skip counts said what the cell could not - CI's floor leg ran 13 of
  that module's comparisons and skipped 11, and 3.9 and 3.10 skip the same 11 - so the row named one
  reader while the file asks two. `STDLIB_SINCE` bounds a stdlib reader against the floor read out
  of `requires-python`, and `_oracle_locals()` now takes an `importorskip` binding from the syntax.
  The TOML verdict stays `yes`, and stays honest: `tomlkit` is declared unbounded and reaches every
  leg, so what 3.8 through 3.10 lose is the three-way agreement, not a dialect judged by nothing.
  That module imports `tomlkit` outright now - a guard no marker bounds is dead code while it is
  green and a silent skip the moment it is not, and once `tomllib` counts as bounded its blanket
  `pytestmark` would have denied the credit and manufactured a hole that does not exist. Seen by
  mutation: setting `tomllib`'s arrival version to the floor's reddens both new controls.

- **The tier table's counts are now written by the measurement that checks them** — the row was
  already pinned to the probe by a comparison test, so a stale digit was a red run rather than a
  lie; what was missing was a way to *produce* the right one. That gap cost PR #549 all 14 test
  legs: a new test file moved the Python count and the author had to know to retype it, and two open
  pull requests that both move the row force a replay of whichever lands second.
  `python scripts/quality_matrix.py --write-rows` now rewrites the sentences from the same
  measurement, keeps the prose around each number, and refuses to report success on a table it can
  no longer parse; `--check-rows` prints the drift instead. Demonstrated on this branch: replaying
  onto `47349338` (which carries #553's tests) moved the Python count 1,822 → 1,824 and one flag
  wrote the row.

- **`pyq` answers the `jq` and `yq` command line** - an argv that is not one of pyq's own
  subcommands is now read as `jq` 1.8 or mikefarah `yq` v4 (chosen by the name the binary was
  invoked as, or forced with `--as jq|yq`) and translated into the equivalent subcommand. Accepted
  forms are judged against the real tools rather than a hand-written expectation:
  `crates/pyrs-yaml-cli/tests/dialect_vs_reference.rs` runs `jq-1.8.2` and `yq v4.54.1` when they
  are on `PATH`, refuses to accept any reference inside this workspace (the first version of the
  probe found our own copy of `yq.exe` on `PATH` and reported nine perfect matches that proved
  nothing), and compares stdout byte for byte over 24 invocations including `.a.b`, `.list[] |
  select(.x > 1)`, `del(.a.n)`, `.a.n = 9`, `-o json`, `-r`, `-N`. Anything the shell cannot answer
  with the same document is refused by name and exits 2 with empty stdout: pipes other than one
  trailing `select`, every jq function (`keys`, `length`, `map`, `to_entries`), jq edit
  expressions (pyq writes YAML where jq writes JSON), jq's `--arg`/`-s`/`-c`/`-S`/`--stream`, and
  yq's `-n`/`-e`/`-P`/`--split-exp`/`--front-matter`/`--xml-*`/`--lua-*`. `pyq get` gained
  `--no-doc`, which is the one behaviour difference the probe found: yq v4 prints a multi-item
  stream without `---` separators, so that is now the dialect default. The whole contract is
  stated in `pyq --help`.

- **The mid-range interpreters stop re-running the suite on the paid runners** — the planner made
  the floor, the top *and* the coverage carrier run the suite on every OS, so
  `test (macos-latest, 3.12)` spent 173 s (run 39424748440) asserting what
  `test (ubuntu-latest, 3.12)` already asserts with the threshold attached, on a runner GitHub bills
  at 10x. Measured on PR #550's green run (47 jobs, 96.8 wall-min of legs): macOS is 22.3 of those
  minutes but ~223 of the ~300 billed minute-equivalents, and the two mid-range cells on the costly
  OSes were the only suite legs whose role is defined to sit on ubuntu. `scripts/ci_matrix_plan.py`
  now derives what an expensive runner owes from the declared range alone — first and last — in
  `expensive_suite_versions`, the demoted cell stays in the grid as a wheel smoke instead of
  disappearing, and `check()` refuses a plan that moves a gate role onto a costlier OS. 14 suite
  legs become 12; the weekly schedule still runs all 24 cells with the suite, which is what licenses
  the demotion rather than hiding it.

- **The test matrix stopped waiting for a wheel it never installs** — `build` compiled two artifacts
  in one job: the abi3 wheel the suite legs install, and the `--find-interpreter` release recipe the
  compliance report reads. Measured on two pull-request runs of 44 jobs each: the abi3 wheel was
  uploaded at +134 s (Windows), +135 s (macOS) and +151 s (ubuntu) in the first and at +172 s, +192
  s and +167 s on the same three in the second, while the release build behind it cost 45–346 s more
  per operating system, so `build` closed at +485 s and +477 s and every one of the 24 legs of
  `test` and `wheel-smoke` began 291–471 s after the only artifact it reads already existed. The
  release recipe is a job of its own now, `release-wheels`, and `tests/test_ci_artifact_wiring.py`
  pins the two producers apart by reading each job's own `upload-artifact` and `download-artifact`
  inputs: a job that uploads both families, or a leg that waits on a producer whose artifact it
  never installs, is named by the run. Against the previous `ci.yml` that check reports the merged
  job, so the shape it refuses is the one that shipped. Makespan is projected from 856 s and 846 s
  to roughly 500 s — the fan-in still waits on the slowest release build — and this pull request's
  own run is the measurement. Runner minutes are unchanged, 100.2 and 93.1: no work was removed,
  only the queue in front of it.

- **Every performance number on the site now comes from one committed measurement** — the
  wall-clock tables on the four locales' performance and feature pages were transcribed by hand;
  the 2026-08-17 figures had no script or record behind them, and the same calls now measure
  22–108× faster than those readings. `scripts/bench_snapshot.py` times batches of calls (nine
  batches of at least 0.1 s, fastest reported) and commits the reading with its interpreter,
  library versions and fixture sizes to `.ci/perf-snapshot.json`; `--docs` rewrites all twelve
  tables from it, and `tests/test_perf_docs.py` compares every cell back, so a table cannot drift.
  The prose had drifted separately: the index, round-trip, PyYAML-compatibility, benchmark,
  comparison and feature pages still quoted 21–43×, 55–177×, 48–100×, 123–371×, 37× and 85× after
  the tables withdrew them, so the gate now derives every speed range quoted as prose on all 24
  pages that carry one and refuses the withdrawn date and ranges by name. The pages now state
  29–34× parsing and 223–368× serialization against PyYAML, 68–81× and 541–876× against
  ruamel.yaml, and 45–59× / 102–125× on the round-trip guides. The yaml-rs trade-off paragraphs
  say 2–4× faster serialization and about 2.5× slower `YamlDocument` parsing, measured with the
  same timer; the 2–4× is named rather than derived because it comes from the cross-library
  benchmark, not the snapshot.

- **The Rust benchmark set now measures TOML, JSON and the 1.1 resolver** — the CodSpeed Rust leg
  ran one file in which every case was YAML text in, YAML or Python out, so the two engines this
  tree has grown most since and the profile that decides what a plain scalar means had no
  wall-clock series. Six benches join `crates/pyrs-yaml/benches/yaml_bench.rs`:
  `from_toml_medium`/`to_toml_medium`, `from_json_medium`/`to_json_medium`,
  `resolve_yaml11_type_many` and `needs_quotes_over_scalars`. The four format benches read the
  `bench_inputs` documents the instruction gate measures, and carry the Ir scenarios' names so a
  wall-clock move and an instruction-count move can be read as one pair. No floor moves: CodSpeed
  keeps ranking and the zero-regression verdict stays with `.ci/ir-baseline.json`.

- **The MSRV job installs the MSRV it claims to test** — `msrv-check` installed
  `dtolnay/rust-toolchain@stable`, so it compiled the workspace on current stable and called that
  the minimum: `rust-version = "1.96"` documents a floor without enforcing one, and a dependency
  needing 1.97 would have passed the job and failed at a user's install. The step now pins the
  `1.96` all eight crates declare, and `tests/test_msrv_gate.py` fails in both directions — a
  workflow pin that moves away from the manifests, or a manifest that moves away from the pin.
  The floor was measured before the pin changed: `cargo +1.96.0 check --workspace --locked` and
  `cargo +1.96.0 test --workspace --locked` both pass on this tree.

- **A second registry records the residuals the matrix cannot measure** —
  `.ci/quality-holes.json` can only describe a hole `scripts/quality_matrix.py` can derive, so its
  empty list read as "no residual" while understood, deliberately open behaviour remained: a TOML
  table-header note that cannot keep its line, a container `set_comment` whose note lands on the
  last child, and a scalar-path `sort_keys` that answers `None` while a missing path raises. The
  new `.ci/known-residuals.json` registers each with a repro node id, an owner and a checkable
  exit; the pinned test carries `@pytest.mark.known_residual`, and
  `tests/test_known_residuals.py` fails in both directions — an entry with no live repro, or a
  marked repro with no entry. Fixing one reddens its characterization test, which forces the
  entry out in the same change.

- **The aarch64 wheel legs build on native arm64 runners** — `publish.yml`'s `linux` and
  `musllinux` aarch64 rows move from `ubuntu-22.04` under qemu to `ubuntu-24.04-arm`, and the
  glibc leg runs its suite there through `uv` instead of emulation. s390x is the only wheel suite
  still emulated; the `pyq` aarch64 cross-build, armv7 and ppc64le are unchanged.

- **`main`'s post-merge job now proves the claim it used to print** — the only job trunk runs for a
  code change asserted in an `echo` that the pushed tip's "identical tree already passed the full CI
  matrix", with nothing measuring it and no path that could fail. `scripts/check_main_provenance.py`
  asks that sentence's three questions against the API — which pull request produced the commit,
  whether the pushed tree is one of that pull request's commit trees, and whether
  `Test matrix (all legs)` reported `success` for the commit carrying it — and refuses otherwise.
  The content claim held on every tip measured (`07f988fa`, `60f670d0`, `225cf373`), so the hole
  was in the shape of the defence. The gate's own first trunk run then went red for the wrong
  reason, and the fix is part of the same change: a commit can carry several records of that check
  name, the API does not list them oldest-first, and a `workflow_dispatch` run of this workflow
  leaves a `skipped` beside the pull request's real verdict — its verdict job only runs on
  `pull_request`. Reading the last row read that absence as a refusal. `newest_verdict` drops the
  records that judged nothing and sorts the rest by the time each job finished, so `fddcc76e`
  passes and seven ancestors still refuse, each for a verdict that exists.

- **The per-format defence table is derived, and two of its cells were wrong** —
  `scripts/quality_matrix.py` now computes `QUALITY_MATRIX.md`'s format table from the files
  that declare the defence (the fuzz targets, the `proptest!` blocks, each `test_*_crosslib.py`
  and the dependency markers in `pyproject.toml`), and `tests/test_quality_matrix.py` pins every
  cell. Measured against it, `schema validation` was under-reported — `validate_schema.rs`
  parses before it validates, so its `fuzz parse` cell is `yes` — and the property tier is 27,
  not 25, because the count demanded a `prop_` name while `pyrs-event`'s two properties are
  named in prose. `partial` in the parity column now means a declared interpreter or
  architecture bound on the oracle, not an impression.

- **JSON5 `\xHH` escapes parse — the reader had every other JSON5 escape** — `load_json5` and
  `from_json5` accepted `\v`, `\0` and the line continuation but rejected `\x` followed by two hex
  digits, which the JSON5 escape set includes, while the shipped stub already said `load_json5`
  "Accepts the full JSON5 grammar". Measured against the reference: pyjson5 gives `{'a': 'A'}` for
  `{a: '\x41'}` and pyrs reported "invalid escape sequence". The two digits are now accumulated one
  byte at a time — `from_str_radix` would accept a leading `+`, making `\x+1` a valid escape — and
  strict JSON and JSONC keep rejecting the form. Rust tests pin both quote styles, `\x0A` and
  `\x00`, the truncated and non-hex tails, and a malformed escape straddling a multibyte character;
  a pyjson5 parity test pins the acceptance and both narrower dialects' refusal.

- **JSON5 escaped keys and every line terminator, plus the table that keeps them** — two
  productions the same stub promised were still missing: a backslash-u escape inside an unquoted
  key, which the ES5 `IdentifierName` grammar JSON5 inherits allows wherever an identifier
  character may appear, and a backslash before CRLF, LS or PS inside a string. The continuation
  gap was also a disagreement between the two scanners of one dialect — the CRLF form parsed in a
  double-quoted string and failed in a single-quoted one. An escaped character is now taken as the
  code point it stands for instead of being re-checked against `ID_Start`/`ID_Continue`, which is
  what makes an escaped digit or dot a legal key while the literal character is not, and both
  quote styles now share one terminator rule. The audit that found them is a gate as well:
  `tests/test_json5_spec_table.py` compares 41 accepted and 11 rejected productions against
  `pyjson5`, because a syntax the reader never accepted is invisible to the property and fuzz
  tiers, whose oracles can only compare paths that already exist.

- **A flow mapping cannot spell a block key, and the generator no longer draws one** — a random
  20k-case property-tier run went red on `{? [null]: A, ? [null]: B}`: a mapping holding the same
  one-item sequence twice, spelled block once and flow once. Under a block parent that pair really
  is two keys — `? - null` beside `? [null]` re-reads as two entries, measured — but inside `{...}`
  a block container has no legal form, so the writer emits the flow token for both and the reader's
  duplicate-key refusal is the correct answer. The tree was the generator's rather than a
  document's: `arb_mapping` filtered its keys with a context-free rule. `keys_collide_in_flow`
  folds both keys to the only spelling a flow parent allows before comparing them, normalizing a
  copy so `KeyIdentity` stays the single model of what a spelling means, and the agreement pin now
  re-asks its whole table under a flow parent, holding the one direction the generator is
  accountable to: a pair the rule lets through has to survive the reader.

- **The defence matrix now measures its external authorities** — the per-format table in
  `QUALITY_MATRIX.md` gains a sixth column, `external authority`, derived by
  `scripts/quality_matrix.py`: `yes` when a `tests/test_*_suite.py` or
  `tests/test_*_spec_table.py` exists for that dialect. Until now the table credited the unit,
  property, fuzz and parity tiers only, so the strongest gates in the repository —
  `test_yaml_suite.py`, `test_json_test_suite.py`, `test_toml_test_suite.py` and
  `test_json5_spec_table.py` — were invisible on the page that lists the defences, while parity,
  which compares two implementations, was credited as if it asked the grammar.
  `schema validation` reads `no`, because nothing outside asks its rules.
  `tests/test_quality_matrix.py` asserts the column cell-by-cell and names one file per row, so
  dropping a suffix from the derivation reddens the test instead of quietly deleting a column.

- **A JSON string of nothing but a Unicode space kept its value** — `load_json5` answered
  `{None: 1}` for a document whose key is U+2028 alone, while `load_json` and `load_jsonc` gave back
  the string and `pyjson5` agreed with them. The hub stores a decoded JSON string as a YAML scalar,
  plain unless `needs_quotes` says otherwise, and the JSON resolver trims a plain scalar with
  Unicode rules — `resolve_json_type` keeps `str::trim` deliberately, because JSON5's structural
  whitespace really is that wide — so a text beginning or ending with a non-ASCII space lost that
  character and a text made only of one resolved to null. The quoting rule knew ASCII space and
  NBSP; it now knows both ends of all 25 Unicode `White_Space` code points, which is where the two
  tables stop disagreeing. It is asked on the character path the byte pass already reaches for any
  text holding a non-ASCII byte, so a text that is pure ASCII pays nothing.
  `tests/test_json_whitespace_scalar.py` sweeps the set in key and value position through the three
  dialects and asks `pyjson5` and `json` that the key is a string and not null.

- **A JSON5 string can hold a raw control byte** — the double-quoted scanner stopped on every
  byte below 0x20 and refused it, which is RFC 8259 §7's rule for strict JSON rather than JSON5's:
  the dialect forbids only a line terminator inside a quote. A document holding a real tab was
  therefore refused by `load_json5` while `pyjson5` read it, and this file's own single-quoted
  string had always copied the same byte through — one dialect answering twice, decided by which
  quote opened the string. The stop set is now the dialect's: JSON5 takes every C0 byte except CR
  and LF, strict JSON and JSONC keep refusing all 32, and the JSON5 writer escapes what it read,
  so the round trip is still one line. `tests/test_json5_spec_table.py` asks all 30 in both quote
  spellings against the reference, and the two strict loaders are held to the RFC for the same
  bytes.

- **The JSON writers gained a reader they do not own** — the write side of the family was checked
  only against our own reader, and a fixed point is satisfied just as happily by an implementation
  that is wrong twice the same way. `tests/test_json_writer_suite.py` now generates 1,500 trees,
  emits each with `to_json`, `to_jsonc` and `to_json5`, and hands the document to Python's `json`
  and to `pyjson5` — strangers that have never seen the encoder — requiring the value back to be the
  tree that went in. Measured before it was pinned: 0 refusals, 0 value changes on all three
  routes. It bites: making `write_json_string` emit a raw tab reddens both strict legs while the
  JSON5 one stays green.

- **A YAML string spelled the way 1.1 spells a number arrived as a number** —
  `safe_dump({"v": "1_000"})` emitted `v: 1_000`, and PyYAML and ruamel both handed back the
  integer 1000. Same for `8_0`, `0b101`, `+0x1f`, for the legacy bool words `yes`/`on`/`off`/`y`
  and for the date-only timestamp `1990-06-23`, which comes back as `datetime.date`. Twenty-six
  spellings per route, in key position as well as value, on the Python route and on the JSON→YAML
  one. The quoting guard asked only `resolve_core_type`, which is this engine's own 1.2 core
  reader, and every one of those spellings is a string there; a reader on the YAML 1.1 type
  repository types them, and so does this engine's own `schema="1.1"` profile for `y` and `yes` —
  the writer and one of the profiles disagreed about the same token. The guard now asks the 1.1
  table as a second question over four families: the bool words core does not list, `_` inside a
  numeric, a base prefix core cannot see (`0b`, and `0x`/`0o` behind a sign), and `YYYY-MM-DD`.
  Quoting is the safe side of the error — a quoted scalar is never resolved — so each family is
  the shape the table accepts rather than a parser: `a_b`, `0x` and `2026-10` stay plain, and
  `12:00` needs none of this because a colon already refuses plain emission one layer below.
  Output changes only for these spellings: `1_000` now writes as `"1_000"`.

- **The YAML writers gained a reader they do not own** — `tests/test_yaml_writer_suite.py` emits
  through `safe_dump`, `from_json` and the edit layer and hands each document to PyYAML and to
  ruamel.yaml, requiring the value back to be the one that went in; 400 generated trees per route
  and every legacy lexeme in key and value position. Measured after the fix: 0 refusals and 0
  value changes on all three routes, which is what `QUALITY_MATRIX.md` now credits the YAML row
  for beside the official-suite reader gate. The edit layer is held to a narrower rule than the
  data routes, and the file pins both halves: a path segment *addresses* the key a reader resolves
  (issue #336), so `$.~` and `$.42` must keep their plain fold, while a name that would lose its
  type between the two profiles — `rename("$.v", "yes")` wrote a key that reads `true` — is
  quoted. Temporal objects stay outside: `!date` is a local tag a SafeLoader has no constructor
  for, so the strangers refuse that document rather than change a value, which is a boundary of
  the tag design.

- **The TOML writer gained a parser it does not own** — the last of the three write-side cells.
  `test_json_writer_suite.py` and `test_yaml_writer_suite.py` covered two families, and TOML's
  outside tiers (`test_toml_test_suite.py`, `test_toml_crosslib.py`) both ask the reader, so the
  writer was checked only against our own. `tests/test_toml_writer_suite.py` emits 400 generated
  trees through `to_toml` and `YamlDocument.to_toml()` and hands each document to `tomlkit` and, on
  Python 3.11+, `tomllib`. Measured: 1,074 of 1,200 probe trees emitted with 0 refusals and 0 value
  changes, and the 126 that never reached a stranger were one typed refusal,
  `toml-cannot-represent-null` — a value the format has no word for is refused, not given an
  invented spelling. Two boundaries are pinned rather than smoothed over: a `datetime.time` goes out
  as a bare local time, which is TOML 1.1's (1.0.0 has no local-time type and the readers here
  decode it), and an integer beyond i64 goes out quoted, because a string is what the hub calls it
  on every route. No behaviour changed; the gate is insurance against a dialect rule that reaches
  one direction of the pipe and not the other, which is the class two of the last three fixes were.

- **A verbatim `!<tag:yaml.org,2002:X>` reaches the plugin registered as `!!X`** — YAML 1.2 §6.8
  makes the two spellings one tag, and the loader already honoured both for the five tags it names
  by hand (`!!str`, `!!int`, `!!float`, `!!bool`, `!!binary`). For every other name the plugin
  registry is keyed on the rendered text, so the verbatim form fell through to schema resolution:
  measured on the trunk of 2026-10-18, `!<tag:yaml.org,2002:widget> 7` ignored a plugin registered
  as `!!widget` and loaded the integer `7`. The spelling the document used is tried first, so the
  second look is paid for only on a miss, and a URI outside YAML's namespace keeps its current
  reading — resolved as if the tag were absent, not raised.

- **The parity column now recognises an oracle it declares under another name** — the probe
  reads how `pyproject.toml` bounds each cross-library oracle and matched the literal import
  name: `ruamel.yaml` is imported that way and declared as `ruamel-yaml`, so the
  lookup answered `undeclared` and the YAML row reported an oracle installed on every supported leg
  as one the project never declared. The lookup normalises `-`, `_`, `.` and case the way PEP 503
  does, and `tests/test_quality_matrix.py` now refuses any `undeclared` marker, so an oracle the
  probe loses track of fails the run instead of misreporting itself.

- **A schema that omits `extends` resolves the way its documentation says** — `extends` is optional
  and `docs/en/guides/custom-schema.md` states its default as `core`, but the parser only reached
  Core when the key was present with an empty value. A schema that left the key out carried no base
  resolver, which typed every scalar as its own text, and both directions of the verdict inverted
  without complaint: `type: int` refused `port: 80` - the error said `expected int but got
  Str("80")` - and `type: str` accepted it. The Rust row that pinned the old default is now two
  rows, the second recording the boundary that remains: a base name this engine cannot find still
  leaves no fallback, and refusing a typo'd name is a separate call about a public error surface.

- **An edit that orphans an anchor is refused, not written** — on `base: &x 1` with `use: *x`,
  renaming that anchor, replacing its node without carrying its metadata, and moving the definition
  away all answered success and emitted a document whose only remaining reference names an anchor
  nothing defines; this library's own reader refused `base: &a0 1 / use: *x` with
  `found unknown anchor`. The rule was already in the engine — `with_metadata_from` keeps the
  document's anchor name when a value is replaced with its metadata — and these three doors bypassed
  it, so the question is now asked where the drop happens: at `set_path`, which `set_many_path` and
  `move_path` share, at `delete_path`, and at the anchor verbs. The guard is wider than the set of
  aliases that would really dangle: an alias binds the *nearest preceding* anchor of its name, so a
  rename that a later `&x` would have kept alive, and a subtree holding both an anchor and the alias
  naming it, are refused too. Over-refusing costs the caller one more edit; under-refusing writes a
  file nothing can read. Making a rename carry its aliases is a decision about what the verb means,
  and is recorded as one.

- **The edit layer has a fuzz target of its own** — the workspace has nine targets now, and the
  eight before it handed the engine a *document*; nothing drove a *change* to one, which is the
  layer the round-trip promise is about. `fuzz_targets/edit_yaml.rs` reads `document NUL script`,
  picks paths the reader really found, and requires every accepted edit to re-parse and to leave the
  value it wrote where it says it wrote it — with removal claims stated per kind, because a key is
  an identity while a sequence index is a position the next item shifts into. It reached the anchor
  defect above on its first seed replay, again minutes into its first exploration, and reported
  three shapes it records rather than asserts: a sort asked of a scalar answers "ok" and changes
  nothing, a note asked of a node does not always stay on that node, and a flow member carrying a
  tag and a note settles only on the second round - an open defect in the note's flow attribution,
  committed as a seed so the fix inherits the input.

- **A `date` or `datetime` now leaves the YAML writer under the tag every reader knows** —
  `safe_dump()` wrote `!date` and `!timestamp`, and a local tag has no constructor outside this
  library. Measured on the trunk of 2026-10-18: `yaml.safe_load` and `ruamel.yaml` both answered
  such a document with `ConstructorError`, so the value was not mistyped, it was unrecoverable.
  Both objects now leave as `!!timestamp` (`tag:yaml.org,2002:timestamp`), the one temporal tag
  YAML defines, which those readers resolve to a `date` for a date-only text and a `datetime` for
  anything with a time part; this reader makes the same fork, so what a stranger can type, we type
  identically. `!date` and `!timestamp` stay registered and keep resolving, so a document an
  earlier version wrote still reads back as the same object, and a document that arrived with one
  is re-emitted with it unchanged - only objects built from Python take the standard tag. `!time`
  remains a local extension: YAML defines no tag for a bare time of day and PyYAML cannot
  represent one. The hub tag TOML's own values carry is moved by the entry below.

- **TOML's dates and datetimes enter the hub under the same standard tag** — the projection in
  `crates/pyrs-toml/src/lib.rs` tagged a date-only value `!date` and a date-time `!timestamp`, so a
  document that merely passed through (load TOML, write YAML) left with the local spelling the
  entry above stops writing at the Python boundary. Both now carry `!!timestamp` and the load path
  forks on the text, so a TOML date still becomes a `date`, not a midnight `datetime`. It is
  visible in `parse_stream`, whose `tag` field for a TOML temporal is now `!!timestamp`; a bare
  time keeps `!time` as the one local temporal tag, and `!date` still reads.

- **A flow member whose value is a tag with no content keeps its note on the first round** —
  `{a: !t }` spells its value as nothing, so a note written after the tag has no body to sit on:
  the reader hands the note to the enclosing pair and the tag moves behind it, and the document
  settles only on its second emission. Nothing is lost across the round, which is why no other
  tier saw it. The writer now spells that empty body `''` when a note follows it — the same
  decision that writes `~` for an empty value before a note (libFuzzer `edit_yaml`
  crash-b23eed05).

- **A note on a block sequence item keeps the dash line** — an item mapping whose note is set
  through the edit layer had no slot on the `-` line, so the writer pushed the whole mapping onto
  its own lines; the reader then filed the note with the pair that ends the block and the next
  emission compacted the line back, so the text took two rounds to settle. The compact line now
  carries the item's note (`- a: 1  # n`), and when the pair line has already claimed YAML's one
  trailing slot the note moves above the dash (`# n` then `- a: 1  # m`) — both spellings are
  fixed points (libFuzzer `edit_yaml` crash-61474da6).

- **An empty plain scalar is spelled as the null it resolves to** — a scalar with no text and no
  properties rendered as nothing, and the reader does not hand the same node back from an empty
  line: `k:` and `-` both re-read as a null whose own spelling is `~`, so the first emission settled
  only on the second round (libFuzzer `edit_yaml` crash-63749496, reached from `- >` with its style
  set to plain). The writer now writes the null text wherever the body would be empty — nested
  value, root value, sequence item, flow pair value, flow item. A tagged empty is left alone: the
  tag suppresses resolution, so its body keeps the `''` spelling that settles it (#514). An anchor
  is not the same case: it names the node without suppressing resolution, so `- &a3` is now written
  `- &a3 ~` (libFuzzer `edit_yaml` crash-07b6a10e).

- **The TOML writer's dialect promise gained a property tier** — 1.1's one visible freedom is a
  newline (and so a comment) inside an inline table, and nothing asked whether a tree that carries
  no note was handed 1.0-readable text. `prop_toml_undecorated_output_loads_as_1_0`
  (`crates/pyrs-yaml-core/src/fmt_pbt.rs`) now requires that a generated tree with no note anywhere
  reloads under `from_toml_v1_0`, while
  `an_undecorated_inline_table_writes_compact_and_loads_as_1_0` and
  `the_1_0_reader_refuses_the_1_1_inline_table_freedom` (`crates/pyrs-toml/src/writer.rs`) pin the
  inline-table arm by shape and its oracle premise. Measured: forcing the multi-line branch first
  reddened only the named test and left 20 000 generated cases green, so the property's input gained
  a strategy that always draws a document holding an inline table — the same mutation now reddens
  the property in 0.01 s. A third property states the equality claim the tier was missing:
  `prop_toml_roundtrip_preserves_the_tree` compares the tree before and after one round - values,
  shape and order - and two value-dropping mutations reddened it (the text fixed point reddened in
  both runs too, so this is the claim stated directly rather than a strictly stronger oracle). No
  behaviour changed; the counts move to 720 Rust tests and 29 properties.

- **The edit tier now judges what an edited document exports** — a hub document is what
  `doc.to_toml()`, `doc.to_json5()` and the CLI's TOML/JSON output write, so "edit, then export" is
  the route a user takes, and every tier before this one had only handed those writers a *reader's*
  tree: `toml_roundtrip` and `json_roundtrip` never see a tree the splice layer rewrote.
  `edit_yaml.rs` now writes each settled tree in every dialect, compact and pretty alike, and asks
  the two questions those targets already ask — the dialect's own reader must accept the text, and
  its second emission must be the same bytes. A dialect that refuses the tree is a pass, because
  TOML has no top-level scalar and strict JSON refuses a non-finite float. The edit verbs stay
  YAML-only by design (`crates/pyrs-toml/src/lib.rs` says so), so this is one target asking seven
  exporters, not seven edit targets. The same two questions went into `json_roundtrip` for the three
  pretty JSON spellings, which that tier had never asked of text nobody chose to generate.

- **A `set` that replaces a container no longer drops a referenced anchor defined inside it** — the
  guard at `set_path` asked only about the replaced node's own anchor, and that node carried none:
  its child did. `preserve_metadata: true` copies exactly the replaced node's own properties, so
  writing over the mapping deleted the only definition of `x` and left `use: *x` naming nothing —
  text this library's own reader refuses (libFuzzer `edit_yaml` crash-fbd364b3). `delete_path` had
  already been given the wider, subtree-shaped answer through `anchors_in`, so the same question is
  now asked at this door and refuses with the same `anchor-is-referenced` reason, leaving the tree
  untouched.

- **A note written on a tag-only sequence item keeps that line** — `- !fuzz2` spells its value as
  nothing, so a note set after the tag had no body to sit on: the reader filed it as the item's
  leading note and the next emission wrote it as a line of its own above the dash, so the text moved
  on the second round although nothing was lost (libFuzzer `edit_yaml` crash-3dd60cad). The writer
  now closes the pending tag line before an inline note, exactly as it already does before a note
  line, and the item is written `- !fuzz2 ~  # fuzz 47`.

- **A signed zero reaches TOML as the zero TOML reads back** — the integer pass-through keeps a
  plain scalar's own spelling when that spelling is legal TOML, and `-0` is legal text. But TOML has
  no negative zero: `ist = -0` re-reads as `0`, so the second emission moved. A fixed point is
  spelled the way the reader spells it, so a signed zero writes as `0` while the radix spellings the
  reader really does keep (`0xDEAD`, `0b1`) pass through unchanged (libFuzzer `edit_yaml`
  crash-917e103b).

- **An emptied block sequence keeps its dash line** — removing the last item of a block sequence
  leaves the flow flag behind, and the compactness rule vetoed a compactness the emission does not
  have: an empty container has no block form, so it is always spelled `[]` on the key's line. The
  first emission was `-\n  t: []`, the reader answered `- t: []`, and the text moved on the second
  round with nothing lost (libFuzzer `edit_yaml` crash-a58caa38).

- **A JSON string spelled with a long integer keeps its quotes across the hub** — the Core resolver
  answers `Str` for a digit run too long to fit an `i64`, while JSON puts no bound on an integer
  literal, so `"9999999999999999999"` projected as a plain scalar and `to_json` wrote it back as a
  number: `load_json` kept a string and one trip through the hub made it an integer. The projection
  now asks JSON's own number grammar before writing a string plain, which is the question the writer
  will ask of it (libFuzzer `json_roundtrip` crash-09341b75).

- **The `schema="1.1"` profile resolves the numeric families 1.1 defines** — `1_000` is the integer
  1000, `0b101` is 5, `+0x1f` is 31, `+.inf` is infinity and `017` is 15, each the answer PyYAML's
  loader gives the same text. The profile used to add only the legacy booleans to Core, while the
  writer clause #504 had been quoting these families for a reader this engine did not have — so
  `schema="1.1"` disagreed with the engine's own quoting rule. Timestamps and sexagesimal stay text
  (the temporal-tag ruling keeps untagged text text; a colon already forces quoting one layer
  below), and `0o17` stays typed here while PyYAML reads it as text, because the profile is Core
  *plus* and `0o` is a Core prefix.

- **A string that only a 1.1 reader types is no longer emitted plain** — `0x_1f` is 31 to PyYAML and
  text to Core, and the clause answering "would a 1.1 reader type this?" checked underscores only on
  decimal tokens, so the Python string `"0x_1f"` went out plain and came back a number. The gap was
  found by a new invariant test that asks, of every text this profile types, that `needs_quotes`
  agrees.

- **The TOML writer's section header stopped materialising its key** — `to_toml` collected each
  root table's header text into a `String` of its own before writing it after every top-level pair,
  while the section list already carried the key node; the header arm can append through
  `push_scalar_key` exactly as a pair line does. That was the writer's only materialised key:
  `to_toml_medium` reads 26,741,057 counted instructions against its 28,939,722 floor, -7.60%, one
  allocation and one copy fewer per header.

- **A root table that TOML cannot place after its pairs keeps its position inline** — TOML admits
  no root key/value after a root table header, so `to_toml` wrote every section after every pair and
  the mapping's order did not survive the round trip: a tree with `0` holding a table and then `a`
  came back as `a`, `0`. The generator reached it at the 20 000-case property tier
  (`prop_toml_roundtrip_preserves_the_tree`); such a table is now written inline, at its own place,
  while every order a parsed document can carry keeps the header form.

- **A sequence's own note keeps its dash line** — a note the edit layer attaches to a
  *container* was written above the line that ended that container's block. When the container
  is a sequence, the line that ends it is one of its items' own dash lines, and granit reports a
  note written below a dash line as that item's leading note — so the second emission moved the
  text off the sequence and into its item (`edit_yaml` crash-08b30707: `?ss:` over two tagged
  empty items). A sequence's note now goes above the sequence's first line, which is where the
  reader files a sequence's own note and what `lift_first_item_notes` already relies on, while a
  mapping's note stays above the pair that ended its block, because that is where a pair's note
  is read back.

- **The benchmark #522 broke now asserts its contract in the suite that runs** — the
  `Python benchmarks` leg went red on `806811c5` on `assert '[server]' in ...` while the required
  `Test matrix (all legs)` reported success. #522 makes a root table that sits before a later root
  pair go inline, so the fixture's `[server]` header is no longer the emission; the assertion was
  describing the pre-#522 spelling, not a property of the document. It now lives in the normal
  suite (`tests/test_toml.py::TestToToml::test_benchmark_fixture_roundtrips_by_values_and_order`),
  because benchmark modules are deselected without `--codspeed`. Reloading the emission catches a
  lost or altered key; comparing the root key list catches the order, and swapping two emitted root
  lines keeps the first comparison equal while flipping the second. The benchmark body is back to
  the measured call alone.

- **The instruction-count gate is now part of the merge verdict** — the `Instruction-count baseline`
  job lived in `.github/workflows/codspeed.yml`, but `Test matrix (all legs)` fans in over
  `ci.yml`'s jobs only, so its result was a report: #522's red `Python benchmarks` leg reached
  trunk while the required check was green, and the Ir gate could have followed it. The job moves
  to `ci.yml` unchanged and runs on the same `changes` classification as the other heavy legs;
  `matrix-verdict` now waits on it, so a performance regression refuses a merge instead of arriving
  on trunk. The two CodSpeed jobs stay advisory in `codspeed.yml`: sub-10% wall-time noise makes
  them a ranking channel, not a merge decision.

- **The Ir gate now refuses a floor the tree has outgrown** — the gate read one edge of the
  baseline: a scenario past the tolerance failed, while a floor far above what the tree now does
  was accepted silently, so the accepted slack widened every time the engine got faster. The
  2026-10-14 refresh had found six writer floors 22–46% below their tree and four readers
  4.7–11.2% below. `scripts/ir_gate.py` now reads both: a scenario more than 2% below its floor
  warns as `FLOOR_STALE`, more than 5% fails, and the failure names the refresh dispatch instead
  of `--update`, which would move the floor to the local host. The first enforcing-image refresh
  under the rule (run 39242949404, artifact `ir-baseline-39`) needed no `accept_drift` and moved
  `to_toml_medium` by −7.12% — accepted by the old gate, refused by the new one — and
  `from_toml_medium` by −3.00%; the largest upward move was `serialize_block` +0.32%, and every
  scenario repeated within a 0.000% spread. `.ci/ir-baseline.json` is that artifact unedited.

- **A mapping note over a nested sequence settles in one round** — the note attached to a mapping
  whose block ended inside a nested sequence was written above the last item's dash line, where the
  reader reports it as that item's leading note: `ss:\n  - !a\n  # fuzz 35\n  - !a\n` re-read as
  `ss:\n  - \n    # fuzz 35\n    !a\n  - !a\n`, so the first emission was not a fixed point. The
  writer's `pair_line_start` field, which places a container's own inline note, was left pointing at
  the sequence item because a block value writes its own starts into the field; each pair now
  restores its own line before returning, so the note lands above the pair that ended the mapping's
  block. The note's owner still moves — the ruling on `set_comment` for a container leaves that as
  it is — but the text settles in one round, pinned over three shapes
  (`a_mapping_note_over_a_nested_sequence_settles_in_one_round`).

- **A TOML integer keeps the spelling it was written with** — `1_000`, `+5`, `-1_000` and the
  radix forms lost their source text on the way through the hub: the parser cleaned the token before
  recording it, so `to_toml(from_toml("a = 1_000"))` came back as `a = 1000`. Both separators and an
  explicit sign are legal TOML (`underscored` and `signed-int` in the ABNF), so the loss was in the
  recording, not in the grammar. The parser now keeps the token text whole; the projection tags a
  spelling only when YAML Core resolves a different value (`0x1f` and `+5` stay untagged, while
  `1_000`, `0b1101_0110` and `-1_000` carry `!!int`), and the writer's literal gate accepts the same
  grammar it emits. A tagged integer is authoritative over the quotes the YAML emitter needed for a
  leading `-`; an untagged quoted `"42"` stays a string. Values are unchanged and agree with
  `tomllib`; pinned by `TestIntegerSpellingFidelity`,
  `decimal_spellings_reach_the_hub_word_for_word` and
  `a_quoted_integer_stays_a_string_unless_a_tag_says_otherwise`.

- **A declared `%YAML` version is honoured on request, and the output says so** — a document's
  header used to be reported by `version()` and ignored by the reader, so `%YAML 1.1` resolved
  under Core and the directive was dropped from the output. Passing
  `honor_version_directive=True` to `parse`, `parse_file`, `parse_all_docs`, `safe_load`,
  `safe_loads` or `YAML(...)` now lets a `1.1` declaration select the YAML 1.1 profile
  (`flag: yes` reads `True`, `012` reads `10`); `1.2`, `1.3` and an undeclared document still
  resolve under Core, and an explicit `schema` wins over the declaration. With the flag on, an
  honoured declaration is re-emitted as `%YAML <version>` before its `---` — after edits and
  `reparse()` too — so a Core reader cannot silently retype the text; a later declaration in a
  stream is preceded by the `...` end marker the grammar requires. The default is unchanged
  (flag off, Core resolution, no directive in the output); `schema` now defaults to `None` so
  "not given" and "explicitly Core" are distinguishable. Pinned by
  `TestYamlVersionNegotiation`.

- **The benchmark pages now name the measurement they actually use** — `codspeed.yml` has run
  `mode: simulation` since the workflow was added, but `AGENTS.md` and all four locale benchmark
  and features pages said "WallTime". The pages now say simulation counts instructions, and the
  millisecond tables are marked as wall-clock readings recorded on one machine on 2026-08-17, not
  CodSpeed output. The medium fixture is 687 B, not ~500 B; the Rust table is labelled as the
  `cargo codspeed` simulation leg over `pyrs_yaml_core::bench_inputs`, whose "small" document is
  19 B against the Python table's 95 B, so same-named rows are not comparable. In the feature
  matrix PyYAML now checks anchors/aliases, explicit tags and merge keys (measured on 6.0.3), and
  the type-stub row says what the packages ship: PyYAML none, ryaml and yaml_edit yes. The ABI3
  row names the real floor, py3.8+.

- **`safe_dumps_async` is published in `pyrs_yaml.__all__`** — the alias is assigned next to
  `safe_dumps` in `python/pyrs_yaml/__init__.py`, and three locales' API pages already documented it
  as a member, but only its three async siblings were declared:
  `from pyrs_yaml import safe_dumps_async` resolved while the declared public surface said the name
  did not exist. No behaviour changes, the name was already importable;
  `tests/test_api_doc_signatures.py` now compares the same 35 names across all four locales'
  `reference.md` with no exception for this one.

- **The direct-load eligibility decision moved into the engine crate** — whether a document falls
  inside the value-only fast path is a property of the document and the grammar, not of the Python
  object the value becomes, so the decision and the two tests that pin it now live in
  `pyrs-yaml-core` (`crates/pyrs-yaml-core/src/parser/yaml/direct.rs`). The binding crate keeps
  the materialization, the only part that needs a Python object to exist, and its Rust test tier
  is now empty. The premise this step started from - that the Rust test tier needs a Python
  runtime - does not hold at the binary level: of the 848 artifacts `cargo test --workspace
  --no-run` builds, exactly one names a `python*.dll`, and it is the extension cdylib itself. What
  stays coupled is the build graph - `pyo3`, `numpy` and their build-time crates are still in the
  binding crate's closure - and the binding crate's `ir_gate` bench harness, which starts CPython
  to count instructions. No behaviour changes: `safe_load` and its siblings answer exactly the
  documents they answered before.

- **The engine's CI legs no longer build the Python runtime** — `cargo test --workspace` and the
  20 000-case property tier now pass `--exclude pyrs-yaml`, and `PYO3_PYTHON` goes with them.
  Measured on this tree: the first command resolves 250 compilation units over 153 packages, and its
  package set names `pyo3`, `pyo3-ffi`, `pyo3-macros`, `pyo3-macros-backend`, `pyo3-build-config`
  and `numpy`; the same command with the exclusion resolves 220 units over 139 packages and names
  none of them. What is given up is zero, and that is what licenses the exclusion: `cargo nextest
  run --all` and the same run with the exclusion both execute 748 tests, and the extension has no
  doctests to lose (`cargo test -p pyrs-yaml --doc` reports 0). The rule that keeps the two claims
  together is `tests/test_ci_rust_leg_exclusion.py`, which reddens when a workspace-wide test leg
  stops excluding the crate the decoupling gate names as the extension, keeps `PYO3_PYTHON` while
  excluding it, excludes a different member instead, or when the MSRV leg starts excluding it —
  because that leg's whole purpose is proving the declared floor for every crate, the shipped one
  included. No published behaviour changes: the same 748 Rust tests and the same Python suite run.

#### Removed

- **The invented rule validator is retired; `Document.validate()` is the only schema surface** —
  `validate_against_schema()` and `validate_against_registered_schema()` are removed, and a
  `validate:` section in a schema definition is no longer read. Both names evaluated a rule language
  this repository invented, while `Document.validate()` already delegated to Python's `jsonschema`;
  two validators meant two error contracts and two answers to "is this document valid". Callers move
  to `Document.validate(schema)`, whose `YamlValidateError` carries the reference engine's message,
  or to `jsonschema.validate()` directly. The CLI's `validate` binds `--schema` and `--schema-file`
  to one parameter (a JSON Schema file) and checks syntax only when neither is given. Scalar-type
  resolution is untouched: `register_schema()`, `load_schema()`, `list_schemas()`,
  `YAML(schema=...)` and the `rules:` list still apply. The Rust types that existed only for the
  retired language (`TypeSpec`, `ValidateKind`, `ValidateRule`, `SchemaValidationError`) are
  deleted, and the matrix's `schema validation` row leaves with them: the table derives four rows
  now, and `.ci/quality-holes.json` measures `holes: []`. The fuzz target that exercised the row is
  renamed `validate_schema` → `parse_schema`, because it fuzzes the surviving `rules:` grammar
  rather than JSON Schema evaluation. The four locales' guides, quick start, tutorial, feature table
  and API pages document `Document.validate()`, and the shipped stub names neither removed function.

#### Fixed

- **The two newest releases are folded like every older one** - `## [v0.18.1]` and `## [v0.18.3]`
  reached all five release-note pages without the `<details markdown="1">` fold that every release
  since v0.12.x carries, because `scripts/check_doc_metadata.py` read a fold's attributes and never
  asked whether one existed. Both sections are folded now, and the checker fails a released section
  that has no fold or no `<summary>` - four new tests among the eleven, including the shape that
  shipped.

- **The wheel legs install what the suite imports** - `.ci/requirements-test.txt` is the only
  requirement file the release workflow installs, and it named neither `tomlkit` nor `json5`, which
  four test modules import at module scope. The first Publish run for this release reddened five
  wheel legs at collection before a wheel uploaded and the `Release` job skipped, so nothing reached
  PyPI. The PR matrix could not have caught it: it installs the dev groups, where both are declared.
  Both are declared now, and `scripts/check_wheel_requirements.py` (a prek hook, gated by
  `tests/test_wheel_requirements_gate.py`) judges every module-scope import in `tests/` against that
  file, so the drift is caught at the edit that causes it rather than at a tag.

- **The CLI command-surface inventory gate now executes** - `tests/test_cli_binding_parity.py`
  resolved the App by reading `getattr(pyrs_yaml.cli, "app")` first, and
  `python/pyrs_yaml/cli/__init__.py` binds no such attribute: it defines only `main()`. Any earlier
  test that runs `from pyrs_yaml.cli.app import app` - which `tests/test_cli.py` does at module
  scope - binds that name to the SUBMODULE, so the fixture returned a module, `_commands` was
  missing, and the guard skipped with a message blaming cyclopts' private layout. Measured:
  `pytest tests/test_cli_binding_parity.py` passes 22, adding
  `tests/test_cli.py::test_entry_point_without_cyclopts` before it makes the inventory assertion
  SKIPPED, and the same skip is in the CI log of `test (ubuntu-latest, 3.10)` on this repository's
  own runner - so the assertion that the registered command set equals the documented inventory had
  not run in a full session anywhere. The fixture now accepts a package attribute only when it
  carries the command table; a missing `_commands` is a failure naming the installed cyclopts
  version rather than a skip that is indistinguishable from drift; and one control test asserts the
  resolution never lands on the module, so the ordering regression reddens instead of going quiet.
  The inventory itself was checked against the live App before the change: 18 documented names
  against 18 registered, `missing=[] extra=[]`, so this makes the guard honest rather than
  uncovering a surface drift.

- **Every pyq fixture directory now carries a counter as well as the process id** - the dialect
  tier's comparison directories got that guard when a shared path reddened `rust-lint`, and the
  other writes in the same crate still named their directory after the process alone: six sites in
  `crates/pyrs-yaml-cli/tests/cli.rs`, the `pair()` helper and its one manual copy in
  `crates/pyrs-yaml-cli/tests/compare.rs`, and the compat filter test in `src/compat.rs`. No two of
  today's tests pick the same prefix or tag, so this is the same invariant applied everywhere rather
  than a second reproduced collision; the counter removes the dependency on that staying true, and
  `compare.rs` loses its manual copy of `pair()`'s two writes.

- **The dialect tier shared one fixture directory between its comparisons** -
  `crates/pyrs-yaml-cli/tests/dialect_vs_reference.rs` named its temporary fixture directory after
  the process, and the Rust harness runs a test binary's tests on threads inside that one process,
  so every comparison rewrote `doc.yaml` and `doc.json` in the same place while another was reading
  them. `fs::write` truncates before it writes, and the damage is measured on PR #556's `rust-lint`:
  `jq [".a", "doc.json"] answered differently`, ours `(0, "")` against the reference's
  `(0, "null\n")` - the two sides read the same file at two different moments. Only that job
  installs `jq` and `yq`, so a collision here can surface as a red `rust-lint` and never as a
  skipped test. Each call now gets its own directory (process id plus a counter), and
  `each_comparison_gets_a_fixture_directory_no_other_comparison_reads` refuses a shared path -
  putting the pid-only name back fails it, which is how the control was checked.

- **The `no_std` gate could only disagree with itself** - `tests/test_no_std_gate.py` compared the
  crate lists in `ci.yml` against a set copied into the test, so a crate that gained `#![no_std]`
  with no bare-metal build, or lost the attribute while still being built, kept the job green. The
  set is read from the crate roots now. Two controls hold it: one runs the reader over synthetic
  sources that tell the attribute apart from the sentences quoting it, the other refuses an empty
  foundation, because empty matched against empty would have looked like a pass.

- **An empty authority column was prose, not a measurement** - `QUALITY_MATRIX.md` said the schema
  surface has no outside reader to ask its rules, while `scripts/quality_matrix.py` could emit no
  hole for an empty authority column: `.ci/quality-holes.json` stayed at `holes: []`, which read as
  no short board. The probe emits `authority-tier:schema-validation` now, the registry carries it
  with the corpus that closes it, and the control runs the rule on rows built by hand in both
  directions - empty emits the slug, filled emits nothing.

- **`pyq` read a sequence slice as a key** - `[0:2]` is how both `jq` and mikefarah
  `yq` spell a range, but the bracket parser fell through to its key branch, so
  `.list[0:2]` looked up a key named `0:2`, missed, and the compatibility shell answered
  `null` for it. The grammar now rejects a `:` it cannot address and the shell refuses
  the construct by name; a quoted bracket (`["k:v"]`) is still a key. Under
  `--as jq|yq` a `select` that matches nothing also stopped being an error: both tools
  print nothing and exit 0, and so does the shell now, while native `pyq get` keeps
  reporting both.

- **`pyq get --json` answered one match and many matches differently** — a single
  match went through the indented writer while a multi-match stream wrote each value
  on one line, so `.servers` and `.servers[*]` disagreed about what `--json` means.
  Every match now gets the same rendering, which is also the shape `jq` writes per
  output value. The compatibility shell gained the other half of that agreement: under
  `--as jq|yq` a path the document does not carry prints `null` and exits 0, as both
  real tools do, while native `pyq get` keeps reporting the miss and exiting 1.

- **A CLI-only edit demanded an extension rebuild that could not change it** - the
  freshness gate globbed every directory under `crates/` when deciding which files
  are compiled into the loaded module, so the standalone `pyq` crate counted as
  shipped and every edit confined to it cost a `maturin develop` and a red run. The
  crate set is derived now, from the binding manifest's `path` dependencies taken
  transitively with dev-dependencies excluded; both directions are controls, so a
  shipped source still names itself and a CLI source no longer does.

- **A merge cannot give a mapping a second null key** - `~`, `null`, the empty key and a bare
  `Null` node all resolve to null, and the reader folds two null keys into one entry on purpose
  (that is what lets `: a` + `: b` parse). The merge filter asked a different question: text for
  untagged scalars, whole-node equality for everything else. So a `<<` source contributed its null
  key beside the one the mapping already owned, and two merge sources could both name it - the
  emission wrote `~:` twice at one level, and the re-read dropped the pair with its value and the
  anchor it carried. `same_emitted_key` now asks the reader's own predicate, and the merged set is
  folded by that same identity, so "the first source to supply a key wins" holds for every spelling
  of null rather than only for keys that already compared equal. Found by the property tier at
  20,000 cases (CI seed `c9ef4fd3...`, where `prop_mapping_order_preserved` and
  `prop_tag_preserved` failed on one tree for this one reason), pinned by
  `a_merge_never_repeats_a_null_key_the_mapping_owns` and the seed replayed from
  `crates/pyrs-yaml-core/proptest-regressions/pbt.txt`.

- **The API pages state the parameter list the stub declares** — `docs/{zh,ja,ko}/api/reference.md`
  documented `parse_file`, `parse_all_docs`, `safe_load` and `safe_loads` with one or two parameters
  while the shipped stub declares five and six, and the `%YAML` version-negotiation gate
  `honor_version_directive` appeared in no user-facing page at all; `reparse()` was documented with
  a `schema` default the extension does not use. All four locales now agree with the stub, the
  parameter is documented in the pages that own it, and `tests/test_api_doc_signatures.py` compares
  both sides through `ast` — so a shortened signature is a red run, not a reader's surprise.
  `YamlDocument.flush_source()`, the one public callable named nowhere in the prose pages, is now
  documented in all four locales, and the gate also asserts every public name a stub declares
  appears in each locale's pages.

- **An anchor the duplicate-key fold removes is inlined, not abandoned** — `: abase: &x 1` +
  `use: *x` + a second empty key duplicates the first, so the fold drops the entry whose *value
  subtree* defines `&x`. The repair pass was handed only the dropped entry's own anchor, the name
  lost its definition, and the emission was `use: *x` — text this library's own reader refused
  ("found unknown anchor"), which the engine promises never to produce. The fold now registers
  every definition leaving the tree, so the surviving use is inlined with its anchor and the
  output reads back and settles in one round (libFuzzer `edit_yaml` crash-68617eaf). Pinned by
  `a_definition_folded_away_from_inside_the_dropped_value_is_still_emitted`, the Python-visible
  `test_a_folded_away_anchor_definition_still_reaches_the_output`, the new anchor-graph corpus
  gate and the replayed seed `fuzz/seeds/edit_yaml/former-crash-68617eaf.seed`.

- **A headed item's note goes above the outermost header of the chain** — a note a block container
  cannot spell above its own `&a` / `!tag` line was written above the *nearest* header, and the
  reader hoists a first item's leading note one level per emission, so the note kept climbing: on
  `- &a3\n  - &a0 !fuzz2\n    - {"-(.0bb:": v6}  # fuzz 47`, annotating the innermost item emitted
  it under `&a0` and the second round moved it under `&a3`. The writer now follows the outermost
  header whose gap the value starts and writes the note above that one, so the first emission is
  the fixed point (libFuzzer `edit_yaml` crash-2d978c7b). Pinned by
  `a_headed_items_note_settles_above_the_outer_sequences_header` and the replayed seed
  `fuzz/seeds/edit_yaml/former-crash-2d978c7b.seed`. The header target travels as an argument
  through the item writers rather than as serializer state, because the state version was not
  free: measured on the instruction gate's Rust harness against the committed floors, it cost
  +0.92% on `serialize_small` and +0.75% on `serialize_block`, while the argument version costs
  +0.0006% and +0.19%.

- **An anchored item's note settles above the header it cannot split** — `set_comment_path` on a
  block mapping that is a sequence item and whose block ends in a tag-only line (`- a: !o`) wrote
  the mapping's inline note between its `&a` header and the body. The reader files a note there as
  the mapping's *leading* comment, so the next emission hoisted it above the header: the first
  output was `&a0` / `# note` / `a: !o` and the second `# note` / `&a0` / `a: !o`, so the edit
  needed two rounds to settle. The writer now puts that note above the header line, where the
  reader hands it back from, and the first emission is the fixed point (libFuzzer `edit_yaml`
  crash-b22d0653). Pinned by `an_anchored_items_note_settles_above_its_header` and the replayed
  seed `fuzz/seeds/edit_yaml/former-crash-b22d0653.seed`.

- **A YAML float spelling TOML refuses is written in TOML's grammar** — Core YAML resolves `083e6`
  as a float: its exponent form allows a leading zero in the integer part, while TOML's
  `float-int-part` is a `dec-int` and has no such spelling. The writer's pass-through gate only
  looked for an `e`, so exporting a document whose value is `083e6` wrote `lis6 = 083e6`, and the
  TOML reader refused its own writer's text (libFuzzer `edit_yaml` crash-71feb9d9). The gate now
  validates the whole float grammar before passing a spelling through; a spelling TOML cannot read
  is rendered from the value instead. Pinned by
  `a_yaml_float_spelling_toml_refuses_is_written_in_toml_grammar` and the replayed seed
  `fuzz/seeds/edit_yaml/former-crash-71feb9d9.seed`.

- **A compact item's note settles above its sequence's header** — a compact `- key: value` item
  whose pair line already carries a note of its own writes the item's note one line up, above the
  dash. When the sequence also writes an `&a` / `!tag` header, that line sits between the header
  and the body, where the reader files the note as the sequence's *leading* comment, so the next
  emission hoisted it above the header: round one wrote `&a0` / `# note` / `- k: v  # note` and
  round two `# note` / `&a0` / `- k: v  # note`. The first item now takes the header's line start
  and writes that note above it, which is where the reader reports it from (libFuzzer `edit_yaml`
  crash-2fc59020). Pinned by `a_compact_items_note_settles_above_the_sequences_header` and the
  replayed seed `fuzz/seeds/edit_yaml/former-crash-2fc59020.seed`.

</details>

### [v0.18.1] — 2026-10-17

<details markdown="1">
<summary>free-threaded 3.15t wheels · Linux arm64 · merge-key notes kept</summary>

#### Added

- **Free-threaded wheels for Python 3.15t and Linux arm64** — the release now builds a
  `cp315-abi3.abi3t` wheel for Windows, macOS and glibc Linux (x86_64 and arm64), so 3.15t and later
  install a prebuilt artifact instead of compiling from source; Linux arm64 gains its `cp314t`
  and `abi3t` wheels from a native arm64 runner, where the interpreter exec that qemu could not
  perform is a native call. Each wheel is installed, and the suite run, on the interpreter it
  targets.

#### Fixed

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

### [v0.18.0] — 2026-10-17

<details markdown="1">
<summary>five-format event streams · explicit tags · round-trip fidelity · faster I/O</summary>

#### Added

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

#### Changed

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

#### Fixed

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

#### Performance

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

### [v0.17.0] — 2026-10-01

<details markdown="1">
<summary>pyq parity flags · JSONC/JSON5 input · pyq diff/merge · automated GitHub Release</summary>

#### Added

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

#### Changed

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

#### Fixed

- **`pyrs-json` module documentation** — it still claimed comments were stripped on read and never
  re-emitted; since #122 they ride the AST comment slots and the JSONC/JSON5 writers reproduce them.

</details>

### [v0.16.0] — 2026-10-01

<details markdown="1">
<summary>cross-library parity suites · load_json · TOML 1.1 + toml-test · JSON5/JSONC · pyq ·
perf</summary>

#### Added

- **JSONC block-comment hot-sample bench** — objective §测试覆盖 5 names "block-comment" as a
  required hot sample; previously only inline `//` comments were measured. New fixture drives
  `test_load_jsonc_block_comments` with 50 pairs + header/footer blocks, catching scanner
  regressions on the `/* ... */` path via CodSpeed.
- **PyYAML + ruamel.yaml cross-library parity for YAML** — objective §测试覆盖 3 names both as
  oracles; previously only benchmarks and a printout. `tests/test_yaml_crosslib.py` closes the
  correctness gap: 20 canonical documents × 5 parity surfaces + 2 documented divergences (dup-key
  strictness, YAML 1.1 legacy booleans schema-scope) = 122 tests. Optional deps skipif clean.
- **tomlkit cross-library parity for `load_toml`** — objective §测试覆盖 3 names tomlkit as an
  oracle; previously tomlkit only appeared in benchmarks. `tests/test_toml_crosslib.py` adds 24
  tests covering 11 canonical constructs with three-way agreement (pyrs/tomlkit/ tomllib), pins the
  `>i64` spec-strict divergence, and asserts the `-2^63` boundary. Optional dep, skipif clean.
- **orjson as strict-JSON oracle for `load_json`** — objective §测试覆盖 3 required correctness
  parity against orjson; previously orjson only appeared in benchmarks. 16 canonical documents
  assert byte-for-byte agreement, and 12 non-strict forms (comments, trailing commas, single quotes,
  `NaN`/`Infinity`/`-Infinity`, hex, leading zero, `+.5`, `5.`) are rejected by both orjson and
  `load_json`. orjson rejects the bare literals that stdlib `json.loads` accepts under
  `allow_nan=True`, so it is the stronger RFC 8259 oracle. Optional dep, `skipif` clean.
- **CLI ↔ Binding parity gate (`tests/test_cli_binding_parity.py`)** — the Pillar 1 invariant is now
  an executable contract: CLI's registered commands are checked against a fixed 18-name inventory,
  every `to-X` / `from-X` verb requires its matching `YamlDocument.to_X` / `from_X` / `load_X`
  sibling, the `load_*` family symmetry (json/jsonc/json5/toml) is asserted, and
  editing/validate/compliance verbs map to live Python API. A drift on either surface now fails CI.
- **`load_json` property tests + CodSpeed benches** — Hypothesis
  (`test_load_json_matches_stdlib_json` + `test_load_json_matches_load_jsonc_on_strict_domain`) pins
  the strict loader against `json.loads` for every generated canonical document and asserts both
  loaders agree on the strict domain; a widening of the fast path or a fallback drift now surfaces
  as a property failure. Three CodSpeed benches (`test_load_json_large` / `_floats` / `_escapes`)
  mirror the load_jsonc samples so the strict binding layer is tracked.
- **`load_json` (strict) — completes the `load_*` family parity** — the binding exposed `load_jsonc`
  / `load_json5` / `load_toml` but the strict RFC 8259 counterpart was missing.
  `pyrs_yaml.load_json(s)` now matches `json.loads` on canonical documents and rejects the
  JSONC/JSON5 extensions (`//`, `/* */`, trailing commas, single quotes, bare `Infinity`/`NaN`,
  `0x…`) with a typed `YamlParseError`. The fast path shares `json_fast::try_load` with `load_jsonc`
  (which bails on every non-canonical byte — zero widening risk); declined constructs route through
  the STRICT `from_json` AST parser. Closes the last gap under the CLI ↔ Binding parity entry: every
  CLI format now has its `load_*` sibling, Pillar 1 complete. Re-exported and in `__all__`; `.pyi`
  regenerated via `maturin generate-stubs`.
- **Dialect writer fixed-point properties** — `fmt_pbt.rs` promised a writer fixed point in its
  header but never implemented one. Four proptests now hold it for JSON/JSONC/JSON5/TOML (the sole
  input filter drops hand-built ASTs whose distinct keys spell the same JSON name — outside RFC
  8259's object domain). The gate immediately surfaced three real comment-fidelity defects (Fixed).
- **Hot-spot benchmark corpus** — seven CodSpeed wall-time benches target the historically fragile
  serialization paths: YAML block-scalar documents (all six header spellings `|`, `|-`, `|+`, `>`,
  `>-`, `>+`) and comment-dense documents, TOML multi-line strings / radix integers / underscore
  separators / exponents / datetimes, and JSON5 exotic number forms (hex, `+.1`, `5.`, `Infinity`,
  `NaN`, single quotes, trailing commas). Fixtures live in `tests/data/yaml_samples.py`, benches in
  `tests/test_benchmark_api.py`. Building this corpus is what surfaced the nested block-scalar
  indentation bug fixed below.
- **Text-level re-parse gate (`prop_output_always_parses`)** — the Rust proptest suite now asserts
  that every generated AST serializes to text the parser accepts again. The AST-vs-AST round-trip
  property silently skipped shapes whose serialized text could not re-parse (`try_roundtrip` returns
  `None`), leaving an entire defect class invisible; the new gate caught six real serializer bugs on
  its first runs (see the Fixed entries below), each now additionally pinned by targeted Rust unit
  tests and a Python regression class (`TestNestedBlockScalarIndent` in
  `tests/test_roundtrip_bugs.py`).
- **TOML datetime offset range** — a numeric UTC offset is now range-checked (hours 00..=23, minutes
  00..=59); `+12:60` / `+24:00` are rejected instead of accepted. Shape was checked but values never
  were (toml-test `invalid/datetime/offset-overflow-minute`; illegal in both 1.0 and 1.1).
- **TOML table-redefinition strictness** — a table created implicitly by a dotted key is now closed:
  a later `[header]` may not re-open it (`[fruit]` + `apple.color` then `[fruit.apple]`; `[t1]` +
  `t2.t3.v` then `[t1.t2]`), and a table cannot be redefined as an array (`[tbl]` then `[[tbl]]`).
  Invalid under BOTH 1.0 and 1.1 (toml-test `invalid/table` duplicate-key/redefine); legit
  super-tables and sibling dotted keys still parse (mis-acceptance 30 -> 21, valid decode-match
  unchanged).
- **toml-test conformance harness** — `tests/test_toml_test_suite.py` runs the official
  [toml-test](https://github.com/toml-lang/toml-test) corpus the same way `test_yaml_suite.py` runs
  the YAML suite: untracked local artifact, `skipif` on absence, measured-floor gates, and a
  type-tag adapter for decode comparison.
- **TOML temporal types decode correctly** — date-only and time-only values now carry distinct
  `!date`/`!time` tags (date-times keep `!timestamp`) so they use `date`/`time.fromisoformat`
  instead of crashing. toml-test surfaced a bare time (`07:32:00`), a seconds-less time (`13:37`),
  and a lowercase-delimiter datetime (`1987-07-05t17:45:00z`) each raising `ValueError` on valid
  TOML; `!time` now pads omitted seconds, `!timestamp` canonicalizes lowercase `t`/`z`, and both
  normalize a fractional-second field of any precision (TOML `.6`; pre-3.11 `fromisoformat` wants 3
  or 6 digits) to microseconds.
- **TOML control-character strictness** — raw C0 codes (NUL, FF, DLE, US, ...) and DEL (U+007F) are
  now rejected inside basic, literal and multi-line strings (only tab, plus newlines in multi-line
  forms, stay legal). toml-test's `invalid/control` corpus surfaced 13 such mis-accepted documents;
  comment bodies now reject the same control codes (5 more `comment-*` documents) and a bare
  carriage return (0x0D outside a CRLF pair) is rejected by a central entry scan (4 more `*-cr`
  documents). The conformance harness now reads corpus bytes verbatim — its earlier `read_text()`
  applied universal-newline translation, rewriting bare CR to LF and masking the whole class.
- **TOML number-literal strictness** — leading-zero decimals (`01`, `-01`), a sign on radix-prefixed
  integers (`+0x1F`, `-0b101` — `signed-int` is decimal-only), and trailing/double underscores
  (`1_`, `1__0`) are now rejected. toml-test `invalid/integer` + `invalid/float` surfaced 23
  mis-acceptances (total 71 -> 48); the prior "radix integers can be signed" behavior was a spec
  violation.
- **TOML inline-table key-collision strictness** — an inline table rejects a dotted key that equals,
  extends, or is shadowed by a defined path (`{ a = 1, a.b = 2 }`, `{ a.b = 1, a.b.c = 2 }`);
  sibling paths (`{ a.b = 1, a.c = 2 }`) stay legal. toml-test `invalid/inline-table`
  duplicate-key/overwrite surfaced these (total 48 -> 39).
- **TOML non-ASCII string crash fixed** — the basic and multi-line basic string parsers advanced
  byte-wise and could slice mid-character on multi-byte input (U+00A0, …), panicking; both now
  consume whole characters. Found via the toml-test corpus; completes #153 for the single-line/JSON
  paths.
- **Format fuzz + robustness fixes** — new `proptest` property tests fuzz the TOML/JSON/JSONC/JSON5
  parsers and writers (no-panic + re-parseability). They found and fix a mid-character slice panic
  in the TOML and JSON string parsers and a JSONC/JSON5 inline-`//`-comment bug that swallowed the
  following `,`/`}`.
- **YAML merge/alias property fuzz** — the `proptest` suite now generates well-formed
  anchor/alias/merge-key docs (single/sequence aliases, inline-map merges, and the scalar/null
  sources the #166 fix rejects), including a self-referential anchor and repeated alias refs.
  `arb_custom_node()` emits `meta.anchor` but never an `Alias`, so the alias/merge paths
  (the #163/#166 structure class) were previously unfuzzed in-process.
  `prop_merge_alias_never_panics` asserts parse + merge resolution never panic or overflow and that
  any parsed tree re-serializes and re-parses cleanly.
- **CLI format parity** — the CLI gains `to-toml`/`from-toml`, `to-jsonc`/`from-jsonc` and
  `to-json5`/`from-json5`, mirroring `to-json`/`from-json`, so every binding format is reachable
  from the command line.
- **JSON string-escape fast path (perf)** — `load_jsonc` decodes the eight simple two-byte escapes
  inline instead of bailing the whole doc to the AST path; escape JSON stays on the fast path (~15x
  faster than the AST route). Matches `json.loads`; `\u`/invalid escapes still route through the AST
  path.
- **JSON float fast path (perf)** — `load_jsonc` parses canonical floats (decimals/exponents)
  directly into Python objects instead of bailing the whole doc to the AST path; values match
  `json.loads` exactly (correctly-rounded parse). A new benchmark exercises the branch for
  regression tracking.
- **Faster JSON string serialization (perf)** — strings with no escapable byte are bulk-copied in
  one `push_str` instead of per-char UTF-8 re-encoding; string-heavy `to_json` ~35% faster (41 -> 27
  ns/item), byte-identical output.
- **`YamlDocument.to_toml()`** — documents emit TOML straight from the AST, mirroring
  `to_json`/`to_jsonc`/`to_json5`, instead of the `to_toml(doc.to_yaml())` round-trip.
  Byte-identical output; the writer is ~4.3x faster than `tomli_w`.
- **`to_json` native serializer (perf)** — `YamlDocument.to_json` now uses the native engine instead
  of `to_dict()` + `json.dumps`. Byte-identical for ASCII, ~10x faster (1200-item ~1450µs → ~120µs,
  beating `json.dumps`). Non-ASCII now raw UTF-8 (like `to_jsonc`/`to_json5`) instead of `\uXXXX`;
  still valid JSON.
- **JSON object keys written directly (perf)** — the writer emits mapping keys straight into the
  buffer (no per-key `String` alloc); compact `to_json` ~2x faster (~120µs → ~60µs), byte-identical,
  now #2 in the field.
- **JSON load fast path** — `load_jsonc` parses canonical strict JSON straight into Python objects,
  skipping the `CustomNode` AST (~5-6x faster, now ahead of stdlib `json.loads`). Non-canonical
  input (floats, escapes, comments, huge ints, trailing commas) falls back to the general path;
  values unchanged.
- **TOML multi-line string fidelity** — a TOML multi-line string is projected as a
  `ScalarStyle::Literal` YAML block (surviving the text hub) and re-emitted by `to_toml` as a `"""`
  block instead of an escaped single line. Values round-trip byte-for-byte and the output is
  idempotent; a single-line string stays single-line. No AST struct change (reuses `Literal`).
- **TOML document-level comment fidelity** — `to_toml` now emits the root mapping's leading comment,
  so a document-opening standalone `# note` survives a TOML → hub → TOML round trip instead of being
  dropped (the TOML counterpart of the JSON writer's `emit_root_leading`). Native TOML parses and
  comment-free documents are unaffected.
- **JSON5 Unicode identifier keys** — unquoted object keys accept the full Unicode `ID_Start` /
  `ID_Continue` set (not ASCII only), so `from_json5` / `load_json5` parse
  `{ é: 1, 名: 2, हिन्दी: 3 }`. Built on the `unicode-ident` tables (rustc’s own lexer crate) for
  exact per-script conformance including combining marks. Gated on JSON5, so strict `from_json` /
  `from_jsonc` still require quoting. Lone UTF-16 surrogates stay rejected. Adds a dependency
  (`unicode-ident`).
- **JSON5 Unicode structural whitespace** — `from_json5` / `load_json5` accept the whitespace JSON5
  adds to RFC 8259's four: VT, FF, NBSP, every Unicode `Zs` separator, the LS/PS line terminators
  and ZWNBSP (U+FEFF). Built on `std` `char::is_whitespace` (minus NEL U+0085) plus U+FEFF — no new
  dependency. Strict `from_json` / `from_jsonc` still reject all of them.
- **JSON5 line continuation and `\'` escape** — double-quoted JSON5 strings accept a
  backslash-newline line continuation (removing both) and an escaped single quote (`\'`); strict
  JSON / JSONC still reject both. Mirrors the single-quoted handling from #125, completing JSON5
  string fidelity.
- **JSON5 string escapes `\v` and `\0`** — `from_json5` accepts vertical tab (`\v`) and NUL (`\0`)
  in double- and single-quoted strings; strict JSON / JSONC still reject them. Completes JSON5
  grammar with #120 (numbers) and #124 (value semantics).
- **JSON5 value semantics on load** — `load_json5` resolves the JSON5-only number forms (`0x1F`→31,
  `+7`→7, `5.`→5.0, `Infinity`/`NaN`) to real numbers via a new `Schema::Json5`, leaving strict
  JSON/JSONC loaders unchanged and `to_json5_text` still emitting the source spelling.
- **JSON5/JSONC reachable through the public API** — `pyrs_yaml.from_json5` / `load_json5`, and
  `YamlDocument.to_jsonc()` / `to_json5()` (native-engine emit, so comments and JSON5 styles
  survive). Also fixes a reachability gap: `from_jsonc` / `load_jsonc` were never re-exported from
  the `pyrs_yaml` package, so `pyrs_yaml.from_jsonc(...)` raised `AttributeError`; they now appear
  in `__all__`. `to_jsonc`/`to_json5` gained `emit_root_leading` so a document-level standalone
  comment is preserved. New JSON-family benchmark coverage added to `test_benchmark_api.py`.
- **JSON5 writer (`to_json5_text` / `to_json5_text_pretty`)** — contract-B step 2. Serializes the
  AST back to JSON5, restoring single-quoted strings and the `0x…`/`.5`/`+7`/`Infinity`/`NaN`
  numeric forms the parser preserves, plus `//` comments. Keys are always quoted (lossless). Shares
  an internal `Mode` (Json/Jsonc/Json5); strict and JSONC output unchanged.
- **JSON5 numeric forms on the parser** — `from_json5` (new `allow_json5_numbers` axis) accepts hex
  (`0xDECAF`), leading/trailing dot (`.5`, `5.`), leading `+` (`+7`), leading zero (`07`), and bare
  `Infinity` / `NaN` / `-Infinity`, each keeping its exact source text for a future JSON5 writer.
  STRICT / JSONC stay gated off and reject them as before. `from_jsonc`'s stale "comments are
  stripped" doc was corrected.
- **TOML inline-table interior comment fidelity** — PR #119 captures interior `# ...` comments in
  inline tables (own-line above a member → leading, same-line after its value → trailing) and
  threads them through the IR so they round-trip instead of vanishing. Undecorated inline tables
  keep the compact one-line form; a decorated one nested in an array goes multi-line. Also repaired
  a latent #114 bug where `skip_all_blank` mistook a comment's own terminator newline for a blank
  line.
- **YAML receiver writes standalone comments into `decor.leading_comment`** — PR #117b moves the
  last engine (the granit-parser receiver) onto the slot convention PR #114 / #115 established.
  Standalone notes on scalars / mappings / sequences land in `NodeMeta::decor.leading_comment`
  rather than the older `comment(standalone = true)`. Hand-built fixtures stay equal thanks
  to #117's normalisation, and `CustomNode::remove_comment` now clears **both** slots so Python
  `Node.remove_comment()` still behaves correctly on YAML-parsed docs.
- **Cross-slot standalone normalisation + Python `Node.leading_comment`** — `NodeMeta::eq` / `Hash`
  treat the standalone note as one concept across the newer `leading_comment` slot (TOML / JSON) and
  the older `comment(standalone = true)` slot (YAML receiver + hand-built fixtures). Setters /
  removers act atomically on both, and the YAML serializer reads the normalised view so
  `to_yaml(toml_ast)` keeps its leading notes. Python `Node.leading_comment` getter / setter /
  remover mirror `Node.comment` and now surface TOML / JSONC standalone notes.
- **TOML 1.1.0 grammar** — `from_toml` parses to the v1.1.0 spec (2025-12-18). Four additions:
  **(A1)** multi-line inline tables + trailing commas; **(A2)** `\xHH` basic-string byte escape;
  **(A3)** `\e` = U+001B; **(A4)** optional seconds in time and date-time. `TomlDialect::V1_0` /
  `from_toml_v1_0` remain as strict 1.0.0 escape hatch. A parser-side off-by-one in the
  space-separated date-time detection was fixed alongside.
- **JSON dual-slot comment fidelity** — the JSONC parser writes standalone (`// ...` on its own
  line) notes onto the new `leading_comment` slot from #114, while inline (`// trailing`) stays on
  `comment`. Object members and array elements can now carry both notes on the same node —
  impossible under #112's single-slot model. The writer prefers `leading_comment` and falls back to
  `comment` with `standalone = true` for hand-built fixtures.
- **TOML blank-line + dual-slot comment fidelity** — `NodeMeta` grows
  `leading_comment: Option<Comment>` and `blank_before: bool` (both excluded from structural `Hash`
  / `PartialEq`), letting a section header or AOT element carry both the standalone note above and
  the inline trailing note after `]` without either displacing the other. `to_toml` reproduces
  blank-line groupings; the very first pair never emits a leading blank line. Hand-built /
  YAML-origin nodes still rendering via the writer's fallback read.
- **JSON5 dialect** — `pyrs_yaml_core::json::from_json5(text)` and
  `from_json_with_options(text, JsonParseOptions)` accept the full JSON5 axis set: trailing commas,
  single-quoted strings, unquoted identifier keys, and line/block comments. Each axis is togglable
  via `JsonParseOptions`; `STRICT`, `JSONC`, and `JSON5` constants ship as presets.
- **JSONC/JSON5 bindings and CLI** — `pyrs_yaml.from_jsonc(str)` returns YAML text;
  `pyrs_yaml.load_jsonc(str)` returns a Python dict / list. `pyq from-json` grows `--jsonc` and
  `--json5` flags so `tsconfig.json` / `settings.json` files feed straight into the verb pipeline.
- **JSONC comment preservation** — `from_jsonc` now attaches captured `//` line and `/* */` block
  comments onto the AST's `NodeMeta::comment` (standalone on the key node, inline on the value
  node), mirroring the TOML model from PR #109. The matching pair `to_jsonc_text(node)` /
  `to_jsonc_text_pretty(node, indent)` emits them back at their original positions; block comments
  collapse to `//` on output. Strict writers `to_json_text` / `to_json_text_pretty` remain
  byte-identical, so consumers opt into preservation selectively.
- **pyq multi-document edits** — `-A/--all-docs` now covers every edit command
  (set/delete/rename/move/append/insert/sort-keys) plus `to-json -A` (JSON array, Python parity).
  Each document splices against its own segment of the stream (`MultiDocEditor` +
  `DirtyUnit::shifted`): untouched documents and all `---` separators keep their exact bytes, a plan
  miss skips its document (Python try/ skip semantics; all-miss still exits 1), and one layout-dirty
  document falls back alone instead of de-pinning its neighbours.
- **JSONC parsing** — `pyrs_yaml_core::json::from_jsonc(text)` and
  `from_json_with_options(text, JsonParseOptions)` accept `// line` and `/* block */` comments at
  any whitespace position (the dialect used by TypeScript's `tsconfig.json` and VS Code's
  `settings.json`). Comments are stripped, not preserved. Trailing commas and other JSON5-only forms
  remain rejected so the accepted language stays a strict superset of RFC 8259. `from_json` is
  unchanged (strict mode by default).
- **TOML comment fidelity** — the parser now captures both standalone (`# ...` on its own line above
  a pair or section header) and inline (`key = value # ...` / `[name] # ...`) comments and attaches
  them onto the shared AST through `NodeMeta::comment`. `to_toml(from_toml(src))` re-emits those
  comments at their original positions so `pyq edit` and `YamlDocument.set()` no longer strip
  annotation notes from a TOML round trip. Whitespace fidelity (blank-line separators between pairs)
  stays on the writer's default layout per the design doc.
- **TOML numeric source fidelity** — `to_toml(from_toml(src))` preserves hex (`0xDEADBEEF`) and
  octal (`0o755`) integer spellings and exponent form floats (`1e10`, `-3.14e-2`) verbatim.
  Underscore separators, explicit `+` signs, negative radix (`-0x1F`) and binary (`0b101`)
  canonicalise to decimal because YAML Core schema cannot re-read them, keeping the shared AST
  interoperable with the YAML pipeline. Comment fidelity and JSONC arrive in follow-up PRs per the
  design doc.
- **pyq feature completion** — the CLI reaches parity with the Python CLI's surface:
  `rename`/`move`/`append`/`insert` splice edits, `validate` (parse check, or against
  schema-language rules via `--schema rules.yaml`), `frontmatter` (`--body-out` splits the body),
  and `-A/--all-docs` on `get`/`fmt`/`to-json` for multi-document streams. Fixing the wiring
  surfaced a core engine bug: `move_path` returned only the destination INSERT unit, so the spliced
  text kept a duplicate of the moved subtree (invisible whenever documents fell back to
  re-serialization); it now returns both units and the bindings apply them through their batch
  splice path.
- **`pyq` filter verbs** — structured jq-style post-processing on the match stream:
  `--select 'PATH OP LITERAL'`, `--sort-by PATH` / `--desc`, `--unique`, `--first` / `--last`,
  `--skip N` / `--take N`, and `--join SEP`, applied in the fixed pipeline order
  `select -> sort -> unique -> slice` then `join` on `get` and the `from-*` converters. Deliberately
  flags, not an expression language: the predicate is one micro-grammar parse (micro ~40 lines),
  mixed comparison kinds are `false` (documented divergence from jq's total order), and startup
  stays instant.
- **`pyq completion`** — print a shell completion script for bash, zsh, fish or PowerShell
  (`pyq completion bash > ...`), powered by `clap_complete` (approved addition to the CLI crate's
  dependencies; it stays inside the `pyrs-yaml-cli` binary and does not touch the Python
  distribution).
- **`pyq sort-keys`** — sort the keys of the mapping at any path (`$` for the root), in place or to
  stdout, through the same core plan/splice engine as `set`/`delete` - closing the parity gap with
  the Python CLI's `sort-keys` command.
- **Command-line interface** — a new `pyrs-yaml` command (opt-in via `pip install "pyrs-yaml[cli]"`,
  requires Python 3.10+) exposing the library from the terminal: `fmt` (round-trip reformat
  preserving comments/anchors/ order), `get` (JSONPath queries with `--format yaml|json|text`),
  `set` / `delete` / `rename` (path-based edits with `--inplace`, `--string`, `--create-missing`),
  `validate` (CI-friendly exit codes), and `to-json` / `from-json` conversions. All commands read
  stdin via `-` and default to stdout output. Implemented in pure Python (`python/pyrs_yaml/cli/`)
  on top of [Cyclopts](https://github.com/BrianPugh/cyclopts) as an optional extra, so the base
  install keeps zero extra dependencies and Python 3.8 support.
- **CLI expansion** — `sort-keys` (sort mapping keys at a path), `move` (relocate a subtree to an
  existing destination), `frontmatter` (extract Markdown front matter as YAML, optional body split),
  and `compliance` (YAML Test Suite report with `--json`) commands; `-A/--all-docs` multi-document
  mode on `fmt`/`get`/`set`/`delete`/`rename`/`sort-keys`/ `validate`/`to-json`; and mutually
  exclusive `validate --schema <name>` vs `--schema-file <path>`. The undocumented
  `python -m pyrs_yaml.compliance` entry point was removed in favor of the subcommand.
- **`YamlStream` is now importable** — `from pyrs_yaml import YamlStream` works as documented in the
  API reference and type stubs; the class was previously returned by `YAML().load_stream*()` but
  never exported from the native module.
- **CLI `move --all-docs`** — `move` now accepts `-A/--all-docs`, applying the subtree move to every
  document where both paths resolve (same semantics as `set`/`delete`/`rename`), so the
  multi-document flag covers all edit commands.
- **Docs ↔ API consistency guard** — `tests/test_docs_api.py` scans every locale doc page for
  `pyrs_yaml.…` chains, `import pyrs_yaml…`, and `from pyrs_yaml … import …` claims, and fails if
  any referenced symbol does not exist at runtime (~965 claims checked).
- **Optional third-party type plugins** — `!duration` (`pendulum.Duration`), `!arrow`
  (`arrow.Arrow`), and `!ulid` (`ulid.ULID`) auto-register when the corresponding library is
  installed (`_register_third_party` in `python/pyrs_yaml/plugins/_builtin.py`). Each uses a
  distinct tag so existing `!timestamp` / `!date` / `!uuid` handlers are unaffected; a plain stdlib
  `timedelta` is never matched by `!duration`.
- **pydantic-settings YAML source** — `PyrsYamlConfigSettingsSource`
  (`python/pyrs_yaml/settings.py`) is a drop-in replacement for
  `pydantic_settings.YamlConfigSettingsSource` that parses with pyrs-yaml (YAML 1.2 core schema)
  instead of PyYAML. It is exported lazily so `import pyrs_yaml` never requires pydantic-settings;
  install with `pip install "pyrs-yaml[settings]"` (Python 3.10+). `dump_pydantic` and `parse_as`
  now use the same lazy module-level `__getattr__` export pattern.
- **`pyq` — native Rust CLI crate** — `crates/pyrs-yaml-cli` (workspace member, clap-based) puts
  `pyrs-yaml-core` behind a jq/yq-style command line with no Python at runtime: `fmt`
  (comment-preserving round-trip), `get <path>` (JSONPath-lite with `--json`/`--raw`),
  `set <path> <value>` and `delete <path>` (yq-style edits with `--create-missing` and
  `-i/--inplace`, round-trip-preserving output), `to-json` (order preserved), `to-toml`, `from-json`
  / `from-toml` / `from-ini`; input format by extension with `--input` override, stdin via `-`,
  non-zero exit with the core's stable error text.
- **TOML and INI exchange formats** — hub-and-spoke multi-format support with YAML as the single
  editable representation: `from_toml`/`to_toml` convert TOML text ⇄ YAML text (Rust `toml_edit`),
  `load_toml` reads TOML directly into Python values (datetimes via the built-in `!timestamp`
  plugin; TOML strings never re-resolve), and `load_ini` reads INI via stdlib configparser (strict,
  read-only). TOML output rejects inexpressible shapes with stable errors; round-trip editing stays
  YAML-only by design.

#### Changed

- **granit-parser 1.1 → 1.3** — bumped the YAML event parser from 1.1.0 to 1.3.0. The upgrade is a
  semver-compatible minor bump inside the 1.x line: 1.2.0 added optional `Options` fields for
  exotic-document limits, 1.2.1 tightened a few parse results to match the YAML spec, and 1.3.0
  added two defaulted `Input` methods (`fetch_block_scalar_line` and
  `take_quoted_scalar_ascii_chunk`) that let the scanner step over block and quoted scalar bytes
  faster. The project consumes the parser through `Parser::new_from_str` and implements only
  `EventReceiver` / `SpannedEventReceiver`, never `Input`, so no source changes were required — the
  new trait methods resolve to their default implementations. Full suite green:
  `cargo nextest run --all` (359), `pytest` (1436 + 43 numpy), pure-Rust `--no-default-features`
  build, and the YAML test-suite compliance gates unchanged.
- **Native JSON & TOML cores** — the `serde_json` and `toml_edit` dependencies are gone.
  `pyrs-yaml-core` ships an RFC 8259 JSON engine (byte-level scanner, verbatim number spelling so
  `from_json → to_json` is byte-stable and precision-preserving, typed line/column errors, strict
  rejections for trailing commas, leading zeros, lone surrogates, unescaped control characters, and
  multi-root documents) and a TOML 1.0 engine covering the full grammar — bare/quoted/dotted keys,
  basic/literal/ multiline strings, decimal / hexadecimal / octal / binary integers with underscore
  separators, floats including `inf`/`nan`/exponent forms, and offset/local date, time, and
  date-time — every rejection surfacing as `ParseError::Syntax` with 0-indexed `line`/`col` in the
  granit-parser house style. Public surface unchanged; round-trip tests and `tests/test_toml.py`
  green with the native path.
- **Internal duplication cleanup** — benchmark fixtures composed from shared blocks, PyO3 path-edit
  methods delegate to the existing `apply_metadata_edit` helper, and repeated
  file-read/error-mapping and line-offset boilerplate collapsed into shared functions. No public
  behavior change; the duplicate-code rate measured by jscpd drops from 5.25% to 3.45%.
- **`YamlDocument.validate()` caches compiled validators** — the first successful validation against
  a schema (JSON text *or* dict) caches a compiled `jsonschema` validator; later calls skip schema
  parsing, meta-schema checking, and validator construction. Dict schemas are keyed by object
  identity with a deep-copy snapshot guard: in-place mutation is detected by `==` and transparently
  recompiles. The cached path raises `exceptions.best_match(validator.iter_errors(instance))`,
  identical to `jsonschema.validate()` semantics. WSL wall-time: `document_validate` −98%.
- **Structural dedupe of parser/serializer kernels** — mapping and sequence rendering share one
  `write_container_node` skeleton (output byte-identical, `serialize_*` medians −5~11%); single- and
  multi-document parse entry points share one `load_ast` error contract; the schema resolution
  chains share `bool_word`/`numeric_tail` and YAML 1.1 no longer re-checks the core's null/bool
  words per scalar; anchor registration (`register_anchor`) and the standalone-vs-inline comment
  taxonomy (`is_standalone_placement`) are single-sourced across the AST and stream receivers. Repo
  duplicate rate 3.38% → 2.60%.

#### Fixed

- **`\u` / `\x` escapes followed by a multibyte char panicked the parser** — fixed-width escape
  readers sliced `&self.text[pos..pos+width]` by byte offset; a multibyte char after a JSON `\u` or
  TOML `\xHH`/`\uXXXX`/`\UXXXX` landed the slice mid-character and aborted (the #153 slice-crash
  sibling). They now byte-slice + UTF-8-validate, rejecting cleanly. Found by the dialect fuzz;
  pinned by deterministic Rust regression tests.
- **A literal `<<` key with a non-merge value was silently dropped** —
  `load(safe_dump({"<<": None}))` returned `{}`, losing the key. The merge resolver consumed any
  `<<` as a merge even for a Null/Scalar source. A `<<` is only a merge when its value is an
  alias-to-mapping, inline mapping, or sequence of those; Null/plain-scalar `<<`, and an alias-free
  `<<` that yields nothing (`<<: []`, `<<: [1, 2]`), now stays an ordinary key and round-trips.
  Alias / alias-bearing paths are always consumed (#166 guard holds; a `<<: [*a, *a]` cannot
  re-expand); yaml-test-suite holds 405/406. Surfaced by the round-trip property fuzz, pinned by
  deterministic Rust regression tests for null, scalar, and sequence forms.
- **TOML deep nesting overflowed the native stack and aborted the process** — the TOML parser had no
  nesting budget (JSON has `DEFAULT_MAX_DEPTH`, YAML has `parse` `max_depth`), so `parse_value` →
  `parse_array` / `parse_inline_table` recursed without limit. A deeply nested array/inline table
  crashed the interpreter outright (verified: exit `0xC00000FD` STACK_OVERFLOW) — the TOML analogue
  of the #166 YAML merge overflow. The parser now tracks `depth` and returns
  `ParseError::MaxDepthExceeded` past 1000, mirroring JSON. Covered by an in-process Python boundary
  test, a subprocess crash canary, and a big-stack Rust unit test.
- **Dialect writers/parsers lost or misplaced document-level comments** — a file-leading `// note`
  was misclassified as inline and stolen by the first JSONC/JSON5 object member (vanishing on empty
  containers) instead of landing on the root the writer annotates; the JSON-family **and TOML**
  writers now trim comment bodies at emit to match the parser's trimmed storage (an untrimmed
  comment used to oscillate whitespace across passes); and a comment-only TOML document keeps its
  leftover note on the empty root instead of serializing to `""` (a hand-built non-empty root
  carrying both a root and a first-element note is outside TOML's representable domain and filtered
  from the property). Leading comments across all five formats now reach a byte-stable fixed point
  (three pinned Rust tests).
- **Nested block scalar bodies kept their parent's indentation** — a literal or folded scalar under
  a nested key emitted its body lines one fixed indent step from column zero instead of one step
  below the `b: |` header line, so every nested block-scalar shape re-parsed as an error or a wrong
  value. The writer now threads a `block_base` (the parent line's column) through every emission
  site; round-trip is text-exact for all seven nesting shapes. Found by the TOML hot-spot bench.
- **The serializer emits only re-parseable YAML** — five spelling defects caught by the new
  text-level gate: double anchor/tag headers when a mapping value moved to its own line pre-emitted
  a header the child also wrote; block scalars inside flow collections (or key position) now demote
  to double-quoted; flow containers starting their own line and the complex-key value marker `:`
  lost their parent indentation; complex keys with a standalone comment/tag serialized ambiguous
  text (the note now moves above `?` and the body gets its own deeper lines); and plain scalars with
  edge whitespace or embedded flow indicators (`,[]`) inside flow collections are now quoted instead
  of corrupting the token. Tagged empty containers fold their header onto the `{}`/`[]` line;
  compact dash items refuse to inline commented values. Pinned by nine Rust tests and Python
  regressions.
- **TOML rejected the legal minimum i64 integer** — `from_toml`/`load_toml` failed on
  `-9223372036854775808` (`i64::MIN`): the signed path parsed the unsigned magnitude first and
  overflowed before negation ran. The sign now parses with the digits (`i64::from_str` accumulates
  negatively), signed floats keep their exponent spelling, and the old negate pass is gone. Found
  via the new Python-side Hypothesis dialect fuzz (`tests/test_property_dialects.py`, with stdlib
  `json`/`tomllib`/`pyjson5` oracles and type-strict equality; it also pins the two known
  AST-ambiguous spellings — bare JSON5 `Infinity`/`NaN` literals and >i64 digit strings). Rust
  regression: `toml::parser::tests::i64_lower_bound_negative_integer_is_accepted`.
- **Wrongly-indented flow sequence continuation is rejected again** — upgrading the YAML parser to
  granit-parser 1.3 (see *Changed*) silently began *accepting* a multi-line flow collection whose
  continuation line is indented no further than its enclosing block key (yaml-test-suite `9C9N`:
  `flow: [a,` then `b,` at column 0), regressing strictness `405/406 → 404/406` — invisible to the
  suite's ≥95% threshold gate, so it passed green CI. An in-tree post-parse guard in the AST
  receiver now tracks the enclosing block indentation and rejects an under-indented flow
  continuation, restoring `405/406`. The guard uses only spans the parser already computes, so
  correctly indented multi-line flows are untouched. `9C9N` is now pinned as a per-case hard gate
  (literal input, no `skipif`) in `tests/test_yaml_suite.py`, plus a Rust unit test
  (`parser::tests::flow_continuation_under_indented_is_rejected`).
- **Self-referential merge keys no longer overflow the native stack** — a `<<` whose expansion
  points back at its own anchor (`a: &a` containing `b: {<<: *a}`) expanded forever inside
  `resolve_merge_keys`, exhausting the native stack and killing the whole interpreter process
  (Windows exit `0xC00000FD`, a segmentation fault). The cycle guard was applied only while
  *collecting* merged pairs, never while *walking* the expansion, so recursive re-entry was never
  caught. The anchor guard is now path-scoped: an anchor's name stays on the recursion path while
  its expansion is walked, and a merge that resolves back to an ancestor already on that path
  terminates as an empty expansion instead of recursing. An acyclic AST cannot carry PyYAML's cyclic
  dict, so a self-merge now bottoms out at `{}` rather than crashing. Four related merge-semantics
  defects were fixed in the same pass: a null / scalar / sequence merge source no longer survives as
  a literal `<<` key, an inline mapping used directly as a merge value (`<<: {x: 1}`) now merges,
  and a non-alias element in a merge sequence (`<<: [*a, {y: 2}]`) keeps its inline map. Covered by
  six Rust and nine Python regression tests (`merge::tests`,
  `tests/test_gaps.py::TestSelfReferentialMerge166`). Reported by
  [@bourumir-wyngs](https://github.com/bourumir-wyngs) in #166.
- **NumPy serialization no longer reads Python memory without the GIL** — the ndarray writer
  borrowed the array's data buffer via `unsafe { as_slice() }` and then iterated that borrowed slice
  *inside* `py.detach`, i.e. after releasing the GIL. `&[T]` is `Send` no matter its provenance, so
  the borrow checker could not catch it, but the memory is owned by Python and another thread could
  resize or write the array concurrently — an unsound data race / UB that only surfaces under
  concurrency. The buffer is now snapshotted into Rust-owned memory while the GIL is still held
  (`slice.to_vec()`), and only the scalar→node conversion runs off-thread. This was the *only*
  `unsafe` buffer borrow in the binding layer; every other `py.detach` site was audited and touches
  Rust-owned state only (AST, source text, `BufWriter<File>`). Covered by
  `tests/test_numpy.py::TestNumpyConcurrency`. Reported by
  [@bourumir-wyngs](https://github.com/bourumir-wyngs) in #165.
- **Repeated alias references no longer resolve to `None`** — `to_dict()` expanded aliases behind a
  *global* visited-anchor set that was never cleared, so only the first reference to any anchor
  produced a value and every later one silently degraded to `None`:
    ```yaml a: &x 1 b: *x      # 1 c: *x      # was None, now 1```
    The blast radius was wider than "the second reference": two sibling references inside one
    container poisoned each other too (`{a: &x {p: 1}, b: {q: *x}, c: {q: *x}}` gave `b` a value and
    `c` a `None`). The guard is now scoped to the current recursion path — pushed for the duration
    of one expansion, popped afterwards — so repeated and sibling references each get their own
    fully built value while genuine cycles still terminate. `<<` merge resolution and the AST itself
    were audited and are unaffected. Six new PyYAML-parity cases in `tests/test_direct_load.py` pin
    the agreement, and two tests that had pinned the buggy output as expected behaviour were
    rewritten. Reported by [@bourumir-wyngs](https://github.com/bourumir-wyngs) in #163.
- **Document header comments no longer vanish on nested first values** — the parser kept one shared
  comment slot for all in-progress containers, so a nested container start clobbered a standalone
  header note before it reached the AST (dropped at parse time; invisible to `to_dict`, fatal for
  `dump`). Slot handling is now a per-container stack.
- **Splice edits no longer duplicate leading comments** — when a regenerated region text carried the
  pair/item's own standalone note, the replaced range skipped the old note line and both survived;
  the plan now widens the range over note lines (`pyq set`/`delete` and the bindings' splice path
  share the fix).
- **`!timestamp` accepts a trailing `Z` on every supported Python** — `datetime.fromisoformat` only
  recognizes the UTC-`Z` suffix from 3.11; the plugin now normalizes `...Z` to `+00:00` first,
  fixing `Invalid isoformat string` on 3.8–3.10 for YAML `!timestamp` scalars and TOML datetimes via
  `load_toml` / `from_toml`.

#### Performance

- **Direct event→Python load materialization** — `safe_load`, `safe_loads` and `YAML().safe_load*`
  build Python objects in one pass over the granit event stream instead of constructing the full AST
  and walking it again in `convert.rs`; schema resolution, raw-text mapping keys and duplicate-key
  errors stay identical. Anchored/tagged/merge/multi-document shapes fall back via zero-cost
  pre-vetoes. WSL wall-time: scalar-heavy `safe_load` −21~25%, family overall −13~18%, fallbacks
  unchanged.
- **Anchor extraction byte gate** — `extract_anchors` returns empty after a single `&`
  byte-containment check; anchor-free documents skip the per-character quote state machine.
  Rust-side `parse_*` benches improve 11–18% at the median; the scan itself drops 1.5µs → 38ns.
- **Interned stream-event dict keys** — fixed per-event keys from `parse_stream` / `load_stream`
  reuse interned strings (`pyo3::intern!`). WSL wall-time: `parse_stream` −34%,
  `parse_stream_multidoc` −39%, `load_stream` −22%.
- **Decomposition micro-benchmarks** — new `granit_events_*` benches isolate the pure granit
  event-pipeline cost from AST construction (bench-only).
- **Multi-document parse without per-document clones** — `on_document_end` moves the completed
  document into the collection instead of deep-cloning it. WSL wall-time: `parse_all_docs` −9.7%,
  `safe_loads` (multi-doc) −9.5%, `YAML().safe_loads` −6.7%.
- **Streaming write reuses one buffer across documents** — new `direct_dump_into` writes each
  document into a reused `String`, and `dump_iterable` skips `normalize_doc`'s re-copy in the normal
  case. WSL wall-time: `dump_stream_multi_doc` −27.2%, `dump_stream` −4.4%.
- **Scalar fast paths in the AST builder** — `unescape_double_quoted` returns early without
  backslashes; `detect_chomping` pulls lines lazily instead of collecting the whole document per
  block scalar. WSL wall-time: `to_dict` family −4~9%, scalar-type loads −3~4%, no regressions.

#### Docs

- **Corrected the numpy guide's 0-D scalar section (all locales)** — the snippet claimed 0-D
  arrays "reshape to a single-element list" (`assert data == [42]`); shipped behavior (pinned by
  `tests/test_numpy.py`) serializes bare scalars (`assert data == 42`). The en page gained a warning
  admonition for the 0-D `bool` → `1.0` rust-numpy quirk.

</details>

### [v0.15.0] — 2026-08-19

<details markdown="1">
<summary>Node metadata and style API · schema file IO · deep editing · numpy back on
cp314t</summary>

#### Added

- **Node metadata setters/getters** — `Node.comment` / `Node.anchor` / `Node.tag` read properties
  and `set_comment` / `set_anchor` / `set_tag` (plus `remove_*` variants), backed by new path-based
  edit operations. Editing an alias or a missing path raises; standalone comments on inline scalar
  values and sequence items are now serialized on their own indented lines (fixes pre-existing
  broken round-trip for `child:\n  # c\n  val` and `- a\n# c\n- b`).
- **Verbatim tags** — `set_tag("!<tag:yaml.org,2002:str>")` now produces a verbatim tag (empty
  handle), and verbatim tags parsed from source survive round-trip: `Tag`'s `Display` emits `!<...>`
  wrapping for empty-handle tags, `parse_tag` recognizes the `!<...>` form, and stream events
  serialize tags through `Display`.
- **Schema file IO and listing** — `load_schema(name, path)` reads a schema definition from a file
  and registers it; `list_schemas()` returns all registered schema names (built-in
  `failsafe`/`json`/`core`/`yaml1.1` plus custom).
- **Node style/format setters/getters** — `Node.scalar_style` / `Node.flow_style` / `Node.chomping`
  read properties and `set_scalar_style` / `set_flow_style` / `set_chomping` methods.
  ScalarStyle/Chomping now derive `Copy`. Non-scalar nodes return `None` / are no-op; aliases and
  missing paths raise.
- **Schema structural validation** — a `validate` section in a schema definition adds structural
  checks (path-qualified scalar types, `sequence_of`/`mapping_of` containers, `required`);
  `validate_against_schema(data, schema_yaml)` raises `YamlValidateError` listing every failure.
- **`Node.copy()`** — deep-copies a subtree as a standalone Python value (dict/list/scalar),
  detached from the document, for pasting via `set_value()`.
- **Deep editing API** — `doc.set_many({path: value})` sets multiple paths (with wildcard `[*]` and
  deep-scan `..` support) in a single splice burst; `doc.sort_keys()` orders mapping keys in place;
  `Node.move(new_path)` relocates a subtree; `Node.path` / `Node.find_first()` / `Node.value_eq()`
  add path access, first-wildcard lookup, and value comparison.
- **Property-based testing for 0.14+ features** — Rust proptests for `validate_node`, schema
  parsing, style-settings round-trip; Python hypothesis tests for `set_many` wildcards,
  metadata-edit preservation, `sort_keys` idempotency. `hypothesis` moved to the `test` group so CI
  runs property tests.
- **Serializer fix** — standalone comments on empty flow containers (`key: {}` / `key: []`) no
  longer produce invalid YAML; demoted to inline.

#### Changed

- **NumPy re-enabled on free-threaded (cp314t) wheels** — the `--no-default-features` flag is
  removed from the cp314t build lines; rust-numpy 0.29 supports free-threaded Python, so
  `numpy.ndarray` serialization is now available on free-threaded wheels when NumPy is installed
  (auto-detected at runtime).

#### Docs

- **Corrected stale references across all locale docs (en/zh/ja/ko)** — `saphyr-parser` →
  `granit-parser`, YAML compliance 98.1% → 99.75% (405/406 suite cases), ABI3 support 3.9–3.13 →
  3.8–3.15 (py3.9+ → py3.8+), and benchmark tables updated to current CodSpeed CI numbers (parse
  21–43×, serialize 55–177× faster than PyYAML). Rust-side benchmark sections migrated from
  Criterion to divan (`benches/yaml_bench.rs` → `crates/pyrs-yaml/benches/yaml_bench.rs`).

</details>

### [v0.14.1] — 2026-08-15

<details markdown="1">
<summary>quoting and escape edges: single-quote backslash, BOM, noncharacters, multibyte
wrap</summary>

#### Fixed

- **Single-quoted scalars with backslash + control/noncharacter** — such values now use
  double-quoting; single quotes cannot escape control chars/noncharacters.
- **Noncharacters and BOM quoted** — `needs_quotes` / `needs_double_quoted` now require quoting for
  U+FFFE/U+FFFF/plane-end noncharacters and U+FEFF (BOM).
- **Double-quoted escape width** — code points above U+FFFF now emit the 8-digit `\Uxxxxxxxx` form
  (the 4-digit `\u` form is BMP-only).
- **Folded plain-scalar continuation indent** — continuation indent is derived from the value's
  start column so nested sequence/mapping items stay indented past the parent block indent.
- **Multi-byte wrap boundary** — `wrap_plain_scalar` floors the wrap slice to a char boundary
  instead of panicking on 4-byte UTF-8 straddling.
- **`hypothesis` in publish test requirements** — `.ci/requirements-test.txt` pins
  `hypothesis>=6.113.0` so the publish workflow can run the property suite.

#### Added

- **`scripts/fuzz_panics.py`** — high-volume local Hypothesis fuzz harness with a hostile strategy
  across dump/parse/edit/idempotency.

</details>

### [v0.14.0] — 2026-08-14

<details markdown="1">
<summary>YAML Schema language and pluggable resolution · quoted scalars · empty
collections</summary>

#### Added

- **YAML Schema Language** — define custom schemas with `rules` mapping regex patterns to YAML
  types; register via `register_schema()`.
- **Inline dict schema** — `schema` parameter accepts `dict` in `YAML()`, `parse()`, `safe_load()`,
  etc.
- **Community Plugins** — `CustomType` base class with `from_yaml`/`to_yaml`/`can_parse`/`validate`;
  register via `register_type()`.
- **Built-in plugins** — `!timestamp` (datetime) and `!set` registered by default.

#### Changed

- **Schema resolution is pluggable** — `SchemaResolver` trait + `Schema` enum with global
  `SchemaRegistry`. Built-in schemas retain zero-cost dispatch.
- **`node_to_pyobject` and `direct_dump` check registered `CustomType`s** — tagged scalars convert
  via `from_yaml()`; objects serialize via `to_yaml()`.
- **`get()` is literal-key only** — `YamlDocument.get()` no longer guesses JSONPath for keys
  containing `.` or `[`; every key is treated as a top-level mapping key, consistent with
  `__getitem__`/`__setitem__`. Path access stays available via `find()`/`node()`.

#### Fixed

- **Quoted scalars always load as strings** — implicit type resolution now applies only to plain
  scalars (YAML 1.2): `safe_load('"true"')` returns the string `"true"`, not `True`. The serializer
  keeps negative numbers round-tripping through the document (`to_yaml`) path.
- **Lone-quote keys round-trip** — mapping keys that are a single `'` or `"` are emitted as quoted
  scalars instead of unparseable YAML.
- **Empty collections emit `{}`/`[]`** — dumping empty mappings/sequences no longer yields an empty
  document that re-parses as `None`.

</details>

### [v0.13.0] — 2026-08-10

<details markdown="1">
<summary>MSRV 1.96 and edition 2024 · direct writer and load fast paths · granit migration</summary>

#### Changed

- **Rust MSRV raised to 1.96 and edition bumped to 2024** - both crates now declare
  `rust-version = "1.96"` and `edition = "2024"`; CI pins the `build`/`test-freethreaded` jobs to
  Rust 1.96 for deterministic wheel builds and adds an `msrv-check` job running
  `cargo check`/`cargo test` at the MSRV to prevent silent MSRV drift (the `rust-lint` job stays on
  `stable`). The floor is set above PyO3 0.29's own baseline (rustc 1.83) for std API headroom (e.g.
  `assert_matches!`, stabilized 1.96) with no code migration needed. `TAG_REGISTRY` (tag handler
  storage) refactored to `std::sync::LazyLock`, dropping the `Mutex<Option<...>>` indirection.

#### Performance

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

#### Fixed

- **`float_to_yaml_string` round-trip fix** — appends `.0` when Rust Display drops the decimal (`42`
  → `42.0`) so floats round-trip as floats instead of becoming ints.
- **Reverted `count_nodes` pre-allocation** — the full AST traversal cost more than the
  reallocations it avoided (serialize_10mb was ~14% slower); buffer growth is left to the Vec.

#### Added

- **`max_depth` on stream & frontmatter APIs** — `parse_stream(yaml, on_event, max_depth)`,
  `read_markdown(path, schema, max_depth)`, `read_markdown_str(content, schema, max_depth)` accept
  `max_depth` (default 1000). Stream parsing now enforces the nesting-depth limit via core
  `parse_stream_with_options` (previously stream events had no depth limit).
- **Pydantic integration** — `dump_pydantic()` serializes a Pydantic model to YAML string via
  `model_dump(mode='json')` + `safe_dump`; `parse_as()` parses YAML string into a Pydantic model
  instance. Both use lazy imports, no hard dependency on pydantic. (#61)

#### Internal

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

### [v0.12.1] — 2026-08-06

<details markdown="1">
<summary>set(create_missing) · walk/scalars · monorepo workspace · parse hot paths,
Arc<str></summary>

#### Added

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

#### Changed

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

#### Fixed

- **`set(create_missing=True)` nested chain build** - the created mapping chain no longer duplicates
  the first segment as a nested key level.
- **`set(create_missing=True)` eligibility** - freshly created keys are now eligible for the value
  write (the eligibility check no longer runs after the synthetic pair is inserted).
- **Standalone comments before simple mapping keys** - round-trip previously dropped standalone
  comments attached to simple-key nodes; now preserved (two regression tests).

</details>

### [0.11.7] - 2026-08-04

<details markdown="1">
<summary>release-guard static assertions replace the always-red stub check · numpy
tracking</summary>

#### Changed

- **stub-build-check replaced with release-guard** - the always-red container build (`validate.yml`)
  that deliberately failed to reproduce the v0.10.0 `--generate-stubs` failure mode is replaced with
  three static assertions that **pass** when the repo is correct: `grep` guards `publish.yml`
  against `--generate-stubs`, `git ls-files` asserts the committed `.pyi` is tracked, and `test -f`
  checks `py.typed` exists. The job now gives green CI on correct state, red only on regression.

#### Added

- **Numpy free-threaded tracking** - ROADMAP.md now tracks `rust-numpy` free- threaded support
  status (PyO3/rust-numpy#476) as a dependency for re-enabling ndarray serialization on cp314t
  wheels when the Rust binding matures.

</details>

### [0.11.6] - 2026-08-04

<details markdown="1">
<summary>cp314t wheels drop numpy · free-threaded CI validation · install docs</summary>

#### Changed

- **Free-threaded (cp314t) wheels are now numpy-free** - built with `--no-default-features`, so
  rust-numpy is excluded entirely (smaller binary, no runtime probe). `safe_dump` on a
  `numpy.ndarray` raises `YamlTypeError` on free-threaded builds; GIL builds (Python 3.8-3.15) keep
  full ndarray serialization.

#### Added

- **Free-threaded CI validation** - `test-freethreaded` job now builds and tests with
  `--no-default-features`, matching the shipped free-threaded wheel configuration.
- **Install docs** - `docs/{en,zh,ja,ko}` note that free-threaded wheels are numpy-free (ndarray
  serialization unavailable on cp314t).

</details>

### [0.11.5] - 2026-08-04

<details markdown="1">
<summary>parser robustness items closed by an audit that found no fixable case</summary>

#### Changed

- **Parser robustness items 3/4/5 closed via Phase 0 strictness audit** — the 70-probe corpus
  (indentation, block-mapping keys, flow context) compared against a PyYAML oracle showed **no
  fixable accepted-but-invalid case** (64/70 match; the 6 divergences are deliberate YAML 1.2 /
  yaml-test-suite requirements where PyYAML is the outlier, and one deliberate duplicate-key
  strictness). Compliance stays at **99.75% (405/406)**. Full write-up in `ROADMAP.md` §v0.11.5 and
  `tests/test_strictness_audit.py`.

#### Added

- `tests/test_strictness_audit.py` — 70-probe strictness regression corpus pinning current
  rejection/acceptance behavior (both directions), so future parser changes cannot silently regress
  strictness or over-reject.

</details>

### [0.11.4] - 2026-08-04

<details markdown="1">
<summary>duplicate empty keys accepted (2JQS) · compliance counts correct rejections · tab
decoding</summary>

#### Fixed

- Duplicate null/empty mapping keys no longer error (`: a\n: b`, `~: a\n~: b`) — matches
  yaml-test-suite 2JQS; real duplicate keys still raise `YamlDuplicateKeyError`
- Compliance harness: correctly-rejected invalid YAML now counts as pass (was lowering the rate
  despite compliant behavior)
- Compliance harness: `convert_special_chars` tab decoding via regex — any run of `—`/`‖` + `»` is
  one tab, fixing tab-encoded suite cases

#### Changed

- YAML Test Suite pass rate gate raised from >75% to **≥95%**; current rate **99.75%** (405/406)
- Known deviation documented: `ZYU8` (`%YAML 1.1 1.2`) is rejected by design (invalid per YAML 1.2
  grammar, matches PyYAML/libyaml)

</details>

### [0.11.3] - 2026-08-03

<details markdown="1">
<summary>streaming write · line-offset cache · publish pre-validation · compliance report</summary>

#### Added

- Streaming write: `YAML.dump_stream(file_obj, iterable)` / `YAML.dump_file(path, iterable)` with
  document-level constant memory, auto `---` separators, and `explicit_start`/`explicit_end` flags
- `YamlDocument` `with` context manager: snapshot/rollback transaction scoping
- `compliance_report()`: public YAML Test Suite pass-rate reporting (version-consistent)

#### Changed

- Edit-burst line-offset cache: internal O(N+edit) carry-through in the splice layer (public API
  unchanged)
- `compute_compliance` moved from tests to `pyrs_yaml.compliance`; version no longer hardcoded

#### Fixed

- Changelog mirror drift guard: prek hook + CI job assert root/mirror `[Unreleased]` sync
- Publish stub pre-validation: CI reproduces v0.10.0-class `--generate-stubs` container failures
  before Release

</details>

### [0.11.2] - 2026-08-03

<details markdown="1">
<summary>parse skips splice eligibility · linear-cursor layout check (v0.11.0 regression)</summary>

#### Added

- `YAML.load_stream(file_obj)` / `YAML.load_stream_file(path)`: lazy event iterators with O(anchors
    - chunk) memory

#### Performance

- **Parse no longer computes splice eligibility** — the O(document) layout check now runs lazily on
  the first edit via `YamlDocument.splice_checked`, restoring the v0.11.0 regression: parse_comments
  -59%, parse_anchors -42%, parse/roundtrip/edit -10~35% all back to v0.10.0 levels
- **Linear-cursor layout check** — replaces per-node binary search over precomputed line offsets
  (monotonic source-order traversal)

#### Changed

- `parse_with_options` returns `CustomNode` (was `(CustomNode, bool)`); splice eligibility is now
  internal to `YamlDocument` and computed on demand

</details>

### [0.11.0] - 2026-08-02

<details markdown="1">
<summary>surgical serialization: an edit writes back without re-laying-out the document</summary>

#### Added

- **Surgical Serialization** — byte-level source span tracking on every AST node; segment-based
  splice — edits regenerate only the touched region, untouched text is byte-copied
- proptest fidelity property tests (new dev-dependency)
- 10MB edit-flush benchmarks (divan)

#### Changed

- `flush_source` now splices segments; falls back to full serialization for flow-style regions,
  non-default layout documents, merged keys, CRLF/BOM documents, and after materialization
  (single-burst model)
- Splice edits preserve `---`/`...`/directive marker lines as untouched bytes (full serialization
  previously dropped them — deliberate behavior difference)

</details>

### [0.10.0] - 2026-08-01

<details markdown="1">
<summary>in-place editing API, with edit benchmarks</summary>

#### Added

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

#### Changed

- `YamlDocument.source()` now returns `str` and lazily re-serializes after in-place edits

</details>

### [0.9.0] - 2026-08-01

<details markdown="1">
<summary>CPython 3.13/3.14/3.15 and no-GIL · tag handler registry · pydantic · .pyi stubs</summary>

#### Added

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

#### Changed

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

#### Fixed

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

### [0.8.0] - 2026-07-30

<details markdown="1">
<summary>YAML() instance API · Python Node API · doc.version · MergedView · lifecycle
warnings</summary>

#### Added

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

#### Changed

- `parse()` / `safe_load()` now delegate to `YAML().parse()` / `.safe_load()` as syntactic sugar
- `YamlDocument` now stores `version` field for document metadata

</details>

### [0.7.1] - 2026-07-30

<details markdown="1">
<summary>ryaml benchmark comparison · compliance threshold raised · Divan benchmark
migration</summary>

#### Added

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

#### Changed

- CI benchmark job installs `ryaml` for cross-library comparison
- `benchmark_compare.py` now delegates timing to `pytest-benchmark` and serves as a feature
  comparison/reporting tool

</details>

### [0.7.0] - 2026-07-29

<details markdown="1">
<summary>serializer max_depth guard · hot-path optimization · pytest-benchmark migration</summary>

#### Added

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

#### Changed

- `pytest-benchmark` replaces raw `timeit` in Python benchmarks
- CI benchmark job now runs `pytest --benchmark-json` instead of standalone script

#### Removed

- `write_inline_comment` method — inlined at all call sites
- `Comment` import from serializer — no longer needed

</details>

### [0.6.0] - 2026-07-27

<details markdown="1">
<summary>async serialization · JSON Schema validation · to_json() · incremental re-parse</summary>

#### Added

- **Async serialization** — `safe_dumps_async`, `safe_dump_async`, `safe_loads_async`,
  `safe_load_async` via `asyncio.run_in_executor` (`python/pyrs_yaml/async_dump.py`)
- **JSON Schema validation** — `YamlValidateError` exception + `YamlDocument.validate(schema)`
  method (accepts `str` or `dict`); delegates to Python `jsonschema` module
- **`YamlDocument.to_json()`** — serialize document to JSON string (uses Python `json.dumps`)
- **Incremental re-parse** — `YamlDocument` now stores source text (`doc.source()`);
  `doc.reparse(resolve_merges=True, schema="core")` re-parses in-place
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

</details>

### [0.5.0] - 2026-07-27

<details markdown="1">
<summary>Serializer::write_node fix · YAML_SCHEMA constant · development documentation</summary>

#### Fixed

- **`Serializer::write_node`** — `.unwrap()` on `values.iter().next().unwrap()` in
  `block_mapping`/`block_sequence` replaced with safe indexed access to eliminate potential panic on
  edge-case ASTs
- **`YAML_SCHEMA` constant** — typo `yamorg2002` corrected to `yamlorg2002` (matches YAML 1.2 spec
  URL)
- **Development documentation** — `AGENTS.md` updated with mandatory `uv run` prefix for Python
  commands and direct `cargo` for Rust commands

</details>

### [0.4.0] - 2026-07-27

<details markdown="1">
<summary>132 gap-filling tests: i18n, multi-document, bytes input, suite cases, fidelity</summary>

#### Added

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

#### Changed

- Fixed version sync: `python/pyrs_yaml/__init__.py` `__version__` updated from 0.2.0 to 0.4.0 to
  match Cargo.toml/pyproject.toml
- Removed stale 0.2.0 wheel artifacts from `dist/`

</details>

### [0.3.0] - 2026-07-27

<details markdown="1">
<summary>NumPy ndarray serialization (N-D) · quoted-scalar typing · negative round-trip</summary>

#### Added

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

#### Fixed

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

#### Changed

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

### [0.1.0] - 2026-07-25

<details markdown="1">
<summary>initial release: YAML 1.2 AST with full metadata, round-trip, PyYAML-compatible
API</summary>

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

</details>
