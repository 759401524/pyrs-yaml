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

## The shape chosen

The environment becomes part of the floor's key instead of a footnote next to it.

1. The baseline grows an `environments` table. Only scenarios whose counted region includes the
   interpreter are keyed there; every other scenario keeps its single committed number, because a
   key nobody needs is a key that can go stale on its own.
2. The comparing path reads its own `environment()` string and takes the floor from that entry. An
   entry for an image never measured is not a pass and not a regression: the gate exits 2, which
   this repository already reserves for a check that could not run. The remedy is one refresh
   dispatch, and the job prints which key it wanted.
3. `--update` merges into the entry named by the environment it is running in rather than rewriting
   the table, so a refresh on a new image adds coverage instead of silently withdrawing coverage
   from the images it did not run on. The refusal of an incomplete baseline stays as it is.
4. `--accept-drift` keeps naming scenarios, unchanged: drift is still judged within one image, which
   is the only place the comparison means anything.
5. `scripts/check_ir_floor_quotes.py` and `QUALITY_MATRIX.md` section 2 quote floor values, so the
   quote check gains the image it is quoting from — a number lifted from one image and read against
   another is the exact misreading that marker was written to stop.
6. Each of those rules gets a falsification test in `tests/test_ir_gate_provenance.py`, in the house
   style: a mutation per rule that names the assertion it reddens, and a clean file restored
   byte-for-byte after.

## What this page does not claim

It does not claim a keyed floor has been measured: nothing has been run against a two-key baseline
yet, so the first implementation PR must bring its own readings from two images before the table can
hold two rows. It does not claim the pool has exactly two live images — the jobs sampled named two,
which bounds only the sample. And it does not claim the binding rows will settle once keyed:
the 2.97 % spread was one pair of images, and the tolerance stays at the committed `tolerance_hint`
unless a measurement says otherwise.
