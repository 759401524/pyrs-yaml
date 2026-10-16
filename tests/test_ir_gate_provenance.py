"""The gate names the compiler term before its numbers can be read as a verdict.

Measured on 2026-10-16 with one tree on one WSL host, changing only the toolchain: on the baseline's
pinned `rustc 1.97.1` no scenario exceeded the tolerance (worst `to_python_medium` +0.16%, nineteen of
twenty-two inside +-0.003%), and on the box's default `rustc 1.99.0` five did - `serialize_medium`
+9.08%, `serialize_block` +8.86%, `to_toml_medium` +3.55% - while `serialize_block_scalars` moved the
other way, -7.71%. Both signs at that size is codegen, and a run that says nothing about it reads as a
regression someone has to re-derive from scratch. The enforcing job installs the pin out of the
baseline; the note exists so a local run finds out it did not.
"""

from __future__ import annotations

import importlib.util
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
GATE = REPO_ROOT / "scripts" / "ir_gate.py"


def _gate():
    spec = importlib.util.spec_from_file_location("ir_gate", GATE)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_the_same_build_needs_no_explanation():
    assert _gate().toolchain_note("rustc 1.97.1 (8bab26f4f 2026-07-14)", "rustc 1.97.1 (8bab26f4f 2026-07-14)") is None
    # A baseline written before the field existed is not a mismatch; it is an unknown.
    assert _gate().toolchain_note(None, "rustc 1.99.0 (b940084d7 2026-09-28)") is None


def test_a_newer_compiler_is_named_as_a_different_build_not_a_slower_host():
    note = _gate().toolchain_note("rustc 1.97.1 (8bab26f4f 2026-07-14)", "rustc 1.99.0 (b940084d7 2026-09-28)")
    assert note is not None
    assert "1.97.1" in note and "1.99.0" in note
    assert "different code" in note and "not a slower host" in note
    # The note has to be actionable, or it is a warning about arithmetic.
    assert "RUSTUP_TOOLCHAIN=1.97.1" in note


def test_a_rebuild_of_the_same_version_is_not_reported_as_a_version_change():
    note = _gate().toolchain_note("rustc 1.97.1 (8bab26f4f 2026-07-14)", "rustc 1.97.1 (deadbeef 2026-07-14)")
    assert note is not None
    assert "the same version, a different build" in note
    assert "RUSTUP_TOOLCHAIN" not in note, "the pin was honoured; pointing at it again would mislead"


def test_the_run_time_note_no_longer_quotes_a_stale_residual():
    """The sentence this replaces named a host term that the same measurement now contradicts.

    The gate used to tell a local reader that a WSL number had once been +1.45% off on
    `serialize_block_scalars`; measured against the pinned compiler that scenario now reads -7.71% on
    `rustc 1.99.0`, so the figure was never the host's - and a note that quotes a number nobody can
    reproduce teaches the reader to distrust the wrong term. The size belongs in a dated section of
    `docs/dev/perf.md`, where a reader can find the run it came from.
    """
    text = GATE.read_text(encoding="utf-8")
    assert "A WSL-generated number was measured 1.45% off" not in text
    assert "docs/dev/perf.md" in text
