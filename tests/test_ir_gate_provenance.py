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


def test_only_the_interpreter_counting_harness_is_keyed_by_image():
    """The engine rows do not move with the image, so keying them would state a claim nobody measured.

    Measured at one tree with both harness digests equal across images `20261026.354.1` and
    `20261023.334.3`: `load_toml_medium` read 142,467,241 on one and 144,904,012 on the other, while the
    engine writer rows agreed to 120 instructions. The set is derived from the harness map the
    measurement already builds, so a scenario that moves between harnesses re-classifies itself rather
    than going stale in this file.
    """
    gate = _gate()
    keyed = gate.keyed_by_image(
        {
            "to_python_medium": "pyrs-yaml/ir_gate",
            "load_toml_medium": "pyrs-yaml/ir_gate",
            "serialize_medium": "pyrs-yaml-core/ir_gate",
            "parse_medium": "pyrs-yaml-core/ir_gate",
        }
    )
    assert keyed == {"to_python_medium", "load_toml_medium"}, keyed


def test_a_keyed_environment_compares_against_its_own_row_and_no_other_number():
    gate = _gate()
    ref = {
        "scenarios": {"load_toml_medium": 144_946_295, "serialize_medium": 32_648_581},
        "image_floors": {
            "github-runner ubuntu24, image 20261023.334.3": {"load_toml_medium": 144_904_012},
        },
    }
    floors, refusal = gate.image_floors(ref, "github-runner ubuntu24, image 20261023.334.3", {"load_toml_medium"})
    assert refusal is None
    assert floors["load_toml_medium"] == 144_904_012, "the row for this image is the floor"
    assert floors["serialize_medium"] == 32_648_581, "an unkeyed scenario keeps its single floor"


def test_a_row_that_never_measured_a_keyed_scenario_refuses_rather_than_borrowing_the_flat_number():
    # A scenario the harness counts with an interpreter has no comparable number across images, so a row
    # missing it cannot be judged by the flat table: that table belongs to whichever image refreshed it.
    # Adding a scenario to the binding harness is therefore a refresh, not an edit of the baseline.
    gate = _gate()
    ref = {
        "scenarios": {"load_toml_medium": 144_904_012, "to_python_medium": 386_743_139},
        "image_floors": {
            "github-runner ubuntu24, image 20261023.334.3": {"load_toml_medium": 144_904_012},
        },
    }
    floors, refusal = gate.image_floors(
        ref,
        "github-runner ubuntu24, image 20261023.334.3",
        {"load_toml_medium", "to_python_medium"},
    )
    assert refusal is not None, "the row does not carry every scenario this run keys"
    assert "to_python_medium" in refusal, "the refusal names the scenario it cannot judge"
    assert floors == ref["scenarios"], "a refusal returns the flat table, never a mixed floor"


def test_a_host_that_names_no_image_keeps_its_flat_floor_instead_of_refusing_itself():
    """Keying is a rule about runner images, not about every machine that ever runs the gate.

    Refusing a local host would take away the cheapest reading of "did my change cost instructions",
    including for the scenarios whose numbers do not depend on an image at all - and a row would be no
    use to that host, since comparing a host against an image's number is the thing the key is for.
    """
    gate = _gate()
    ref = {
        "scenarios": {"load_toml_medium": 144_946_295},
        "image_floors": {
            "github-runner ubuntu24, image 20261026.354.1": {"load_toml_medium": 142_467_241},
        },
    }
    floors, refusal = gate.image_floors(ref, "Linux 6.8.0-49-generic (x86_64)", {"load_toml_medium"}, on_image=False)
    assert refusal is None, "a host outside the runner channel is judged, not refused"
    assert floors == ref["scenarios"], "the flat table is the floor for a host with no image"
    same_host_on_an_image = gate.image_floors(ref, "github-runner ubuntu24, image 20261012.334.1", {"load_toml_medium"})
    assert same_host_on_an_image[1] is not None, "the same table still refuses an unknown image"


def test_which_runs_are_keyed_is_decided_once(monkeypatch):
    """`environment()` and the image test read one predicate, so they cannot disagree.

    A second copy of the `ImageOS`/`ImageVersion` check is how a run ends up named as a runner image and
    then told it is not on one.
    """
    gate = _gate()
    for name in ("ImageOS", "ImageVersion"):
        monkeypatch.delenv(name, raising=False)
    assert gate.runner_image() is None
    assert not gate.environment().startswith("github-runner")
    monkeypatch.setenv("ImageOS", "ubuntu24")
    monkeypatch.setenv("ImageVersion", "20261026.354.1")
    assert gate.runner_image() == ("ubuntu24", "20261026.354.1")
    assert gate.environment() == "github-runner ubuntu24, image 20261026.354.1"
    monkeypatch.delenv("ImageVersion")
    assert gate.runner_image() is None, "one identifier alone is not an image"


def test_an_image_the_baseline_never_measured_refuses_rather_than_passing_or_failing():
    gate = _gate()
    ref = {
        "scenarios": {"load_toml_medium": 144_946_295},
        "image_floors": {
            "github-runner ubuntu24, image 20261023.334.3": {"load_toml_medium": 144_904_012},
        },
    }
    floors, refusal = gate.image_floors(ref, "github-runner ubuntu24, image 2026999.999.1", {"load_toml_medium"})
    assert refusal is not None, "a number from one image compared to a floor from another is a verdict nobody drew"
    assert "2026999.999.1" in refusal and "Ir baseline refresh" in refusal, refusal
    assert "load_toml_medium" in refusal, "the message names what would have been keyed"
    assert floors == ref["scenarios"], "the refusal carries no rewritten floors"


def test_a_baseline_written_before_the_key_existed_is_still_judged_and_not_refused():
    """The table is optional on purpose.

    Refusing on an absent table would make the first run after this change red on every pull request,
    because `Test matrix (all legs)` counts only `success`: the gate would have produced the deadlock it
    exists to diagnose.
    """
    gate = _gate()
    ref = {"scenarios": {"load_toml_medium": 144_946_295}}
    floors, refusal = gate.image_floors(ref, "Linux 5.15.0 (x86_64)", {"load_toml_medium"})
    assert refusal is None
    assert floors == ref["scenarios"]


def test_a_refresh_adds_its_image_without_withdrawing_the_ones_it_did_not_run_on():
    """The table is merged, in both directions.

    Replacing it would silently narrow coverage to one image, and then every other image's enforcing leg
    reads a refusal it did not earn. Replacing the row under `--only` would do the same inside one image:
    the scenarios that partial run never measured would lose their floors.
    """
    gate = _gate()
    rows = {
        "image-a": {"load_toml_medium": 100, "to_python_small": 200},
        "image-b": {"load_toml_medium": 105, "to_python_small": 205},
    }
    merged = gate.merge_image_row(rows, "image-a", {"load_toml_medium": 110})
    assert merged["image-a"] == {"load_toml_medium": 110, "to_python_small": 200}, merged["image-a"]
    assert merged["image-b"] == rows["image-b"], "a refresh on one image says nothing about another"
    assert rows["image-a"]["load_toml_medium"] == 100, "the caller's table is not mutated in place"


def test_a_run_that_names_no_image_adds_no_row_to_the_table():
    """A key in this table is a runner image, so a host that names none files nothing.

    Writing the platform string would invent a key no enforcing leg can ever ask for, and the table
    would read as coverage it does not have.
    """
    gate = _gate()
    rows = {"github-runner ubuntu24, image 20261026.354.1": {"load_toml_medium": 100}}
    merged = gate.merge_image_row(rows, "Windows 10 (AMD64)", {"load_toml_medium": 110}, on_image=False)
    assert merged == rows, "the table keeps exactly its images"
    assert all(not key.startswith("Windows") for key in merged)
