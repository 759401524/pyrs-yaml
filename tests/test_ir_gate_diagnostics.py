"""The Ir gate must quote rustc, not cargo's summary, when a harness build fails.

Measured on 2026-10-21 while retiring the rule validator: the binding harness stopped at a missing
`-lpython3.12`, and the gate printed only

    build failed:
       Compiling pyrs-yaml v0.18.1 (...)
    error: could not compile `pyrs-yaml` (lib) due to 1 previous error

because `build_exe` passes `--message-format=json` - which moves diagnostics to stdout as
`compiler-message` records - and printed `res.stderr` alone. The cause was invisible without
rerunning cargo by hand, which made a red gate cost a reproduction to read. These tests hold the
printed message to the stream that carries it, and keep the stderr fallback for failures that are
not rustc diagnostics at all.
"""

from __future__ import annotations

import importlib.util
import json
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
GATE = REPO_ROOT / "scripts" / "ir_gate.py"
SUMMARY = "error: could not compile `pyrs-yaml` (lib) due to 1 previous error\n"


def _gate():
    spec = importlib.util.spec_from_file_location("ir_gate", GATE)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _compiler_message(rendered: str) -> str:
    return json.dumps({"reason": "compiler-message", "message": {"rendered": rendered}})


def test_a_compile_error_is_reported_from_the_stream_that_carries_it():
    stdout = "\n".join(
        [
            json.dumps({"reason": "compiler-artifact", "executable": None}),
            _compiler_message("error: rust-lld: unable to find library -lpython3.12\n"),
            json.dumps({"reason": "build-finished", "success": False}),
        ]
    )
    text = _gate().build_diagnostics(stdout, SUMMARY)
    assert "unable to find library -lpython3.12" in text
    assert "could not compile" not in text, "the summary is the symptom, not the cause"


def test_a_failure_that_is_not_a_rustc_diagnostic_falls_back_to_stderr():
    # A missing toolchain or an unreadable manifest never emits `compiler-message`.
    text = _gate().build_diagnostics("", "error: no such command: `bench`\n")
    assert "no such command" in text


def test_every_rendered_error_is_reported_not_only_the_first():
    stdout = "\n".join([_compiler_message("error[E0308]: first\n"), _compiler_message("error[E0433]: second\n")])
    text = _gate().build_diagnostics(stdout, SUMMARY)
    assert "E0308" in text and "E0433" in text
