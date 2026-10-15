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

- **The writer's stance on a mapping its own reader resolves as one key is now a ruling, with both
  halves pinned.** — issue #336 asked what `to_yaml` should do when a mapping holds `!E null`
  carrying a note beside a bare `!E null`. Folding the pair on write was built and priced: from
  +31.481% to +33.655% on the five `serialize_*` scenarios with `parse_small` unmoved, against a
  floor that admits 0.50% (`docs/dev/perf.md`), and reporting it needs the same pass over every key
  of every mapping — so neither belongs on the write path, and neither is needed there. The boundary
  that resolves a key is the one a caller reaches: `doc["~"] = 1` then `doc["null"] = 2` writes the
  single line `~: 2` and re-reads with one null key, the first spelling kept and the last value
  winning (`tests/test_edit.py::TestKeyIdentityAcrossSpellings`). A `NodeMap` built directly in Rust
  emits both lines, and the reader is the part that refuses them as a duplicate or answers one entry
  shorter; that shape is pinned rather than inferred
  (`serializer::the_write_path_keeps_both_lines_of_a_pair_its_reader_resolves_as_one`). No behaviour
  changed — the additions are `#[cfg(test)]`, documentation, and an empty open-item table in
  `ROADMAP.md`.

- **The defence matrix now measures how strongly a property asserts, not only how many there are.**
  — `property_functions()` counted 25 `prop_*` functions, and that count is what the page offered as
  the property tier's coverage. `conditional_property_assertions()` reads each body for a guard of
  the shape `if let Some(_) = try_*(..)` and found 7 of the 25 asserting only inside it: those
  properties state nothing about the samples their own helper declined to return. The rate behind
  the guard was measured rather than argued — 4000 trees drawn from `arb_custom_node()`, compared
  with their re-parse, left **3423 filtered and 0 unparseable** — and that is the mechanism by which
  a writer emitting `null: v` for a document's `? *x` key, a mapping one entry shorter than the tree
  it was given, stayed green while the tier ran (PR #410 fixed the writer; this is why nothing saw
  it). The blind spot is registered as `property-tier:conditional-assert` with its checkable exit
  named, listed in `QUALITY_MATRIX.md`, and pinned in both directions: the hole disappears when the
  guard list is empty, the page test pins its own sentence to the two measured counts, and on a
  fixture tree the detector names only the guarded function — not the file that *defines* the
  helper, which a "does the text mention `try_roundtrip`" probe would have reported as coverage.

- **A Python run now refuses to speak for a binary older than the sources on disk.** —
  `python/pyrs_yaml/` holds whichever extension was built there last, so a `pytest` run after
  `jj edit` or `jj rebase` can silently test another branch's engine, and the result looks like an
  answer: this repository has been misled three times in one session that way, which is why
  AGENTS.md tells the reader to rebuild after moving the working copy. That obligation was prose.
  `tests/test_extension_freshness.py` compares the loaded native module against every file compiled
  into it — each crate's `src` tree and the Cargo manifests — and names the offenders and the
  rebuild command instead of leaving a bisect. Two decisions keep it honest rather than noisy:
  modules a crate root declares under `#[cfg(test)]` are excluded, derived from those declarations
  rather than kept as a list (editing a property test and re-running Python needs no rebuild, and a
  list would rot into excluding nothing), and an extension installed outside the working copy is
  skipped with a reason, because these sources did not build it. Verified by mutation: one stray
  file under a crate's `src` tree reddens the run and names itself, and removing it returns green.

- **The instruction gate measures the event path now, so the scalar-ownership ruling has a number to
  be argued with.** — Nothing on the gate ever entered `parse_stream`: every `parse_*` scenario
  builds an AST, and so does every bridge scenario. The maintainer ruled on 2026-10-14 that the
  neutral event should borrow its scalar text (`Cow<'a, str>`, owning only what an escape decoded),
  which is a claim about allocations — so the scenario comes first and the code change follows it:
  `stream_medium` reads `MEDIUM_YAML` through `parse_stream` two thousand times and counts the
  events it gets, baselining at 275,227,226 counted instructions against `parse_medium`'s
  321,836,091 for the same text, with three samples agreeing to the instruction (spread 0.000%).
  Taking the number before the change is what makes the gate compute the gain instead of a sentence
  asserting it. `QUALITY_MATRIX.md` section 2's scenario count moved to twenty-two with it, and the
  prose-count test added today failed first and correctly — the drift it now prevents is exactly the
  kind this change would otherwise have introduced silently.

- **The empty explicit key the parser accepts is now a documented, tested shape.** — a bare `?`, a
  `?` followed by a space and a `?` followed by a tab are documents under the pinned parser, not
  errors: each parses to the mapping whose key is the null scalar, and because the writer has no
  marker for an *empty* explicit key, the only honest emission is `~: ~`. The source's spelling is
  lost while its data is not — `? a\n: b` comes back compact as `a: b\n` for the same reason — and
  only a key that cannot round-trip without the marker keeps one (`? ?` emits the nested-explicit
  `?\n  ~: ~\n:\n  ~`). Ruled 2026-10-14: accepted rather than refused, because the pinned parser
  decides what is valid YAML and refusing would have this library reject input the `yaml-test-suite`
  accepts. `serializer::tests::a_degenerate_explicit_key_is_accepted_and_re_emits_the_null_pair` now
  asserts the exact re-emitted form of six such inputs and that each is a fixed point after one
  round, and `docs/dev/boundaries.md` records it beside the other explicit-key families.

- **A prose count in the defence matrix is now checked against the file it describes.** —
  `QUALITY_MATRIX.md` section 2 said the baseline carried "seventeen" scenarios while
  `.ci/ir-baseline.json` carried twenty-one, which is also the number the refresh job prints and the
  number this repository's own test docstring names. The keys of that file were already pinned, so a
  reader would have found the claim green — the same shape as every other defect this file records:
  a check that passes for a reason unrelated to what it assures.
  `tests/test_ir_baseline_workflow.py` now counts the scenarios in the committed file and requires
  section 2 to state the same one, so adding a scenario to `benches/ir_gate.rs` has to revisit the
  sentence or redden the run. Verified by mutation: restoring the old "seventeen" fails this test
  and nothing else, and the page was returned byte-identical afterwards. The same paragraph carried
  two measurement claims that today's cross-host recheck (PR #402's comment) proved imprecise, and
  they are corrected rather than removed: the "agrees with a baseline generated elsewhere by at most
  0.10%" figure holds for the engine channel and is wider there than stated for the binding one
  (+0.26% on `to_python_medium`, +0.19% on `to_python_small`, against that channel's own 0.076–0.24%
  within-run spread), so the bound is now named per channel; and `serialize_block_scalars`, the
  scenario whose name carried the old 1.45% drift, now reads 12,288,634 on WSL2 against the
  refresh's 12,288,686 — 0.0004% between two hosts.

- **The instruction-count gate can now say *where* a scenario's instructions go.** — every
  optimization round in `docs/dev/perf.md` began from the same throwaway probe: build the release
  benchmark unstripped, run it under callgrind twice, subtract `--setup-only`, rank the functions.
  `python scripts/ir_gate.py --profile serialize_medium` is that probe registered (`--top N` widens
  the table, and a scenario name this tree does not build is refused with the list it does). The two
  bugs the scratch versions had are now tests: reading the callgrind event file by hand summed
  8,854,254 instructions for a run whose own summary said 4,506,831 and attributed nearly all of
  them to `?`, so the tool parses `callgrind_annotate`'s table instead; and bucketing rows by a
  regex over function names once put TOML's hash cost at 12.46% because `Hasher` matched the *type
  parameter* of `IndexMap<…, BuildHasherDefault<…>>::insert*` rather than any digest work — so rows
  print exactly as named, never grouped, and `tests/test_ir_profile.py` asserts that. The gate's
  binary ships stripped, which profiles as `0x000000000006cc10'2` where it should read
  `write_plain_scalar`, so the flag keeps symbols itself; that changes the artifact's bytes and none
  of the measured code, and the pass/fail path is untouched — `--profile` prints and exits 0.

- **The `no_std` job now runs the host tests of every foundation crate.** — it built all five for
  `thumbv7em-none-eabi` but tested only `pyrs-ast` and `pyrs-schema`, so 112 host tests that exist
  under `default-features = false` (58 in `pyrs-json`, 54 in `pyrs-toml`) had never run on the path
  they claim to support, and `cargo test --no-default-features -p pyrs-toml` did not even compile: a
  recursion-limit case spawns a thread to get a 16 MiB stack, and a thread is not available without
  std. That case is now `#[cfg(feature = "std")]`, with the reason and what the `no_std` side is
  left without written where the gate can be read; the guard it tests is `alloc`-only logic and the
  same code still runs there. Building for a bare-metal target proves the code links; only the tests
  prove it works, and now they run. (`AGENTS.md`'s no_std claim is measured, not asserted.)

- **`pyrs-event`: the document event vocabulary is its own crate.** — the second core abstraction
  the architecture pillar names beside the AST. `StreamEvent` and its eleven variants moved out of
  `crates/pyrs-yaml-core/src/parser/stream.rs` into a `#![no_std]` + `alloc` crate that depends only
  on `pyrs-ast` - for `ScalarStyle` and `Tag`, so a stream and a tree built from it cannot disagree
  about what a style or a tag is - while the engine re-exports both names, so
  `pyrs_yaml_core::parser::{StreamEvent, StreamEventType}` keeps resolving and no call site changed.
  The shape is deliberately *unchanged*: it is still YAML's vocabulary, because a neutral one has to
  be designed against a second producer and none exists yet. `docs/dev/event-stream.md` records the
  ruling (option B) and the sequence that makes it true - JSON producer, TOML producer, then a
  replay property test showing events rebuild the AST the direct parsers build; this is step one of
  that roadmap, and it is why the crate's own documentation says which fields are YAML's grammar
  rather than a claim that JSON has anchors. No behaviour change, no new dependency, nothing added
  to a hot path: the thumbv7em `no_std` build of the crate is the gate that keeps the layering
  honest, and it now runs in the `no_std` job alongside the other foundation crates. — (details:
  quality-ledger (cu))

- **CI runs the suite on Python 3.15.** — The release is out (first release 2026-10-09, status
  `bugfix` in the devguide table), the `3.15` trove classifier and the `abi3t` + `abi3t-py315`
  feature pair were already committed, and the abi3 wheel installs on 3.15 - but no job had ever
  executed the suite there, so the version was claimed while untested. `test` now spans 3.8-3.15 (21
  legs → 24), and `test-freethreaded` became a two-leg matrix rather than one interpreter: `3.14t`
  is the case that cannot load the published `abi3t` wheel, `3.15t` is the floor where it applies.
  The README pair and `docs/*/installation.md` now say the same and explain why two free-threaded
  wheels exist. The 3.15 legs are measured by CI, not locally: this box's `uv` offers no final 3.15
  build yet. Three competitor packages (`ryaml`, `yaml-edit`, `yaml-rs`) are capped below 3.15: they
  ship no 3.15 wheel and their pinned pyo3 refuses the interpreter, so the install died before the
  suite could run - those comparison suites now skip on 3.15 instead. The matrix also stopped
  failing fast, after one leg's failure cancelled 22 siblings and the run reported nothing about any
  other interpreter. — (details: quality-ledger (ci))

- **A CI job can no longer run the suite over corpora it never downloaded.** — `coverage` invoked
  `pytest tests/` with no clone step, so all three conformance suites (`yaml-test-suite`,
  `toml-test`, `JSONTestSuite`) skipped there and the job still reported green.
  `tests/test_ci_corpus_gates.py` now derives the corpora from whatever `tests/*.py` references
  under
  `Reference/`, derives the jobs from whatever invokes `pytest tests/`, requires each job to
  download each corpus, and fails if a removed download step stops being noticed. The three clone
  steps were added to the coverage job. — (details: quality-ledger (cc))

- **JSON gained a conformance corpus gate, and refusals are checked by shape, not just by fact.**
  — `tests/test_json_test_suite.py` runs JSONTestSuite's 283 determinate cases through the strict
  RFC 8259 loader: 95 must parse, 188 must be refused, and the 35 the corpus itself calls
  implementation-defined are excluded by count as well as by name. Measured on the first run: zero
  disagreements, so the floor is 1.0 rather than a comfort value. A second test asks how each
  refusal happened - anything reaching the parser must answer with a typed `YamlParseError`, so a
  `RecursionError` or a panic cannot pass as a rejection. Twelve files are stored as bytes that are
  not valid UTF-8 and cannot reach a text-taking loader; that count is asserted too, and the
  boundary it reveals (no bytes entry point) is named in `docs/dev/quality-ledger.md` (cb) rather
  than papered over. CI clones the corpus beside toml-test and yaml-test-suite, and the runner
  refuses to pass on an empty corpus. — (details: quality-ledger (cb))

- **The localized-docs gate reports scripts it had never heard of.** —
  `check_cjk_localisation.py` keyed on Han, Kana and Hangul, so a Cyrillic word inside a Japanese
  sentence passed every hook in the tree. The rule is inverted: Latin, Han, Kana and Hangul are
  allowed and every other alphabetic script is reported, so Greek, Arabic, Thai and Devanagari need
  no entry to be caught. Latin stays allowed on purpose - identifiers and unit signs are
  unavoidable in technical prose, and a first cut that ignored this flagged sixty-plus `µ` in the
  benchmark tables. Proven with the control the ledger promised: `теперь` inserted into
  `docs/ja/changelog.md` reddens the gate, and withdrawing the new branch reddens 2 of 18 gate
  tests and nothing else. Closes #346. — (details: quality-ledger (bx))

- **The instruction-count baseline can no longer be loosened as a side effect of regenerating it.**
  — `ir_gate.py --update` wrote whatever it measured; nothing compared the new numbers with the
  committed ones. Measured on #342: `to_json_medium` reads 21,444,770 and 21,444,836 on two runs of
  the enforcing job - one build, agreeing inside 0.0003% - while a refresh artifact carried
  22,048,805, which is +3.84% over the committed 21,232,786, more than seven times the 0.005
  line the same file declares. That artifact was not the same code: its binary digest differs,
  and the digests are now written into the baseline and printed on every run, so two numbers
  cannot be compared without naming the bytes behind them. `--update` now refuses any scenario
  moving up past that line unless it is named with `--accept-drift <scenario>`, prints accepted
  drift to the job log, and never counts an improvement as drift. The tolerance is unchanged and no
  value moved. — (details: quality-ledger (bv), (by))

- **The baseline refresh job can be told which move is intended.** — `ir_gate.py --update` refuses
  to write a scenario that moves up past the line the baseline itself enforces unless that scenario
  is named, and `.github/workflows/ir-baseline.yml` ran the bare command: the job that exists to
  move numbers onto the enforcing image could not move them when a change was meant to pay for one.
  Measured 2026-10-12, a dispatch at the #360 head built both harnesses, measured all 21 scenarios,
  printed every sample, then exited on "`--update` would move 3 scenario(s) past the 0.5% line" with
  nothing written. The job now takes an optional `accept_drift` input - space- or newline-separated
  scenario names, one `--accept-drift` flag each - while an empty input keeps the plain `--update`,
  so refusing stays the default and the log says which of the two ran. Four tests added to
  `tests/test_ir_baseline_workflow.py`, each proven by mutation: making the flag inert reddens
  `test_the_input_is_actually_passed_to_the_guard` alone, and hard-coding `to_json_medium` into the
  loop reddens `test_an_empty_input_keeps_the_refusal` alone. No baseline value and no tolerance
  moved. Measured on its first real use the line the step prints counted flags instead of scenarios
  (three names logged as 6), so it now counts the names and a fourth test pins it. — (details:
  quality-ledger (cj))

- **The quality ledger's lettered sections are in order, and a gate keeps them there.** — `### (xy)`
  sections grow by insertion at whichever anchor a branch used, and two had ended up behind later
  letters: `(bh)` after `(bi)`, `(ch)` after `(cj)`. Nothing was red, because
  `(details: quality-ledger (ch))` still resolves; the file simply no longer matched its own
  numbering. Both moved as whole blocks (98 and 46 lines) with the file's line multiset asserted
  unchanged, so a reorder cannot silently drop or duplicate a paragraph.
  `tests/test_ledger_section_order.py` requires the headings to run strictly increasing and no
  letter to repeat, each proven by mutation - renaming a section to a letter already used reddens
  both tests, since a repeat is also a step backwards. Gaps are not reported: `bs`/`bu`/`cd` were
  claimed and closed elsewhere, while `ak`-`an` exist here as the older bold lead-in form, not as
  headings. — (details: quality-ledger (ck))

- **The CI interpreter matrix is planned from what the package declares, and its two hidden gates
  are roles now.** — `ci.yml` wrote `os x python-version`, 24 cells. Measured over 144 legs, a leg
  costs 138 s on ubuntu, 186 s on macOS and 251 s on Windows, and 76% of that is the same ~2 500
  assertions re-run per interpreter: 78.2 minutes of runner time per pull request. Over the last 30
  runs no failure was isolated to one interpreter version or to one OS. `scripts/ci_matrix_plan.py`
  now builds the grid from `pyproject.toml`'s classifiers: every declared version runs the suite on
  ubuntu, 3.8 / 3.12 / 3.15 run it on all three OSes, and the other ten cells install that
  platform's real abi3 wheel and round-trip one document through YAML, JSON, JSONC, JSON5 and TOML
  (`scripts/wheel_smoke.py`, 22-68 s). The full cross product runs with the suite every Sunday.
  Measured: 78.2 -> ~49 min per pull request (-37%), 24 -> 14 suite legs, no cell unexercised. The
  reason this is a restructure and not a cut: `--cov-fail-under=78` and the compliance summary were
  selected by `matrix.os == 'ubuntu-latest' && matrix.python-version == '3.12'`, so deleting that
  cell switched both gates off *green*. They are leg roles now, and the planner job validates the
  plan before any leg is paid for - shown by four doctored-plan controls and a test requiring that a
  new `3.99` classifier grows a leg. — (details: quality-ledger (cl))

- **YAML comments, anchors and tags survive a round trip, gated on every seed.** —
  `crates/pyrs-yaml-core/tests/note_survival.rs` replays the committed seed corpus on every
  `cargo nextest run` and requires every note the reader recorded to appear in the emitted text. The
  round-trip tier's oracle is text idempotence, which any stable-but-short-a-note document passes:
  five silent note losses stayed green behind it. — (details: quality-ledger — Note survival, (w),
  (ac))

- **Nested and aliased merge keys are applied instead of dropped.** — `<<: {<<: {x: 1}}` kept both
  `<<` levels as data (`{'<<': {'<<': {'x': 1}}}`) where PyYAML and ruamel read `{'x': 1}`, and
  `<<: {<<: {x: 1, y: 1}, y: 2}` lost `x` outright, as did the block form `- <<:` under `<<:`. The
  collector no longer skips a source key the target "already owns" by comparing whole nodes.
  — (details: quality-ledger (ae), (af), (ag), (o), (q), (s))

- **The instruction-count gate covers every format bridge, in both directions.** — `from_jsonc`,
  `from_json5`, `to_jsonc_text`, `to_json5_text`, `to_python_small/medium/anchors` and the JSON/TOML
  readers now have baselines in `.ci/ir-baseline.json`. On one hub AST the three writers measure
  21,232,786 / 22,176,851 / 22,974,851 instructions: emitting comments costs 4.4% and the JSON5
  spelling 8.2% over strict JSON. — (details: quality-ledger (am), (ap), (aq), (as), (ax))

- **The quality defence measures itself, and gaps surface as findings.** —
  `scripts/quality_matrix.py` re-derives `QUALITY_MATRIX.md` from the files that declare the defence
  (`.github/workflows/*.yml`, `prek.toml`, `fuzz/Cargo.toml`, `scripts/check_*.py`, the Ir bench,
  `.ci/ir-baseline.json`) instead of transcribing it, and every hole it reports is registered with
  its exit named in both directions and a test that fires it on an injected tree. — (details:
  quality-ledger (ah), (ai), (aj), (ak), (an), (ao))

- **A changeset that moves the product must move the release notes.** —
  `scripts/check_changelog_coupling.py` applies two rules to the pull request's file list: a diff
  touching `crates/`, `python/pyrs_yaml/`, `fuzz/`, `scripts/`, `tests/` or a shipped manifest must
  touch a changelog, and touching one of the five mirrors means touching all five. Added after a
  feature shipped with no release note anywhere. — (details: quality-ledger (al), (ar))

- **Timing floors sample their phases adjacently and print the evidence.** — `tests/timing.py` is
  the single sampler: candidate and reference are measured inside each block, the verdict is the
  block minima with a floor of two pair wins, and the failure message quotes every pair so a red
  says which side moved. A `macos-latest` run reporting `139.4us vs 359.6us (2.58x)` while three of
  five blocks sat near 3x their own floor is the case this settles; the cross-library floors keep 5x
  and top-3 (measured 38x-280x against PyYAML, 71x-77x against ruamel). — (details: quality-ledger
  (aw), (ay))

- **Docs prose fits 100 display columns; formatter-split headings are rejected.** — `ROADMAP.md` had
  a 21,850-character line and `docs/ja/changelog.md` one of 884. `scripts/check_doc_wrapping.py`
  measures display columns (a fullwidth glyph counts as two) and carries `--fix`;
  `check_doc_headings.py` rejects the heading a formatter invents when a wrapped continuation line
  begins with an issue reference. — (details: quality-ledger (au), (av))

- **Each engine has a fuzz target, with a weekly run and a blocking PR tier.** — four libFuzzer
  targets: `parse_yaml` (single and stream), `yaml_roundtrip` (parse, serialize, re-parse,
  serialize-idempotence), `parse_json` (three dialects crossed with three writers, each output
  re-parsed) and `parse_toml` (1.0/1.1 plus writer re-parse), seeded from `fuzz/seeds/`. Only
  targets and seeds are tracked; corpora never enter the repository. — (details: quality-ledger
  (aj))

- **`pyrs-ast`, `pyrs-schema`, `pyrs-json` and `pyrs-toml` are `no_std`-capable.** — the four crates
  build against `alloc` alone: `indexmap` and `thiserror` lose their default `std` features, and a
  new opt-in `std` feature re-enables `std::error::Error` impls and the `RandomState` hasher. `std`
  stays on by default, so every existing consumer keeps the same node-map type.
  — (details: boundaries — Known Engine Boundaries)

- **`pyq` ships as a prebuilt binary with every release.** — a `pyq` job in `publish.yml` builds the
  native CLI for six platforms and attaches the archives to the GitHub Release, so a standalone
  binary no longer needs a Rust toolchain. The Linux legs compile inside manylinux2014 containers
  via `cross`, pinning a glibc 2.17 floor. — (details: perf — Leaderboard & Performance Status)

- **The committed type stub is checked against the built extension.** —
  `scripts/check_stub_drift.py` regenerates `python/pyrs_yaml/pyrs_yaml.pyi` through the declared
  route and fails on any difference; the job runs on `windows-latest` because maturin's
  introspection route cannot load the extension in the Linux containers. Before it, CI only asserted
  the file exists and is tracked, so a binding signature change could leave the public typing
  contract behind. — (details: quality-ledger (ap))

- **The schema language can name a container.** — `type: map` and `type: seq` assert the shape of a
  node without saying what it holds, and `map` / `seq` are accepted as member types too, so
  `mapping_of: seq` says "every value of this mapping is a sequence" - a claim that had no spelling,
  since the `[*]` wildcard reaches sequence indices and not mapping keys. `mapping`, `object`,
  `sequence`, `array` and `list` are accepted spellings, because a schema is typed by a person
  rather than emitted by a tool. A container type with no `path` is refused where it is written:
  with no node named it would select nothing and check nothing. — (details: quality-ledger (bm))

- **The instruction files are measured, not trusted.** — `scripts/check_hole_claims.py` asserts that
  every quality-hole id named in `AGENTS.md` or `CLAUDE.md` is one the matrix measures now. The file
  that motivated it is `AGENTS.md` itself, which for a whole milestone described `mkdocstrings` as
  rendering nothing - a present-tense instruction naming a gap #321 had closed, still pointing an
  agent the wrong way. Nothing was wrong with the measurement: the matrix compares
  `.ci/quality-holes.json` with `QUALITY_MATRIX.md` in both directions and stopped at the derived
  document. A token counts as a hole id only when its kind is one the probe can emit - read off
  `quality_matrix.py` rather than listed beside it - so `line:column` in prose is not a claim, and a
  vocabulary the checker cannot fully read exits 2 instead of passing. Dated records keep their ids:
  history may name a closed hole, an instruction file is present tense. — (details: quality-ledger
  (bn))

- **Published signatures are compared with the shipped object, not with the page.** —
  `scripts/check_doc_signatures.py` reads every fenced signature on the 28 API pages and asks the
  running package whether it accepts those parameters: 238 blocks, 279 names. A wrong parameter name
  is valid HTML, so the site builds green over it, and a checker that read only the generated `.pyi`
  would have blamed the documentation for `read_markdown(path=...)` - the one case where the
  documentation was right. Legitimate abbreviations (a required prefix, an optional tail dropped)
  pass; a name, order or required parameter that no declaration has fails.
  `tests/test_stub_runtime_parity.py` pins the layer below it: the stub's parameter names must equal
  the object's for all 40 module functions, which is the invariant whose absence caused both
  defects. — (details: quality-ledger (bo))

- **The standard `!!` tags now decide the loaded type.** — `!!str`, `!!int`, `!!float`, `!!bool` and
  `!!binary` are read from the tag's own definition rather than from the document's implicit schema,
  in both spellings a document uses: the shorthand, and the verbatim `!<tag:yaml.org,2002:str>`.
  Measured against PyYAML and ruamel, which agree with each other on every case: `!!str 1.20` is the
  string `"1.20"` (it loaded as the float `1.2`, changing the type *and* dropping the trailing
  zero), `!!float 1` is `1.0`, `!!bool yes` is `True`, `!!binary aGk=` is `b'hi'`, and an `!!int`
  past i64 is still an integer. A text that is not the tagged type now raises `YamlTypeError`
  instead of answering with a string; an unknown application tag stays lenient, which is what lets
  the plugin system keep working, and that choice is stated in a test rather than left to luck. —
  (details: quality-ledger (bq))

- **The interpreter floor is now checked where it actually breaks: at call time, not just at
  import.** — `tests/test_scripts_import_on_supported_python.py` imported every `scripts/*.py` on
  every leg, and its own docstring named the half it could not see: an API younger than the floor
  raises when the *function runs*, so a module that imports cleanly on 3.8 can still die there. That
  half cost a full matrix run. A static scan over `scripts/`, `tests/` and the shipped `python/`
  package now holds the catalogue - `write_text`/`read_text` with the `newline` keyword,
  `removeprefix`, `zip(strict=)`, `functools.cache`, `int.bit_count`, `datetime.UTC`, `StrEnum`,
  `ExceptionGroup`, `typing.override` - with string literals and comments stripped first, so that
  naming an API is not the same as calling it, and every pattern proven to fire by one fixture
  compared for set equality, so a dead regex cannot report a clean tree. Two candidate entries were
  rejected as written, both measured: `\.walk\(\)` returned seven findings that were this project's
  own `walk()` verb, and `@override\b` found the `typing_extensions.override` shim this repository
  standardises on. A pattern that collides with the code it polices is worse than no pattern,
  because the fix is an exemption and the exemptions outlive the API. — (details: quality-ledger
  (cn))

- **The JSON writers are read back by a reference implementation, on every valid document.** — the
  corpus gated which documents the parsers *accept*; nothing gated what the writers *emit*, so
  `to_json`, `to_jsonc` and `to_json5` were checked only against this repository's own reader. All
  95 valid documents now re-read through stdlib `json` (190 writer re-reads) and `to_json5` through
  `pyjson5`. Equality is type-strict, because `[1] == [1.0]` and `0 == False` are both `True` in
  Python: a value-only comparison accepts the exact defect class #358 fixed. Three mutation controls
  carry the evidence. Writing an integer as `{i}.0` reddens 38 of 190 re-reads and 4 of 95 JSON5
  re-reads, naming input, output and emitted text; reversing the writer's pair loop reddens six
  documents, reported by the value comparison, which walks keys in document order; and emitting
  every member twice - still valid JSON, and equal as dicts because the repeats collapse - is caught
  only by the pair lists, 18 of 190. The first control passed when it was tried, and that was the
  finding: `write_plain` returns a JSON-spelled number verbatim through its fidelity pass, so a
  corpus that arrives via `from_json` never reaches the number-normalising arm the substitution
  lives in - making it observable took that pass switched off as well. The YAML spellings that arm
  exists for (`0x1F` as `31`, `!!str 7` as `"7"`) are pinned on the Rust side by
  `exotic_scalars_quote_or_normalize`. What the gate cannot see is written in its docstring rather
  than in a footnote: the corpus is comment-free and anchor-free, so #359's comment slots stay the
  business of `tests/test_json.py`. — (details: quality-ledger (co))

#### Changed

- **The stub generator moved to maturin 1.15.0, deliberately and measured.** — the pin exists
  because `python/pyrs_yaml/pyrs_yaml.pyi` is generated by one specific version and
  `scripts/check_stub_drift.py` carries the rules that reconcile that output with this repository's
  declarations, so moving it has to be its own change and `tests/test_docs_config_gate.py` reddens a
  silent move. Measured here: 1.15.0's generated stub is byte-identical to 1.14.1's - 608 lines, and
  the drift checker reports the committed file already matches the regeneration route - so no rule
  and no expected count changed. The only thing that moved is where the generator writes: `--out`
  now receives `target/stubs/pyrs_yaml/pyrs_yaml.pyi` instead of `target/stubs/pyrs_yaml.pyi`, which
  is the `GENERATED` path. Adopted for the upstream `abi3t` `PyModExport` entrypoint-check fix,
  which is the build path this project's `abi3t` and cp314t wheels use.

- **The round-trip tier names every difference instead of hiding behind a filter.** — seven of the
  25 properties used to assert only inside `if let Some(parsed) = try_roundtrip(&node)`, so the
  samples their own helper declined were never spoken about; measured, the filter discarded 3423 of
  4000 drawn trees. `try_roundtrip` is gone: every cycle is classified against an enumerated list of
  tolerable normalizations - `NullToken`, `PlainRequoted`, `BlockRestyled`, `EmptyContainerFlow`,
  `NoteRehomed`, `NoteForcesBlockLayout`, `InlineContainerFlow` - and a difference the list cannot
  name fails the property. That is what surfaced the writer and reader defects behind #418
  through #422 and #424, each pinned by its own test, and it closes
  `property-tier:conditional-assert`: the probe measures an empty guard list,
  `.ci/quality-holes.json` is empty, and the row in `QUALITY_MATRIX.md` stands as closed. The two
  order properties were re-claimed rather than re-filtered - a key compares as kind-tagged entries
  instead of serialized text, and mapping order compares per position, because whether a container
  reads back as flow is the reader's normalization and not an order fact.

- **The tag grammar left the PyO3 binding, and its tests no longer need libpython.** — `!!int`,
  `!!float` and `!!binary` lexical reading lived in `crates/pyrs-yaml/src/py/tags.rs`, so its six
  tests linked libpython although every function they call takes `&str` and returns a Rust value:
  Cargo's lib dependencies apply to a test target too, and `pyo3` is declared without
  `extension-module`. The grammar is now `pyrs_yaml_core::tagged`, which took the Python-free lane
  from 526 to 532 passing tests while the workspace total held at 604. The binding keeps only what
  turns a value into a `Py<PyAny>`.

- **`granit-parser` 1.3 → 2.0.** — the major bump needed no source change (neither removed public
  helper, `found_tabs` nor `has_valid_yaml_ws`, is referenced anywhere in `crates/` or `python/`),
  `cargo nextest run --workspace --exclude pyrs-yaml` reports 579 passed,
  `uv run pytest tests/ -k "not numpy"` reports 2678 passed / 10 skipped - which is the part that
  matters, because it is the comment, anchor and cross-library compliance tier that a compile cannot
  vouch for - and `scripts/ir_gate.py` on the gate's rustc 1.97.1 finishes "OK: every scenario
  within tolerance of the committed baseline" (`to_python_medium` +0.26%, `to_python_small` +0.13%,
  `from_json_medium` −0.00%). `cargo tree` confirms the graph really carries v2.0.0 while measuring.

- **Two mapping keys are the same key only where the reader says so.** — the rule that decides
  whether two keys collide in text, `keys_collide_in_text`, compared a scalar's raw value and
  ignored the rest: it called `!a k:` and `!b k:` one key, and `null:` and `'null':` one key, while
  the reader that gets the emitted text back keeps both entries in each pair (a tag is part of the
  line, and a plain `null` resolves to a null key where the quoted spelling stays a string). The
  rule is the model the writer's fold is about to be built on, so an over-eager model would have
  folded away two entries the document can hold. It now compares tags, and treats a plain null
  spelling and its quoted twin as different keys — plain spellings that resolve to a boolean or a
  number are not exempt, because there the reader really does compare text. The pinned table holding
  the rule beside the writer and the reader it models grows from 15 classes to 18.

- **Schema validation asks the tag before the text.** — a validate rule checked only a scalar's
  *spelling*, so a document writing `v: !!str 1` satisfied a rule demanding an int while the loader
  read that value as the string `"1"` and the JSON and TOML writers emitted it as one - one AST, two
  meanings (#340). The check now resolves through the tag when the tag is a standard one (`!!str`,
  `!!int`, `!!bool`, `!!float`, `!!null`, and the verbatim `!<tag:yaml.org,2002:...>` form), using
  the grammar the spoke writers already use - `standard_tag_kind` plus `TagKind::resolve` - so there
  is one source of that grammar rather than two to keep agreeing. A tag the text cannot satisfy is
  refused instead of re-typed: `!!int hello` no longer validates as a string. A tag the spec says
  nothing about (`!x 1`) keeps the ordinary implicit resolution, so it is still a number.
  `!!int 0b1010` now validates as the integer 10 it states, on the YAML 1.1 lexeme set the loader
  already implements.

- **A `CustomNode` is 144 bytes, not 208.** — half a node used to be its `NodeMeta`: 128 of 208
  bytes, five `Option` slots, and that size sits on the hot path of every reader — written when a
  node is built, copied whenever a node moves, and checked field by field when the tree is dropped
  (`docs/dev/perf.md`: `memcpy` is 7.88% and teardown 4.58% of `from_json_medium`). Two steps ship
  as one because neither is free on its own. Every crate outside `pyrs-ast` now reaches the slots
  through the type's own accessors — `comment()` / `set_comment()` / `take_comment()`, the same trio
  for the anchor, the tag and the source range, `decor_mut()` for the boxed leading-note bundle,
  `is_identity_clean()` where the serializer used to ask a pattern whether a node carries none of
  them, and `with_comment()` / `with_tag()` / `scanned()` where a struct literal named every field —
  and the two slots only an anchored or tagged source ever fills now share one pointer. Measured
  paired on the compiler the gate pins (rustc 1.97.1): `from_json_medium` −4.8%, `serialize_anchors`
  −4.0%, `parse_inline_merge` −3.5%, `parse_medium` −1.5%, `from_toml_medium` +0.7%, and nothing
  outside the 0.50% tolerance. Which fields may move is the finding: boxing the source range as well
  — which the YAML adapter records on *every* node it reads — cost `parse_medium +5.9%` to buy
  `from_json_medium −7.4%`, so a per-node-written slot stays flat and only `{anchor, tag}` goes
  behind the box. `node_layout_is_part_of_the_hot_path_contract` pins 72 and 64 for the metadata,
  and both sizes the node can have - 144 with the default `std` feature, 128 with `no_std`, where
  the node map's own hasher differs - because a node this shape is a measured claim, not a detail.

- **TOML binary integers keep their spelling.** — `b = 0b1010` came back as `b = 10`. The spoke
  canonicalised it on purpose: YAML Core has no `0b` lexeme, so an untagged plain `0b1010` reaches
  the hub and re-reads as a *string*, and design rule D1 ("only spellings YAML Core also accepts get
  preserved") dropped the spelling rather than corrupt the value. That argument expired when the
  bridges learned to read tags (`(bu)`, and the ordering #340 set for it): the projection now keeps
  the text and states the type, so `from_toml("b = 0b1010")` is the hub scalar `!!int 0b1010` - the
  loader reads it as `10`, `to_toml` writes `0b1010` back, and the JSON writers project `10`. One
  document, one type, in every format that sees it, and the TOML text settles in one round. What is
  still not faithful is what TOML itself cannot spell: `0b1101_0110` loses its separators, and a
  hand-built *untagged* `0b1010` still re-reads as a string - the tag is what carries the type. The
  undecided `!!binary` mapping is untouched and stays open on that issue. — (details:
  quality-ledger (cp))

- **The coverage gate stopped duplicating a whole suite run.** — `coverage` created its own
  virtualenv, downloaded the ubuntu wheel, cloned all three corpora and ran `pytest tests/` a second
  time on the interpreter `test` already covered (ubuntu/3.12). It is now one flag on that leg, with
  the same `--cov-fail-under=78`, and the three clone steps moved into a shared composite action
  (`.github/actions/fetch-corpora`) used by both whole-suite jobs. That shape can hide the very gap
  the clone gate exists to catch - a job fetching everything looks like one fetching nothing - so
  `tests/test_ci_corpus_gates.py` now follows a local `uses:` into the action's own steps, and a new
  test pins both edges: resolved names must cover every corpus, an unresolvable path must report
  every corpus missing. Renaming the action directory reds three tests, which is what proves the
  resolution is load-bearing. The fan-in `needs:` lost `coverage`, as the gate comparing it against
  every job in the file requires. — (details: quality-ledger (ci))

- **A `bytes` value is written as the tag it means.** — the built-in plugin dumped `!binary`, a
  *local* tag. The standard name is `!!binary`, so every document this library wrote with binary
  data was unreadable to the libraries whose tag it had borrowed - and an empty `bytes` wrote
  `!binary` with nothing after it, which came back as `None` and raised inside the plugin. Dumps
  now emit `!!binary`, empty payload included, and PyYAML reads what we write, asserted. The local
  spelling stays registered for loading: dropping it would turn a spelling fix into data loss for
  documents already on disk. — (details: quality-ledger (bq))

- **A validate rule that names a path asserts the node it names.** — measured before the change:
  `$.config` with `mapping_of: str` passed when `config` was a scalar or a sequence, `$.port` with
  `type: int` passed when `port` was a mapping, and `sequence_of: int` passed over `[[1, 2]]`. Every
  check sat inside an `if let` for the one node kind it could describe, so a node of any other kind
  was walked past - a validator answering "valid" about a document it never looked at. Members are
  asserted the same way: a nested sequence is not an `int`. A pathless rule keeps its old, different
  meaning, because it cannot name a node and so selects the nodes it can describe - pathless
  `type: str` still means every scalar is a string, whatever else the document holds. A rule about
  an aliased node is still not decided: an alias carries a name rather than a shape, and the
  validator holds no anchor table. — (details: quality-ledger (bm))

- **One `[*]` names one element, not a whole subtree.** — `$.rows[*]` also matched `$.rows[0].a`,
  because the matcher compared a prefix and a suffix and required only that something sat between
  them. It stayed invisible while rules ignored the shapes they were not written for; once they
  assert one, every nested value under a matched element became a false violation. `$.rows[*].a`
  still reaches the member it names. — (details: quality-ledger (bm))

- **A validate rule carries one check.** — a rule that spelled `type` and `mapping_of` ran only the
  check written last and never mentioned the other, because every parser arm assigned to the same
  field. Two checks in one rule now fail the schema parse; `required` is orthogonal and still
  combines with any check. — (details: quality-ledger (bm))

- **The strict JSON writers refuse a non-finite float instead of respelling it.** — RFC 8259 has no
  literal for infinity or NaN, and `to_json` / `to_jsonc` answered by quoting the hub's text:
  `{"b": ".inf"}`, which `load_json` hands back as a string. A number became a string on the way
  out, indistinguishable from a document whose value really was that text - the same class of
  loss #312 fixed on the way in. The alternatives were measured rather than weighed: `null` (what
  `JSON.stringify` emits) is a different number, and a bare `Infinity` is text this library's own
  strict reader rejects on purpose, being stricter than CPython's `json.loads` here by design. So
  the writers now raise `YamlSerializeError` carrying the stable reason key
  `json-cannot-represent-non-finite`, and `to_json5` - the dialect that owns the token - stays a
  round trip. The rejection also stopped masquerading as an internal fault:
  `SerializeError::UnsupportedValue` is a new variant for "the format has no room for this value",
  which is what the JSON and TOML writers had been reporting as `internal-error` for keys, aliases,
  nulls and timestamps alike.

- **`pyq validate` accepts `--input` and honours the format actually read.** — it hardcoded the YAML
  parser, so every non-YAML config it was pointed at (`pyproject.toml`, `package.json`) was rejected
  outright. It now takes `--input auto|yaml|json|jsonc|json5|toml` and routes through the shared
  loader.

- **PR-tier fuzzing is blocking, so a crash cannot be merged past.** — the step-level
  `continue-on-error` was a ratchet kept only while `main` carried drift this tier correctly
  flagged. Re-checked against this tree: all four targets replay the committed seed corpus clean
  (5/59/6/5 seeds, nightly-2026-08-15, cargo-fuzz 0.13.2). — (details: quality-ledger (aa))

- **The docs toolchain is at the current stable releases, and the stub generator is deliberately
  held back.** — `zensical` 0.0.56 to 0.0.69 and `mkdocstrings-python` 1.11.1 to 2.0.9
  (`mkdocstrings` 1.0.6, `griffelib` 2.2.0), both requiring Python 3.11, while `maturin` stays at
  1.14.1 because the committed `.pyi` and the drift checker's rules describe that generator's
  output. All fourteen configured handler options were checked against the installed
  `PythonOptions`, and an unrecognised one is now caught by a test because the build ignores it in
  silence. — (details: quality-ledger (ba))

- **The API reference is generated from the library's own docstrings and type stub.** —
  `docs/<locale>/api/{reference,yaml-document,yaml-instance,node,merged-view}.md` now carry
  `mkdocstrings` directives, so a signature on the site comes out of the code rather than being
  retyped. Rendered and verified in all four locales: 23 signatures under `YamlDocument`, 41 under
  `Node`, 48 on the module page, while the page that names no directive stays at zero.
  `exceptions.md` remains hand-written because the generated stub declares only four classes and
  none of the exception types. — (details: quality-ledger (be))

#### Fixed

- **A quote that opens no node cannot hide a comment.** — #421 taught the anchor-name scan to ignore
  a `#` inside quotes, and it trusted any quote after a blank to open such a region. A plain key's
  own apostrophe (`a '`) therefore opened a phantom string whose close sat inside the comment, the
  real comment opener went unseen, and a `&` written in that comment became the node's anchor name:
  the value came back as `&*~=9VjQq=8`, an alias spelling no anchor can hold. A quoted node begins a
  token, so the scan now asks which token precedes the quote — a boundary, or the anchor or tag
  property the node follows.

- **A nested container's note is written above its own line, never below its body.** — the writer's
  last resort for a container's inline note was a bare comment line at that container's indent, and
  the reader spans a node only through the break that ends the node's *own* line: deeper than column
  0 the note fell outside the span, was filed with the line that follows it, and the second emission
  lost it. The round-trip tier minimized to `- - |-` with a body under it, then to the same loss
  with no block scalar in sight, so the rule is stated by indent. Column 0 keeps the old placement,
  where the note line is the document's tail and the reader does report the note back to the
  container.

- **A key's note rides only its own pair's line, never a nested header.** — #419 taught the writer
  to place a key's note on a block scalar's header line when the value is a block scalar, but the
  position it consulted was "the last block header written", wherever in the document that was. A
  pair whose value is a *container* writes its own pairs after that header, so an outer key's note
  landed inside a nested pair's header line: `A: >` + `# a` + `-5` splits the header, merges the
  two comments, and leaves a body no block declares - the emitted text no longer parsed at all. The
  property tier's 20k-case run found it (`misplaced bracket`); the slot is now taken only while the
  header line and the pair line are the same line.

- **An anchor keeps its own name past a `#` that only looks like a comment.** — the reader recovers
  an anchor's display name by scanning left from the node the parser pointed at, refusing every `&`
  that sits after a comment-opening `#` on that line. The guard also treated a `#` *inside a quoted
  scalar* as a comment, on the belief that no anchor can follow a quoted value on the same line -
  true of a block pair, false inside a flow collection: `{a: &x "x # y", b: &y 2}` re-read `b` as
  `&x`, so two nodes came back sharing one name and every `*y` alias pointed at nothing. The scan
  now tracks quoting - `''` inside single quotes, a backslash inside double, and a quote counting
  only where a scalar may begin, because granit's name grammar lets an anchor name itself hold a
  `"`. The class it belongs to is the round-trip tier's: 11 anchor-name changes per 4000 generated
  trees, and this is the shape the reporter minimises to.

- **The compact `- key: value` form prints the first key's own notes.** — `write_mapping_pair`
  writes a simple key's leading notes above its pair, and the compact sequence loop copied that rule
  for every pair *except* the first, because the first shares the dash line and has no line of its
  own: `- null: " "` came back with the key's `# 2` gone. Those notes now go above the item's line.
  Found by shrinking the property tier's residual note losses down to the smallest tree that still
  loses one; with that case closed, the measured losses over 4000 generated trees are one per 4000 —
  a standalone note on a block value, named in the ledger rather than absorbed.

- **A block mapping key's inline note survives a value that holds the pair's slot.** —
  `key: value  # one` has one trailing slot, and when both the key and the value claimed it the
  writer gave the slot to the value and the key's note disappeared: `? a # kn` + `: b # vv` emitted
  `a: b  # vv`. The note now spends a line break, the way the flow path has been spending one. That
  placement was measured before it was chosen: a comment line written directly above a key that is
  not the mapping's first belongs to **that key**, so `a: 1` + `# K` + `b: 2  # V` re-reads with `K`
  on `b` and the note keeps its owner; only a first pair hands its key's note up to the container,
  and the text survives either way. The compact `- a: 1` form and the block-scalar body carried the
  same hole and close with it. Over 4000 generated trees, notes that never reached the emitted text
  fell from 907 to 257, and the block-key class of them from 707 to 34.

- **A block scalar no longer loses its value when its header moves line or takes a note.** —
  an indentation indicator counts columns from the *parent node*, and PyYAML, ruamel and this
  engine's own reader all agree on that (measured over `a: |2`, `- |2` and `? k` spellings). The
  writer counted from the header line instead — the same column, until a value is pushed onto a
  line of its own, which a leading note does — and then the body sat two columns deeper than the
  number on the header claimed, so every reader took those columns as content: the value `" "`
  came back `"   "`. The second shape is the same class: a simple key's inline note has no
  spelling before its `:`, so the writer rides it at the end of the pair line, and for a block
  value that end is the last *body* line, where `#` is content — a noted key over `|-` emitted
  `abc  # n` and read back `"abc  # n"`, three bytes the tree never held. Both came out of
  attributing the property tier's filtered mismatches per node position: 434 of 4000 drawn trees
  carried the shift, and after the two fixes none do. The note now rides the header line, where a
  reader hands it to the value.

- **A block scalar no longer invents the trailing break its header denotes.** —
  `key: |-\n  line1\n  line2\n` reads as `line1\nline2`; ask that node for clip
  (`set_chomping("clip")`, or hand the writer a tree whose `Clip` value ends in no break) and it
  wrote `|`, which *denotes* one trailing break, so the document came back a byte longer than the
  tree that produced it. `effective_chomping` now corrects only the claims no header can spell -
  `Clip` and `Keep` for a value without a trailing break become `Strip` - and leaves a `Strip` claim
  alone, because `-` is expressible and asking for it is the edit the caller means;
  `test_set_chomping_strip` pins that other direction. Found through the property tier's mismatch
  taxonomy over 4000 drawn trees: 255 of them, and every one the same shape - a value of length N
  read back as length N+1. The pinned block fixtures all ended their values in `\n`, which is how
  the rule stayed uncovered across five writer revisions of byte-exact assertions; those five now
  carry `|-`/`|-4`, which is the spelling the body actually carries.

- **The shipped typing contract describes the class the package offers.** —
  `python/pyrs_yaml/pyrs_yaml.pyi` is generated from the extension, while
  `python/pyrs_yaml/__init__.py` attaches `node`, `find`, `walk`, `scalars`, `merged`, `set`,
  `set_many`, `insert`, `append`, `delete`, `rename` and `sort_keys` to `YamlDocument` after import,
  so those twelve members lived in the runtime and in no artifact a checker reads: measured with
  mypy 2.4.0 against the installed package, `document.node` named a member the stub never declared.
  The first domino sat one layer above it — the public wrappers of `parse` and `parse_file` were
  annotated `-> Any`, and `parse_all_docs -> list[Any]`, erasing the binding's own `YamlDocument`,
  so no checker reached the class through the entry point users call first. Each half is repaired
  where it lives: honest return types on the wrappers, and a declared reconciliation that reads the
  members off the compiled module's classes (parameters, defaults and docstrings included) and
  writes them into the stub, counted against `EXPECTED_ATTACHED_MEMBERS` so a member appearing or
  disappearing is a review rather than a silently narrower contract. A member the package does not
  annotate stops the derivation instead of gaining a written `Any`, and the type-check gate now
  ships the sibling modules the stub imports — a scratch package holding only the `.pyi` had been
  reporting `import-not-found` for files every wheel contains. `Node` and `MergedView` were never in
  the hole, being Python modules a `py.typed` package already types, which is where the
  open-questions row overstated it. Checker and object now agree on the call the runtime always
  rejected: `document.node("a")` is a `Too many arguments` error at check time and the `TypeError`
  it has always raised. Delete the twelve declarations and
  `test_the_stub_declares_every_public_member_of_a_class_it_names` reddens naming all twelve.

- **A note on a member of a flow collection is written, not dropped.** — `{A: x  # V}` read into a
  tree that kept `V` and re-emitted `{A: x}`: the reader offers each flow member one slot, the
  line-end comment it attaches to that member, and the flow writer used none of them, so the note
  lived only in memory and whoever read the text next never saw it. Measured over 4020 drawn trees,
  1650 of them first differed on a comment. A note now continues one column deeper than the bracket
  that opened it (`- {A: x  # V` then `}`), because `#` closes its line and a value at column 0
  under `- {` or `k: {` reads as a wrongly indented flow continuation — eight spellings measured
  readable and their own fixed point. A member carrying two notes has no second slot, so its
  container goes block and every note keeps its own line (maintainer ruling, 2026-10-14). The same
  call answered a second question: a mapping or a sequence used as a key inside a flow mapping came
  back keyed by `null` — the sibling of the alias-key loss PR #410 closed — and keeps its `?` marker
  now (`{? {}: v}`), which ruamel reads as an empty-mapping key. Note text is emitted the way the
  reader hands it back, blanks at either end gone and a note left with nothing to say written
  nowhere, so the first emission is canonical rather than settling on a second round.

- **An alias used as a mapping key is written back instead of dropped to `null`.** — with
  `a: &x [1, 2]` and a key of `? *x`, the writer re-emitted `null: v`: the complex-key test in
  `write_mapping_pair` covered a mapping or a sequence node, so an alias fell through to the
  plain-key path and resolved to nothing — a different mapping, one entry shorter than the tree that
  produced it. The compact `*x: v` was not an available spelling, which is what the open-items row
  had claimed: this engine's reader refuses it (`simple key expected ':'`), so the writer carries
  the `?` marker — on the marker line in block style (`? *x`) and inside the braces of a flow
  mapping (`{? *x : v}`), and both load alike in every ruamel.yaml the lock resolves: 0.18.16 on the
  Python 3.8 legs, 0.19.1 elsewhere, where its scanner does raise on the compact form. A blanket
  claim about that scanner reddened the 3.8 legs and was replaced by measuring both versions. Why no
  tier saw the loss: the AST strategy never builds a mapping keyed by an alias, and the round-trip
  property *filters* a mismatching tree instead of failing it, so the changed shape was dropped
  rather than reported. `pbt::tests::prop_alias_key_survives_the_round_trip` builds the shape and
  asserts unconditionally; reverting the writer's predicate fails that property and the block unit
  test — 2 of 344, nothing else — and the Python tier pins the emission byte for byte plus ruamel's
  read of it.

- **The unhashable-key refusal stopped pointing at two routes that refuse it too.** — the message
  that came in with the complex-key projection told the reader to project the document with
  `to_json()` or `to_toml()`. Measured on this tree, a complex key raises in both: the JSON writer
  answers `json-object-key` and the TOML one `toml-keys-must-be-scalars`. So the advice led to two
  more errors rather than an exit, and it contradicted the function's own comment, which says the
  message names the shape and stops there. The text now states the one route that works — read it
  with `parse()` and keep the key on the AST — and says plainly that the other projections refuse
  this key as well. `tests/test_parse.py` pins both halves, so a reintroduced recommendation fails.
  Which inputs raise is unchanged, and the wrong wording never reached a release.

- **A complex mapping key no longer reaches Python as the node's Rust debug text.** —
  `safe_load("? ?")` handed back a dict whose key was `'Mapping { pairs: {Scalar { value: "~"…'`,
  and `? [1, 2]` the same shape of string, because the conversion preferred a debug format over
  dropping the pair. Neither choice was right: dropping loses an entry the document wrote, and the
  invented key is an ordinary string, so it re-emits — the corruption then arrives in the next
  document looking like a fixed point. Measured against the reference libraries on 2026-10-14 and
  then split by shape: a sequence key becomes a tuple (`{(1, 2): 'v'}`, exactly what ruamel.yaml
  produces, while PyYAML refuses it — a documented divergence toward the library that keeps the
  data); a nested container is refused by Python's own `unhashable type: 'list'`, because only the
  outer level converts; a mapping key and an alias key are refused here with a message naming the
  shape. Scalar keys, including a tagged key whose constructor returns something unhashable, keep
  their existing text fallback — that path was not broken and is unchanged. Eleven tests cover it,
  mutation-checked: against the previous code all eleven fail while the other 173 in those two
  files do not move. `ROADMAP.md` records the half this change found and did not paper over — an
  alias in key position is still written back as `null: v`, which is a serializer gap, not a
  projection question.

- **A path edit finds the key the reader means, not the spelling it met.** — `~`, `NULL` and a
  `Null` node are one mapping key to the reader, and two scalars carrying different tags are two
  keys, but the editing layer compared a key's *text* only. So setting `null` in a document holding
  `~: 2` missed that entry and appended a second line - and the text handed back was one this
  library's own reader refuses as `duplicate key: null` (#336). A path segment now resolves to what
  the reader resolves the key to, so that edit becomes `~: 3` on one line, while the pairs that must
  stay distinct do: a quoted `'null'` key is still its own entry, and `yes` is not null.

- **The CLI test harness no longer panics when a refusal closes its stdin.** — `run_with_stdin`
  wrote the child's stdin and panicked on any error, but a CLI that refuses its arguments exits
  *before* it reads, so the write races the closed pipe: the same invocation passed locally and
  reddened `msrv-check` with `write stdin: Os { code: 32, kind: BrokenPipe }`. The harness now takes
  the handle, tolerates the failed write, and closes it before the wait — which is also what a child
  that *does* read needs (end-of-file, not a pipe held open). Nothing is papered over: the
  assertions on exit code and stderr still decide the test.
  `a_refusal_closes_stdin_before_the_harness_finishes_writing` makes the interleaving deterministic
  by feeding 240 KB, more than a pipe buffer; against the old harness it fails every time
  (`Os { code: 109, kind: BrokenPipe }`), and the 48 CLI tests pass with the fix. — (details:
  quality-ledger (cs))

- **Re-reading compact JSONC or JSON5 no longer erases a run of comments.** — A node introduced by
  two or more same-line comments kept only the last: the reader gives a same-line note the single
  inline slot, so each overwrote the one before. The compact writers emit a node's notes exactly
  that way, so text this library wrote itself lost comments on being read back
  (`{/* note 0 *//* note 1 */`) and the second round differed. Such a run now goes to the leading
  list, which holds any number in source order; a lone same-line comment still owns the inline slot,
  so the spelling a human writes is unchanged. The cost is one nuance, recorded: with several
  comments on a line the writer can no longer tell which was same-line, so the pretty form puts each
  on its own line. Nothing is dropped and nothing moves between rounds. — (details: quality-ledger
  (ch))

- **`to_jsonc` and `to_json5` no longer drop comments the document carries.** — Of the eleven
  positions a JSONC comment can occupy, three were printed by `to_yaml` and lost by both dialect
  writers: a standalone note on a pair's value node, a line-end note on its key, and a note after
  the last member. The mapping loop read two of the four slots, and the text stayed valid JSONC,
  which is what made the loss silent. The pretty route now prints the two missing slots above the
  pair; the compact route comments with `/* … */` instead, because a `//` given its own line inside
  a flow container is re-read as trailing the member it precedes - and two of them come back
  swapped - which the writer's fixed point refuses. A note whose text contains `*/` is neutralised
  in the compact spelling so it cannot close its own comment. — (details: quality-ledger (cg))

- **A Python integer wider than 64 bits no longer leaves the dumper as a float.** —
  `safe_dump({"k": 10 ** 30})` wrote `k: 1000000000000000000000000000000.0`: the fallback meant for
  protocol-convertible types called `extract::<f64>()`, which succeeds on every `PyLong`, so the
  caller's number was replaced by the decimalisation of its nearest double. Keys were mangled the
  same way, and two wide keys could then collide on re-read. Both writer routes emit the exact
  `str()` digits for an `int` outside `i64`; an `int` subclass whose text is not a number keeps the
  previous route. The reader's `i64` cap is unchanged, so `safe_load` returns those digits as a
  string — the value survives, the type does not. — (details: quality-ledger (cf))

- **JSON5 no longer accepts leading zeros its grammar forbids.** — `[012]`, `[-01]` and `[-012]`
  parsed as JSON5 numbers because the parser carried an explicit relaxation claiming "JSON5
  tolerates a leading zero", and a test pinned that claim as intended behavior. JSON5's
  `DecimalIntegerLiteral` is `0 | NonZeroDigit DigitalDigits?` — the production RFC 8259 uses — and
  `pyjson5` rejects all three. The relaxation is removed and the test inverted. Three dialect rules
  that were only measured are now asserted: every valid JSON is accepted by both supersets; where
  the reference refuses an invalid document so must we; and JSONC may diverge from strict JSON only
  where a comment marker is present. Corpus loosening for JSON5 drops 30 → 27 and containment stays
  clean. — (details: quality-ledger (ce))

- **`to_toml` emitted text that was not valid TOML.** — Two escaping defects, both surfaced by the
  toml-test encoder corpus added for the writer: DEL (U+007F) went out raw inside a basic string
  (`k = "x<DEL>y"`, rejected by `tomllib` and by this crate's own parser), and a run of newlines
  escaped the first one then wrote the rest as real line breaks inside a single-line string. Every
  newline of a run is escaped now, and DEL joins the C0 codes in both string forms. The gate that
  found it judges our output with `tomllib`, never with our own reader. — (details: quality-ledger
  (ca))

- **An empty tagged value no longer emits a space the document did not have.** —
  `write_anchor_tag` appends a separator so the tag token ends before its content, and a plain
  empty scalar never brings content, so `n: !!null` re-emitted as `n: !!null` - and
  `n: !!null  # c` wrote three spaces before its note. The drop happens in `write_scalar`, where
  the emptiness, the style and the tag are in scope, one byte before `pending_tag_insert_at` is
  recorded, so a line that a
  following note closes still spells `!- ~` and no note changed owner. Two earlier routes were tried
  and rejected: popping at the terminator panicked on the index the pending mechanism had taken, and
  suppressing the key separator moved a note off its owner's line. Closes #339. — (details:
  quality-ledger (bz))

- **A standard tag now decides the type in every format, not only on load.** — the loader began
  honouring `!!str` / `!!int` / `!!float` / `!!bool` in #335, but the bridges to JSON, JSONC, JSON5
  and TOML still resolved the scalar *text* and dropped `meta.tag`, so one document meant two
  things: `v: !!str 1.20` loaded as the string `"1.20"` while `to_json` emitted `1.2` - the type
  changed *and* the trailing zero the author wrote on purpose disappeared. The rule now lives in one
  place, `pyrs-schema` (`standard_tag_kind` / `TagKind::resolve`, YAML 1.1's lexemes, both the `!!x`
  shorthand and the verbatim `!<tag:yaml.org,2002:x>`), and the JSON writer, the TOML writer and the
  CLI's typed comparison all ask it. A tag the text cannot satisfy is refused
  (`*-tag-text-mismatch`) rather than quietly re-typed as a string. Local tags (`!int`), unknown
  `!!` suffixes and the undecided `!!binary` mapping are unchanged, and pinned as unchanged. The
  cost is measured on the enforcing image and accepted: a pair loop that now asks the tag first
  spends `to_json_medium` +1.12%, `to_jsonc_medium` +1.03% and `to_json5_medium` +0.82%, while
  `to_toml_medium` moves -0.07% and nothing else crosses the line. `.ci/ir-baseline.json` is
  regenerated inside this pull request with those three scenarios named to the drift guard; the same
  run tightened `from_json_medium` -0.46% and `from_jsonc_medium` -0.42% without being asked to. —
  (details: quality-ledger (bu))

- **An edit no longer overwrites the new value's type with the old value's tag.** —
  `doc.set("$.d", b"hi")` emitted `d: aGk=`. The assignment builds a scalar carrying `!!binary`, and
  the splicing editor's metadata inheritance then took `comment`, `anchor` *and* `tag` from the node
  being replaced, so the type the caller assigned was deleted by the very function that
  documents "replaced scalars keep their comment/anchor/tag". The rule is now per field: the
  document owns the note and the anchor name (an alias in the same document still has to resolve),
  the value owns its type. An incoming value that carries no tag keeps the author's spelling -
  `port: !!int 8080` set to `9090` still reads `!!int` - and mappings, sequences and nulls inherit
  the same way, because they shared the defect. — (details: quality-ledger (bt))

- **Two spellings of null are one key; the generator had been handing both to the writer.** — the 20
  000-case property tier reported `DuplicateKey("null")` for `!E null: A  # 0` beside `!E null: a`.
  A `Null` node carries no text of its own and the writer emits the word `null` for it, so the two
  lines re-read as one key. `keys_collide_in_text` - the rule that keeps generated mappings
  representable - looked at scalar text and at empty containers and never at a null. It now covers
  every spelling a reader resolves to null, and separates what genuinely distinguishes two such keys
  (a tag; an anchor, but only once a tag is present) from what distinguishes nothing (a note, which
  rides the line rather than the key). Rule, writer and reader are pinned against each other in both
  directions: a pair the rule calls distinct must still hold two entries when its text is re-read
  strictly, and a pair it calls identical must be refused or folded. — (details: quality-ledger
  (br))

- **An inline dict schema no longer loses the parts the emitter did not know about.** —
  `_schema_to_yaml` is what `safe_load(schema={...})` and `YAML(schema={...})` run to turn a Python
  dict into the schema text the engine parses. It built that text line by line and knew only
  `extends` and `rules`, so a dict carrying a `validate` section was registered with those rules
  dropped silently - and a schema with no rules accepts every document, so nothing reported it. Its
  one hand-written quoting rule (double quotes when the value contains an apostrophe) also produced
  a double-quoted scalar with an unescaped `"` inside it for a pattern holding both quote
  characters: `invalid trailing content after double-quoted scalar`. The dict is now written by
  `from_dict` - the same serializer that guarantees round-trip fidelity everywhere else - so every
  key survives and arbitrary pattern text is quoted as written. — (details: quality-ledger (bp))

- **Two public functions accepted a keyword their own stub said they did not.** —
  `pyrs_yaml.read_markdown(path="note.md")` raised `TypeError` while the committed stub declared the
  parameter `path`: the Python wrapper in `__init__.py` had renamed it to `content` while the
  function opens the file it is given, so `read_markdown(content=text)` - the call the name
  suggested - failed on the filesystem instead. The wrapper now says `path`, `read_markdown_str`
  keeps `content`, and both document the difference. The same drift in the other direction left
  `validate_against_schema` declared as `schema_yaml` in the stub while the callable has always
  taken `schema`; the native parameter is renamed and the stub regenerated through the declared
  route. — (details: quality-ledger (bo))

- **Three locales' API pages typed parameters the library never had.** —
  `register_schema(name, schema: str | dict)` and `register_type(tag, type_handler, priority)`
  appeared in the zh/ja/ko reference pages while both layers declare
  `register_schema(name, schema_yaml: str)` and `register_type(name, handler)` — and `priority=`
  raises `TypeError`. The English page was correct, so this was translation drift, not a stale copy:
  the pages now type what the object accepts, and the gate refuses the regression in any locale. —
  (details: quality-ledger (bo))

- **A located validation error still names the path it matched.** — `SchemaValidationError` printed
  either `line:column: message` or `path: message`, so the moment a node carried a source range the
  schema author lost the half they had written: `1:9: expected map but got sequence` never says
  which key. Both halves print now. — (details: quality-ledger (bm))

- **A mapping key now means what the same text means as a value.** — `1: a` loaded as `{"1": "a"}`
  while `a: 1` loaded as `{"a": 1}`, and `~: 1` as `{"~": 1}` while `a: ~` gave `{"a": None}`: one
  document meant two things depending on which side of the `:` a scalar sat, so a config keyed by an
  integer, bool or null could not be reached by lookup. Keys now resolve through the same schema
  path as values, agreeing with PyYAML and ruamel; `load_toml` quotes any key either schema would
  re-type. — (details: quality-ledger (as), (at))

- **The anchor-name grammar matches the reader, closing a drift family.** — the writer could emit an
  anchor name the reader refused to re-ingest (`found unknown anchor`), which broke the engine's "we
  never emit unparseable output" contract: names ending in `:` lost a character each round, quoted
  names swallowed a line break, and the raw scanner invented anchors out of comment text and
  overlapping `&`. It now reads a name the way granit does, as one maximal run of anchor characters.
  — (details: boundaries — Fuzz findings, (x), (y), (z))

- **Notes keep their owner across markers, tags and containers.** — a family of positions where a
  note changed owner or vanished between the first emission and its re-read: a tag-only value
  (`k: !`) adopting the note line after it, a container's inline note landing on a line that cannot
  hold one, a `#` inside a quoted scalar evicting a note by a raw-byte scan, a compact `- key:` line
  copying text but not note slots, and a leading-comment *slot* that kept only the last of a stack -
  `# alpha` + `# beta` + `key: 1` came back as `# beta`. Leading notes are a list now and every
  writer respects it. — (details: quality-ledger (h), (i), (m), (n), (p), (t), (u), (v), (ad))

- **Merge identity and depth fixed; no more native-stack overflow.** — `<<: &b` is a literal,
  null-valued merge key and folded against the real `<<:` below it, re-homing the dropped entry's
  notes but not the anchor it carried; an untagged `y` with a note and a merged `y` were two
  `IndexMap` keys, so `to_yaml` printed `y:` twice; a nested or self-referential merge chain fed the
  resolver's tail recursion with the cycle guard already popped and overflowed the native stack (a
  58-byte libFuzzer find); and a `<<` whose key node carried a comment was invisible to the pass.
  — (details: quality-ledger (o), (q), (r), (s), (y))

- **Block scalars settle in one emission, and their indicators are detected correctly.** —
  `>+8\r\r#` reads as a folded scalar of one line break, `Keep`, explicit indent 8; the writer kept
  the indicator and emitted `>+8\n\n`, which re-reads as `Clip` with no indicator, and the *second*
  round wrote `>\n\n` whose value is `""` because Clip strips trailing breaks - the value decayed to
  empty while every round looked stable. Folded newline runs, `4RWC`-style explicit indents and
  blank-first-line literals are closed under re-parse by the same work. — (details: quality-ledger
  (aa), (ab), (r))

- **Unicode blanks, BOMs and tag suffixes no longer corrupt a document.** — the blank set, BOM
  placement and tag-suffix boundaries reached by the dialect fuzzers are fixed against the reader's
  own grammar, each pinned by a committed seed and a deterministic Rust regression test.
  — (details: boundaries — Fuzz findings, Blank set)

- **The JSON and JSONC comment scanners no longer panic mid-character.** — the line-comment and
  unterminated-block-comment scans stepped `pos` one *byte* at a time, so a trailing multi-byte
  character (U+FEFF, reached in ~25 seconds of fuzzing) left `pos` inside it and the next
  `&text[pos..]` slice panicked with "not a char boundary". Comments now advance a full code point
  and malformed input rejects cleanly. — (details: boundaries — Fuzz findings)

- **Duplicate keys are rejected by value, and only null keys fold.** — comparison is by value, so
  `{a: 1, a: 2}` still raises `YamlDuplicateKeyError`, while empty and null keys (`: a` + `: b`,
  `~: a` + `~: b`) fold as yaml-test-suite `2JQS` requires. A key whose node carried a comment also
  stopped being unreachable: `CustomNode::hash` folded the normalised comment view while `eq`
  compared the raw slot, so `doc["key"]`, `in` and merge expansion answered "no such key" for a key
  the document plainly holds. — (details: quality-ledger (ae))

- **Empty containers serialise inline, without a second pass.** — `safe_dump({"a": {}})` produced
  `"a:\n  {}\n"` while the AST writer produced `"a: {}\n"`, and under a sequence item
  `- \n  {}` re-read as a flow node and moved again. Both texts re-read to the same data, which is
  why no round-trip test saw it: the invariant the fuzz tier asserts is that the *text* is a fixed
  point. — (details: quality-ledger (ab))

- **`pyrs-toml` builds for bare metal again, and inline-table notes stay in place.** — the
  stacked-note work had put `std::mem::take` into a `#![no_std]` crate, which every host build
  forgave and `no-std-check` would not; six sites use `core::mem::take` now. Separately, a note on a
  non-last member of a multi-line inline table was emitted after the separator comma - and TOML
  cannot keep a comma inside a comment - so the reader re-homed it as the following key's leading
  note and the text never settled. — (details: boundaries — Known Engine Boundaries)

- **Linux free-threaded wheels ship, and `pyq` builds its artifacts.** — the wheel matrix only built
  free-threaded artifacts for Windows and macOS, so Linux users on the GIL-less interpreter had
  nothing to install: the GIL-enabled `cp38-abi3` wheels are ABI-incompatible with `Py_GIL_DISABLED`
  and `abi3t` starts at 3.15. The cross-architecture `pyq` legs also register qemu binfmt handlers
  and self-verify inside the manylinux image, because the host has the translator but no foreign
  loader. — (details: quality-ledger (aq))

- **Six defects in the gate instrument itself are fixed, including the MSRV leg.** — three checkers
  could not be imported on Python 3.8 (annotations evaluated at import, `str.removeprefix`); the
  scenario probe named one harness file, so a second channel was ungateable; the Ir-harness crate
  set stopped at `[dependencies]` and described a graph it had only half read; a baseline refresh
  ran on rustc 1.99.0 against a 1.97.1 record and moved `serialize_medium` +6.7%; the fan-in check
  waited on three of eleven jobs; and the property tier ran proptest's default 256 cases because no
  workflow set `PROPTEST_CASES`. — (details: quality-ledger (ah), (an), (ao), (ap), (aq), (ar))

- **An entry can no longer be filed where no reader looks.** — one commit added a release note to
  all five mirrors and placed it above the preamble in `CHANGELOG.md`, inside the `tags:` list in en
  and zh, and between the front matter and the first heading in ja and ko - outside the changelog
  body in every file, and green under the checker, because "the same version headers exist" says
  nothing about where a reader looks. `placement_errors()` polices position. — (details:
  quality-ledger (ar))

- **Published changelog pages carry their own description, and a folded release renders.** — the
  width fixer skipped only the opening `---` of a page's YAML front matter, so `title:`,
  `description:` and `tags:` were re-flowed onto one line in four locale pages; the deployed
  generator published them anyway with the site's description, and the newer one fails the build
  outright. Folded releases also opened with a bare `<details>`, which Python-Markdown treats as
  opaque - measured on the deployed version, 11 headings and 3 literal `####` leaks against 69
  headings and 0 with the attribute. `scripts/check_doc_metadata.py` gates both. — (details:
  quality-ledger (az))

- **The type stub shipped in every wheel parses, so type checkers read the contract.** — maturin
  1.14.1 copies each runtime `__doc__` into a triple-quoted string verbatim, and two Rust doc
  comments contain a backslash, so `python/pyrs_yaml/pyrs_yaml.pyi` failed `ast.parse` with a
  truncated unicode escape. With `py.typed` set, that file is the typing contract users' mypy and
  pyright read, and no documentation generator could reach the API through it. The drift route now
  escapes docstring backslashes under a counted tripwire and requires the derived text to parse.
  — (details: quality-ledger (bb))

- **Release notes describe their release, not only its first line.** — every `<summary>` in the
  folded history was the version's first entry, so a reader deciding whether to expand saw one
  bullet out of ninety-three (v0.16.0: 64 Added, 14 Fixed, 8 Performance). Each now states the
  release as a whole, and `0.11.4`, `0.11.3` and `0.1.0` - the three that never got a fold - are
  folded with the rest, in all five mirrors. — (details: quality-ledger (bc))

- **A prose-only pull request could not be merged, and no pull request ever built the site.** —
  Branch protection requires exactly one check, `Test matrix (all legs)`, and the workflow that
  produces it excluded `*.md` and `docs/**` from its triggers. PR #319 arrived with every check it
  produced passing and the merge refused with
  `Required status check "Test matrix (all legs)" is expected`; `--admin` was refused too, because
  admin enforcement is on. `ci.yml` now triggers on every pull request, a `changes` job classifies
  the changeset and the heavy legs are conditioned on that answer, while `docs-gates` always runs
  the documentation gates and `scripts/build-docs.py` for four locales under `--strict` - the way
  the registered rendering blind spot closed. — (details: quality-ledger (bd))

- **The typing contract shipped in every wheel declared no exception type.** — the package exports
  ten error types and `python/pyrs_yaml/pyrs_yaml.pyi` declared none, because maturin 1.14.1's
  introspection route walks no PyO3 `import_exception!` class; `except pyrs_yaml.YamlParseError:`
  was therefore invisible to mypy and pyright. The drift route now appends the declarations derived
  from the built classes (name, base, docstring), counts them against `EXPECTED_EXCEPTION_CLASSES`,
  orders a base before its subclass so the file parses, and exits 2 rather than emitting a contract
  missing them. `exceptions.md` is generated in all four locales as a result. — (details:
  quality-ledger (bf))

- **The shipped stub parsed and still did not type check.** — mypy reported five errors *inside*
  `python/pyrs_yaml/pyrs_yaml.pyi`: `Name "u32" is not defined`, `Name "Callable" is not defined`,
  and three `Invalid type comment or annotation` for `Py<PyAny>` - all attributed to this library in
  the user's editor, while every AST-level gate stayed green. maturin 1.14.1 copies Rust spellings
  into annotations and never imports `Callable`, so the route now rewrites the spellings (counted, 4
  sites), merges annotation-referenced names into the `typing` import, and
  `scripts/check_stub_types.py` runs mypy and `ty` against the artifact in CI. `ty` also flagged
  `-> dict` from our own fidelity template, now `dict[Any, Any]`; its 32 remaining strictness
  findings are recorded, not enforced. The gate then went red on every CI test leg for want of the
  tool it calls: a binary that is not installed raises instead of returning a code, so
  unavailability now answers exit 2 and a skipped test, never a pass. Its own negative control then
  decayed: it asserted against `origin/main`, and main held the clean stub the day this fix merged,
  so the control now names an immutable ref - the commit that carried the defect, with `v0.17.0` as
  fallback - checks the file for the spellings it should contain, and runs in CI where both checkers
  exist. — (details: quality-ledger (bg))

- **The prose fixer no longer writes the gap its own checker reports.** — adding a Japanese entry
  made `check_doc_wrapping.py --fix` answer `1 gap(s) [wrap residue]` and refuse to converge: the
  fixer chose the glue between two physical lines from a script class that omits U+3001, so a line
  ending in `、` was joined to the following kana with a space, while the residue class in that same
  file does include the code point and calls such a gap an artifact. One rule, written twice,
  encoded differently - and each repair pass recreated the finding it reported. `join_lines` now
  asks the two residue patterns themselves, and the added tests hold both halves: a `、` boundary
  fuses, `は docs.md にある` keeps its space. — (details: quality-ledger (bi))

- **A JSON5 non-finite float kept its type through the hub, in both directions (#312).** —
  `from_json5` returns the hub document, and the two ends of that text disagreed about the same
  three words: YAML Core resolves `.inf` / `-.inf` / `.nan` as floats and the bare word `Infinity`
  as a string, so storing the source spelling turned a number into a string for every consumer that
  re-read the projection — `load_json5("{a: Infinity}")` resolved in memory and was right,
  `parse(from_json5(...))` handed back `'Infinity'`. Measured first, and the report described half
  of it: the quoted direction was wrong the opposite way, because `quoted_or_plain` asked
  `needs_quotes` - a core-schema question - whether a decoded JSON string needed quoting, so
  `"Infinity"` was stored plain and `load_json5('["Infinity"]')` returned `[inf]` where
  `load_jsonc`, `load_json` and `json.loads` all returned `['Infinity']`. One document, two types,
  decided by which loader happened to read it. The emitter now derives the dialect's bare token from
  the resolved *value*, the hub stores YAML's own spelling, and the JSON5 resolver accepts the
  spellings the parser feeds it. Strict JSON and JSONC keep quoting a non-finite float they cannot
  spell: that ruling stays open in `ROADMAP.md` rather than being decided quietly inside a bug fix.
  — (details: quality-ledger (bh))

- **A schema rule path with a non-ASCII key aborted the process, and `$` is a rule target now.** —
  `rule_path_to_segments` advanced its cursor one byte at a time while pushing whole characters, so
  `$.café` stopped inside the `é` and the next slice panicked with "start byte index 4 is not a char
  boundary" - a `PanicException` out of `validate_against_schema`, reached by a valid schema rather
  than a malformed one (`$.emoji😀key` behaved the same way). The walk now advances by the decoded
  character's width. Fixing it exposed the second half: a separator was required before the
  empty-path branch was tested, so a bare `$` - the document itself - could never be a rule target,
  while `$x` stays unparseable rather than being read as a key. `path: $` with `mapping_of`
  validates the root's values, and the sweep that found the crash left a follow-up open in
  `ROADMAP.md`: `mapping_of`/`sequence_of` check elements but not the container's kind. — (details:
  quality-ledger (bj))

- **The fuzz tier had no target for the schema language, and a crash lived there (PR #329).** — six
  targets fed the engine documents; none ever fed it a schema, which is how `path: $.café` was able
  to abort the process in code review's blind spot (ledger (bj)) - every hand-written schema test
  used ASCII keys, so no tier in the matrix could reach the path navigator. The seventh target
  splits its input on the first NUL and passes the halves to `parse_schema_yaml` and
  `validate_node`, so one corpus entry carries both grammars and the interaction between them.
  Seeded with the exact input that used to crash, it joins the pull-request tier (deterministic
  `-runs=0` replay of the seeds) rather than waiting on the weekly sampler. Its bite is
  demonstrated, not assumed: with the fix reverted the replay surfaces one crash input at
  `schema_language.rs`, and with the fix in place the same seed replays clean, as does 150 s of
  exploration.

- **`check_doc_wrapping.py --fix` refuses paths that are not governed pages.** — Pointed at a Python
  module it treated the file as Markdown prose and re-flowed it: the docstring, then the statements
  under it, joining `def`, `if` and `raise` onto shared lines until the module raised SyntaxError.
  Measured on `scripts/ci_matrix_plan.py` during the matrix work, which is why the file was
  reconstructible rather than lost. The hook cannot reach that path - `prek.toml`'s `files:` pattern
  filters before the script runs - so the safety lived in the caller and the command line had none.
  `GOVERNED` now mirrors that pattern inside the script, `main` refuses any other path with exit 2
  and writes nothing, and it names every offending argument. Four tests, including a control that
  switches the guard off and requires the fixer to really rewrite the file - otherwise the passing
  test only shows this input survived. — (details: quality-ledger (cm))

- **Seven `Path.write_text` calls that passed the `newline` keyword raised `TypeError` on Python 3.8
  and 3.9.** — the keyword arrived in 3.10, and exactly the four legs that still run the whole suite
  on the two oldest interpreters answered it (`3.8` on all three OSes, `3.9` on ubuntu), failing
  inside fixture setup before reaching the behaviour under test. Three were written this week; four
  had sat unnoticed in `scripts/check_doc_wrapping.py`, `scripts/check_stub_drift.py`,
  `scripts/check_stub_types.py` and `tests/test_stub_types_gate.py`, green only because nothing
  executed those paths on a floor interpreter - the two stub scripts run in a modern Windows job,
  and that gate test skips on 3.8, where a shallow clone has no known-bad tag to read. Each now
  writes through `open(..., newline="\n")`, which has accepted the keyword since 3.5 and produces
  byte-identical output. Verified on real CPython 3.8.20 and 3.9.25 rather than by reading version
  tables: 235 of 238 on 3.8, 218 of 218 on 3.9; the three left are scripts importing `pyrs_yaml`,
  for which a bare environment has no wheel. — (details: quality-ledger (cn))

#### Performance

- **The event stream borrows its scalar text, and the gate priced the move at −9.192%.** —
  `StreamEventType::Scalar` owned a `String` copied out of granit's own `Cow`: one allocation plus
  one `memcpy` of the value's bytes for every scalar in a document. The maintainer ruled
  `Cow<'a, str>` on 2026-10-14, and the scenario that could argue with the ruling was added first
  (#405), so the number here is computed rather than asserted: `stream_medium` reads 249,928,334
  counted instructions against the committed 275,227,226, three samples identical to the instruction
  (spread 0.000%). The same run moved nothing else by more than +0.202% — the cross-host residual
  this repository already sizes, the readings being WSL against values committed from the enforcing
  image — and the AST path is not reachable from the change at all. A tree-free read now costs about
  22% less than building the node model on the same text, where the gap was 14.5%. Where the copy is
  still paid is stated rather than hidden: the lazy `StreamIterator` and the incremental
  `YamlStream` each detach at a boundary that cannot hold a borrow (a `#[pyclass]` carries no
  lifetime, and the chunked reader rotates the buffer an event points into), so only the `on_event`
  callback path pays nothing new.
  `parser::stream::tests::a_plain_scalar_borrows_the_source_and_a_decoded_one_owns_its_text` pins
  the split — a plain scalar's bytes lie inside the source's range, an escaped double-quoted value
  owns its own — so restoring the copy fails by name, and `stream_medium`'s floor moved down to the
  new number.

- **The instruction-count floor moved down to the engine that earned it.** — four optimization PRs
  landed between refreshes, and the committed baseline had not followed them. The `Ir baseline
  refresh` workflow re-measured all twenty-one scenarios on the enforcing image (`github-runner
  ubuntu24`, image 20261004.327.1, rustc 1.97.1 — the same toolchain the baseline already named, so
  this moves numbers and not a compiler) and its artifact is committed unchanged: `to_toml_medium`
  53,218,877 → 28,763,688 (−45.95%), `serialize_anchors` −30.25%, `serialize_small` −29.83%,
  `serialize_block` −26.42%, `serialize_medium` −24.81% (32,648,850), `serialize_block_scalars`
  −21.99%, then the readers `from_json5_medium` −11.17%, `from_toml_medium` −7.29%,
  `from_json_medium` −5.82%, `from_jsonc_medium` −4.72%. Nothing moved up past 0.10% — the seven
  scenarios that moved at all in the wrong direction are parser and binding paths, and all of them
  sit inside the 0.5% line the file itself declares. A floor left 46% above the engine is a gate
  that tolerates a 46% regression in silence, which is precisely the failure the zero-regression
  promise exists to prevent. The same pair of runs sizes what a local measurement can be trusted
  for: on identical sources WSL reads `serialize_medium` at 32,648,888 against the image's
  32,648,850 (+0.0001%) and `to_python_medium` at 386,665,602 against 385,871,983 (+0.21%), so the
  +1.45% cross-image figure `ir-baseline.yml` quotes is closed for the pure-Rust harness and what
  remains lives in the CPython-linked one. No number here was transcribed by hand.

- **The YAML writer asked the same question twice per node, in two different ways.** — Two rows
  of the same profile owned one document: `write_plain_scalar` and `write_scalar_for_key` at 9.21%
  and 9.29% of `serialize_medium` with ~10% more in the pattern-scan rows under them, and
  `write_indent` at 8.10%. The first was one function, `needs_double_quoted`, answering "can this
  token go out plain?" with four `contains`, two `chars().any` and sixteen `starts_with` over the
  same text; it is now one byte pass, with the character-level half - NBSP, the BOM, a C1 control, a
  YAML non-character - entered only when that same pass saw a byte at or above 0x80, because UTF-8
  never hides one of those inside another character. That is the reasoning `pyrs-schema`'s
  `needs_quotes` already uses; the writer's copy had not caught up. The second was a `Vec<String>`
  cache indexed by width - a length test per line, a `resize` at each new depth, a `" ".repeat`
  allocating one string per distinct width, then a borrow of the entry to copy - replaced by a slice
  of a constant 64-column run of spaces. Equivalence, not speed, is what is pinned:
  `needs_double_quoted_agrees_with_the_scan_for_every_code_point` keeps the six-scan form as a
  reference and asks both the same question over every code point below U+10000 in five positions -
  alone, leading, trailing, interior, beside a space - plus a stride through the astral planes and
  the non-characters explicitly. It caught the first version's mistake unaided: the tail read
  `!saw_high_byte || …`, which quoted every plain ASCII string, and the reference disagreed on
  `"a b"` before any other test ran. Mutating `b == b':'` out of the byte test reddens it and
  `prop_output_always_parses` together. Paired on rustc 1.97.1 as three binaries from three commits
  of one branch, two repeats agreeing to the instruction: the byte pass gives `serialize_anchors`
  -28.01%, `serialize_medium` -18.09% and `serialize_block_scalars` -13.05%; the indent table gives
  `serialize_medium` -8.20%, `serialize_block` -10.82% and `serialize_small` -29.81%; together
  `serialize_medium` is -24.81%, with `parse_medium` +0.00% as the control that calls neither.

- **The TOML writer stopped allocating a temporary for everything it emitted.** — `to_toml_medium`
  counted 53,218,850 instructions against `to_json_medium`'s 21,283,040 for the *same* document:
  every line went through `writeln!`, every key and every value was a `String` of its own before
  being copied into the document, and an array or inline table built a `Vec<String>` to join,
  formatted that, then copied it again. Measured as three binaries from three commits of one branch,
  so each step carries its own number: the document emitter pushes instead of formatting (-19.00%),
  and `quote_key`/`quote_basic`/`quote_multiline`/`value_str` became
  `push_key`/`push_quoted`/`push_multiline`/`write_value` appending into the caller's buffer
  (-33.27%) - -45.95% together, 53,218,850 down to about 28,761,000 counted instructions, with
  `from_toml_medium` -0.15%, `serialize_medium` +0.01% and `to_json_medium` +0.00% as controls that
  call none of it. The claim is byte-identity, not merely correctness, and tests cannot make it:
  they assert what the output should be, not that it is unchanged. Both trees therefore ran over
  every TOML document in the repository - 712 distinct inputs (777 valid `toml-test` files plus the
  fuzz corpus and seeds, de-duplicated by content hash), routed TOML -> hub AST -> TOML with a
  second pass to catch text that only moves on re-emission - and agreed exactly: 242 emitted
  documents, every SHA-256 matching, the same error class per rejection, the same 9 shapes that are
  not fixed points, zero differing records. What makes a differential mean something is checking
  that the "before" tree lacked the new helpers before trusting its output (grep: 0 occurrences),
  since two identical trees pass for the same reason a no-op would. `quote_key` is gone;
  `quote_basic` and `quote_multiline` live on as `quoted` and `multiline` inside the test module,
  because an assertion wants a value back and shipped code does not. `docs/dev/perf.md` keeps the
  attribution, the arithmetic, and what the writer still spends.

- **The JSON reader stopped paying for two calls that had nothing to do.** — attribution on
  `from_json_medium` found `Parser::ws` at 6.63% of the scenario on a fixture with *no whitespace at
  all*: roughly ninety calls per parse, thirty-odd instructions each, for a question whose answer is
  almost always "the next byte is not separation". `attach_pending` was the same shape at 3.23% on a
  document with no comments, met by every key and every value. `ws` is now an inlined guard over one
  predicate, `at_separation`, with the loop moved to `ws_extended` unchanged, and `attach_pending`
  returns as soon as it sees an empty list. The guard is a sound superset of the loop in the
  direction that matters — if the loop would consume a byte, the guard says so — and that is pinned
  by `the_whitespace_guard_never_declines_what_the_loop_would`, which asks guard and loop the same
  question over all 128 ASCII bytes, five multi-byte separators plus one non-separator, three
  comment shapes and three dialects, and requires the same position and pending-note count back.
  Mutation-checked: making the guard answer `false` for `/` fails it with `left: (0, 0) right:
  (4, 1)` on `"//x\n1"` under JSONC, which is the under-call that would have dropped a comment
  silently. Paired on rustc 1.97.1 as three binaries from three commits of one branch, so each guard
  carries its own number: `from_json_medium` −4.33% then −1.56%, `from_jsonc_medium` −3.52% then
  −1.25%, `from_json5_medium` −4.49% then −1.40%, with `to_json_medium` +0.00% as the control that
  calls neither. `docs/dev/perf.md` records why the first row beats the arithmetic that predicted
  it, and what the guards do not recover.

- **Two readers copy the source run instead of decoding one character at a time.** — the JSON5
  single-quoted scanner and the TOML single-line basic-string scanner were the last two readers
  still calling `chars().next()` and `push(ch)` per code point where the double-quoted JSON scanner
  had long since bulk-scanned to the next byte that needs a decision and copied the run. On the
  gate's own harness a nine-character TOML value cost ~540 counted instructions. Paired on rustc
  1.97.1, same tree, alternating binaries, two repeats agreeing to the instruction:
  `from_toml_medium` −5.55%, `from_json5_medium` −5.76%, with `from_json_medium` +0.00% and
  `to_toml_medium` −0.00% as the controls that say the change moved those two and only those two.
  The stop sets are unchanged and each half is pinned by a mutation-checked test: TOML still rejects
  a control character or a line terminator inside a basic string, JSON5 still *accepts* a control
  character there, and both still reject unterminated input that ends after multi-byte text.
  `docs/dev/perf.md` keeps the attribution that found it, the multiline twin this change
  deliberately leaves alone, and the TOML reader's remaining structural cost — 169,449,833
  instructions against JSON's 98,994,186 for the same document, because it builds an intermediate
  table model before the hub AST — named as the next question, not as a closed one.

- **The instruction-count floor moved down to the engine that earned it.** — the
  `Ir baseline refresh` workflow re-measured all twenty-one scenarios on the enforcing image
  (`github-runner ubuntu24`, rustc 1.97.1) once the node-map hasher was on `main`, and its artifact
  is now the committed baseline: `from_json_medium` 120,751,446 → 99,000,356 (−18.01%),
  `from_toml_medium` −16.25%, `parse_anchors` −12.95%, `to_python_anchors` −11.17%, with the write
  scenarios inside +0.03%…−1.80%. A floor left 18% above the engine is a gate that tolerates an 18%
  regression in silence, so the zero-regression promise now refers to these numbers. No value here
  was transcribed by hand; `scripts/ir_gate.py --update` wrote the file on the runner.

- **The node map hashes its keys with FxHash on every build, not only under `no_std`.** — with
  `std` on, `NodeMap` was `IndexMap<K, V, std::hash::RandomState>`, so putting one mapping key in
  the table cost SipHash-1-3: `callgrind` measures 10,214,000 self instructions over 180,000 calls
  reaching `Hasher::write` from `CustomNode::hash` — 8.98% of `from_json_medium` — and the feature
  split even disagreed about the node's size (144 bytes against 128). Both sides now name the
  crate's own fixed-seed `FxHasher`, which is what the embedded build has always used. Paired on
  the compiler the gate pins (rustc 1.97.1): `from_json_medium` −12.75%, `parse_medium` −4.95%,
  `to_json_medium` +0.11%. The trade-off the `no_std` side had already accepted is now the whole
  build's, and it is written on `NodeMap` rather than left implicit: no HashDoS randomisation, so
  a document written to collide degrades a node map to linear probes. Landing it also meant naming
  `NodeMap` at the ~40 sites that spelled `IndexMap<CustomNode, CustomNode>` and so asked for
  `RandomState` without saying so — the same fix the `no_std` split had always needed. (details:
  docs/dev/perf.md "The node map was hashing its keys with SipHash")

- **The scalar resolvers stopped trimming text that had nothing to trim.** — every JSON, TOML and
  YAML scalar asks `resolve_core_type` "is this a number or a word?", and the first thing that
  answer did was strip edge separation with a two-ended predicate walk: 38 counted instructions per
  call, on strings that almost never start or end blank, and paid twice over by a YAML 1.1 scalar
  because `resolve_yaml11_type` trimmed before delegating to the core chain. Every blank YAML
  accepts is ASCII (`SP TAB CR LF NUL`), and UTF-8 keeps an ASCII byte out of a multi-byte sequence,
  so the first and last byte decide whether there is anything to strip at all: two reads of a class
  table built at compile time *from* `is_yaml_blank` - not a restatement of it - now answer before
  the walk is attempted, and the walk still runs whenever either end really is blank. Measured in
  one pair of runs of the same harness binary: `from_json_medium` −0.74%, `from_toml_medium` −0.92%,
  and because the writers resolve through the same crate `to_json_medium` −1.68% with
  `serialize_medium` −0.36%, while `parse_medium` repeated at +0.0014% as the control that makes
  those two deltas count. `#[inline]` on the guard measured −0.003%, inside the same-binary spread,
  so it was dropped rather than kept as an unmeasured hint. — (details: quality-ledger (ct))

- **The instruction-count floor moved to the code that is on `main`.** — the committed baseline
  still held the pre-`d7177efc`/`dd211c8d` numbers, so the gate's band had widened by drift rather
  than by decision: `from_json_medium` sat 21.61% above what the engine now measures,
  `from_jsonc_medium` 19.83% and `from_json5_medium` 16.72%, and a regression of that size would
  have passed silently. Regenerated through the `Ir baseline refresh` channel on the enforcing image
  — toolchain read from the file itself (`rustc 1.97.1`), image `20261004.327.1`, largest of three
  samples per scenario, binary digests written with the numbers — so nothing is transcribed and
  nothing crossed the 0.5% line upward: the three scenarios that ticked at all did so by 0.0001% of
  process noise. The write scenarios did not move, which is the shape two changes to how keys are
  hashed and quoted should leave behind.

- **Quoting a string stopped scanning it three times.** — every bridge asks
  `needs_quotes` "would this text be misread as a plain scalar?", and the answer read the value as
  characters and then searched it twice more for NBSP and BOM. Neither can occur in ASCII text, so
  one byte pass now decides the ASCII case and hands any text carrying a byte above `0x7F` to the
  character rule unchanged. The exhaustive control
  `needs_quotes_agrees_with_the_char_rule_for_every_code_point` compares the two paths against the
  character rule over the whole code point range - 1,114,112 points, the 2,048 surrogates no `char`
  can hold excepted - and it convicts a wrong byte rule: shifting the
  bound by one (space counted as a control) reddens it with `first: Some((32, true, false))`.
  `callgrind` on the gate's harness, same host: `from_json_medium` -6.89% and `from_json5_medium`
  -1.87%, while `from_toml_medium` (-0.0009%) and `parse_medium` (+0.0002%) — the two scenarios this
  change does not enter — repeated across the runs, which is what makes the first two deltas
  evidence rather than ambient noise. — (details: quality-ledger (cr))

- **A node's digest reads exactly what its equality reads, and every reader got cheaper.** —
  `CustomNode::eq` compares a scalar's value, style, chomping and three identity fields; the
  explicit block indentation indicator is not among them, yet `Hash` hashed it. Two nodes the map
  calls one key therefore routed to different buckets — the same contract `plain_and_noted` already
  guards, for the other field. `Hash` no longer reads `block_indent`, and an undecorated node writes
  nothing for metadata instead of three `Option` discriminants. `callgrind` over the instruction
  gate's own harness, same host and same build flags, prices it: `Hasher::write` self cost 20.95M →
  9.95M instructions, and the whole process -7.80% on `from_json_medium`, -7.71% on
  `from_json5_medium`, -5.66% on `from_toml_medium`, -3.38% on `parse_medium`. Key comparison did
  not grow (`__memcmp_avx2` 10,720,143 → 10,720,033), so the cheaper digest did not cost probe
  quality on the counted fixture. The measured route is the `std` one, where the node map is keyed
  on `RandomState` — `no_std` already keys it on a fixed-seed FxHash (`ast.rs:29`).
  `.ci/ir-baseline.json` on the enforcing image stays the arbiter, and those scenarios should move
  down. — (details: quality-ledger (cq))

- **Keeping the comments the dialect writers used to drop is paid for in counted instructions.** —
  `to_jsonc` and `to_json5` now read the two comment slots per pair they had never looked at (#359),
  and `ir_gate.py` prices the wider pair loop at `to_jsonc_medium` +4.77%, `to_json5_medium` +3.78%
  and `to_json_medium` +0.94%. `.ci/ir-baseline.json` was regenerated on the image that enforces it
  with those three named via `--accept-drift`; the same refresh tightened `to_toml_medium`
  by -4.90% - drift accumulated while the baseline stood still, not a regression - and recorded the
  SHA-256 of both harness binaries, so a future delta names the bytes it describes. The tolerance is
  unchanged. — (details: quality-ledger (cg))

- **Tag emission is table-driven, and escaping no longer allocates.** — putting tag encoding on the
  reader's own character classes made the per-byte membership test part of the serialize hot path,
  so it became a 128-entry compile-time table (the alphanumeric test folds into the same lookup) and
  the `%XX` escape writes from a hex-digit table instead of `format!`, which allocated a fresh
  `String` per escaped character. — (details: perf — Leaderboard & Performance Status)

- **A document with many null keys parses in linear time.** — the null-key fold first rescanned the
  mapping for every null key: invisible on an all-null document, whose folded entry sits at slot 0,
  and quadratic on the shape that matters, where 2k distinct keys followed by 2k null keys grew
  12.4x for a 4x input (99 ms at 8k + 8k). The mapping remembers its null key's slot and keeps the
  scan only as a correctness fallback. — (details: quality-ledger (ae))

- **Sub-10% questions are settled on the committed instruction baseline.** — two runner jobs measure
  `serialize_block_scalars` 1.44% apart while the same binary repeats within +/-0.0005%, so a wall
  clock below a runner's noise decides nothing: the baseline now records the environment that
  produced it and `scripts/ir_gate.py` says when a run comes from a different one. — (details:
  quality-ledger (am), (as))

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
