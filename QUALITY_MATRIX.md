# Quality Defence Matrix

Assessment of how this repository finds defects, and the gate that keeps the assessment
true. Everything below is measured out of the files that declare the defence
(`.github/workflows/*.yml`, `prek.toml`, `fuzz/Cargo.toml`, `scripts/check_*.py`,
`crates/pyrs-yaml-core/benches/ir_gate.rs`, `.ci/ir-baseline.json`) by
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
| fuzz | 4 targets, 87 seeds of which 60 are `former-crash-*` | a historical crash cannot regress (PR tier replays seeds with `-runs=0`) | only the YAML round trip is generated as a round trip |

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
| JSON / JSONC / JSON5 | yes | yes | yes | **no** | partial |
| TOML | yes | yes | yes | **no** | yes |
| schema validation | yes | yes | no | n/a | n/a |

A parse-only target cannot see a writer. `fmt_pbt` does test JSON/JSON5/TOML writers, but
its three fixed-point properties fail at 20,000 cases and pass at CI's default, so the
tier that would catch that regression is the one tier CI does not have.

## 2. Gate integrity

Blocking on every pull request, measured: `clippy` (with `--all-targets`, since this
measurement), `cargo test --workspace`, MSRV
check, `no_std` bare-metal build, pytest on 3 OSes × 7 Python versions, free-threaded
pytest, coverage floor, CodSpeed (Rust + Python), the callgrind Ir gate, the fuzz seed
replay, changelog-mirror / localized-script-purity / release-guard / stub-drift, and —
as of this document — the `prek` hook set over the whole tree (`hygiene.yml`).

Five blind spots were found by measuring, not by reasoning. One had already done damage in
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

The strongest evidence that a hook tier is not decoration came from its own author: the five
files written while building this matrix came out of the editor as CRLF, and
`scripts/check_line_endings.py` named them before anything was committed. That is the
failure mode, reproduced by the tool that polices it, caught by the tool.

Non-obvious consequence for performance claims: the Ir gate is the only reproducible
instrument in the set, and it is what flagged a real cost in #292 (plain-string mapping
keys, ~+19% in a drift-free local ratio) that wall-clock CodSpeed had already noticed at
−10%. Local wall clock cannot resolve a ~10 ns per-key delta; the gate can, and it does so
with a 2% line instead of a vibe.

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
| `fuzz-no-roundtrip:pyrs-json` | the JSON family has three writers and no target that re-reads what they emit | a `json_roundtrip` target in the matrix with its own seed directory |
| `fuzz-no-roundtrip:pyrs-toml` | same shape for TOML | a `toml_roundtrip` target in the matrix with its own seed directory |
| `property-tier:default-case-count` | three writer fixed-point properties fail at 20k cases and are invisible at 256 | a blocking job at an elevated case count, which requires those three to hold first |

Closed while this document was written, and therefore absent from the registry on
purpose: the hook tier being unwired, `cargo fmt` reaching no job, CI's clippy skipping
tests and benches, the two YAML writers having no comparison against each other
(`tests/test_route_parity.py` pins their output byte-for-byte and found the same
empty-container spelling still unfixed at the mapping-value site of `direct_dump` —
the second occurrence of the same class, the day after the hole was registered), the
line-ending policy having no enforceable form, and the conflict-marker-shaped hole
above.

## Improvement plan

Ordered by how much defence per unit of work, with the acceptance test named — a plan item
is done when its test is green, not when the change is merged.

- **P0-A, line-ending and hook policy** (`done`): `.gitattributes` declares LF,
  `scripts/check_line_endings.py` enforces the absolute rule, `prek.toml` runs it as a
  hook, `hygiene.yml` runs the hook set in CI, and the polluted files are normalised.
  Acceptance: `tests/test_quality_matrix.py` no longer reports the hook holes, and
  `tests/test_line_endings_gate.py` proves the checker fires on an all-CRLF file.
- **P0-B, property tier elevation** (`blocked by` three writer fixed points): add a
  non-blocking high-case job first, then flip it to blocking as each property closes.
  Acceptance: the `property-tier` hole deleted from the registry.
- **P1-C, matrix gate** (`done`): this document, the measurement script, the registry and
  the test that compares them.
- **P1-D, matrix spaces** (`partly done`): the route-parity table over the two YAML
  writers landed and closed its hole; the JSON and TOML round-trip fuzz targets are
  next. Acceptance: the remaining `fuzz-no-roundtrip` holes deleted from the registry
  in the PRs that close them.
- **P2-F, Ir baseline breadth** (`pending`): regenerate the baseline in the gate's own
  execution environment and add `to_json`, `to_toml` and inline-merge scenarios, so the
  perf gate covers the paths the binding actually exposes.
  Acceptance: scenario count in `.ci/ir-baseline.json` matches the bench, including the new
  ones, and the gate passes with them.

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
