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
