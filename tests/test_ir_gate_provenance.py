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


def test_a_floor_the_tree_has_outgrown_is_reported_with_its_size():
    warned, stale = _gate().floor_staleness(
        {"cheap": 1_000_000, "grew": 1_000_000, "flat": 1_000_000},
        {"cheap": 970_000, "grew": 1_010_000, "flat": 1_000_000},
    )
    assert warned == [("cheap", (1_000_000 - 970_000) / 1_000_000)]
    assert stale == [], "3% is drift to name; the refusal line is 5%"


def test_a_floor_five_percent_out_of_date_refuses_to_be_called_a_baseline():
    warned, stale = _gate().floor_staleness(
        {"near": 1_000_000, "far": 1_000_000},
        {"near": 979_000, "far": 949_000},
    )
    assert [name for name, _ in warned] == ["near"], "2.1% is on the warning side"
    assert [name for name, _ in stale] == ["far"], "5.1% is past the refusal line"


def test_the_warning_line_is_a_boundary_and_a_missing_measurement_is_not_a_verdict():
    warned, stale = _gate().floor_staleness(
        {"exactly_two": 1_000_000, "not_measured": 1_000_000},
        {"exactly_two": 980_000},
    )
    assert warned == [] and stale == []


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


def test_every_measured_process_runs_under_both_environment_pins():
    """The two inputs a measured process can pick for itself are named, not assumed.

    `GLIBC_TUNABLES` closed the 1.44% cross-host term; `PYTHONHASHSEED` closes the term the binding
    harness drew from the interpreter, which spread `load_json_long_strings` 1.121% over three refresh
    runs on one binary digest while the engine scenarios on the same bytes agreed inside 0.00001%.
    Dropping either key makes the tolerance describe a distribution rather than the code, so this
    fails by naming what went missing instead of letting a run pass on a noisier instrument.
    """
    env = _gate().MEASUREMENT_ENV
    assert "hwcaps" in env.get("GLIBC_TUNABLES", ""), "the ISA cap pin is what makes the host term zero"
    assert env.get("PYTHONHASHSEED") == "0", (
        f"PYTHONHASHSEED is {env.get('PYTHONHASHSEED')!r}; an unpinned seed gives every draw a fresh "
        "dict layout, and the binding floors stop meaning the code they were drawn from"
    )


def test_the_pairing_note_no_longer_quotes_a_floor_into_a_docstring():
    """Same defect class as a stale paragraph: a measurement copied into prose nothing re-reads."""
    text = GATE.read_text(encoding="utf-8")
    assert "21,444,780" not in text and "21,444,880" not in text
    assert "within tens of instructions" in text, "the size survives, the digits that dated it do not"


def test_perf_md_no_longer_points_at_an_agents_claim_it_does_not_contain():
    """Three sentences credited `AGENTS.md` with calling the binding channel noisy.

    Measured, not remembered: `grep -c noisy AGENTS.md` is 0, and the file carries no binding-noise
    claim at all - its Ir bullet speaks of drift and of the compiler term. Worse, the credited claim was
    the wrong attribution: the wide reading was CPython's per-process hash seed, now pinned, which is
    what `docs/dev/quality-ledger.md` (gj) records. A pointer at an absent authority teaches a reader to
    check the wrong file, so this fails if the phrasing comes back.
    """
    perf = (REPO_ROOT / "docs" / "dev" / "perf.md").read_text(encoding="utf-8")
    for phrase in (
        "`AGENTS.md` already names as the noisy",
        "`AGENTS.md` already gives",
        "`AGENTS.md` already calls noisy",
    ):
        assert phrase not in perf, f"{phrase!r} credits AGENTS.md with a sentence it does not contain"
    assert "hash seed" in perf, "the reason the wide reading had is named, not just un-cited"
