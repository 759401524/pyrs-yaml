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
scenario spread at most 0.002% on the engine channel (`parse_inline_merge`, 7,792 instructions out of
451M) and at most 0.19% on the binding channel (`to_python_small`), and across jobs and hosts — with the
glibc caps pinned, which is what makes the two comparable — the enforcing job agrees with a baseline
generated elsewhere by at most 0.10%. What the old 2% was: the distance between a WSL-generated baseline
and the same code measured on a runner, +1.45% on `serialize_block_scalars` — now explained as the ISA
that glibc binds at start-up (the bullet above), which is why the caps are pinned rather than the line
widened: a tolerance sized to absorb a host's CPUID flags would be a shrug, not a gate.

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

**How much of that band is left is itself a measurement, and it was taken again here.** Against the
refreshed baseline the enforcing job moved every scenario by at most 0.10%, so the band is essentially
unused — which is not what an earlier version of this paragraph reported: in #299's measurement the same
comparison spread from −2.35% (`parse_anchors`) to +1.61% (`serialize_anchors`), and in this branch's own
first run `serialize_block_scalars` came in at +1.44% before the ISA was pinned. Drift of that kind is
not noise; each time it was a property of the method that has now been closed. Three things follow, and
all three have now happened. `.github/workflows/ir-baseline.yml` regenerates the numbers
with the steps the enforcing job uses, which is where every value in `.ci/ir-baseline.json` comes
from — seventeen of them, since the binding harness joined and since each cross-format
bridge grew its reader. `ir_gate.py --update --only <scenario>`
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

### Registered blind spots

Each id below is emitted by `scripts/quality_matrix.py` and registered in
`.ci/quality-holes.json` with the date it was measured and the concrete state that removes
it. Deleting an entry that the measurement still reproduces fails CI; keeping one the
measurement no longer reproduces fails CI.

| hole | why it matters | exit |
| --- | --- | --- |
| `docs-generation:plugin-unused` | the site configuration declares the Python docstring handler, its load paths and fourteen render options, and not one page under any locale contains a `:::` directive — measured by `mkdocstrings_directives()`, which finds zero. So `docs/<locale>/api/*.md` are hand-typed signatures, nothing compares them with the generated `python/pyrs_yaml/pyrs_yaml.pyi`, and a bump of the docs toolchain cannot be checked against output it never produces: the render options were verified to still be accepted (all fourteen are fields of the installed `PythonOptions`) and none of them can be observed doing anything | a page under `docs/en/api/` carries a directive the handler renders and its zh/ja/ko twins carry the same, or the `handlers.python` tables are deleted from `zensical.toml`; `mkdocstrings_declared()` without `mkdocstrings_directives()` emits the hole, and either fix removes it |
| *none measured* | the site was rendered only by a workflow that triggers on push to `main`, so a page that fails when *rendered* was found by whoever opened it: four changelog pages carried metadata the generator cannot parse through three merged pull requests, each with every gate green, because the joined text satisfied the width rule, the heading rule, the mirror checker and `rumdl` — the damage was inside every measurement the tree had | satisfied by `ci.yml`'s `docs-gates` job, which runs the page-metadata, heading, mirror, purity and i18n gates and then `scripts/build-docs.py` for all four locales under `--strict` on every pull request; `renders_the_site()` measures it off the declaring files rather than off a list kept beside them |
| *none measured* | a required check that never reports is not a green light but a deadlock: branch protection names `Test matrix (all legs)`, `ci.yml` excluded `*.md` and `docs/**` from its triggers, and PR #319 arrived with every check it produced passing and the merge refused with `Required status check "Test matrix (all legs)" is expected` — a documentation change could not merge by any route an ordinary contributor has | satisfied by the same change: `ci.yml` triggers on every pull request, a `changes` job classifies the changeset, and the verdict tolerates a skipped heavy leg only while the classifier reports `code=false` — tested in both directions in `tests/test_matrix_verdict_gate.py` |
| *none measured* | the mirror checker compares version headers, which translation leaves identical, so an entry present in three mirrors and missing from two used to pass it — `docs/en` was one `[Unreleased]` entry behind and `docs/zh` one Added and five Fixed behind, with one Changed bullet no other mirror had. Condensing `[Unreleased]` into user-facing entries, written into all five pages in the same pass, closed the divergence, and `check_changelog_mirrors.py` now asserts the per-section counts instead of printing them | satisfied: the five pages report equal counts and the probe derives nothing |

This table is a measurement, not a mood: `scripts/quality_matrix.py` re-derives it on every
pytest run and `tests/test_quality_matrix.py` fails in both directions — a hole that appears
unregistered, and a registered hole the measurement no longer reproduces. It has been empty twice,
and every transition since has been measured rather than declared. It was empty after the property tier
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
  `cargo nextest`. Both closures came with a correction to how this document first
  described the gap, recorded above rather than edited out.
  Acceptance: every hole this item covered deleted from the registry, and the measurement
  agreeing with what is left.
- **P2-F, Ir baseline breadth and provenance** (`done`): the `to_json`, `to_toml` and inline-merge
  scenarios are in the gate, and so are the three `to_python_*` scenarios that close
  `perf-coverage:binding-layer` — 15 measured scenarios, 15 committed numbers, all generated rather than
  transcribed: `--update` writes every key including the sample size, `--update --only` can no longer
  write a short baseline, each scenario is the largest of three samples, and every run prints its samples,
  their spread, the binary hashes and the pinned environment. The 0.5% line is sized from that spread
  (≤0.002% engine, ≤0.19% binding within a run) and holds across jobs and hosts (≤0.10%) because the one
  environmental input the measurement could not control — the ISA glibc binds from the VM's CPUID flags —
  is pinned rather than tolerated. Acceptance: `tests/test_ir_baseline_workflow.py` pins the committed
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
