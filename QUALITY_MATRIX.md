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
| unit / integration | 509 Rust `#[test]`, 1,216 Python test functions | a named shape behaves as documented | anything not named — the whole reason the other two tiers exist |
| property | 22 `prop_*` functions (proptest), default 256 cases each | no crash + an invariant over a generated grammar | shapes outside the generator; invariants weaker than equality |
| fuzz | 6 targets, 117 seeds of which 60 are `former-crash-*` | a historical crash cannot regress (PR tier replays seeds with `-runs=0`) | the discovery tier runs on a schedule, so a new shape is found weekly rather than per-PR |

Orthogonality, tested rather than asserted: the same three defect families that the recent
fuzz tier found (comment attribution, empty-container spelling, nested merge application)
are invisible to the property tier at its committed case count, and the two data-loss
defects (`<<` merge never applied when nested, mapping keys never resolved) were invisible
to *both* Rust tiers and only became visible when a Python test compared this engine with
PyYAML and ruamel. Three tiers over one engine are not three defences: coverage is
orthogonal only where a second implementation exists to disagree with.

Per-format matrix, as measured:

| format | unit | property | fuzz parse | fuzz round trip | cross-library parity |
| --- | --- | --- | --- | --- | --- |
| YAML | yes | yes | yes | yes | yes |
| JSON / JSONC / JSON5 | yes | yes | yes | yes | partial |
| TOML | yes | yes | yes | yes | yes |
| schema validation | yes | yes | no | n/a | n/a |

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
writers fails instead of passing vacuously.
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
now runs a blocking `property tier (20k cases)` job. That closed the last hole then open; the registry
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
measurement), `cargo test --workspace`, the 20k-case property tier, MSRV
check, `no_std` bare-metal build, pytest on 3 OSes × 7 Python versions, free-threaded
pytest, coverage floor, CodSpeed (Rust + Python), the callgrind Ir gate, the fuzz seed
replay, changelog-mirror / localized-script-purity / release-guard / stub-drift, the changelog
coupling check (`scripts/check_changelog_coupling.py`, evaluated on the pull request's file list
because a version-header comparison cannot see a release note that was never written), and —
as of this document — the `prek` hook set over the whole tree (`hygiene.yml`).

Six blind spots were found by measuring, not by reasoning. One had already done damage in
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
  repository's own jobs turn out not to measure the same thing.** The Ir baseline was generated on one
  kind of machine and enforced on another, and the 2% line used to be sized around that difference
  (`serialize_block_scalars`: +0.78% locally against +2.23% on CI). Four explanations have been tested
  and all four failed. `.gitattributes` normalising CR bytes inside `BLOCK_SCALAR_YAML`: adding `-text`
  moved the runner's number by 28 instructions out of 16,020,906 — though the probe did expose a real
  reproducibility hole, the fixture's *stored* bytes depending on which tree the last author committed,
  now closed and asserted by `tests/test_line_endings_gate.py`. Drift between runner images: two images
  on one commit agree to 0.0018%. The committed values having been generated off-runner: refuted — the
  enforcing job reports the numbers the file carries. A restored build cache making
  `.github/workflows/ir-baseline.yml` compile differently: refuted by deleting the cache step and
  watching the job report 15,792,928 anyway.

    What is left is a **1.44% disagreement between two jobs in this repository**, on
    `serialize_block_scalars`, with the same image family, the same pinned rustc 1.97.1, the same script
    and byte-identical sources (`git rev-parse` of `crates/pyrs-yaml-core/src/bench_inputs.rs` agrees
    across them): eight runs of the refresh job — cached and uncached, on two images — produced
    15,792,8xx to 15,792,9xx, and two runs of the `Instruction-count baseline` job in `codspeed.yml`
    produced 16,020,942 and 16,020,915. Every other scenario agrees between the two jobs to within 0.08%,
    and inside a job a scenario is stable to ~30 instructions, so this is a property of how the
    measurement is taken, not of noise. It is also the WSL-to-runner gap of the earlier paragraphs
    reproduced between two runner jobs on the same image, which retires the machine-class explanation and
    leaves the cause unnamed. `build_exe` now prints the SHA-256 of the binary it measures and both jobs
    print `nproc`, `valgrind --version`, `rustc -vV` and the CPU model, so the next statement about it
    will be a measurement rather than an inference.

    What the exercise did establish is that the file's own prose was not generated. Its
    `generated_by.note` cites a commit that is not PR #299's head and predates the toolchain pin — that
    job would have measured with the image's `@stable`, rustc 1.99.0, which moves this scenario −6.6% and
    `serialize_medium` +6.7% — and its environment string reads `ubuntu-24.04` where `environment()`
    emits `ubuntu24` for that image. So the note is gone; `--update` writes every key, including the
    sample size the numbers were drawn with (which the enforcing run reads back), the key set is pinned
    by `tests/test_ir_baseline_workflow.py`, and the reasoning lives here. Saying "every value comes from
    the environment that enforces it" was premature while two of those environments disagreed; the honest
    form is that a baseline is a claim about a *job*, and this repository has two that do not agree.
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
  a job added without widening the fan-in reddens the suite. The remaining half is outside a pull request - the name has to be added to branch
  protection - so this row is the honest state: closed in software, open in configuration.

The strongest evidence that a hook tier is not decoration came from its own author: the five
files written while building this matrix came out of the editor as CRLF, and
`scripts/check_line_endings.py` named them before anything was committed. That is the
failure mode, reproduced by the tool that polices it, caught by the tool.

Non-obvious consequence for performance claims: the Ir gate is the only reproducible
instrument in the set, and it is what flagged a real cost in #292 (plain-string mapping
keys, ~+19% in a drift-free local ratio) that wall-clock CodSpeed had already noticed at
−10%. Local wall clock cannot resolve a ~10 ns per-key delta; the gate can, and it does so
with a measured line instead of a vibe. That line is 0.5%. Within one run, three samples of each
scenario spread at most 0.002% on the engine channel (`parse_inline_merge`, 7,792 instructions out of
451M) and at most 0.24% on the binding channel (`to_python_small`), so 0.5% is ~250x the engine's
within-run spread and ~2x the binding's worst. What the old 2% was: the distance between a
WSL-generated baseline and the same code measured on a runner, +1.45% on `serialize_block_scalars` —
and that offset is now reproduced **between two runner jobs on the same image**, which retires the
machine-class explanation without replacing it (the bullet above carries the measurements). The
tolerance is therefore argued from the spread the instrument shows inside a job, and the cross-job
disagreement is an open defect of the instrument, not a margin the line absorbs.

**The binding channel had to be widened before it could be gated.** At 500 iterations
`to_python_small` came out 0.83% apart between two runs of one commit — more than the tolerance it
was about to be held to — while the engine scenarios agreed to 0.0009%. The loop was too small for
the variation inside it, so the harness runs 2 000 iterations like its engine sibling, which brought
its within-run spread down to 0.076-0.24%. Each scenario is also sampled three times and the *largest*
value is what gets committed, because a single draw of a distribution that wide enforced at 0.5% is a
flaky gate. The sample count travels inside `generated_by` and the enforcing run reads it from there:
max-of-3 against a single sample, or the reverse, is a different instrument wearing the same name, and
the run prints every sample and spread so the claim is auditable in the job log rather than a sentence
in a comment.

**How much of that band is left is itself a measurement, and it was taken again here.** The enforcing
job's own table against the refreshed baseline moved `serialize_block_scalars` by +1.44% and every
other scenario by at most 0.08% — the first is the cross-job disagreement above, and the rest says the
band is essentially unused: on twelve of fifteen scenarios the gate has 0.42 points of headroom left,
where the ledger of #299 had measured 0.39 on one and a *negative* margin on another. Three things
follow, and all three have now happened. `.github/workflows/ir-baseline.yml` regenerates the numbers
with the steps the enforcing job uses, which is where every value in `.ci/ir-baseline.json` comes
from — fifteen of them, since the binding harness joined. `ir_gate.py --update --only <scenario>`
merges into the committed file and refuses to write a baseline with a missing number, because the
first version silently reduced the scenario set to the one it had measured. And the file has to be
re-generable in full: its first committed version carried a hand-written `generated_by.note`
explaining its own provenance, which turned out to be wrong in three checkable ways (recorded in
the bullet above), and `--update` writes no prose at all. The explanation lives here; the file's
keys are pinned by `tests/test_ir_baseline_workflow.py` to exactly what the job writes.

## 3. Root-cause depth

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

## 4. Regression protection

What a closed defect leaves behind, per the discipline this repository already runs: a
minimised seed under `fuzz/seeds/**/former-crash-<hash>.seed` (60 of them), a named
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

## Registered blind spots

Each id below is emitted by `scripts/quality_matrix.py` and registered in
`.ci/quality-holes.json` with the date it was measured and the concrete state that removes
it. Deleting an entry that the measurement still reproduces fails CI; keeping one the
measurement no longer reproduces fails CI.

| hole | why it matters | exit |
| --- | --- | --- |

This table is a measurement, not a mood: `scripts/quality_matrix.py` re-derives it on every
pytest run and `tests/test_quality_matrix.py` fails in both directions — a hole that appears
unregistered, and a registered hole the measurement no longer reproduces. It has been empty twice,
and both transitions were measured rather than declared. It was empty after the property tier
closed; the same measurement then asked a new question — not which scenarios the perf gate names,
but which crates it can link — and the crate that serves the Python API failed it. That is how
`perf-coverage:binding-layer` was registered (#302), with the artefact that would remove it named
in its exit criterion.

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

## Improvement plan

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
  `cargo nextest`. Both closures came with a correction to how this document first
  described the gap, recorded above rather than edited out.
  Acceptance: every hole this item covered deleted from the registry, and the measurement
  agreeing with what is left.
- **P2-F, Ir baseline breadth** (`partly done`): the `to_json`, `to_toml` and inline-merge scenarios are
  in the gate, and so are the three `to_python_*` scenarios that close `perf-coverage:binding-layer` — 15
  measured scenarios, 15 committed numbers, all generated rather than transcribed: `--update` writes every
  key including the sample size, `--update --only` can no longer write a short baseline, each scenario is
  the largest of three samples, and every run prints its samples and their spread. The 0.5% line is sized
  from that within-run spread (≤0.002% engine, ≤0.24% binding) instead of the +1.45% figure the ledger used
  to quote.
  **What is not done is the claim this item was meant to deliver.** "The numbers come from the environment
  that enforces them" held of the image and of the compiler, and no further: two runner jobs in this
  repository, same image family, same pinned rustc 1.97.1, byte-identical sources, disagree by 1.44% on
  `serialize_block_scalars` (section 2), and the build cache was tested as the explanation and refuted.
  Until that is attributed — the binary hashes and the host fields both jobs now print exist to answer
  it — a baseline is a claim about the job that generated it, and `.ci/ir-baseline.json` is generated by a
  job that is not the gate. Acceptance: one instrument measures and enforces the same numbers, evidenced
  by matching hashes across the two jobs, and `ir-unbaselined` / `ir-stale-baseline` stay empty.

## Reading this document

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
