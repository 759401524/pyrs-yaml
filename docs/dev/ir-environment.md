# The Ir gate's environment

The instruction-count gate compares counted instructions against `.ci/ir-baseline.json`. The file is
generated on a GitHub runner by `scripts/ir_gate.py --update` and enforced on a GitHub runner by the
same script without the flag, and `environment()` records which of the two wrote it as
`github-runner {ImageOS}, image {ImageVersion}`. The comparing path prints a note when the recorded
string is not the current one and then judges anyway. This page records what that difference was
measured to mean, and the shape the gate is going to use for it.

Related documents: [performance status](perf.md), [quality ledger](quality-ledger.md).

## What one job says about itself

Three readings, each taken from the job's own saved log rather than from memory:

| reading | refresh run 87 | enforcing leg of PR #659 |
| --- | --- | --- |
| runner-image banner | `20261025.353.1` | `20261012.334.1` |
| `ImageVersion` that `environment()` returned | `20261026.354.1` | `20261023.334.3` |

Within one job the two identifiers disagree — by a day in the first column and by two patch digits
in the second — and each of them agrees with itself when compared to the same source in another job.
So a claim about the environment is only admissible between like sources: an `ImageVersion` against
an `ImageVersion`, a banner against a banner. The field is not noise; it is a second name for the
same thing, and the gate has always read the right one for its own comparison.

## What moves with it

Refresh run 87 and the enforcing leg of PR #659 measured the same tree with the same pinned
`rustc 1.97.1`, the same valgrind, and the same two binary digests
(`3376c9d821eea00d`, `ce6804e2b95d37b1`):

| scenario family | difference between the two jobs |
| --- | --- |
| ten `to_*` writer floors | at most 120 instructions, against floors of 17-28 million |
| `load_toml_medium`, `load_toml_long_strings` | 1.71 % and 2.97 % |
| `to_python_*` | 0.07 % |

The binding harness is the one that counts an interpreter as well as the engine, because its loop
calls `import`ed Python and pays for `PyDict` and `PyObject` work inside the measured region. That
half is what tracks the image. The engine harness calls nothing outside the crate and did not move.
The pinned `GLIBC_TUNABLES` ISA mask closes the string-routine term — see `(gc)` in the ledger — and
does not reach the interpreter's allocator selection, which is the surviving term here.

## Why "refuse unless the image matches" is not implementable

The two jobs landing on different pool images is the observed normal case, not an exception: both
refresh runs of PR #659 wrote `20261026.354.1` as their own environment, while the enforcing leg of
that same pull request read `20261023.334.3`. A gate that declines to answer on that difference
declines on almost every
pull request. And a declined answer is not neutral here: `scripts/check_matrix_verdict.py` counts
only `success`, and `Test matrix (all legs)` is the single required check name, so the refusal is a
red required check and no pull request merges until a refresh happens to run on the image the pool
is handing out. This repository has already measured that shape and named it a deadlock rather than
a pass.

## The shape, as it reads in the tree now

<!-- landed: scripts/ir_gate.py::image_floors -->

The environment is part of the floor's key rather than a footnote next to it.

1. `.ci/ir-baseline.json` carries an `image_floors` table keyed by the string `environment()`
   returns, holding only the scenarios whose counted region includes the interpreter — the set
   `keyed_by_image()` derives from the harness map the measurement itself builds, so a scenario that
   changes harness re-classifies itself instead of going stale in two places. Every other scenario
   keeps its single committed number, because a key nobody needs is a key that can go stale on its
   own.
2. The comparing path takes its floor from the row named by its own environment (`image_floors()`)
   and prints which scenarios it took from the row. A table with no row for this environment is
   neither a pass nor a regression: the gate exits 2 and names the key it wanted, so one refresh
   dispatch closes it. A row that exists but omits a scenario the harness keys refuses the same way
   — that scenario would otherwise be judged against the flat table, which is a number from
   whichever image last refreshed it, exactly the comparison the key exists to prevent.
   Both refusals apply to a run that names a runner image, because that is the only thing a row can
   describe. A host outside that channel — the WSL run a contributor uses to ask whether their own
   change cost instructions — keeps judging the flat table and prints that the interpreter rows are
   not its floor. Refusing there would take the reading away from the one place it was always
   indicative, including for the scenarios whose numbers no image affects.
3. `--update` folds its numbers into its own row through `merge_image_row()` instead of rewriting
   the table, so a refresh on a new image adds coverage rather than withdrawing the rows of the
   images it did not run on; under `--only` the row keeps the numbers that run did not measure. A
   host that names no image writes no row at all — a row's key is an image, and filing a platform
   string there would create a key no enforcing leg can ever ask for.
4. A baseline with no table at all is judged exactly as it was before. That is deliberate: refusing
   on an absent table would have made the first run after this red on every pull request, because a
   refusal is not a `success` and `Test matrix (all legs)` is the one required name.
5. `--accept-drift` still names scenarios and still judges drift within one image, which is the only
   place that comparison means anything.
6. The shape contract reads the key the same way: every row names a runner image, an origin naming
   an image must carry its row, and an origin naming a host must not appear as one. The assertion
   that opened this section - "the recorded environment is in the table" - was written before the
   narrowing and would have demanded a row the gate now refuses to write, so a locally re-baselined
   file would have failed a test that claims only to check the shape.
7. Each rule has a falsification test in `tests/test_ir_gate_provenance.py` and the file shape is
   asserted in `tests/test_ir_baseline_workflow.py`: nine mutations, each reddening exactly the one
   test that owns it — the refusal never firing, the absent table refusing, the row being ignored,
   an incomplete row comparing anyway, `--update` replacing the table, a local host refused, no host
   counting as an image, a non-image run filing a row, and the shape contract restored to its
   unconditional form — with the file under test restored byte-for-byte afterwards.

Two rows are in the table today, both read out of job logs rather than typed: `20261023.334.3` from
the refresh run `40636638961` whose artifact wrote the current floor, and `20261026.354.1` carried
through that run from the job which last measured it. `load_toml_medium` reads 142,467,241 and
144,902,515 between them, which is the 1.71 % a single floor could not hold.

What the marker rule closes: `scripts/check_ir_floor_quotes.py` compares a marked reading against
the row the mark names, so a paragraph quoting an interpreter-counting floor has to say which
image's floor it means — `load_toml_medium @ 20261023.334.3` — and a keyed mark that names no image,
or names an image the file has no row for, is refused instead of being resolved against the flat
column. <!-- landed: scripts/check_ir_floor_quotes.py::version_of --> That distinction is not
pedantry: which image the flat column describes is a property of whichever run the last refresh
happened to land on, not of the sentence quoting it, and the two rows for `load_toml_medium` differ
by 1.71 %. A scenario the file does not key keeps the bare mark, and giving it an image is a refusal
too — there is no row to name. `tests/test_ir_floor_quote_gate.py` forces all four shapes, and the
four mutations of the checker that would let each one through were run against the file to confirm
the tests are sensitive to them.

## What this page does not claim

It does not claim the pool has exactly two live images — the jobs sampled named two, and a third
image is refused until one refresh adds its row. Nor are the rows offered as an easier verdict: on
`20261026.354.1` the row for `load_toml_medium` is 142,467,241 where the flat table now carries
144,902,515 - the `334.3` number, because `334.3` is the image that wrote this file - so keying
tightens the `354.1` floor by 1.71 % rather than excusing it.

What it does claim is measured: a CI leg has taken its floor from a row. PR #662's own `ir` leg ran
on `20261023.334.3`, printed all seven keyed names and `OK: every scenario within tolerance`, with
three loader scenarios at +0.000 % of their own row and `load_json_medium` at +0.45 % — inside the
line, ninety per cent of it, and recorded with its attribution in the ledger rather than smoothed
over. That row has since been paid on the same image by the refresh named above
(`57,111,529 → 57,368,032`), which leaves the enforcing reading 0.003 % from its floor; `(hk)` keeps
both numbers and the fact that the sibling refresh on `354.1` is not in this file. One leg is not a
distribution: what the line will keep absorbing per image is still a question each refresh answers
for itself.
