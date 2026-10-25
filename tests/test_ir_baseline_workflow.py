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

import os
import re
import shutil
import subprocess
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

    `binaries` joined that set when the digests started being written (#349), and the assertion was
    left behind: it passed only because the committed file was older than the field. That is the same
    class of defect this file exists to catch - a gate that reads green for a reason that has nothing
    to do with what it claims - so the key set now includes it and the digests are required to name
    every harness the gate measures, not merely to exist.
    """
    import json

    gate = _load(Path("scripts") / "ir_gate.py")
    data = json.loads(BASELINE.read_text(encoding="utf-8"))
    assert set(data) == {
        "toolchain",
        "generated_by",
        "binaries",
        "tolerance_hint",
        "scenarios",
    }, sorted(data)
    assert set(data["binaries"]) == {f"{crate}/{bench}" for crate, bench in gate.HARNESSES}, (
        "the numbers are recorded without naming the bytes behind them"
    )
    assert all(len(digest) == 16 for digest in data["binaries"].values()), data["binaries"]
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


def test_the_log_counts_the_names_not_the_flags():
    """The first real use of this input printed a number that was twice the truth.

    Run 38326122129 named three scenarios and the step logged `accepting drift for 6 scenario(s)`:
    the count came from `args`, which holds a flag *and* its value per name. That line is what a
    reviewer reads to decide whether an artefact was opened deliberately, so the number on it has to
    be the number of scenarios rather than a quantity nobody named. The branch decision is equivalent
    either way - a flag count is twice the name count, and the measured behaviour of both halves is
    recorded in the ledger - so it is keyed on `names` to keep the message and its condition the same
    quantity instead of asking a reader to know that `args` holds pairs.

    Proven by mutation: pointing both the count and the condition back at `args` reddens this test
    alone and leaves the three wiring tests green.
    """
    text = WORKFLOW.read_text(encoding="utf-8")
    block = text[text.index("- name: Measure and rewrite the baseline") :]
    assert "accepting drift for ${#names[@]} scenario(s)" in block, (
        "the log counts flags, which is twice the number of scenarios named"
    )
    assert "if [ ${#names[@]} -gt 0 ]" in block, "the branch decides on the wrong array"
    assert "${#args[@]}" not in block, "a flag count leaked back into the message"


MATRIX = REPO_ROOT / "QUALITY_MATRIX.md"

# The words section 2 uses when it counts the scenarios out loud, up to the fifties.
_ONES = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
]
_TENS = {2: "twenty", 3: "thirty", 4: "forty", 5: "fifty"}


def _count_word(n: int) -> str:
    """The English words section 2 uses when it counts the scenarios out loud."""
    if n < 20:
        return _ONES[n]
    tens, ones = divmod(n, 10)
    return _TENS[tens] if not ones else f"{_TENS[tens]}-{_ONES[ones]}"


def test_the_page_counts_the_scenarios_the_file_carries():
    """`QUALITY_MATRIX.md` section 2 states how many scenarios the baseline holds; it has to be right.

    The page carried "seventeen of them" while `.ci/ir-baseline.json` held twenty-one - the count that
    the refresh job itself prints, and the count this file's own docstring names. Nothing read it,
    because the file's *keys* were pinned and its *prose* was not, which is the same shape as the
    defect above: a check that passes for a reason unrelated to what it claims to assure. A scenario
    is added by editing `benches/ir_gate.rs`, and if that edit never revisits this sentence, the
    number here silently becomes a historical one.
    """
    import json

    count = len(json.loads(BASELINE.read_text(encoding="utf-8"))["scenarios"])
    page = MATRIX.read_text(encoding="utf-8")
    phrase = f"{_count_word(count)} of them, since the binding harness joined"
    assert phrase in page, (
        f"the baseline holds {count} scenarios; QUALITY_MATRIX.md section 2 says otherwise, "
        f"and no gate but this one reads it"
    )


def test_a_refresh_can_be_asked_where_a_scenario_spends_its_instructions():
    """A floor answers "how much" and never "where", and only the image's answer pairs with it.

    `scripts/ir_gate.py --profile` has been the registered tool for that question since #401, but it
    has only ever been run by hand, and a hand-run has a fatal pairing problem: the local build and
    the committed floor are different bytes. That is exactly how a -21.23% reading on
    `documents_short_strings` stayed unattributed (ledger (gk)) - the local profile puts 26.78% of
    the loop in `LineTable::line_col`, and the image's number for the same function is simply unknown.
    Declaring the input is one half; the step has to read it and pass one `--profile` flag per name.
    """
    text = WORKFLOW.read_text(encoding="utf-8")
    assert re.search(r"^ {6}profile:", text, re.M), "no way to ask for attribution"
    block = text[text.index("- name: Per-function attribution") :]
    assert "inputs.profile" in block, "the input is declared and never read"
    assert re.search(r"args\+=\(--profile \"\$name\"\)", block), block
    assert re.search(r"ir_gate\.py \"\$\{args\[@]", block), "the flags are built and dropped"


def test_an_empty_profile_input_costs_no_extra_measurement():
    """Attribution is on request, or the refresh job pays callgrind twice for nobody's question.

    A step that ran `--profile` unconditionally would re-measure every scenario it just measured - and
    `--profile` rebuilds with symbols (`CARGO_PROFILE_RELEASE_STRIP=false`), so the extra pass is not
    even the same binary. The guard is the point, so the condition has to sit on the step and the
    default has to stay empty.
    """
    text = WORKFLOW.read_text(encoding="utf-8")
    head = text[: text.index("- name: Per-function attribution")]
    assert 'default: ""' in head[head.index("profile:") :], "a non-empty default profiles by surprise"
    step = text[text.index("- name: Per-function attribution") :]
    assert re.search(r"if: \$\{\{ inputs\.profile != '' \}\}", step), step[:400]
    assert "${#want[@]}" in step, "the log has to name how many scenarios it will attribute"


def _step_body(name: str) -> str:
    """The dedented `run: |` body of one step, so a test can hand it to a shell.

    A step is shell, and shell can be executed. Reading it as text is precisely what let a split that
    consumes one line pass for a step that was told to read two names.
    """
    text = WORKFLOW.read_text(encoding="utf-8")
    start = text.index(f"- name: {name}")
    run_at = text.index("run: |", start)
    lines = text[run_at + len("run: |") :].splitlines()
    body = []
    for line in lines:
        if line.strip() and not line.startswith(" " * 10):
            break
        body.append(line[10:])
    return "\n".join(body).strip() + "\n"


def _run_step(tmp_path: Path, tag: str, body: str, var: str, value: str) -> str:
    """Run one step body with its `python3` replaced by an echo, and return everything it printed."""
    bash = shutil.which("bash")
    assert bash, "the step is shell, so a leg without bash cannot ask this question at all"
    script = tmp_path / f"step-{tag}.sh"
    # `Path.open`, not `write_text`: the newline keyword on `write_text` arrived in 3.10, while the
    # supported floor is 3.8 and `test_scripts_import_on_supported_python.py` reads this file's AST
    # for exactly that - it names the call rather than running it.
    with script.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(f'python3() {{ echo "$@"; }}\n{body}')
    result = subprocess.run(
        [bash, str(script)],
        capture_output=True,
        text=True,
        env=dict(os.environ, **{var: value}),
        cwd=REPO_ROOT,
    )
    assert result.returncode == 0, (script.read_text(encoding="utf-8"), result.stdout, result.stderr)
    return result.stdout


def test_a_newline_separated_profile_input_reaches_every_named_scenario(tmp_path: Path):
    """`read` consumes one line, so a two-line input meant one name and a silent half-job.

    Measured, not assumed: refresh run 40221999337 took a `profile` input carrying
    `documents_long_strings` and `documents_short_strings` on two lines - exactly what the input's own
    description invites, "Space- or newline-separated scenario names" - and logged "attributing 1
    scenario(s): documents_long_strings". The baseline artefact was real and the attribution was half,
    and no text assertion could see it because the flag is present in the script either way. The
    control below runs the same body with the flattening taken back out, which is what proves this
    test can fail rather than only describing the fix.
    """
    names = "documents_long_strings\ndocuments_short_strings\n"
    body = _step_body("Per-function attribution")
    out = _run_step(tmp_path, "profile", body, "PROFILE", names)
    assert re.findall(r"--profile (\S+)", out) == ["documents_long_strings", "documents_short_strings"], out
    assert "attributing 2 scenario(s)" in out, out

    control = body.replace("tr '\\n' ' '", "cat")
    assert control != body, "the step no longer flattens separators, so this control measures nothing"
    dropped = _run_step(tmp_path, "profile-control", control, "PROFILE", names)
    assert re.findall(r"--profile (\S+)", dropped) == ["documents_long_strings"], (
        f"the control is meant to reproduce the dropped name and printed: {dropped!r}"
    )


def test_the_drift_guard_sees_every_name_on_its_own_line(tmp_path: Path):
    """The same split guards `accept_drift`, where a dropped name is a refusal nobody asked for.

    Here the failure direction is safe - an unnamed scenario past the tolerance stops the write - and
    that is exactly why it was never noticed: the job fails loudly for the wrong reason. The step's
    own log line is the thing a reviewer reads to decide whether an artefact was opened on purpose, so
    it has to carry the same two names the input did.
    """
    names = "serialize_medium\nstream_medium\n"
    body = _step_body("Measure and rewrite the baseline")
    out = _run_step(tmp_path, "drift", body, "ACCEPT_DRIFT", names)
    assert re.findall(r"--accept-drift (\S+)", out) == ["serialize_medium", "stream_medium"], out
    assert "accepting drift for 2 scenario(s)" in out, out
