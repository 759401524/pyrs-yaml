"""Wiring test for the Ir baseline refresh job, including the defect its first run revealed.

`ir-baseline.yml` exists to move the baseline to the environment that enforces it. Its first real
run dispatched on `main`, used `rust-toolchain@stable`, and produced a file whose
`serialize_medium` was +6.7% and `serialize_small` +5.8% against the committed numbers - with the
code untouched, because `stable` was rustc 1.99.0 and the baseline had been made on 1.97.1.
Committing that artifact would have made a compiler regression the new normal, invisible to the gate.

So the job now reads the toolchain from the baseline unless a `toolchain` input says otherwise, and
this file pins that: the pin travels with the numbers, a deliberate move stays explicit, and the
output remains an artifact a human commits rather than an automatic push.
"""

from __future__ import annotations

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ir-baseline.yml"
BASELINE = REPO_ROOT / ".ci" / "ir-baseline.json"


def test_the_refresh_job_exists_and_is_manual_only():
    text = WORKFLOW.read_text(encoding="utf-8")
    assert "workflow_dispatch" in text
    assert not re.search(r"^\s*(push|pull_request|schedule):", text, re.M), (
        "a refresh that runs on its own stops being a decision about the baseline"
    )


def test_the_toolchain_default_is_the_baseline_not_the_runner():
    """The measured defect: `@stable` silently changed the compiler and moved every number."""
    text = WORKFLOW.read_text(encoding="utf-8")
    assert "dtolnay/rust-toolchain@stable" not in text, (
        "`stable` resolves to whatever the image ships: the first run of this job moved "
        "serialize_medium +6.7% against a baseline made on the pinned compiler"
    )
    assert ".ci/ir-baseline.json" in text and "['toolchain']" in text, (
        "the toolchain must default to the version recorded in the baseline"
    )
    assert re.search(r"inputs\.toolchain", text), "a deliberate pin move needs an explicit input"


def test_the_result_is_reviewed_and_never_auto_pushed():
    text = WORKFLOW.read_text(encoding="utf-8")
    assert "upload-artifact" in text, "the artifact is the review surface"
    assert "git --no-pager diff -- .ci/ir-baseline.json" in text, "the job must show what moved"
    assert not re.search(r"^\s*git (push|commit)", text, re.M), "a refresh never pushes by itself"


def test_the_baseline_records_a_single_toolchain():
    """The pin the job honours has to exist and be parseable the way the workflow parses it."""
    import json

    data = json.loads(BASELINE.read_text(encoding="utf-8"))
    version = data["toolchain"].split()[1]
    assert re.fullmatch(r"\d+\.\d+(\.\d+)?", version), data["toolchain"]
    assert data["generated_by"]["environment"], "provenance is the point of the file"


def _load(rel):
    import importlib.util

    spec = importlib.util.spec_from_file_location(rel.stem, REPO_ROOT / rel)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_the_committed_baseline_is_exactly_what_the_job_writes():
    """No key of `.ci/ir-baseline.json` may be hand-written.

    The refresh job's output is an artifact a human commits, and the file committed after the first
    run carried a `generated_by.note` that `--update` never writes. Committing the next artifact would
    then have deleted that paragraph, and the diff would look like the note had been *decided* rather
    than dropped: a file that is part measurement and part transcription cannot be re-generated
    faithfully, and cannot be reviewed as one either. The prose moved to `QUALITY_MATRIX.md`; what is
    left here is the rule that the committed keys are the generated keys.
    """
    import json

    gate = _load(Path("scripts") / "ir_gate.py")
    data = json.loads(BASELINE.read_text(encoding="utf-8"))
    assert set(data) == {"toolchain", "generated_by", "tolerance_hint", "scenarios"}, sorted(data)
    assert set(data["generated_by"]) == {"environment", "rustc", "tool", "repeats", "note"}, sorted(
        data["generated_by"]
    )
    assert data["generated_by"]["repeats"] >= 1, "the gate's sample size is part of the claim"
    assert data["generated_by"]["repeats"] == gate.DEFAULT_REPEATS, (
        "the enforcing job reads this count, so a baseline made with a different one is a different gate"
    )
    assert data["generated_by"]["note"] == gate.PROVENANCE_NOTE, "the note is generated, not transcribed"


def test_the_sample_size_is_read_from_the_baseline_not_from_memory():
    """The gate has to enforce the same envelope the baseline was drawn with.

    max-of-3 versus a single sample is not a cosmetic difference: the binding channel moves 0.83%
    run to run, so a one-shot enforcing run against a max-of-3 baseline reads low, and a max-of-3
    enforcing run against a single-sample baseline reads high. Precedence is explicit > recorded >
    default, and a missing or zero recorded count falls through instead of configuring a gate that
    measures nothing.
    """
    gate = _load(Path("scripts") / "ir_gate.py")
    assert gate.resolve_repeats(None, None) == gate.DEFAULT_REPEATS
    assert gate.resolve_repeats(None, 5) == 5
    assert gate.resolve_repeats(2, 5) == 2
    assert gate.resolve_repeats(None, 0) == gate.DEFAULT_REPEATS
    assert gate.resolve_repeats(0, None) == gate.DEFAULT_REPEATS
    assert gate.resolve_repeats("", "nonsense") == gate.DEFAULT_REPEATS


def test_the_gate_measures_every_harness_the_repository_declares():
    """`ir_gate.py`'s harness list and the manifests must not be allowed to disagree.

    A crate that declares an `ir_gate` target the gate script does not know about is measured by
    nobody, and its scenarios would only surface as `ir-unbaselined` holes after someone noticed the
    count changed. The reverse - a harness entry pointing at a crate that dropped the target - makes
    `--update` fail on a build error, which is the louder half and already covered by the script's own
    refusal to write a baseline from an unreadable harness.
    """
    gate = _load(Path("scripts") / "ir_gate.py")
    matrix = _load(Path("scripts") / "quality_matrix.py")
    assert sorted(crate for crate, _bench in gate.HARNESSES) == matrix.ir_harness_owners(), (
        "a harness was added or removed in one place only"
    )
    assert all(bench == "ir_gate" for _crate, bench in gate.HARNESSES), gate.HARNESSES


def test_a_baseline_move_has_to_be_named_before_it_is_written():
    """`--update` may not loosen the gate as a side effect of being run.

    The numbers are not invented: they are the artifact diff a PR #342 dispatch printed for
    `to_json_medium` - the committed 21,232,786 against the 22,048,805 that dispatch measured. That
    second number came from a different build (its own SHA-256 names it), so it is not a regression in
    the writer; it is a different shape of the change. Either way, committing it would have granted the
    JSON writer 3.84% of silence - seven times the line the same file declares - and the diff would
    have shown only a number.
    """
    gate = _load(Path("scripts") / "ir_gate.py")
    assert gate.drift_report({"to_json_medium": 21_232_786}, {"to_json_medium": 22_048_805}, 0.005) == [
        "to_json_medium: 21,232,786 -> 22,048,805 (+3.84%)"
    ]


def test_movement_the_tolerance_already_permits_is_not_reported():
    """The guard has to bite at the line and not before it, or it would block every refresh.

    +0.26% is inside a 0.5% line, and -1.3% is an improvement: refusing either would train the operator to
    pass `--accept-drift` on sight, which is how a guard dies.
    """
    gate = _load(Path("scripts") / "ir_gate.py")
    assert gate.drift_report({"parse_small": 49_565_307}, {"parse_small": 49_694_000}, 0.005) == []
    assert gate.drift_report({"parse_small": 49_565_307}, {"parse_small": 48_940_000}, 0.005) == []


def test_a_scenario_without_a_committed_number_is_not_drift():
    """New coverage is not a loosening.

    A harness scenario the baseline has never carried has nothing to move relative to; `--update` still has
    to refuse to write an *incomplete* baseline, which that path does on its own terms.
    """
    gate = _load(Path("scripts") / "ir_gate.py")
    assert gate.drift_report({}, {"to_json5_medium": 23_602_792}, 0.005) == []


def test_the_refusal_is_wired_into_the_update_path():
    """The comparison has to run where the file is written, not merely exist.

    `drift_report` is pure, so the three tests above pass with the guard unhooked from `--update`; this is
    the half that keeps it hooked, and it reads the source because no test can afford a valgrind run.
    Checked by renaming the call inside the branch once: this test reddened with "nothing compares before
    writing" while the three pure tests stayed green, which is exactly the failure mode it exists for.
    """
    source = (REPO_ROOT / "scripts" / "ir_gate.py").read_text(encoding="utf-8")
    assert "args.accept_drift" in source, "no scenario can be named, so the guard cannot be satisfied"
    assert "drift_report(committed, final, line)" in source, "nothing compares before writing"
    assert "--accept-drift" in source, "the escape hatch itself is gone"
    # The branch ends at the *statement* that follows it, four spaces deep: the same condition text appears
    # nested inside `--update` itself, and splitting on the bare string cuts the slice short before the
    # lines this test exists to check.
    following = re.search(r"\n    if not BASELINE\.exists\(\):", source)
    assert following, "the enforcing path moved; re-derive the slice below"
    update_branch = source[source.index("if args.update:") : following.start()]
    assert "drift_report" in update_branch, "the check runs after the file is already written"
    assert "BASELINE.write_text" in update_branch


def test_two_builds_are_never_reported_as_the_same_code():
    """The pairing that would have prevented issue #344.

    A refresh artifact was compared with a gate run and the 2.82% between them was filed as an
    environment defect. The digests say why that could not be true: the two builds were different
    bytes, so the difference was code all along. Measured on matching bytes the same two jobs are 21
    instructions apart, which is the reproducibility the 0.5% line rests on.
    """
    gate = _load(Path("scripts") / "ir_gate.py")
    (line,) = gate.binary_pairing({"pyrs-yaml/ir_gate": "40a4a290df069cb2"}, {"pyrs-yaml/ir_gate": "a64ca39b772fae54"})
    assert "differs from the baseline build (was 40a4a290df069cb2, now a64ca39b772fae54)" in line
    assert "movement below is code" in line, "the pairing must attribute, not merely observe"


def test_identical_bytes_are_reported_as_a_real_pairing():
    """The case the guard exists to allow: a comparison that means something.

    `serialize_block_scalars` measured 15,796,883 and 15,796,904 under these two digests, across jobs.
    """
    gate = _load(Path("scripts") / "ir_gate.py")
    digest = "364e072f5fea6687"
    (line,) = gate.binary_pairing({"pyrs-yaml-core/ir_gate": digest}, {"pyrs-yaml-core/ir_gate": digest})
    assert f"matches the baseline ({digest})" in line, line
    assert "identical bytes" in line and "the host rather than the code" in line, (
        "the other half of the rule: matching bytes make movement an environment question"
    )


def test_a_baseline_without_recorded_digests_says_so():
    """Every committed baseline predates this field, and that is a fact worth printing.

    Silence here would read as 'paired, nothing to report' - the exact misunderstanding that produced
    issue #344. Both shapes of "no record" count: a missing key and an empty one.
    """
    gate = _load(Path("scripts") / "ir_gate.py")
    current = {"pyrs-yaml/ir_gate": "a64ca39b772fae54"}
    for recorded in (None, {}):
        (line,) = gate.binary_pairing(recorded, current)
        assert "records no digests" in line, (recorded, line)
        assert "by name only" in line, (recorded, line)


def test_a_harness_the_baseline_has_never_seen_is_reported_unpaired():
    """A new channel must not look like a matching one.

    The binding harness arrived after the engine's, so the first run pairing against that baseline had
    one match and one new; collapsing both into silence would hide the half that cannot be judged.
    """
    gate = _load(Path("scripts") / "ir_gate.py")
    lines = gate.binary_pairing(
        {"pyrs-yaml-core/ir_gate": "364e072f5fea6687"},
        {"pyrs-yaml-core/ir_gate": "364e072f5fea6687", "pyrs-yaml/ir_gate": "a64ca39b772fae54"},
    )
    assert len(lines) == 2, lines
    assert any("matches the baseline" in line for line in lines), lines
    assert any("is new to the baseline" in line for line in lines), lines


def test_the_pairing_is_wired_into_the_run_and_the_file():
    """A pure function can be written and never consulted, in either direction.

    The four tests above all pass with `binary_pairing` deleted from `main()` and the `binaries` key
    absent from the payload, so this reads the source: the guard prints during enforcement and the
    digests travel with the numbers they describe. Proven by mutation - renaming the call in the
    enforcing branch reddened this test alone, leaving the pure ones green.
    """
    source = (REPO_ROOT / "scripts" / "ir_gate.py").read_text(encoding="utf-8")
    assert 'binary_pairing(ref.get("binaries"), binaries)' in source, "nothing names the bytes at enforcement"
    assert '"binaries": binaries' in source, "the digests are measured but never recorded"
    assert "def binary_pairing(" in source, "the pairing exists only as an inline expression"


def test_a_refresh_can_be_told_which_move_is_intended():
    """The guard of #345 made the refresh job unable to do the one thing it exists for.

    Measured rather than reasoned: a `workflow_dispatch` on the #360 head built both harnesses,
    measured all twenty-one scenarios, printed every sample, and then exited on
    `--update would move 3 scenario(s) past the 0.5% line` - the refusal was correct and the channel
    had no way to express the exception the script itself provides. So a baseline that a change is
    meant to pay for could not be generated by the job that generates baselines, leaving
    transcription from a job log as the only route, which is the failure mode
    `test_the_committed_baseline_is_exactly_what_the_job_writes` exists to prevent.

    The names arrive as one input and become one `--accept-drift` flag each. Checked against the
    workflow source because no test can afford a valgrind run; proven by mutation, in both
    directions - dropping the flag from the command line reddened
    `test_the_input_is_actually_passed_to_the_guard`, and hard-coding a name that always passes
    reddened `test_an_empty_input_keeps_the_refusal`.
    """
    text = WORKFLOW.read_text(encoding="utf-8")
    assert re.search(r"^ {6}accept_drift:", text, re.M), "no way to name an intended move"
    block = text[text.index("- name: Measure and rewrite the baseline") :]
    assert "inputs.accept_drift" in block, "the input is declared and never read"


def test_the_input_is_actually_passed_to_the_guard():
    """Reading the input is not the same as reaching the command that enforces.

    `--accept-drift` is what `drift_report`'s refusal keys off, so a step that collects names and
    then runs a bare `--update` still fails every refresh it was written to allow.
    """
    text = WORKFLOW.read_text(encoding="utf-8")
    block = text[text.index("- name: Measure and rewrite the baseline") :]
    assert "--accept-drift" in block, "the names are read but never passed"
    assert re.search(r"ir_gate\.py --update.*\$\{?args", block, re.S), "the flags are built and dropped"


def test_an_empty_input_keeps_the_refusal():
    """The default must stay the guard, or the guard is decorative.

    An unconditional `--accept-drift` for every scenario - or a `yes |` in front of the step - would
    let any dispatch loosen the baseline, which is precisely what #345 stopped. So the empty default
    has to be an empty flag list, and the step has to say so out loud in the log.
    """
    text = WORKFLOW.read_text(encoding="utf-8")
    block = text[text.index("- name: Measure and rewrite the baseline") :]
    assert 'default: ""' in text[: text.index("- name: Measure and rewrite the baseline")], (
        "a non-empty default would accept drift nobody named"
    )
    assert re.search(r"for name in .*names.*do\n\s+args\+=\(--accept-drift", block), block
    assert "no scenario named" in block, "the log cannot distinguish a guarded run from an open one"
    flags = re.findall(r"args\+=\(--accept-drift (\S+)\)", block)
    assert flags == ['"$name"'], f"the flag list must grow only from the input, one name per iteration (found {flags})"
