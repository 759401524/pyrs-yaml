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

Usage:
    python scripts/ir_gate.py                 # measure and gate
    python scripts/ir_gate.py --update        # re-measure, rewrite the baseline
    python scripts/ir_gate.py --only serialize_small --only parse_medium
    python scripts/ir_gate.py --tolerance 0.02
    python scripts/ir_gate.py --report-only   # measure, never fail (diagnostics)

Requires valgrind. On this development box run it inside WSL:
    wsl -e bash -lc "python3 /mnt/d/PycharmProjects/pyrs-yaml/scripts/ir_gate.py"
"""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import subprocess
import sys
import tempfile

REPO = pathlib.Path(__file__).resolve().parent.parent
BASELINE = REPO / ".ci" / "ir-baseline.json"
CRATE = "pyrs-yaml-core"
BENCH = "ir_gate"


def run(cmd: list[str], **kw) -> subprocess.CompletedProcess:
    """Never `shell=True`: argument quoting has burned this project before."""
    return subprocess.run(cmd, check=False, text=True, **kw)


def build_exe() -> str:
    """Return the release benchmark binary, via cargo's JSON so no filename
    guessing (hash suffixes change with every build).

    `--features ir-gate` is the only way this target is built: it carries
    `required-features`, which keeps it out of every default build — including
    `cargo codspeed run`, which executes each discovered benchmark with no
    arguments and would otherwise trip the binary's usage error.
    """
    res = run(
        [
            "cargo",
            "bench",
            "-p",
            CRATE,
            "--bench",
            BENCH,
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
        if exe and f"{BENCH}-" in pathlib.Path(exe).name:
            return exe
    sys.exit("cargo did not report an ir_gate executable")


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


def rustc_banner() -> str:
    return run(["rustc", "-V"], capture_output=True).stdout.strip()


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--update", action="store_true", help="rewrite the baseline")
    ap.add_argument("--only", action="append", default=[], help="scenario filter")
    ap.add_argument(
        "--tolerance", type=float, default=0.01, help="allowed growth over baseline (fraction, default 1%%)"
    )
    ap.add_argument("--report-only", action="store_true", help="print measurements, never fail")
    args = ap.parse_args()

    exe = build_exe()
    listed = run([exe, "--list"], capture_output=True).stdout.split()
    scenarios = [s for s in listed if not args.only or s in args.only]
    toolchain = rustc_banner()

    measured = {s: loop_instructions(exe, s) for s in scenarios}

    if args.update:
        BASELINE.parent.mkdir(exist_ok=True)
        payload = {"toolchain": toolchain, "tolerance_hint": args.tolerance, "scenarios": measured}
        BASELINE.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print(f"wrote {BASELINE.relative_to(REPO)} ({len(measured)} scenarios)")
        return 0

    if not BASELINE.exists():
        print(f"no baseline at {BASELINE.relative_to(REPO)}; run --update first")
        return 1

    ref = json.loads(BASELINE.read_text(encoding="utf-8"))
    if args.report_only:
        for s, ir in measured.items():
            print(f"{s:26} {ir:12,}")
        return 0

    failures: list[str] = []
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
        if growth > args.tolerance:
            failures.append(f"{s}: +{growth:.2%} over baseline (tolerance {args.tolerance:.2%})")

    if failures:
        print("\n" + "\n".join(f"FAIL  {f}" for f in failures))
        print("\nIf this change is intended, re-baseline deliberately:\n  python scripts/ir_gate.py --update")
        return 1
    print("OK: every scenario within tolerance of the committed baseline")
    return 0


if __name__ == "__main__":
    sys.exit(main())
