"""Self-test for the consumer of the fuzz sampler's findings (`scripts/fuzz_file_issues.py`).

The gap this covers: `.github/workflows/fuzz.yml`'s aggregate job wrote a table to
`$GITHUB_STEP_SUMMARY` and printed `::warning::`, and nothing read either of them. Five scheduled
runs in a row (2026-10-03 through 2026-10-31) failed with crash inputs in their archives, and every
finding reached the fix queue only because a person opened the run page.

The cases below decide whether "one issue per target" stays one issue per target (exact-title
dedupe, a near-miss title must not absorb a new finding), keep fuzzer output out of a command line,
and pin the two wiring facts: the job must carry `issues: write`, and a sampler that failed without
leaving bytes must not exit 0.
"""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

import pytest
import yaml

REPO_ROOT = Path(__file__).resolve().parent.parent
SCRIPT = REPO_ROOT / "scripts" / "fuzz_file_issues.py"
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "fuzz.yml"


@pytest.fixture(scope="module")
def mod():
    spec = importlib.util.spec_from_file_location("fuzz_file_issues", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


class Recorder:
    """Stands in for `gh`: records every call and answers `issue list` from a supplied set."""

    def __init__(self, open_issues=()):
        self.calls = []
        self.open_issues = list(open_issues)

    def __call__(self, args, body):
        self.calls.append((list(args), body))
        if args[:2] == ["issue", "list"]:
            return json.dumps([{"number": n, "title": t} for n, t in self.open_issues])
        return ""

    def verbs(self):
        return [args[:2] for args, _body in self.calls]


def archive(root: Path, target: str, inputs, findings) -> Path:
    """Lay out one downloaded archive the way `merge-multiple: false` produces it."""
    collected = root / "crashes" / f"fuzz-artifacts-{target}" / "collected" / target
    collected.mkdir(parents=True, exist_ok=True)
    for name in inputs:
        (collected / name).write_bytes(b"x")
    if findings:
        (collected / "findings.tsv").write_text(
            "".join(f"{sig}\t{name}\n" for sig, name in findings), encoding="utf-8", newline="\n"
        )
    return root / "crashes"


def test_a_target_with_findings_and_no_open_issue_opens_one(mod, tmp_path):
    crashes = archive(
        tmp_path,
        "toml_roundtrip",
        ["crash-aaa", "crash-bbb"],
        [("panicked at fuzz_targets/toml_roundtrip.rs:12:5 boom", "crash-aaa")],
    )
    run_gh = Recorder()
    code = mod.file_issues(mod.collect(crashes), run_url="https://example.invalid/run/1", dry_run=False, run_gh=run_gh)
    assert code == 0
    assert run_gh.verbs() == [["issue", "list"], ["issue", "create"]]
    create_args, body = run_gh.calls[1]
    assert create_args == [
        "issue",
        "create",
        "--title",
        "fuzz: toml_roundtrip has untriaged crash inputs",
        "--body-file",
        "-",
    ]
    assert "crash-aaa" in body and "crash-bbb" in body
    assert "https://example.invalid/run/1" in body
    assert "panicked at fuzz_targets/toml_roundtrip.rs:12:5 boom" in body


def test_a_second_run_appends_to_the_issue_it_opened_before(mod, tmp_path):
    crashes = archive(
        tmp_path,
        "yaml_roundtrip",
        ["crash-ccc"],
        [("panicked at fuzz_targets/yaml_roundtrip.rs:3:1 boom", "crash-ccc")],
    )
    run_gh = Recorder(open_issues=[(41, "fuzz: yaml_roundtrip has untriaged crash inputs")])
    code = mod.file_issues(mod.collect(crashes), run_url="https://example.invalid/run/2", dry_run=False, run_gh=run_gh)
    assert code == 0
    assert run_gh.verbs() == [["issue", "list"], ["issue", "comment"]]
    assert run_gh.calls[1][0][2] == "41"
    assert "https://example.invalid/run/2" in run_gh.calls[1][1]


def test_a_title_that_only_contains_the_neighbour_is_not_matched(mod, tmp_path):
    """A dated or paraphrased issue about the same target must not absorb a new run's finding."""
    crashes = archive(tmp_path, "parse_toml", ["crash-ddd"], [("panicked at x", "crash-ddd")])
    run_gh = Recorder(
        open_issues=[
            (7, "fuzz: parse_toml has untriaged crash inputs (2026-10)"),
            (8, "fuzz: parse_toml"),
        ]
    )
    mod.file_issues(mod.collect(crashes), run_url="u", dry_run=False, run_gh=run_gh)
    assert run_gh.verbs() == [["issue", "list"], ["issue", "create"]]


def test_fuzzer_output_never_reaches_a_command_argument(mod, tmp_path):
    """A signature is panic text from an untrusted input; it belongs in a body, not in argv."""
    hostile = "panicked at x | `$(touch pwned)` && rm -rf /"
    crashes = archive(tmp_path, "parse_yaml", ["crash-eee"], [(hostile, "crash-eee")])
    run_gh = Recorder()
    mod.file_issues(mod.collect(crashes), run_url="u", dry_run=False, run_gh=run_gh)
    for args, _body in run_gh.calls:
        assert all(hostile not in arg for arg in args), args
    assert hostile in run_gh.calls[1][1]


def test_an_archive_holding_both_mirrors_counts_each_input_once(mod, tmp_path):
    """`upload-artifact` stores `artifacts/<target>/` and `collected/<target>/`; the aggregate's own
    `sort -u` exists because counting paths reported 3 inputs for 2."""
    crashes = archive(tmp_path, "parse_toml", ["crash-aaa", "crash-bbb"], [("panicked at x", "crash-aaa")])
    collected = crashes / "fuzz-artifacts-parse_toml" / "collected" / "parse_toml"
    mirrored = crashes / "fuzz-artifacts-parse_toml" / "artifacts" / "parse_toml"
    mirrored.mkdir(parents=True)
    for crash in collected.glob("crash-*"):
        (mirrored / crash.name).write_bytes(b"x")
    found = mod.collect(crashes)
    assert [f.target for f in found] == ["parse_toml"]
    assert found[0].inputs == ["crash-aaa", "crash-bbb"]
    assert sum(found[0].signatures.values()) == 1


def test_a_target_directory_without_crash_files_is_not_filed(mod, tmp_path):
    (tmp_path / "crashes" / "fuzz-artifacts-parse_json" / "collected" / "parse_json").mkdir(parents=True)
    assert mod.collect(tmp_path / "crashes") == []


def test_a_sampler_that_succeeded_and_found_nothing_is_a_quiet_zero(mod, tmp_path):
    (tmp_path / "crashes").mkdir()
    assert mod.main(["--crashes", str(tmp_path / "crashes"), "--fuzz-result", "success"]) == 0


@pytest.mark.parametrize("argument", ["failure", "cancelled", "skipped"])
def test_a_sampler_that_failed_without_leaving_bytes_is_exit_2(mod, tmp_path, argument):
    """The bytes are gone, so the finding has nowhere to go. Silence here would read as "nothing found"."""
    (tmp_path / "crashes").mkdir()
    assert mod.main(["--crashes", str(tmp_path / "crashes"), "--fuzz-result", argument]) == 2


def test_a_missing_artifact_tree_after_a_failed_sampler_is_also_exit_2(mod, tmp_path):
    assert mod.main(["--crashes", str(tmp_path / "absent"), "--fuzz-result", "failure"]) == 2


def test_the_aggregate_job_grants_issues_write_and_runs_this_script():
    workflow = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
    report = workflow["jobs"]["report"]
    assert report["permissions"]["issues"] == "write", (
        "the aggregate job files issues; without `issues: write` the step fails on a token, not on a finding"
    )
    filing = [step for step in report["steps"] if "scripts/fuzz_file_issues.py" in str(step.get("run", ""))]
    assert len(filing) == 1, "the aggregate job must run the filer exactly once"
    assert filing[0]["if"] == "github.event_name != 'pull_request'", (
        "a pull-request leg replays seeds with -runs=0 and never explores, and a fork PR's token is "
        "read-only, so filing there would fail the job instead of the gate"
    )
