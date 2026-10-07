"""Tests for the fuzz round harness (`scripts/fuzz_rounds.sh`).

The harness is the piece of the quality line that had already broken twice without anyone
noticing: it stopped the round loop on a repeated panic signature, so a second root cause
reaching the same `assert_eq!` was neither counted nor archived (measured: one run reported
"1 distinct crash signature" for `crash-55c199ef` and `crash-5561902a`, two bugs), and the
aggregate report read a `.signatures` dotfile at the archive root, where it has never been
extracted, so that column silently said 0. A shell script that decides whether a merge is
allowed needs the same treatment as engine code — a stub that reproduces each behaviour on
demand, and an assertion per behaviour.

The stub replaces `cargo` on `PATH` and replays a plan file: one line per round, either
`clean` or `<artifact-name>|<bytes>|<panic line>`.

Linux only, deliberately. `scripts/fuzz_rounds.sh` is a POSIX shell harness (bash, seq, cmp,
sed, wc) and CI runs it on ubuntu, which is where these assertions execute. Windows' `bash.exe`
is the WSL launcher and drops the positional arguments a `-c` script needs (measured:
`ARG=[]`), and a test that guesses at mount prefixes (`/c/` vs `/mnt/c/`) is the fragile kind
that fails silently rather than loudly — so the gate is the platform, not the path maths.
Run it locally with the repository's Linux venv:
    wsl -e bash -lc "cd /mnt/d/PycharmProjects/pyrs-yaml && .venv-linux/bin/pytest tests/test_fuzz_rounds.py -q"
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
SCRIPT = REPO_ROOT / "scripts" / "fuzz_rounds.sh"
TARGET = "yaml_roundtrip"

BASH = shutil.which("bash")
pytestmark = pytest.mark.skipif(
    sys.platform != "linux" or BASH is None,
    reason="POSIX shell harness: runs on CI's ubuntu matrix (see the module docstring for the local WSL route)",
)

STUB = """#!/usr/bin/env bash
set -u
calls="$STUB_DIR/calls.txt"
n=$(($(cat "$calls" 2>/dev/null || echo 0) + 1))
echo "$n" > "$calls"
printf 'call %s:' "$n" >> "$STUB_LOG"
for a in "$@"; do printf ' %s' "$a" >> "$STUB_LOG"; done
printf '\\n' >> "$STUB_LOG"
line=$(sed -n "${n}p" "$STUB_PLAN")
case "$line" in
  ""|clean) exit 0 ;;
esac
name=${line%%|*}
rest=${line#*|}
body=${rest%%|*}
sig=${rest#*|}
mkdir -p "$STUB_ARTIFACTS"
printf '%s' "$body" > "$STUB_ARTIFACTS/$name"
echo "$sig" >&2
exit 1
"""


def build(tmp_path: Path, plan: list[str], corpus: dict[str, str] | None = None) -> Path:
    """Lay out a repo skeleton the harness can be run against."""
    (tmp_path / "scripts").mkdir(parents=True)
    shutil.copy(SCRIPT, tmp_path / "scripts" / "fuzz_rounds.sh")
    fuzz = tmp_path / "fuzz"
    (fuzz / "artifacts" / TARGET).mkdir(parents=True)
    (fuzz / "collected" / TARGET).mkdir(parents=True)
    corpus_dir = fuzz / "corpus" / TARGET
    corpus_dir.mkdir(parents=True)
    for name, body in (corpus or {}).items():
        (corpus_dir / name).write_text(body, encoding="utf-8")
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    stub = bin_dir / "cargo"
    stub.write_text(STUB, encoding="utf-8")
    stub.chmod(0o755)
    (tmp_path / "plan.txt").write_text("".join(f"{line}\n" for line in plan), encoding="utf-8")
    return tmp_path


def run(tmp_path: Path, **env: str) -> subprocess.CompletedProcess[str]:
    settings = os.environ.copy()
    settings.update(
        {
            "PATH": f"{tmp_path / 'bin'}:{settings.get('PATH', '')}",
            "STUB_DIR": str(tmp_path),
            "STUB_LOG": str(tmp_path / "calls.log"),
            "STUB_PLAN": str(tmp_path / "plan.txt"),
            "STUB_ARTIFACTS": str(tmp_path / "fuzz" / "artifacts" / TARGET),
            "FUZZ_TIME": "30",
            "FUZZ_ROUNDS": "3",
            "FUZZ_CEILING_MINUTES": "22",
            "GITHUB_STEP_SUMMARY": str(tmp_path / "summary.md"),
            "TMPDIR": str(tmp_path),
        }
    )
    settings.pop("GITHUB_EVENT_NAME", None)
    settings.update(env)
    return subprocess.run(
        ["bash", str(tmp_path / "scripts" / "fuzz_rounds.sh"), TARGET],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        env=settings,
        shell=False,
        cwd=str(tmp_path),
    )


def crash(name: str, body: str, site: str = "yaml_roundtrip.rs:37") -> str:
    # One plan line per round, so a body must not contain a newline; the harness treats the
    # artifact as opaque bytes anyway.
    assert "\n" not in body and "|" not in body, body
    return f"{name}|{body}|panicked at fuzz/fuzz_targets/{site}"


def test_two_inputs_reaching_one_signature_are_both_reported(tmp_path: Path) -> None:
    """The defect this harness had: a repeat signature stopped the loop and hid a second bug."""
    build(
        tmp_path,
        [
            crash("crash-aaa", "body-a"),
            crash("crash-bbb", "body-b"),
            crash("crash-aaa", "body-a"),  # same bytes again: no progress
        ],
    )
    proc = run(tmp_path)
    assert proc.returncode == 1, proc.stdout + proc.stderr
    assert "2 crash input(s) across 1 distinct signature(s)" in proc.stdout, proc.stdout
    collected = {p.name for p in (tmp_path / "fuzz" / "collected" / TARGET).glob("crash-*")}
    assert collected == {"crash-aaa", "crash-bbb"}, collected
    findings = (tmp_path / "fuzz" / "collected" / TARGET / "findings.tsv").read_text(encoding="utf-8")
    assert len([line for line in findings.splitlines() if line]) == 2, findings
    assert "nothing new" in proc.stdout, proc.stdout


def test_clean_round_stops_and_exits_zero(tmp_path: Path) -> None:
    build(tmp_path, ["clean", "clean", "clean"])
    proc = run(tmp_path)
    assert proc.returncode == 0, proc.stdout + proc.stderr
    assert "round 1: clean" in proc.stdout
    assert "no crash in 3 round(s)" in proc.stdout


def test_budget_over_the_ceiling_is_a_configuration_error(tmp_path: Path) -> None:
    """Exit 2 must stay distinguishable from exit 1, or arithmetic reads as a finding."""
    build(tmp_path, ["clean"])
    proc = run(tmp_path, FUZZ_TIME="99999")
    assert proc.returncode == 2, proc.stdout + proc.stderr
    assert "config error" in proc.stderr + proc.stdout


def test_pull_request_mode_replays_the_corpus_once(tmp_path: Path) -> None:
    """The merge-blocking half: deterministic, one invocation, no exploration."""
    build(tmp_path, [crash("crash-aaa", "x")])
    proc = run(tmp_path, GITHUB_EVENT_NAME="pull_request")
    calls = (tmp_path / "calls.log").read_text(encoding="utf-8")
    assert len([line for line in calls.splitlines() if line]) == 1, calls
    assert "-runs=0" in calls, calls
    assert "mode=replay" in proc.stdout
    assert proc.returncode == 1, proc.stdout


def test_collected_input_is_removed_from_the_corpus(tmp_path: Path) -> None:
    """Otherwise the next round re-finds the same bug and the run reports one bug twice."""
    body = "b: ! #&"
    build(tmp_path, [crash("crash-aaa", body), "clean"], corpus={"seed1": body, "seed2": "other"})
    proc = run(tmp_path)
    assert proc.returncode == 1, proc.stdout + proc.stderr
    remaining = sorted(p.name for p in (tmp_path / "fuzz" / "corpus" / TARGET).iterdir())
    assert remaining == ["seed2"], remaining


def test_summary_records_every_round(tmp_path: Path) -> None:
    build(tmp_path, [crash("crash-aaa", "one"), crash("crash-bbb", "two", site="other.rs:1")])
    proc = run(tmp_path)
    assert proc.returncode == 1, proc.stdout + proc.stderr
    summary = (tmp_path / "summary.md").read_text(encoding="utf-8")
    assert "round 1" in summary and "round 2" in summary, summary
    assert "crash-aaa" in summary and "crash-bbb" in summary, summary
    assert "2 crash input(s) / 2 distinct signature(s)" in proc.stdout, proc.stdout
