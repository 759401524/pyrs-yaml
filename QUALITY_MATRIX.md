# Quality Defence Matrix

Assessment of how this repository finds defects, and the gate that keeps the assessment
true. Everything below is measured out of the files that declare the defence
(`.github/workflows/*.yml`, `prek.toml`, `fuzz/Cargo.toml`, `scripts/check_*.py`,
`crates/pyrs-yaml-core/benches/ir_gate.rs`, `crates/pyrs-yaml/benches/ir_gate.rs`,
`.ci/ir-baseline.json`) by
`scripts/quality_matrix.py`. `tests/test_quality_matrix.py` re-runs that measurement on
every pull request and compares it with `.ci/quality-holes.json`, so neither the numbers
nor the list of known blind spots can rot into prose nobody checks.

Verdict, in one line each:

- **Method: systematic.** Every fix in the recent history carries a mutation-attribution
  proof, and falsified hypotheses are recorded rather than quietly dropped.
- **Defence: reactive.** Each gate that exists was built *after* a defect walked past the
  gates that existed then. Nothing measured the coverage of the defence itself until now.

## 1. Test matrix coverage

Measured inventory of the three tiers:

| tier | artefacts | what it can prove | what it structurally cannot reach |
| --- | --- | --- | --- |
| unit / integration | 814 Rust `#[test]`, 1,953 Python test functions | a named shape behaves as documented | anything not named — the whole reason the other two tiers exist |
| property | 29 proptest properties (26 named `prop_*`, 2 in `pyrs-event` named in prose), default 256 cases each | no crash + an invariant over a generated grammar | shapes outside the generator; invariants weaker than equality |
| fuzz | 9 targets, 168 seeds of which 84 are `former-crash-*` | a historical crash cannot regress (PR tier replays seeds with `-runs=0`) | the discovery tier runs on a schedule, so a new shape is found weekly rather than per-PR |

**The ninth target drives a *change*, not a document.** The eight before it hand the engine text to
parse or a tree to write; the layer that rewrites one region of a document - `doc.set()`, the `pyq`
verbs, the round-trip promise itself - had no fuzz tier and no property tier over arbitrary input.
`fuzz/fuzz_targets/edit_yaml.rs` takes `document NUL script`, picks paths the reader really reached,
and requires every accepted edit to re-parse, with the value it wrote readable at the
place it claims to have written it. It paid for itself before it was committed: the
first seed replay reached an anchor rename that emitted `base: &a0 1 / use: *x`, text this library's
own reader refuses, and exploration reached two more doors over the same invariant (a `set` or `move`
that drops a referenced anchor's definition, a `delete` of it). All three are now refused at the place
the drop happens; the four inputs that found them —
`fuzz/seeds/edit_yaml/former-crash-{3cc4245d,c59db74a,b23eed05,61474da6,63749496,07b6a10e}.seed` — replay per PR.

**That target now also asks the export.** A hub document is what `doc.to_toml()`, `doc.to_json5()` and
the CLI's TOML/JSON output write, so "edit, then export" is product-reachable, and every tier before it
had only handed those writers a *reader's* tree: `toml_roundtrip` and `json_roundtrip` never see a tree
the splice layer rewrote. `edit_yaml.rs` writes the settled tree in every spelling a hub document exports as - TOML, strict JSON,
JSONC, JSON5 and the three pretty variants - and requires that dialect's own reader to accept it and its
second emission to be the same bytes; a dialect refusing the tree is a pass, because TOML has no
top-level scalar and strict JSON refuses a non-finite float. The edit verbs stay YAML-only by design -
`crates/pyrs-toml/src/lib.rs` states it - which is why this is one target asking seven exporters rather
than seven edit targets. Seven windows found seven defects: a `set` that replaced a container dropped an
anchor defined *inside* that container (`former-crash-fbd364b3.seed`); a note written on a tag-only
sequence item settled only on the second round (`former-crash-3dd60cad.seed`); a signed zero reached TOML
as text the reader spells back differently (`former-crash-917e103b.seed`); an emptied block sequence
vetoed a compactness its own emission does not have (`former-crash-a58caa38.seed`); a Core YAML float
spelling with a leading-zero integer part (`083e6`) reached TOML as a literal its reader refuses
(`former-crash-71feb9d9.seed`); a note on an anchored item whose block ended in a tag-only line settled
only on the second round (`former-crash-b22d0653.seed`); and a compact sequence item's own note under an
anchored sequence's header settled only on the second round (`former-crash-2fc59020.seed`). Each is fixed where
it happens and pinned by a named Rust test. The same two questions went into `json_roundtrip` for its
three pretty spellings, and that target's window found the family's fifth defect on the *compact* `json`
claim - a string of 119 digits is a string to the resolver but a number to JSON's grammar, so it went
out plain and came back numeric (`former-crash-09341b75.seed`). The pretty spokes have proved they run
(a mutation that makes only the pretty writer emit text its reader refuses reddens
`json (pretty) writer output failed to re-parse` over the seed corpus) but have not yet produced a defect
of their own.

Three claims the target first made are not in it, and each was narrowed by a measurement rather than by
taste. `sort_keys_path` answers "ok" for a path naming a scalar and changes nothing, so the ordering
claim is asked only where the verb has work to do - whether the silent no-op should refuse instead is a
question about a public error surface. And a note asked of a node does not always stay on that node:
`set_comment` on a block sequence emits it on the last item's line (there is nowhere else in the text it
could live) and one discovered shape handed a scalar's note over too, so `set-comment` is held here to
re-parse and settle only. The third was an open defect rather than a boundary when it was written down:
a flow member carrying both a tag and an inline note settled only on the second round - the first
emission writes `a: !fuzz2   # fuzz 47`, the re-read hands the note to the mapping's line, and the next
emission moves the tag after it. Asserting the fixed point would redden the tier on a known open defect,
which gates nothing, so the claim came out and the inputs stayed committed
(`container-note-changes-owner.seed`, `note-may-change-owner.seed`). The writer now spells that empty
body `''`, the same decision that writes `~` for an empty value before a note, the fixed-point claim is
back in the target, and the input that proved the drift is replayed as a former crash
(`former-crash-b23eed05.seed`) rather than carried as an open one.

The one boundary that remains from that narrowing is not silent either:
`api:set-comment-container-owner` is registered in `.ci/known-residuals.json`
with the test that pins today's behaviour, because `quality_matrix.py` can only
derive a hole a probe can see and this is a decision rather than a derivable
hole.

Orthogonality, tested rather than asserted: the same three defect families that the recent
fuzz tier found (comment attribution, empty-container spelling, nested merge application)
are invisible to the property tier at its committed case count, and the two data-loss
defects (`<<` merge never applied when nested, mapping keys never resolved) were invisible
to *both* Rust tiers and only became visible when a Python test compared this engine with
PyYAML and ruamel. Three tiers over one engine are not three defences: coverage is
orthogonal only where a second implementation exists to disagree with.

Per-format matrix, derived by `format_matrix()` and asserted cell-by-cell against this table by
`tests/test_quality_matrix.py` - which is how the wrong cells below were found:

| format | unit | property | fuzz parse | fuzz round trip | cross-library parity | external authority |
| --- | --- | --- | --- | --- | --- | --- |
| YAML | yes | yes | yes | yes | yes | yes |
| JSON / JSONC / JSON5 | yes | yes | yes | yes | yes | yes |
| TOML | yes | yes | yes | yes | yes | yes |
| JSON Schema validation | no | no | no | n/a | n/a | yes |
| Schema rule language | yes | yes | yes | n/a | n/a | yes |

**There are two schema surfaces, and both are now judged from outside.** `Document.validate()`
accepts JSON Schema and delegates evaluation to Python's `jsonschema`, keeping the AST-to-instance conversion, schema
acceptance and the `YamlValidateError` contract on this side. The rule *validator* this repository
invented is gone: it was retired on 2026-10-21 in favour of this standard surface, because nothing
outside ever asked its rules, and reusing a standard engine removed the question instead of answering
it. Its entry points - `validate_against_schema()`, `validate_against_registered_schema()`,
`TypeSpec`, `ValidateKind`, `ValidateRule` and `SchemaValidationError` - are deleted, not deprecated.

The rule *language* that validator read is a different surface, and it survives: `parse_schema_yaml()`,
`RuleResolver` and `YamlTypeKind` are still exported by `pyrs-yaml-core`, `register_schema()`,
`load_schema()` and `list_schemas()` still read a `rules:` block, and `YAML(schema=...)` still
resolves scalars through it. It has its own row in the table above, counted in the file that declares
it rather than in the crate it shares with the YAML engine - measured by `pyrs-yaml-core` as a whole it
would read as defended by hundreds of tests that never mention it. Its unit cell is the eleven
`#[test]`s of `crates/pyrs-yaml-core/src/parser/yaml/schema_language.rs`, its property cell is the one
function that names it, `prop_schema_parse_no_panic`, and its fuzz cell is `parse_schema`.

The JSON Schema row's other cells are structural rather than a claim that nothing anywhere exercises the
surface: the row owns no Rust crate, because the evaluation is Python. The Rust fuzz target that exists
(`fuzz/fuzz_targets/parse_schema.rs`) fuzzes the `rules:` grammar - the scalar-type resolution path -
not JSON Schema evaluation, so the `fuzz parse` cell reads `no` truthfully. The `n/a` pair stays
because the surface judges a document and owns no writer. The rule language's `n/a` pair says the same
thing about itself: it resolves scalars and owns no serializer, so the round trip it could be caught
in belongs to the YAML row.

The measured agreement, 2026-10-21 against the official JSON-Schema-Test-Suite at commit
`0b4a7fa5e79c5e1ed60d5c034b951003bbba7fd4` (46 files, 1302 cases in `tests/draft2020-12`): 1261 cases
reach the expected verdict and 41 are excluded by name. Whole-file exclusion is one: `refRemote.json`'s
31 cases `$ref` an HTTP server the suite starts, and remote retrieval is outside this package's
contract - its 15 expected-invalid cases pass only because an unresolved reference raises, so they are
not credited either. The other ten are boundaries of the delegated engine, `jsonschema` 4.26.0: five
`dynamicRef` and one `vocabulary` case, and four `pattern`/`patternProperties` cases whose `\p{...}`
escapes Python's `re` does not support. The gate names each of the ten by `(file, group, test)` and
asserts the set in both directions - an excluded case that starts passing fails as a stale exclusion,
and a name that is not in the corpus fails as a typo. The set is keyed on the installed reference,
because the lock resolves three versions: 4.23.0 on the Python 3.8 leg needs one more named
`unevaluatedProperties` case (an upstream annotation defect fixed by 4.25.1), while 4.25.1 and 4.26.0
agree with the corpus on it. Replaying the same corpus through
`jsonschema.validate()` without this package reaches the identical verdict on all 1302 cases, so no
exclusion is attributable to the conversion; reuse was the ruling, and the delegated engine and this
surface agree everywhere the engine itself is not the boundary.

**The tier row's `25` was the other error.** Counting `fn prop_` names dropped the two properties in
`crates/pyrs-event/src/lib.rs`, which `proptest!` declares with a `#[test]` attribute and a prose name
rather than the prefix, so the property tier holds 27. A tool that counts a naming convention measures
the convention, not the defence; the rule is now the attribute, with the prefix accepted as well.

**The last column is the one this file used not to measure at all.** `external authority` is `yes` when
a `tests/test_*_suite.py` or `tests/test_*_spec_table.py` exists for the dialect - a gate that asks the
format's own grammar, or an official suite's cases, against something this repository does not own:
`test_yaml_suite.py`, `test_json_test_suite.py`, `test_toml_test_suite.py`,
`test_json5_spec_table.py` for the JSON5 productions inside the JSON row, and
`test_json_writer_suite.py`, `test_yaml_writer_suite.py` and `test_toml_writer_suite.py` for what
the writers emit rather than what the readers accept.
Each file's cell is a verdict, not its name: a file counts when it reaches into a reader no marker
bounds - a call in a test, or a callable wired into a module-level table before any test runs - or, when
every reader it names is bounded, when a call site nothing can skip still asks one. A corpus suite,
which has no reader to be absent, is credited by the corpus it names - a `Reference/` path in a
string its loader receives, not the suffix on its file name. An import is not a use: a stale
`import json` beside comparisons that all ask a bounded reader used to keep the cell at `yes`.
`test_json5_spec_table.py` was the shape this denies - it imported `pyjson5` alone and gated the whole
module on that import, so the column read a file name while the 3.8 floor and 3.14+ judged the JSON5
grammar with nothing. The file
asks the pure-Python `json5` reference now, and a file that regresses to a bounded reader alone is
emitted as a hole of the `authority-tier` kind, named for the file, rather than sitting in prose.
It is deliberately not the
parity column: parity compares two implementations and answers "do we agree", which cannot find a syntax
neither side ever accepted - a path that does not exist has nothing to disagree with.
The JSON Schema row reads `yes` through `tests/test_document_validate_suite.py`. The rule language's
row reads `yes` too: `tests/test_parse_schema_spec_table.py` asks each `type:` name the `rules:`
grammar accepts whether the value it resolves is really an instance of the JSON Schema type that name
denotes, and `jsonschema` - which owns that membership - can refuse it. The decoy beside every row
(a standard type the same value must NOT satisfy) is what stops the confirmation being a rubber stamp;
JSON Schema keeps `boolean` and `integer` disjoint even though Python's `bool` subclasses `int`, so a
tag the grammar got wrong is a red run, not an agreement. The blind spot this closed was registered as
`authority-tier:schema-rule-language` and is now gone, so the registry is empty again - and the reading
of an empty registry, that nothing is unmeasured rather than nothing is unjudged, is a statement about
the measurement, not a claim of coverage.

`partial` in the parity column is measured per dialect, not per family. The column reads the readers
each `test_*_crosslib.py` imports - at module level, or through a `pytest.importorskip` inside the test
that needs one - resolves how each of them is bounded, and then asks a narrower question of every
dialect in the row's own label: does some test call a reader that nothing bounds, in a unit nothing can
skip? A `skipif` in the body, a module-level `pytestmark`, or a decorator that is not `parametrize` (a
skip marker wearing a name - `@needs_pyjson5`) denies the credit, because each of those is a comparison
a leg can lose while still reporting green. The one decorator that does not deny it is a `skipif`
naming the availability of a reader nothing bounds: `pyproject.toml` installs that reader on every leg,
so the guard cannot fire while the cell is green and the comparison runs everywhere. That shape is
`tests/test_yaml_crosslib.py` end to end - every comparison carries `skipif(pyyaml is None)` or
`skipif(not _ruamel_available())`, and both readers are declared without a marker - so the guard is
resolved, not trusted: a helper whose whole body is `return _ruamel is not None` is a capability probe,
while `skipif(sys.version_info < ...)` still denies.

The unit rule is asked of every module, not only the ones with a bounded reader. The cell used to
return on the bounded-reader check before reading a single test, so a family whose readers all reach
every leg read `yes` no matter what its file ran: a `test_*_crosslib.py` whose every comparison carried
`@pytest.mark.skip` took the same branch, and so did a file with no test at all. A reader every leg
installs removes one way to lose a comparison, not the requirement that a comparison exist.

Two things bound a reader and only one of them is a dependency declaration. `pyproject.toml` keeps
`pyjson5` and `orjson` off the floor and off 3.14+; the interpreter keeps `tomllib` off every leg below
3.11, and no requirement can install it there. That second bound was read off the tree's own skip
counts before it was encoded: CI's floor leg ran 13 of `tests/test_toml_crosslib.py`'s comparisons and
skipped 11 while the cell named a single reader, and the same held on 3.9 and 3.10. The file now asks
the backport `tomli`, declared for `python_version < '3.11'` with no skip to fall back on, so the eleven
three-way comparisons run on every leg - `tomllib` above the line and `tomli` below it, each reported
with the bound that applies to it. `tomlkit` is declared with no marker and measured to reach every
leg, so TOML reads `yes`, and that file imports it outright, since a guard nothing bounds is dead code
while it is green and a silent skip the moment it is not. The version a stdlib reader arrives in is
compared against the floor the project states in `requires-python`, so moving the floor moves the
answer rather than leaving a remembered one.

The JSON family is why the question had to be per dialect. `pyjson5` and `orjson` both carry
`python_version >= '3.9' and python_version < '3.14'` and exclude armv7l, s390x and ppc64le, so
neither can be the only reference for a leg this package promises. Strict JSON reads as covered
everywhere:
stdlib `json` is an RFC 8259 reader this repository does not own and no dependency declaration bounds,
and `tests/test_json_crosslib.py` asks it the grammar with `parse_constant` refusing the three
literals the stdlib accepts by default. JSONC and JSON5 now have a reference of the same kind: the
pure-Python `json5` package declares no marker (`requires-python >= 3.8`, a `py3-none-any` wheel), so
every tested leg installs it, and the module calls it from tests nothing can skip. `pyjson5` stays as
a second opinion where its wheel exists, and the two references are not interchangeable - the pure
reader keeps a surrogate pair as two UTF-16 code units and accepts a lone surrogate, where this
engine combines the pair (RFC 8259) and refuses the lone one - so each divergence is asserted rather
than smoothed over. That empty `restricted` list is the exit the registry named, and
`parity-tier:json-jsonc-json5` was deleted with the same measurement that produced it.

**What this row first said was wrong, and the correction is the interesting part.** It
claimed a parse-only target cannot see a writer. Measured: `parse_json` calls
`to_json_text` / `to_jsonc_text` / `to_json5_text` and re-reads each result with all three
readers, and `parse_toml` does the same with `to_toml` — the writers *were* reached. What
they asserted was that the reader accepts its own writer; the re-parse result was bound to
`let _ =` and thrown away. So the missing invariant was never "does the writer produce
loadable text" but "does the writer produce text that has **settled**", and that is a
different question with a different answer:
`to_json(parse(to_json(parse(x)))) == to_json(parse(x))`. `json_roundtrip` and
`toml_roundtrip` now assert it per dialect, and the deterministic halves
(`crates/pyrs-json/tests/roundtrip_corpus.rs`, `crates/pyrs-toml/tests/roundtrip_corpus.rs`)
replay the seed corpus on every `cargo nextest` — 33 rounds from 14 JSON seeds and 30 from
16 TOML seeds, with a floor declared in each file so a corpus that stopped exercising the
writers fails instead of passing vacuously. Two more deterministic replays of the same seed
corpus ask a different question — whether a document's event stream and its tree describe the
same document (`crates/pyrs-toml/tests/event_stream.rs`, and the stream/tree parity cases in
`crates/pyrs-json/src/parser.rs`) — so the event path is gated on every `cargo nextest` too,
not only on the fuzz schedule. The third tier of the same pair sits at the Python boundary:
`tests/test_streaming.py::TestMultiFormatStream` asks, per format, that one verb produces the one
vocabulary — which quote forms a format keeps, which fields stay `None` because that format has nothing
to say about them, that the verdict text is the tree route's, and that the default call is still YAML
event for event. The vocabulary crate is measured by itself as well:
`crates/pyrs-event/src/lib.rs` builds one event per variant and pins that cloning and `into_owned`
carry every field, that a borrowed scalar and the same text owned are equal (the condition under which
a replay comparison means anything), and that owning a comment moves the shared `Arc<str>` instead of
copying it; both failure modes were produced by mutation before those tests were trusted. Its two
generic jobs — turning a byte offset into a position, and owning a borrowed event — now carry a
property tier too: arbitrary documents of mixed one-, two-, three- and four-byte characters must give
every character offset its own `(line, column)`, must collapse the bytes inside a character onto that
character's start, must never run backwards in byte order, and owning an event must change nothing but
where its text lives. Removing the character-boundary clamp reddens the new property alongside the
hand-written table test; zeroing a position inside `into_owned` reddens the ownership property. CI
elevates both to 20,000 cases with the rest of the tier.
The convention those offsets are written in is asked of all three producers at the boundary: a column
counts characters, JSON and TOML convert a byte offset through `pyrs_event::LineTable` while YAML
reports granit's own marker column, and `test_a_wide_key_costs_its_characters_not_its_bytes`
(parametrised over a CJK key and an emoji key in `yaml`, `json` and `toml`) is the only place the three
are asked the same question. It separates them by mutation: a `LineTable` that returns a byte length
reddens exactly the four JSON and TOML cases, and reading `byte_offset()` in the YAML adapter reddens
exactly the two YAML ones.
No writer is unsettled: 33 JSON dialect rounds from 14 seeds and 30 TOML rounds from 16,
every one satisfying `once == twice`.
`fmt_pbt`'s three writer fixed-point properties now hold at 20,000 cases, and two of them never
failed there at all: they were aborting on proptest's default reject budget — measured by
setting `max_global_rejects` back to 1024, which kills all three with `Test aborted: Too many
global rejects` at `fmt_pbt.rs:93` rather than with an assertion. The third was real. A note
carried by a non-last member of a multi-line inline table went out after the separator comma,
and `#` runs to end of line, so the reader hands that note to the *following* key and the second
emission moved it. Withdrawing the fixed rule leaves all 354 tests green at proptest's default
256 cases once the persisted shrink case is removed: that count never generates two members
whose first carries a same-line note, so the case count was load-bearing, not decorative. `ci.yml`
now runs a blocking `property tier (20k cases, shard N/4)` job: four shards over the same case
budget, because the single leg measured 745 s on run 39387605046 and 714 s of that was one test
binary, not the compile. That closed the last hole then open; the registry
carried a new one, found by measuring what the instrument compiles rather than what it names — and
that hole is closed too, by the second harness described under *Registered blind spots*, not by an
edit to the registry.

**The exception list this section describes was itself the error.** Three inputs were
moved out of the replayed corpus and registered as unsettled writers on the strength of
a truncated test line; a temporary probe then printed both emissions for each and every
pair was equal. All three are back in the corpus, which now drives 33 JSON dialect rounds
and 30 TOML ones with nothing excluded. The mechanism that would have kept the exclusions
honest could not tell an invalid input from a settling one from a real defect, so it was
removed rather than trusted, and what is left here is the weaker, truer claim: a probe
that prints values caught in one run what three readings of test output did not.
Blocking on every pull request, measured: `clippy` (with `--all-targets`, since this
measurement), `cargo test --workspace`, the sharded 20k-case property tier (nextest, four partitions),
MSRV
check, `no_std` bare-metal build, pytest on 3 OSes × 7 Python versions, free-threaded
pytest, coverage floor, CodSpeed (Rust + Python), the callgrind Ir gate, the medium hot paths'
heap allocation ceilings (`crates/pyrs-yaml-core/tests/alloc_gate.rs`), the fuzz seed
replay, changelog-mirror / localized-script-purity / release-guard / stub-drift, the changelog
coupling check (`scripts/check_changelog_coupling.py`, evaluated on the pull request's file list
because a version-header comparison cannot see a release note that was never written), and —
as of this document — the `prek` hook set over the whole tree (`hygiene.yml`).

Seven blind spots were found by measuring, not by reasoning. One had already done damage in
`main`; the others were caught before merging, which is the difference between a gate and a
review habit:

- **The hook set ran nowhere in CI.** `ci.yml` only lists `prek.toml` as a watched path,
  and the colocated `jj` workflow does not execute Git hooks at all, so the 17 hooks were
  a local courtesy. Consequence, measured: 15 tracked files reached `main` carrying CRLF —
  12,263 lines of them, including five changelog mirrors — and one of those files turned a
  dozen changed lines into a 2,500-line diff while every gate stayed green. `hygiene.yml`
  now runs `prek run --all-files` on PRs, on `main`, and weekly.
- **`cargo fmt --check` was in no workflow.** It is a hook, and nothing ran hooks. Now
  covered by the same job (the measurement caught this as `hook-skipped-uncovered`, which
  is why that id no longer appears in the registry).
- **The builtin `mixed-line-ending` hook does not implement the policy.** Injected
  all-CRLF left it `Passed`: it fires on *mixed* endings, and a uniformly CRLF file is
  consistent. The absolute rule needed its own hook, `line-endings-lf`.
- **CI's clippy did not lint the test, bench or example sources.** The repository's own
  discipline is `cargo clippy --all --all-targets -- -D warnings`; the workflow ran
  `cargo clippy -- -D warnings`. Anything unlinted in `tests/` or `benches/` surfaced only
  on someone's local hook run. Closed by adding `--all-targets` here, after measuring that
  the wider command is already clean against the whole tree — closing a hole is only free if
  it does not trade a red.
- **A shipped file can carry conflict markers.** The stub `python/pyrs_yaml/pyrs_yaml.pyi`
  was assembled with a materialised `jj` conflict as literal text — 1,204 lines instead of
  597 — and every gate that existed at the time was quiet about it. It never reached
  `main`, but nothing *would have* stopped it: `check-merge-conflict` is a hook, and no job
  ran hooks.
- **A performance gate was comparing numbers whose provenance nobody could name, and two of this
  repository's own jobs measured the same code 1.44% apart.** The Ir baseline was generated on one kind
  of machine and enforced on another, and the 2% line used to be sized around that difference
  (`serialize_block_scalars`: +0.78% locally against +2.23% on CI). Four explanations were tested. Then
  a fifth one held.

    Refuted first, in the order they were tried: `.gitattributes` normalising CR bytes inside
    `BLOCK_SCALAR_YAML` — adding `-text` moved the runner's number by 28 instructions out of 16,020,906,
    though the probe did expose a real reproducibility hole, the fixture's *stored* bytes depending on
    which tree the last author committed, now closed and asserted by `tests/test_line_endings_gate.py`.
    Drift between runner images — two images on one commit agree to 0.0018%. The committed values having
    been generated off-runner — the enforcing job reproduces them. And a restored build cache making
    `.github/workflows/ir-baseline.yml` compile differently: the step was deleted, and the job still
    reported 15,792,928.

    The instrument that found the cause is the binary hash. Both jobs print the SHA-256 of what
    `build_exe` measures, and they printed the *same* one (`ir_gate-762abe271a6273f5`,
    `a0386c17b5f1ef38`) with the same valgrind 3.22.0, the same pinned rustc 1.97.1 and the same `nproc`
    — so neither the build nor the code was in question, only what a binary learns about the CPU at
    start-up. The hosts did differ, in the way that decides it: one reported `AMD EPYC 9V74 80-Core`, the
    other `AMD EPYC 9V45 96-Core`, and the CPUID flags a VM exposes choose which string routine glibc
    binds. `serialize_block_scalars` is the most copy-heavy scenario in the set and the only one that
    moved, which is the pattern that mechanism predicts. Masking the vector caps for every measured
    process (`GLIBC_TUNABLES=glibc.cpu.hwcaps=-AVX512F,-AVX2,-AVX,-SSE4_2,-POPCOUNT`, printed in both job
    logs beside the hashes) settled it: the enforcing job, on a host of a different model, measured
    15,780,929 against a baseline of 15,792,910 — −0.08%, where the same job had measured +1.44% with the
    caps unmasked. Regenerated under the pin, the jobs now differ by tens of instructions on that scenario
    across three different host models — `AMD EPYC 9V74`, `9V45` and `7763` measuring 15,780,927,
    15,780,929 and 15,780,957 (0.0002% apart), where the unpinned pair were 1.44% apart. The
    WSL-to-runner gap this ledger carried
    for three revisions was the same thing, and the machine-class story attached to it had never been
    tested until it was.

    What the exercise also established is that the file's own prose was not generated. Its
    `generated_by.note` cites a commit that is not PR #299's head and predates the toolchain pin — that
    job would have measured with the image's `@stable`, rustc 1.99.0, which moves this scenario −6.6% and
    `serialize_medium` +6.7% — and its environment string reads `ubuntu-24.04` where `environment()`
    emits `ubuntu24` for that image. So the note is gone; `--update` writes every key, including the
    sample size the numbers were drawn with (which the enforcing run reads back), the key set is pinned
    by `tests/test_ir_baseline_workflow.py`, and the reasoning lives here. Saying "every value comes from
    the environment that enforces it" was premature while two of those environments disagreed; what makes
    it true now is that the measurement pins the part of the environment it could not control, and a
    baseline records the method — image, compiler, sample count, pinned glibc capabilities — not just the
    number.

    A second environment input closed the same way, and it was the larger one. Three refresh runs print
    one digest for the binding harness (`7adbeac719edacd6`) and spread `load_json_long_strings` 1.121%
    across their nine samples, while the engine scenarios on those same bytes agreed inside 0.00001% —
    and only one harness links CPython. `PYTHONHASHSEED` was not pinned, so every measured process drew
    its own dict layout. Ask the seed and the answer is exact: with `PYTHONHASHSEED=0` that scenario
    reads 95,136,157 three times over, and all seven binding scenarios collapse (0.2811%, 0.2550%,
    0.1249%, 0.0787% to 0.0000–0.0004%), while the engine harness does not move in a way a reader could
    call a term (`parse_medium` 321,690,260 unpinned against 321,689,870 pinned). The refresh that
    re-drew the baseline under the pin reports every one of the forty-two scenarios within 0.0010% of
    itself across its three samples, which is the instrument being ~800× inside the line it enforces
    instead of straddling it — the same remedy as the glibc caps: pin the input the environment chooses
    rather than widen the line to absorb it. The floors then moved only where the seed had been doing
    the work: −0.526% on `load_json_long_strings`, −0.166% to −0.055% on the other five loaders,
    +0.051% on `to_python_medium`, and the thirty-five engine scenarios by 0.001% or less. Guarded by
    `tests/test_ir_gate_provenance.py`, which names the key if it goes missing.
- **No CI check is required for a merge at all - not one leg of the matrix, not the hook set, not
  the performance gate.** Read from `gh api repos/<repo>/branches/main/protection`:
  `strict: true`, `contexts: []`, `checks: []`. Measured consequence: #298 was rebase-merged while
  `test (windows-latest, 3.8)` - the leg that would have caught two checkers broken on the supported
  Python floor - had never been consulted, and it was not *red*, it was simply not part of the
  question the merge asks. Earlier drafts of this bullet said "20 of 21 legs are advisory", which the
  settings show was still too generous. That is why `Test matrix (all legs)` exists: one name to
  require instead of 21, so a leg that never starts cannot be satisfied by absence, and
  `scripts/check_matrix_verdict.py` refuses `skipped`/`cancelled` rather than reading them as
  agreement. It waits on **every** job in `ci.yml` - 11 of them, not the 3 it started with - and
  `tests/test_matrix_verdict_gate.py` compares that list against the jobs declared in the file, so
  a job added without widening the fan-in reddens the suite. The half outside a pull request is closed
  too: re-reading `gh api repos/<repo>/branches/main/protection` for this revision returns
  `strict: true`, `contexts: ["Test matrix (all legs)"]` and `enforce_admins: true`, so the name this
  document said still had to be added has been added. **The row above it was wrong at the time it was
  written and right about the software half; what it kept asserting was the configuration half, which
  had since moved.** That is the same failure this file exists to catch - a reading recorded once and
  then read as if it were still being taken - and it is why the reading is quoted with its revision.
- **The only job that runs on `main` for a code change asserted its premise in an `echo`.** Every heavy
  leg is gated behind `github.event_name != 'push'`, and `main-gate` explained why that is honest: the
  pushed tip's "identical tree already passed the full CI matrix as a pull request; skipping the
  duplicate spend." An `echo` cannot fail, so the claim - not the measurement - was the trunk's defence.
  The claim held everywhere it was checked: `07f988fa` carries pull request #492's head tree
  (`7b24405da569…`), and the tips `60f670d0` and `225cf373` each carry a head tree whose
  `Test matrix (all legs)` conclusion is `success`. So this is a hole in the shape of the defence, not
  an incident. `scripts/check_main_provenance.py` now asks the assertion's three questions against the
  API - which pull request produced this commit (`GET /commits/{sha}/pulls`, whose answer can be *none*),
  whether the pushed tree is one of that pull request's commit trees, and whether the required check
  said `success` for the commit that carries it - and refuses otherwise; `tests/test_main_provenance.py`
  pins the wiring in both directions, and eight mutants (the echo restored, any verdict counting as
  green, head-only equality instead of tree membership, the `pull-requests: read` scope dropped, an
  unattributed push passing, the `workflow_dispatch` arm removed, the first check-run attempt read
  instead of the newest, the step's `GH_TOKEN` renamed away) each reddened exactly one test on that
  reading - the ordering mutant reddens three of the 19 now, because the fix below added two more
  tests that read the same rule. The dispatch
  arm is there because a job that can only ever run on `push` cannot be exercised before the merge that
  first runs it, and it paid for itself on its first use: the dispatched run reported the API call
  exiting 4 with "To use GitHub CLI in a GitHub Actions workflow, set the GH_TOKEN environment
  variable" - a job's `permissions:` mints `GITHUB_TOKEN`, which `gh` does not read - and the script
  answered honestly with exit 2, "could not ask". On a push to `main` that would have been the gate's
  first and only run, so it would have been red from arrival and the argument for the dispatch arm would
  have been a lesson in why not to ship an untested job. It cannot block anything either:
  `matrix-verdict` deliberately does not wait on this job, asserted rather than assumed. The
  newest-attempt rule was the wrongest part of it, and it took a merge to find out: reading the *last
  row* of the response is not reading the newest attempt. `522b0aac` carries two records of the required
  name - pull request #493's `success`, finished 2026-10-18T01:03:44Z, and the `skipped` a
  `workflow_dispatch` run of the same workflow left on that sha, finished 01:00:39Z - and the response
  lists them oldest-last. The `skipped` is not a verdict either: `matrix-verdict` is gated on
  `github.event_name == 'pull_request'`, so dispatching this workflow on a pull request's own branch
  records an absence beside the real answer for as long as that sha exists. `main`'s first push after
  this gate landed (`fddcc76e`) was refused by that reading - a false red on trunk, the exact failure the
  sentence above was written to prevent, produced by the gate's first run on trunk. `newest_verdict`
  drops the records that judged nothing (`skipped`, a run with no conclusion) and sorts what remains by
  the time the job finished, so the response order is nothing anyone can lean on; the selftest reads nine
  such record sets, and three mutants (the filter dropped, `skipped` counted as a verdict, the sort
  reversed) each reddened two or three of the 19 tests. A dispatch still cannot manufacture a green: only
  a pull request run can report `success` for this name, and a tip whose only record is a `skipped`
  refuses as `never reported`. Replayed over `main`'s last 18 commits after the fix, the tip passes and
  seven ancestors refuse - six intermediate heads of the eleven-commit #483 rebase whose newest attempt
  was `failure`, and #487's first commit `c1d984e0`, which carries no record of the name at all. The tree
  rule is membership rather than equality with the head because a rebase can drop an empty tail commit,
  and the tip's tree is then an earlier commit's. `scripts/dependency_merge_gate.py` reads `last` of the
  same response and carries the same shape; it is named rather than fixed here, because nothing dispatches
  this workflow on a Dependabot head, so no absence can yet sit beside its verdict.

The strongest evidence that a hook tier is not decoration came from its own author: the five
files written while building this matrix came out of the editor as CRLF, and
`scripts/check_line_endings.py` named them before anything was committed. That is the
failure mode, reproduced by the tool that polices it, caught by the tool.

The tier had a blind spot of the opposite kind — a defect the tools could not be asked about,
because it produces *valid* output. A hard-wrapped paragraph whose continuation line begins with an
issue reference is, to a Markdown formatter, an ATX heading: it gets promoted, blanked around, and
the sentence is left split across a heading. Three such headings were in the tree at once, and the
linter, the changelog mirror checker, the i18n checker and the link checker all stayed silent —
nobody had asked whether a heading was *meant*. `scripts/check_doc_headings.py` asks now, as an
eighteenth hook: a heading whose text begins with two or more digits not followed by a dot. The rule
was sized against the prose before it was asserted — run over the damaged text it fires on exactly
those three lines (one had already reached `main`, two more appeared in the working copy while
landing the entries above), and over the 173 tracked pages it reports none of the digit headings this
repository actually means (`### 1-D array`,
`#### 10. メタデータの操作`, `## 1. Test matrix coverage`) — and it skips fenced blocks, because a
scan that did not would report a shell comment in `docs/ja/contributing/site-i18n.md` and demand a
fix to a line that is not Markdown at all. The mechanism was reproduced rather than inferred: a
scratch file whose continuation line began with an issue reference came out of `rumdl fmt` with that
line promoted into a heading, and the new checker named it at the line number.

Non-obvious consequence for performance claims: the Ir gate is the only reproducible
instrument in the set. What it says about #292 is below, and it is not what this paragraph
used to say: the ledger carried "~+19% in a drift-free local ratio" for the plain-string
mapping-key change, a figure taken before this instrument existed and since refuted. The A/B
that replaces it ran three variants through `.github/workflows/ir-baseline.yml` — `main`, the
PR #292 branch as written, and that branch with its byte-only pre-check switched off — twice
each, on two runner images and three host CPU models (`AMD EPYC 9V74`, `7763`, `Intel Xeon`
`8573C`). Resolving mapping keys like values costs **+1.45% to +1.62%** on the binding channel
and **0.00%** on every engine scenario the gate had at that point — which did not include a TOML
reader, and the paragraph below reports what the reader found once #307 added one. The pre-check
written to offset that cost is a
pessimization on every scenario in the set: **+0.72% to +1.03%** on the binding channel it was
meant to protect and **+0.86% to +6.66%** on the engine channel (`to_json_medium` +6.66%,
`to_toml_medium` +3.98%, `serialize_anchors` +2.03%), while switching it off returns each
engine scenario to `main`'s number within 0.003%. Two things are worth keeping separately: the
behaviour change is ~13× cheaper than the ledger claimed, and the mitigation written for it was
worse than doing nothing — a claim in a code comment, never measured, in the one layer the gate
can now see. That the same code repeated to 0.0–0.1 basis points across three CPU models is the
cross-check that the pinned environment holds; the pair's deltas were identical in both runs.

**The verdict was then applied, and the instrument paid for itself on the first use.** The
key-resolution change landed in the pre-check-off form, and the landed cost is what the gate reports:
+1.539% / +1.592% / +1.553% on the binding channel, 0.002% or less on thirteen of the fourteen engine
scenarios, and **+6.196% on `from_toml_medium`** — the reader scenario this section justifies itself
with, because that is the half the change touches and the half no number had ever covered: the bridge
resolves a candidate key through the Core chain *and* the 1.1 chain, where the object view resolves one
scalar once. A cheaper one-chain form is now a proposal the gate can price in a re-baseline instead of
an argument conducted without one.

Local wall clock cannot resolve a ~10 ns per-key delta; the gate can, and it does so
with a measured line instead of a vibe. That line is 0.5%. Within one run, three samples of each
scenario now spread at most 0.0010% on either channel — the binding channel had spread 0.853% inside a
single run before its interpreter's hash seed was pinned, which is the story of the bullet above. Across
jobs and hosts — with the glibc caps and the seed pinned, which is what makes the two comparable — the
engine channel once agreed with a baseline
generated elsewhere by at most +0.05%: all twenty-one scenarios were re-measured on WSL2 against the
artifact the refresh job had just produced for the same `main`, sixteen of them printing 0.00%
(`serialize_medium` 32,648,867 against the runner's 32,648,850), and the run's verdict was
`OK: every scenario within tolerance`. That ceiling is no longer a property of the channel: measured on
this tree, three scanner scenarios read -21.23%, -6.19% and +1.20% off the image's floors from the same
pinned compiler, and (gk) keeps the number rather than the sentence. The binding channel was the
exception, and this paragraph used
to write 0.10% for everything: the same host comparison read +0.26% on `to_python_medium` and +0.19% on
`to_python_small`, which was put down to that channel's own within-run spread. The seed above was most
of it: re-measured on the pinned compiler after the pin, that channel reads +0.15% worst against the
image's floors (`load_json_medium` +0.15%, `load_json_long_strings` +0.10%, `load_toml_medium` -0.09%,
`to_python_anchors` +0.08%), and every scenario on either channel repeats to +/-0.0001% of itself. What
the pin does *not* buy is named in `docs/dev/quality-ledger.md` (gk) with its own numbers: three scanner
scenarios still read -21.23%, -6.19% and +1.20% off the image, and both explanations that could be
tested off-image - our own SIMD dispatch and glibc's routine choice - were measured and refuted. What the old 2% was: the distance between a
WSL-generated baseline and the same code measured on a runner, +1.45% on `serialize_block_scalars` —
now explained as the ISA that glibc binds at start-up (the bullet above), which is why the caps are
pinned rather than the line widened: a tolerance sized to absorb a host's CPUID flags would be a
shrug, not a gate.

**The binding channel had to be widened before it could be gated.** At 500 iterations
`to_python_small` came out 0.83% apart between two runs of one commit — more than the tolerance it
was about to be held to — while the engine scenarios agreed to 0.0009%. The loop was too small for
the variation inside it, so the harness runs 2 000 iterations like its engine sibling, which brought
its within-run spread down to 0.076-0.24% - the rest of that band was the seed, not the iteration
count. Each scenario is also sampled three times and the *largest*
value is what gets committed, because a single draw of a distribution that wide enforced at 0.5% is a
flaky gate. The sample count travels inside `generated_by` and the enforcing run reads it from there:
max-of-3 against a single sample, or the reverse, is a different instrument wearing the same name, and
the run prints every sample and spread so the claim is auditable in the job log rather than a sentence
in a comment.

**How much of that band is left is itself a measurement, and it was taken again here.** Against the
baseline re-drawn under the seed pin the floor moves stay with the channel the pin touches -
`load_json_long_strings` -0.526%, the other five loaders -0.166% to -0.055%, `to_python_medium`
+0.051%, and every one of the thirty-five engine scenarios 0.001% or less - so the band is essentially
unused, which is not what an earlier version of this paragraph reported: in #299's measurement the same
comparison spread from −2.35% (`parse_anchors`) to +1.61% (`serialize_anchors`), and in this branch's own
first run `serialize_block_scalars` came in at +1.44% before the ISA was pinned. That last scenario is
now the cleanest single reading of the method: the same sources measured 12,288,634 on WSL2 against the
refresh job's 12,288,686 — 0.0004% between two hosts, on the very scenario whose name carried the 1.45%
figure. Drift of that kind is
not noise; each time it was a property of the method that has now been closed. Three things follow, and
all three have now happened. `.github/workflows/ir-baseline.yml` regenerates the numbers
with the steps the enforcing job uses, which is where every value in `.ci/ir-baseline.json` comes
from — forty-two of them, since the binding harness joined, since each cross-format
bridge grew its reader, since the event path got a number of its own (`stream_medium`, added to
price the scalar-ownership ruling in `docs/dev/event-stream.md` rather than argue it), since
those bridges grew a long-value twin at both ends, since the long-value shape then reached the
two dialect bridges as well, since the event path grew that same long-value twin (`stream_long_strings`,
which costs 8.25% more than `stream_medium` on the very same eight records), since the
line-delimited route grew a pair of its own (`documents_short_strings` and `documents_long_strings`,
the same eight records at value widths of 9 and 256, 95,854,968 against 328,990,934), since
the TOML loader's own route joined (`load_toml_medium` and `load_toml_long_strings`, driven
through `bench_load_toml` at 144,946,040 and 139,507,647 - 1.20x and 1.29x the tree that
`from_toml_medium` and `from_toml_long_strings` stop at), since the JSON loader's fast route
joined the same way (`load_json_medium` and `load_json_long_strings` through `bench_load_json` at
57,153,557 and 94,893,565 - 0.73x and 0.85x what `from_json_medium` and `from_json_long_strings`
pay to build the node model that route skips), and this row is shared by `load_jsonc` and
`load_json5`, because for bytes the fast reader accepts all three call exactly the same code and
what differs between them is the fallback the engine harness already prices, since the edit layer
joined with one scenario per verb (`edit_set_nested`, `edit_insert_sequence` and `edit_delete_key`) —
the third hot computation `AGENTS.md` names had no floor at all, because no scenario reached
`set_path`, `insert_path` or `delete_path`, and since the JSON producer grew its walk-and-stop pair
(`stream_json_medium` and `stream_json_stop`, the second stopped a quarter into the first): the
cancellation half that `parse_stream(..., on_event=...)` sells had no number either, and one number
alone cannot tell a slower prefix from a slower whole, which is why the pair exists. Growing the set
moves its siblings, which is why the refresh ran on this head rather than locally. Against the file
it replaced, the first dispatch refused to write over one upward move - `load_json_long_strings`
+0.76 % - measured from a binding binary whose digest that file already recorded, so the runner had
moved and the code had not; it was named in `accept_drift` and `tolerance_hint` stayed at 0.5 %. The
file the second dispatch wrote carries four floors past the line in the direction the gate does not
police (`from_json5_long_strings` -0.65 %, `from_toml_long_strings` -0.642 %, `from_json5_medium`
-0.549 %, `from_jsonc_medium` -0.525 %) and holds the other thirty-six inside ±0.466 %.
`tests/test_ir_baseline_workflow.py` now fails if this page's own count of
them disagrees with the file it describes. `ir_gate.py --update --only <scenario>`
merges into the committed file and refuses to write a baseline with a missing number, because the
first version silently reduced the scenario set to the one it had measured. And the file has to be
re-generable in full: its first committed version carried a hand-written `generated_by.note`
explaining its own provenance, which turned out to be wrong in three checkable ways (recorded in
the bullet above), and `--update` writes no prose at all. The explanation lives here; the file's
keys are pinned by `tests/test_ir_baseline_workflow.py` to exactly what the job writes.

**A bridge measured in one direction is not measured.** `to_json_medium` and `to_toml_medium`
numbered the outbound half of each cross-format bridge; nothing read anything back, and the
reading half is where a key's *meaning* is decided — a TOML or JSON key is a string by its own
grammar, and the hub AST marks "string, do not resolve" the only way it can, by quoting, so
`load_toml` has to ask what a key would become under YAML before emitting it. Per-key work on
that path is exactly what #292 adds, and the gate had no line on it: the scenario set grew
`from_json_medium` and `from_toml_medium`, and `quality_matrix.py` keeps the pairing as a rule
rather than a recollection — `ir-bridge-unidirectional` names any `to_<format>_*` /
`from_<format>_*` scenario whose twin is missing, so the next bridge added in one direction only is
reported instead of rediscovered. The readers take *committed* bytes — the writer's output for
`MEDIUM_YAML`, pinned by `crates/pyrs-yaml-core/tests/ir_fixtures.rs` — and how that came about is a
measurement that refuted its own first explanation. Rendering the input in setup, so the writer's cost
would cancel in the subtraction, put `to_json_medium` +1.52% and `to_toml_medium` +0.26% above `main`
with no engine code changed at all — past three times the tolerance on scenarios the addition never
enters. The extra
call site was blamed. Committing the bytes removed the call, and `to_json_medium` still came out
+1.489%: the movement tracks the harness binary changing shape, not how the fixture got there, and the
mechanism is recorded as unresolved rather than re-explained on the spot. `serialize_*` and `parse_*`
in that same binary stayed inside 0.042%. What is settled is the property: a harness edit is not a
neutral act for the numbers that harness already produces, so a re-baseline that follows one has to
say which movements are method and which are code.
`to_python_*` is exempt by construction: it is the language binding, not a text format, and a
probe that invented a `from_python_*` requirement would open its first hole on itself. Both
readers exit rather than reporting a number if the fixture stops parsing, and the path was
proved live rather than assumed: with one arm fed a deliberately unparseable document the
binary printed `from_json_medium iteration 0: YAML parse error: expected a JSON value at line 1
column 4 - the fixture no longer reaches the path being measured` and exited 3.

### 3. Root-cause depth

This is the dimension where the practice is genuinely systematic, and the evidence is in
the ledger: in `ROADMAP.md` the words `mutation` (17), `attribut*` (34) and
`one root cause` (9) appear alongside `wrong` (27), `retired` (5), `falsified` (3),
`refuted` (2), `superseded` (2) and `re-deriv*` (3). Recording the refuted hypotheses is
the part that distinguishes an investigation from a patch note.

Three cases where the surface symptom and the root cause were different things, each
found by measuring first:

- A round-trip drift blamed on comment placement (`#286`) was actually the container
  comment being handed to the key; the previously "stable" spelling was stable *and*
  wrong, which only an ownership assertion could show.
- The `pending_tag_insert_at` corruption (`#283`) looked like a reordering family and was
  a stale index shifted by an earlier insertion — 88 bytes minimised to 15 before the
  mechanism was stated.
- "Shape C: merge-key comment ownership plus indentation flattening" was not a comment
  bug at all. Without comments the nested shape loses data: nested `<<` merges had never
  been applied, and the comment merely made whole-node comparison fail loudly. The
  classification in the ledger was wrong and is corrected there.

The counter-example, and the reason this section is not a victory lap: the same week's key
resolution defect (`1:`, `true:`, `~:`, `.inf:` as keys) was found by a *user-facing*
parity comparison, not by the pipeline that found everything above. Depth of analysis was
high once a signal existed; the signals were the bottleneck.

### 4. Regression protection

What a closed defect leaves behind, per the discipline this repository already runs: a
minimised seed under `fuzz/seeds/**/former-crash-<hash>.seed` (84 of them), a named
regression test in the owning crate, mutation attribution showing that withdrawing the fix
reddens exactly that test, and a changelog entry in all five mirrors.

Two structural gaps, both now enforced:

- **Protection is per instance, not per class.** A seed pins the exact bytes of one crash.
  Nothing pinned "the fuzz tier has no round trip for the JSON family" or "the hook tier
  runs nowhere" — those are properties of the matrix, and the matrix was not measured.
  `tests/test_quality_matrix.py` is the first gate over the gates; it fails both when a new
  hole appears and when a registered hole is closed but left in the registry.
- **Fuzz findings depend on a human reading an artifact.** The sampler reports a
  *lower bound* of distinct signatures (documented in `fuzz.yml`), and until `#284` the
  counting itself was wrong (6 counted for 2 inputs, signature column permanently 0). The
  harness now has its own tests, `tests/test_fuzz_rounds.py`, replaying a plan file through
  a `cargo` stub.

### Registered blind spots

Each id below is emitted by `scripts/quality_matrix.py` and registered in
`.ci/quality-holes.json` with the date it was measured and the concrete state that removes
it. Deleting an entry that the measurement still reproduces fails CI; keeping one the
measurement no longer reproduces fails CI.

| hole | why it matters | exit |
| --- | --- | --- |
| *none measured* | the rule *validator* `validate_against_schema()` judged was invented in this repository, so the unit, property and fuzz tiers all asked the engine what it believes and parity had no second implementation to disagree with; the standard `Document.validate()` surface beside it was replayed against the official draft2020-12 corpus while the invented one had no outside authority at all | closed on 2026-10-21 by retiring the validator in favour of `Document.validate()`: `validate_against_schema()`, `validate_against_registered_schema()`, `TypeSpec`, `ValidateKind`, `ValidateRule` and `SchemaValidationError` are deleted, `tests/test_document_validate_suite.py` is the authority for the standard surface, and the retired validator has no row in `format_matrix()` - the `rules:` language it read is a separate surface, whose own `authority-tier` gap was closed the next day (row below) |
| *none measured* | the `rules:` schema language - the scalar-type resolution grammar behind `register_schema()`, `load_schema()`, `list_schemas()` and `YAML(schema=...)` - was invented in this repository and no specification defines it, so the unit tier (eleven `#[test]`s in `crates/pyrs-yaml-core/src/parser/yaml/schema_language.rs`), the property tier (`prop_schema_parse_no_panic`) and the fuzz tier (`fuzz/fuzz_targets/parse_schema.rs`) all asked the engine what it believes, and parity had no second implementation to disagree with; registered as `authority-tier:schema-rule-language` | closed on 2026-10-22 by `tests/test_parse_schema_spec_table.py`, which asks each `type:` name the grammar accepts a question `jsonschema` owns - is the resolved value really an instance of the JSON Schema type that name denotes - with a decoy type the same value must not satisfy, so the authority can refuse; `authority["files"]` for the row is now `["test_parse_schema_spec_table.py"]` and its verdict `yes` |
| *none measured* | the site configuration declared the Python docstring handler, its load paths and fourteen render options while not one page under any locale contained a `:::` directive — `mkdocstrings_directives()` found zero, so `docs/<locale>/api/*.md` were hand-typed signatures, nothing compared them with the generated `python/pyrs_yaml/pyrs_yaml.pyi`, and a bump of the docs toolchain could not be checked against output it never produced | satisfied: five API pages per locale now carry directives and the four-locale `--strict` build renders them (`yaml-document` 24 content blocks with 23 signatures, `node` 41 and 41, the module page 47 and 48), while the page that names no directive stays at zero — measured by `mkdocstrings_directives()` against `mkdocstrings_declared()` |
| *none measured* | the site was rendered only by a workflow that triggers on push to `main`, so a page that fails when *rendered* was found by whoever opened it: four changelog pages carried metadata the generator cannot parse through three merged pull requests, each with every gate green, because the joined text satisfied the width rule, the heading rule, the mirror checker and `rumdl` — the damage was inside every measurement the tree had | satisfied by `ci.yml`'s `docs-gates` job, which runs the page-metadata, heading, mirror, purity and i18n gates and then `scripts/build-docs.py` for all four locales under `--strict` on every pull request; `renders_the_site()` measures it off the declaring files rather than off a list kept beside them |
| *none measured* | a required check that never reports is not a green light but a deadlock: branch protection names `Test matrix (all legs)`, `ci.yml` excluded `*.md` and `docs/**` from its triggers, and PR #319 arrived with every check it produced passing and the merge refused with `Required status check "Test matrix (all legs)" is expected` — a documentation change could not merge by any route an ordinary contributor has | satisfied by the same change: `ci.yml` triggers on every pull request, a `changes` job classifies the changeset, and the verdict tolerates a skipped heavy leg only while the classifier reports `code=false` — tested in both directions in `tests/test_matrix_verdict_gate.py` |
| *none measured* | the mirror checker compares version headers, which translation leaves identical, so an entry present in three mirrors and missing from two used to pass it — `docs/en` was one `[Unreleased]` entry behind and `docs/zh` one Added and five Fixed behind, with one Changed bullet no other mirror had. Condensing `[Unreleased]` into user-facing entries, written into all five pages in the same pass, closed the divergence, and `check_changelog_mirrors.py` now asserts the per-section counts instead of printing them | satisfied: the five pages report equal counts and the probe derives nothing |
| *none measured* | the pyq jq/yq shell's accepted set was judged by the real binaries only on a host that happened to carry them: the tier skips when neither is installed, cargo captures the `SKIP:` line, and the leg still reported success — so the PR's central claim was host-dependent while reading as measured | satisfied by `ci.yml`'s rust-lint job, which installs the pinned release binaries (`v4.54.1`, `jq-1.8.2`, linux amd64), verifies their sha256, and binds them with `PYQ_DIALECT_YQ`/`PYQ_DIALECT_JQ` — under a binding, a missing or wrong reference fails the leg instead of skipping it, and `dialect_reference_bound()` measures the premise off the workflow text |
| *none measured* | counting `prop_*` functions measures how many properties exist, not whether each one asserts. The probe found seven of the 25 properties guarding their assertion inside `if let Some(parsed) = try_roundtrip(&node)`, so any sample the helper declined to return was never spoken about - measured on 2026-10-14 by drawing 4000 trees from `arb_custom_node()`: 3423 filtered, none failed to parse. That is the mechanism by which a writer emitting `null: v` for a document's `? *x` key stayed green while the tier ran (#410 fixed the writer; this row is why nothing saw it) | closed by the round-trip classifier: each difference between a tree and its re-parse is either asserted or named in an enumerated set of tolerable normalizations (`NullToken`, `PlainRequoted`, `BlockRestyled`, `EmptyContainerFlow`, `NoteRehomed`), so the whitelist is capped instead of open-ended and the probe measures an empty guard list |

This table is a measurement, not a mood: `scripts/quality_matrix.py` re-derives it on every
pytest run and `tests/test_quality_matrix.py` fails in both directions — a hole that appears
unregistered, and a registered hole the measurement no longer reproduces. It has been empty twice,
and every transition since has been measured rather than declared. The last time it emptied was
after the property tier closed; the same measurement then asked a new question — not which scenarios
the perf gate names, but which crates it can link — and the crate that serves the Python API failed
it. That is how `perf-coverage:binding-layer` was registered (#302), with the artefact that would
remove it named in its exit criterion.

It is not empty now. Retiring the rule validator on 2026-10-21 closed the hole the invented
validator had opened, and the `rules:` language beside it kept one open: the surface was measured
into the matrix rather than written off with the validator.

The question that filled it again on 2026-10-14 was about the tier itself: `property_functions()`
counts properties, and a count is satisfied by a property that never asserts.
`conditional_property_assertions()` therefore reads each property body for a guard of the shape
`if let Some(_) = try_*(..)` and reported seven of the 25 properties on the day it was registered; the
round-trip classifier has since replaced that guard, and the probe now reports 0 of the 29 properties.
Those are the measured
`property_conditional_assertions` and `property_function_count`, and the page test pins this sentence
to them — the wording written here first said 7 of 23, and #414 landed the flow-note property that made it
25. It is 27 because the count demanded the `prop_` prefix, and the two properties in
`crates/pyrs-event/src/lib.rs` are declared inside a `proptest!` block with prose names: they existed, ran
in CI, and were not measured. The rule is now the `#[test]` attribute proptest requires, which also
excludes the nested helpers (`key_order`, `mapping_key_orders`, `tag_texts`) that a naive "every `fn` in
the block" rule counted - a first attempt measured 30 that way.
The rate behind the guard was measured rather than argued: 3423 of 4000 drawn trees filtered, zero
unparseable. So the row is not "the property tier is thin"; it is the specific, checkable statement
that seven properties are silent on most of what they generate, which is the shape of the blind spot
that let a lost key read as a passing round trip.

`crates/pyrs-yaml/benches/ir_gate.rs` is that artefact: `to_python_small`, `to_python_medium` and
`to_python_anchors` — the same fixture bytes as the engine harness, measured one layer higher,
reaching the conversion through a `#[cfg(feature = "ir-gate")]` seam (`bench_to_python`) that
mirrors `safe_load`'s AST path and skips the P3 direct-load shortcut deliberately, because an
anchor-free fixture would have reported the shortcut, which is a different quantity than the one
tag- and anchor-bearing data pays for. The entry left the registry because the derived graph gained
`pyrs-yaml`, which is the registry working as designed: the deletion is enforced by the same test
that would have caught an unregistered hole. Three limits are recorded rather than smoothed over:
this binary links CPython, so it builds anywhere and runs only on Linux (measured on Windows —
`cargo bench --no-run` succeeds, then the exe dies at start-up with `0xC000021A`);
`cargo clippy --all --all-targets` does not compile a `required-features` bench target, so the new
file is checked by building it, not by the lint job; and a third was found by running it on the
enforcing image — the harness is a standalone executable that must bring CPython up itself, so the
same feature turns on `pyo3/auto-initialize`, because the first refresh run built both harnesses, read
both scenario lists, and only then failed in the measured loop with "The Python interpreter is not
initialized".

Closing a hole with a second harness is also how a third defect of a familiar class turned up: a
reader that stops early passes silently, which is exactly what #303's graph probe had done. The
scenario probe named `crates/pyrs-yaml-core/benches/ir_gate.rs` outright, so with two harnesses it
would have compared twelve of fifteen names and let `--update` write a baseline missing an entire
channel. `ir_harness_channels()` now derives the harness list from the manifests and compares it
with the harness files on disk in both directions — a declared target whose harness source vanished,
and a harness file whose crate declares no target, so that nothing ever compiles it — and one scenario
name in two harnesses is a finding too, because a single baseline number cannot say which channel
produced it. `ir_gate.py` refuses the same collision at run time, and refuses to write a baseline
from a harness it cannot list: an unreadable harness is not an empty one.

A changelog entry that is present and unreadable is the same blindness in placement form, and nothing
measured it until 401a8057: that commit's hash-fidelity entry landed above the preamble in
`CHANGELOG.md`, inside the `tags:` list of the en and zh frontmatter, and between the frontmatter and
the first heading in ja and ko — outside the changelog body in all five files, while the mirror checker
stayed green because every version header was still present and equally matched. `placement_errors` is
the rule that was missing, and it is a hard one now: no entry bullet before the first version heading,
and none whose nearest heading is a version heading rather than a section heading. The second half was
measured before it was asserted — zero violations across the five mirrors on the tree of the day, so it
costs nothing today and catches the next mis-nested paste. Entry counts per section position are
printed by that checker on every run (section order is locale-independent even where the names are
translated, so the positions are comparable), and their divergence is what `changelog-parity:entry-counts`
registers above. Asserting them was not an option in this changeset: a gate that is red because three
other mirrors are missing translations someone else wrote stops being a gate and becomes noise, so the
backfill is the exit criterion and the registry is the reminder.

Closed while this document was written, and therefore absent from the registry on
purpose: the hook tier being unwired, `cargo fmt` reaching no job, CI's clippy skipping
tests and benches, the two YAML writers having no comparison against each other
(`tests/test_route_parity.py` pins their output byte-for-byte and found the same
empty-container spelling still unfixed at the mapping-value site of `direct_dump` —
the second occurrence of the same class, the day after the hole was registered), the
line-ending policy having no enforceable form, the conflict-marker-shaped hole
above, the property tier running at 256 cases — which is where the TOML inline-table
note placement defect hid, and whose closure needed both a writer fix and a wider reject
budget, because at 20 000 cases two of the three properties were dying on the harness's own
allowance rather than on an assertion — and `perf-coverage:binding-layer`, opened by measuring what
the instrument compiles and closed by building the harness it named.

### Known residuals

`scripts/quality_matrix.py` can only register a hole it can derive, so behaviour that is
understood and deliberately open but that the measurement cannot see is not in
`.ci/quality-holes.json`. Those are registered in `.ci/known-residuals.json` instead, each with the
test that pins today's behaviour, the layer that owns the fix, and a checkable exit.
`tests/test_known_residuals.py` fails when an entry has no live repro or a marked repro has no
entry, and fixing the behaviour reddens the characterization test itself, which forces the marker
and the entry out in the same change. This is a registry of decisions, not a second measurement:
none of these is a shape the matrix is blind to.

| residual | owner | why it is still open | exit |
| --- | --- | --- | --- |
| `engine:container-inline-note-position` | hub / Rust core | a TOML `[sec] # note` keeps its text and its table but not its line: the hub has no container inline-note position, and adding one means changing the locked granit capture model ([boundaries](docs/dev/boundaries.md)) | `to_toml(from_toml("[sec] # note\nx = 1\n"))` writes the note on the header line |
| `api:set-comment-container-owner` | binding writer | `set_comment(..., standalone=False)` on a container lands on the last child's line, and the re-read gives the note that child; the edit fuzz tier therefore withholds the ownership claim (ledger (ei), #113) | the container keeps the note at a position the reader reports back, or the call refuses |

### Improvement plan

Ordered by how much defence per unit of work, with the acceptance test named — a plan item
is done when its test is green, not when the change is merged.

- **P0-A, line-ending and hook policy** (`done`): `.gitattributes` declares LF,
  `scripts/check_line_endings.py` enforces the absolute rule, `prek.toml` runs it as a
  hook, `hygiene.yml` runs the hook set in CI, and the polluted files are normalised.
  Acceptance: `tests/test_quality_matrix.py` no longer reports the hook holes, and
  `tests/test_line_endings_gate.py` proves the checker fires on an all-CRLF file.
- **P0-B, property tier elevation** (`done`): the three writer fixed points hold at 20 000
  cases — one after a real writer fix, two after the reject budget stopped being read as a
  drift — and `ci.yml` runs that count as a blocking `property-tier` job.
  Acceptance: the `property-tier` hole deleted from the registry, which the measurement gate
  enforces in the other direction too.
- **P1-C, matrix gate** (`done`): this document, the measurement script, the registry and
  the test that compares them.
- **P1-D, matrix spaces** (`done`): the route-parity table over the two YAML writers, and
  JSON/TOML round-trip fuzz targets with deterministic corpus halves that run on every
  `cargo nextest`. The JSON pair gained a third target of the same shape - `json_events_replay`
  walks a document twice, through the tree and through the event stream, and asserts the two agree
  on the verdict, on the document and on the notes - with its own seed corpus replayed by
  `crates/pyrs-json/tests/events_replay.rs`. Both closures came with a correction to how this
  document first described the gap, recorded above rather than edited out.
  Acceptance: every hole this item covered deleted from the registry, and the measurement
  agreeing with what is left.
- **P2-F, Ir baseline breadth and provenance** (`done`): the `to_json`, `to_toml` and inline-merge
  scenarios are in the gate, and so are the three `to_python_*` scenarios that close
  `perf-coverage:binding-layer` — 42 measured scenarios, 42 committed numbers, all generated rather than
  transcribed: `--update` writes every key including the sample size, `--update --only` can no longer
  write a short baseline, each scenario is the largest of three samples, and every run prints its samples,
  their spread, the binary hashes and the pinned environment. The 0.5% line is sized from that spread
  (≤0.0010% within a run on either channel, since both environmental inputs the measurement could not
  control — the ISA glibc binds from the VM's CPUID flags and the interpreter's per-process hash seed —
  are pinned rather than tolerated) and holds across jobs on the engine channel; re-measured off-image
  after the pin, the binding channel's local-to-image residual is +0.15% worst, and three scanner
  scenarios are the named exception (`docs/dev/quality-ledger.md` (gk)) rather than a licence to quote a
  ceiling that no longer covers them. Acceptance: `tests/test_ir_baseline_workflow.py` pins the committed
  keys to exactly what `--update` writes and the sample size to the default; the two jobs agree at
  `24fcba0a`; and `ir-unbaselined` / `ir-stale-baseline` stay empty.

### Reading this document

The measurement is a normal pytest module, so it runs in the tier CI already has:

```bash
uv run pytest tests/test_quality_matrix.py -v   # assertions over the matrix
python scripts/quality_matrix.py                # the measurement, no assertions
python scripts/quality_matrix.py --holes        # just the hole ids
```

To add a defence tier, edit the probe; to accept a blind spot, add a registry entry with an
exit criterion and mention its id in the table above. The gate's own discrimination was
verified the way every other fix in this repository is: four injections (a fuzz target
removed from the matrix, the hook step deleted, a registered hole renamed, a phantom hole
named in this document) each reddened the one test written to notice them, and the tree was
restored byte-identical afterwards.
