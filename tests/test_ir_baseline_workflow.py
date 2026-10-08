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
    assert set(data["generated_by"]) == {"environment", "rustc", "tool", "note"}, sorted(data["generated_by"])
    assert data["generated_by"]["note"] == gate.PROVENANCE_NOTE, "the note is generated, not transcribed"


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
