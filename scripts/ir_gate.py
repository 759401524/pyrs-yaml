#!/usr/bin/env python3
"""Instruction-count gate: measure the engine's hot paths in counted
instructions (`callgrind` Ir) and compare them against a committed baseline.

Why: the divan suite reports through CodSpeed, which compares today's runner
against a baseline recorded on different hardware. For sub-10% effects that
number is not reproducible — three pushes that each did strictly *less* work
reported -7.7%, -10.5%, -9.8% for the same benchmark set. `callgrind` Ir repeats
to within 0.004% on the same binary, so a committed baseline can gate a real
one-percent tolerance. See `crates/pyrs-yaml-core/benches/ir_gate.rs` for the
measurement method (setup-only pass subtracted out, fixed iteration count).

Two harnesses feed one baseline: the engine crate's (parse, serialize, the JSON and
TOML writers) and the binding crate's (`to_python_*`, the AST-to-Python conversion
`safe_load` performs). The second one links CPython, so it builds anywhere and runs
only on Linux — measured on Windows, the binary dies at start-up with `0xC000021A`
before printing anything. Run this script where `valgrind` is: the GitHub runner or
WSL, never a bare Windows host.

Usage:
    python scripts/ir_gate.py                 # measure and gate
    python scripts/ir_gate.py --update        # re-measure, rewrite the baseline
    python scripts/ir_gate.py --only serialize_small --only to_python_medium
    python scripts/ir_gate.py --tolerance 0.02
    python scripts/ir_gate.py --report-only   # measure, never fail (diagnostics)

Requires valgrind. On this development box run it inside WSL:
    wsl -e bash -lc "python3 /mnt/d/PycharmProjects/pyrs-yaml/scripts/ir_gate.py"
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import platform
import re
import subprocess
import sys
import tempfile

REPO = pathlib.Path(__file__).resolve().parent.parent
BASELINE = REPO / ".ci" / "ir-baseline.json"
# Two harnesses, one gate. `pyrs-yaml-core` measures parse and serialize; `pyrs-yaml` measures the
# AST-to-Python conversion that users actually call and that the engine harness cannot link at all -
# the gap registered as `perf-coverage:binding-layer`. Both are built the same way and both feed the
# same baseline file, so a scenario from either channel is compared against the same provenance rule.
HARNESSES = (("pyrs-yaml-core", "ir_gate"), ("pyrs-yaml", "ir_gate"))

# Calibrated by measurement, and recalibrated a third time by measurement that refuted the earlier
# explanation.
#
# Engine channel, two refresh runs of this script on two GitHub runner images (20261004.327.1 and
# 20260927.320.1) at one commit: eleven of the twelve scenarios agree to at most 0.0009% (worst
# `parse_anchors`, 3,010 instructions on 341M), and `serialize_block`, `serialize_small` and
# `serialize_block_scalars` agree byte-for-byte. So the line is a real one - 0.5% is ~550x the widest
# observed spread, and a 0.5% instruction regression is a meaningful one.
#
# Binding channel, same two runs: `to_python_small` moved 126,865 instructions on 15.2M - 0.83%, more
# than the tolerance - and `to_python_anchors` 0.045%. CPython's own paths are not as reproducible as
# the engine's at that sample size, so two things changed: the binding harness measures 2 000
# iterations instead of 500 (four times the work, the same order of magnitude as its siblings), and
# every scenario is sampled `--repeats` (3) times with the largest value baselined - a single draw of
# a distribution wider than the line would let a no-op change redden the gate. The run prints each
# scenario's samples and spread, so the claim is auditable in the job log instead of being a sentence
# in this file.
#
# What the 2% used to be: the gap between a baseline generated in WSL and the same code measured on a
# runner, concentrated on `serialize_block_scalars` (+1.45%). Three explanations have been tested and
# all three failed. (1) `.gitattributes` normalising CR bytes inside `BLOCK_SCALAR_YAML`: adding
# `-text` changed the runner's number by 28 instructions out of 16,020,906 - REFUTED, though the probe
# did find a real reproducibility hole (the stored bytes of the measured input depended on which tree
# the last author committed), now closed and asserted by `tests/test_line_endings_gate.py`. (2) Drift
# between runner images: REFUTED, both images report the same numbers. (3) A restored build cache
# making the refresh job compile differently: REFUTED by removing `Swatinem/rust-cache` from
# `.github/workflows/ir-baseline.yml` - the job still reported 15,792,928.
#
# What is left is a difference that correlated perfectly with the *job* and with nothing else: eight
# runs of the refresh job, on two images, cached and uncached, all reported 15,792,8xx-15,792,9xx, and
# two runs of the enforcing job in `codspeed.yml` reported 16,020,942 and 16,020,915 - while every other
# scenario agreed between the two jobs to within 0.08%. Inside each job the number was stable to
# ~30 instructions, so this was not noise. The cause was found with `build_exe`'s binary hashes: both
# jobs measured the *same* bytes (`a0386c17b5f1ef38`), the same valgrind and compiler, on hosts whose
# reported models differed (`AMD EPYC 9V74` against `9V45`) - so the variable was the ISA glibc binds
# for its string routines from the VM's CPUID flags, which `MEASUREMENT_ENV` now pins.
#
# Recorded because the first reading of this data was wrong twice: it looked like proof the committed
# values had been generated off-runner, and then like proof the refresh job's build cache was the cause.
# Both sentences were written into documents before the control runs refuted them, and both corrections
# are here rather than edited out. What remains unexplained is narrower than it looks: why the two
# hosts' CPUID masks differ at all, and why only the copy-heavy scenario notices it.
#
# Because the mechanism is a CPUID mask rather than a code path, a marginal failure on a copy-heavy
# scenario is still a hint about the environment and never a licence to widen the line: re-measure with
# `gh workflow run 'Ir baseline refresh'`, and read the samples, the hashes and the printed env before
# believing a regression.
DEFAULT_TOLERANCE = 0.005

# Environment every measured process runs under, to make the count belong to the code instead of to
# the hypervisor. Two jobs in this repository ran the *same* binary (sha256 `a0386c17b5f1ef38`, printed
# by `build_exe`), the same valgrind 3.22.0, the same pinned rustc 1.97.1, the same `nproc`, on hosts
# both reporting "AMD EPYC 9V74 80-Core Processor", and still counted `serialize_block_scalars` 1.44%
# apart: 15,792,928 in the refresh job against 16,020,942 in the enforcing one, each stable to ~30
# instructions inside its own run. Same bytes, same toolchain, same model name - the remaining
# candidate is the ISA that glibc selects for its string routines at start-up, which comes from the
# CPUID flags a VM exposes and can differ between hosts of the same model. `serialize_block_scalars`
# is the most memcpy-heavy scenario in the set, and it is the only one that moves.
#
# Masking the vector caps makes the choice deterministic - and it is confirmed, not assumed: the two
# jobs now differ by two instructions on that scenario (15,780,927 in the refresh job against
# 15,780,929 in the enforcing one, on hosts of different models), where with the caps unmasked they were
# 1.44% apart. The line was not widened to absorb the host; the input the host controlled was pinned.
MEASUREMENT_ENV = {"GLIBC_TUNABLES": "glibc.cpu.hwcaps=-AVX512F,-AVX2,-AVX,-SSE4_2,-POPCOUNT"}

# Written into every baseline this script generates, verbatim. The first committed baseline carried a
# hand-written `generated_by.note` explaining where its numbers came from; `--update` does not write
# prose, so the refresh job's artifact silently dropped that paragraph, and a file that is part
# transcription and part measurement cannot be re-generated faithfully. The explanation now lives in
# `QUALITY_MATRIX.md` (section 2), and `tests/test_ir_baseline_workflow.py` pins the committed keys to
# exactly what `--update` writes.
PROVENANCE_NOTE = (
    "Written by `scripts/ir_gate.py --update`; no value in this file is transcribed. The reasoning "
    "behind the tolerance, and the measurements that sized it, are in QUALITY_MATRIX.md section 2."
)

# How many times each scenario is measured before one number is kept, and why it is not 1: the binding
# channel was measured twice on two runner images, and `to_python_small` moved 0.83% (126,865
# instructions on 15.2M) while every engine scenario but one agreed inside 0.001%. A baseline that is a
# single sample of a 0.8%-wide distribution would let a no-op change trip the 0.5% line, so the
# committed value is the *largest* of `--repeats` runs: normal variation cannot exceed its own
# envelope, and a real regression still has to clear it. The count is recorded in the baseline's
# `generated_by` because max-of-3 enforced against a single-run number is a different gate.
DEFAULT_REPEATS = 3


def environment() -> str:
    """A one-line description of where this measurement came from.

    Recorded in the baseline so the two cannot be read apart later. `ImageOS`/`ImageVersion` are set
    by GitHub Actions runners; a local run gets the host platform instead, which is how a WSL- or
    macOS-generated baseline becomes visible in the gate's own output rather than silently trusted.
    """
    image = os.environ.get("ImageOS")  # noqa: SIM112 - the name comes from the runner, not from us
    version = os.environ.get("ImageVersion")  # noqa: SIM112 - ditto; uppercasing it reads a blank
    if image and version:
        return f"github-runner {image}, image {version}"
    return f"{platform.system()} {platform.release()} ({platform.machine()})"


def run(cmd: list[str], **kw) -> subprocess.CompletedProcess:
    """Never `shell=True`: argument quoting has burned this project before."""
    return subprocess.run(cmd, check=False, text=True, **kw)


def build_exe(crate: str, bench: str) -> str:
    """Return the release benchmark binary, via cargo's JSON so no filename
    guessing (hash suffixes change with every build).

    `--features ir-gate` is the only way this target is built: it carries
    `required-features`, which keeps it out of every default build — including
    `cargo codspeed run`, which executes each discovered benchmark with no
    arguments and would otherwise trip the binary's usage error.

    The binary's SHA-256 goes to stderr so it lands in the job log. Two jobs that
    pin the same compiler and the same sources were seen to report 1.44% apart on
    one scenario, and the cheapest way to tell "different bytes" from "same bytes on
    different hardware" is to print the bytes' hash.
    """
    res = run(
        [
            "cargo",
            "bench",
            "-p",
            crate,
            "--bench",
            bench,
            "--features",
            "ir-gate",
            "--no-run",
            "--message-format=json",
        ],
        capture_output=True,
        cwd=REPO,
    )
    if res.returncode != 0:
        sys.exit(f"build failed:\n{res.stderr[-2000:]}")
    for line in res.stdout.splitlines():
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            continue
        exe = msg.get("executable")
        if exe and f"{bench}-" in pathlib.Path(exe).name:
            digest = hashlib.sha256(pathlib.Path(exe).read_bytes()).hexdigest()[:16]
            print(f"{crate}/{bench} exe={pathlib.Path(exe).name} sha256={digest}", file=sys.stderr)
            return exe
    sys.exit(f"cargo did not report an {bench} executable for {crate}")


def callgrind_total(exe: str, args: list[str]) -> int:
    """Run `exe args` under callgrind and return the process-wide Ir total."""
    with tempfile.TemporaryDirectory() as tmp:
        out = pathlib.Path(tmp) / "cg.out"
        res = run(
            [
                "valgrind",
                "--tool=callgrind",
                f"--callgrind-out-file={out}",
                "--quiet",
                "--quiet",
                exe,
                *args,
            ],
            capture_output=True,
            env={**os.environ, **MEASUREMENT_ENV},
        )
        if res.returncode != 0:
            sys.exit(f"valgrind failed on {args}:\n{res.stderr[-1500:]}")
        text = out.read_text(encoding="utf-8", errors="replace")
    totals = re.findall(r"^summary:\s+(\d+)", text, re.MULTILINE)
    if not totals:
        sys.exit(f"no callgrind summary for {args}")
    return int(totals[-1])


def loop_instructions(exe: str, scenario: str) -> int:
    """Counted instructions of the measured loop alone: same binary, twice, and
    the setup-only pass subtracted out."""
    full = callgrind_total(exe, [scenario])
    setup = callgrind_total(exe, [scenario, "--setup-only"])
    return full - setup


def loop_samples(exe: str, scenario: str, repeats: int) -> list:
    """Every measurement taken for one scenario, so the spread stays visible."""
    return [loop_instructions(exe, scenario) for _ in range(max(1, repeats))]


def resolve_repeats(explicit, recorded, default: int = DEFAULT_REPEATS) -> int:
    """Which sample size this run uses: `--repeats`, then the baseline's, then the default.

    The recorded count wins over the default because comparing a single sample against a max-of-3
    envelope (or the reverse) silently changes what the tolerance means. A missing or zero value on
    either side is skipped rather than trusted - a gate that takes zero samples of its own baseline
    would pass on nothing, and this project has already been bitten once by an empty set that read as
    a clean result.
    """
    for value in (explicit, recorded):
        try:
            number = int(value)
        except (TypeError, ValueError):
            continue
        if number > 0:
            return number
    return max(1, int(default))


def rustc_banner() -> str:
    return run(["rustc", "-V"], capture_output=True).stdout.strip()


def rustc_short(banner: str) -> str:
    """Just the version token, so the baseline's two toolchain fields cannot disagree."""
    parts = banner.split()
    return parts[1] if len(parts) > 1 else banner


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--update", action="store_true", help="rewrite the baseline")
    ap.add_argument("--only", action="append", default=[], help="scenario filter")
    ap.add_argument(
        "--tolerance",
        type=float,
        default=None,
        help="allowed growth over baseline; defaults to the baseline's tolerance_hint",
    )
    ap.add_argument("--report-only", action="store_true", help="print measurements, never fail")
    ap.add_argument(
        "--repeats",
        type=int,
        default=None,
        help=(
            f"measure each scenario N times and keep the largest (default {DEFAULT_REPEATS}, "
            "or the baseline's recorded count)"
        ),
    )
    args = ap.parse_args()

    # One build per harness, one scenario -> binary map across both. A scenario name is unique by
    # convention (`to_python_*` on the binding side), and a collision would silently measure the wrong
    # channel, so it is checked rather than assumed.
    exes: dict[str, str] = {}
    for crate, bench in HARNESSES:
        exe = build_exe(crate, bench)
        listed_proc = run([exe, "--list"], capture_output=True)
        names = listed_proc.stdout.split()
        if listed_proc.returncode != 0 or not names:
            sys.exit(
                f"{crate}/{bench} listed nothing (exit {listed_proc.returncode}).\n"
                f"{listed_proc.stderr.strip()[-500:]}\n"
                "An unreadable harness is not an empty one: treating it as such would let `--update`"
                " rewrite the committed baseline without that channel's scenarios, and let the gate"
                " pass by comparing nothing. This binary links CPython, so on a host without a"
                " discoverable interpreter it fails at start-up - run the gate on Linux (WSL or CI)."
            )
        for name in names:
            if name in exes:
                sys.exit(f"scenario {name!r} is listed by two harnesses; the measurement would be ambiguous")
            exes[name] = exe
    scenarios = [s for s in exes if not args.only or s in args.only]
    toolchain = rustc_banner()
    env_note = ", ".join(f"{key}={value}" for key, value in sorted(MEASUREMENT_ENV.items()))
    print(f"measurement env: {env_note}")

    # A repeated measurement is part of the baseline's provenance, not a local knob: comparing a
    # single sample against a max-of-3 envelope (or the reverse) silently changes what the tolerance
    # means. An explicit `--repeats` still wins, for diagnostics.
    recorded = None
    if BASELINE.exists():
        try:
            recorded = json.loads(BASELINE.read_text(encoding="utf-8")).get("generated_by", {}).get("repeats")
        except (json.JSONDecodeError, OSError):
            recorded = None
    repeats = resolve_repeats(args.repeats, recorded)

    drawn = {s: loop_samples(exes[s], s, repeats) for s in scenarios}
    measured = {s: max(drawn[s]) for s in scenarios}

    # The spread of the samples is the evidence behind the tolerance and the sample size, so the job
    # log carries it. Without this, "0.5% is a real line" is a sentence rather than a number.
    if repeats > 1:
        print(f"{'scenario':26} {'spread':>8}  samples")
        for s in scenarios:
            values = drawn[s]
            spread = (max(values) - min(values)) / max(values)
            print(f"{s:26} {spread:>7.3%}  " + ", ".join(f"{v:,}" for v in values))

    if args.update:
        # `--update --only X` used to write a baseline containing *only* X: every other
        # scenario's number vanished, and the next gate run reported them as unbaselined.
        # A partial re-measurement therefore merges into what is committed, and refuses to
        # pretend it knows a number it did not measure. A scenario the bench no longer
        # lists is dropped rather than kept as decoration.
        previous: dict[str, int] = {}
        if args.only:
            if not BASELINE.exists():
                sys.exit(f"--update --only needs the existing baseline at {BASELINE}; it is missing")
            previous = dict(json.loads(BASELINE.read_text(encoding="utf-8"))["scenarios"])
        listed = sorted(exes)
        final: dict[str, int] = {}
        for name in listed:
            if name in measured:
                final[name] = measured[name]
            elif name in previous:
                final[name] = previous[name]
            else:
                sys.exit(f"baseline would be incomplete: no number for {name!r} - drop --only or measure it too")
        BASELINE.parent.mkdir(exist_ok=True)
        payload = {
            "toolchain": toolchain,
            "generated_by": {
                "environment": environment(),
                "rustc": rustc_short(toolchain),
                "tool": "callgrind",
                "repeats": repeats,
                "note": PROVENANCE_NOTE,
            },
            "tolerance_hint": args.tolerance if args.tolerance is not None else DEFAULT_TOLERANCE,
            "scenarios": final,
        }
        BASELINE.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8", newline="\n")
        print(f"wrote {BASELINE.relative_to(REPO)} ({len(final)} scenarios)")
        return 0

    if not BASELINE.exists():
        print(f"no baseline at {BASELINE.relative_to(REPO)}; run --update first")
        return 1

    ref = json.loads(BASELINE.read_text(encoding="utf-8"))
    origin = ref.get("generated_by", {}).get("environment", "unrecorded")
    here = environment()
    print(f"baseline generated by: {origin}")
    if origin != "unrecorded" and origin != here:
        print(
            f"  note: this run is on {here!r}, which is not the recorded environment. "
            "A WSL-generated number was measured 1.45% off the runner's on "
            "`serialize_block_scalars`; treat a marginal failure as a reason to re-measure."
        )
    if args.report_only:
        for s, ir in measured.items():
            print(f"{s:26} {ir:12,}")
        return 0

    print(f"each scenario is the largest of {repeats} measurements")
    failures: list[str] = []
    # The tolerance travels with the baseline: a hint recorded at generation time
    # is what the enforcing run should honour, otherwise a change to either file
    # silently re-tunes the gate.
    tol = args.tolerance if args.tolerance is not None else float(ref.get("tolerance_hint", DEFAULT_TOLERANCE))
    if ref.get("toolchain") != toolchain:
        failures.append(
            f"toolchain changed: baseline recorded on {ref.get('toolchain')!r}, "
            f"now {toolchain!r}; re-run --update deliberately"
        )
    print(f"{'scenario':26} {'baseline':>13} {'now':>13} {'delta':>9}")
    for s, ir in measured.items():
        base = ref["scenarios"].get(s)
        if base is None:
            failures.append(f"{s}: not in baseline (added without --update)")
            print(f"{s:26} {'-':>13} {ir:13,} {'new':>9}")
            continue
        growth = (ir - base) / base
        print(f"{s:26} {base:13,} {ir:13,} {growth:+9.2%}")
        if growth > tol:
            failures.append(f"{s}: +{growth:.2%} over baseline (tolerance {tol:.2%})")

    if failures:
        print("\n" + "\n".join(f"FAIL  {f}" for f in failures))
        print("\nIf this change is intended, re-baseline deliberately:\n  python scripts/ir_gate.py --update")
        return 1
    print("OK: every scenario within tolerance of the committed baseline")
    return 0


if __name__ == "__main__":
    sys.exit(main())
