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
import shutil
import subprocess
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


def archive(root: Path, target: str, inputs, findings, layout: str = "nested") -> Path:
    """Lay out one downloaded archive the way `download-artifact@v8` leaves it.

    `upload-artifact` stores `fuzz/artifacts/<target>/` and `fuzz/collected/<target>/` in every
    archive, so those two roots appear together whichever shape the download takes: one directory
    per archive (`layout="nested"`, from `merge-multiple: false`) or the roots extracted straight
    into `--crashes` (`layout="flat"`, which run 41541555317 produced). `scripts/fuzz_rounds.sh`
    copies crash inputs into both mirrors but writes `findings.tsv` only under `collected/`, which
    is what this builds.
    """
    crashes = root / "crashes"
    base = crashes / f"fuzz-artifacts-{target}" if layout == "nested" else crashes
    for mirror in ("collected", "artifacts"):
        directory = base / mirror / target
        directory.mkdir(parents=True, exist_ok=True)
        for name in inputs:
            (directory / name).write_bytes(b"x")
    if findings:
        rows = "".join(f"{sig}\t{name}\n" for sig, name in findings)
        (base / "collected" / target / "findings.tsv").write_bytes(rows.encode("utf-8"))
    return crashes


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
    found = mod.collect(crashes)
    assert [f.target for f in found] == ["parse_toml"]
    assert found[0].inputs == ["crash-aaa", "crash-bbb"]
    assert sum(found[0].signatures.values()) == 1


@pytest.mark.parametrize("layout", ["nested", "flat"])
def test_the_download_shape_never_becomes_the_target(mod, tmp_path, layout):
    """Run 41541555317's download landed `artifacts/` and `collected/` directly in `--crashes`.

    Reading the first path segment then treated each root as a target, so one `yaml_roundtrip`
    archive's three inputs were filed as two issues - `artifacts` (#739) and `collected` (#740).
    A layout that keeps one directory per archive has to read the same way.
    """
    crashes = archive(
        tmp_path,
        "yaml_roundtrip",
        ["crash-aaa", "crash-bbb"],
        [
            ("panicked at fuzz_targets/yaml_roundtrip.rs:37:5 boom", "crash-aaa"),
            ("panicked at fuzz_targets/yaml_roundtrip.rs:37:5 boom", "crash-bbb"),
        ],
        layout=layout,
    )
    found = mod.collect(crashes)
    assert [f.target for f in found] == ["yaml_roundtrip"]
    assert found[0].inputs == ["crash-aaa", "crash-bbb"]
    assert sum(found[0].signatures.values()) == 2


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


def test_the_report_step_survives_an_archive_with_nothing_to_count(tmp_path):
    """A count of zero is a fact to report, not a failure.

    The runner invokes a bash step with `-eo pipefail`, and the step's own `set -uo pipefail` does
    not turn `-e` back off, so a `grep -c` that matched nothing ended the step: the table was left
    header-only and the filing step below never ran, which is how a finding from a push to main
    reached no consumer. Reproduced on this layout before the fix: exit 1, header-only summary.
    """
    bash = shutil.which("bash")
    if bash is None:
        pytest.skip("this case runs the workflow's bash step, and bash is not on this host")
    workflow = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
    step = next(s for s in workflow["jobs"]["report"]["steps"] if s.get("name") == "Report every target's findings")
    # One archive holds crash bytes but no findings.tsv; the other holds a findings.tsv but no
    # crash file of its own. Either one makes a `grep -c` match nothing.
    archive(tmp_path, "parse_yaml", ["crash-aaa"], [])
    archive(tmp_path, "edit_yaml", [], [("panicked at fuzz_targets/edit_yaml.rs:1:1 boom", "crash-bbb")])
    # The download may also extract the archive's own roots into `crashes/` (run 41541555317).
    archive(
        tmp_path,
        "toml_roundtrip",
        ["crash-ccc"],
        [("panicked at fuzz_targets/toml_roundtrip.rs:2:1 boom", "crash-ccc")],
        layout="flat",
    )
    (tmp_path / "report-step.sh").write_bytes(
        b'export GITHUB_STEP_SUMMARY="summary.md"\n' + step["run"].encode("utf-8")
    )
    (tmp_path / "summary.md").write_bytes(b"")

    proc = subprocess.run(
        [bash, "--noprofile", "--norc", "-eo", "pipefail", "report-step.sh"],
        cwd=tmp_path,
        capture_output=True,
        text=True,
    )
    summary = (tmp_path / "summary.md").read_text(encoding="utf-8")
    assert proc.returncode == 0, proc.stderr
    assert "| parse_yaml | 1 | 0 |" in summary
    assert "| edit_yaml | 0 | 1 |" in summary
    assert "| toml_roundtrip | 1 | 1 |" in summary
    assert "fuzz-artifacts-" not in summary, (
        "a row names the target; a row per archive directory counted both mirrors of every input"
    )


def test_the_report_step_uses_no_gnu_only_find_primaries():
    """`-printf` is a GNU primary, and the macOS leg's BSD find rejects it outright.

    The step reads the archive layout with `find` on every leg. On macOS an unknown primary
    makes find exit non-zero, so `grep -c .` saw nothing and the crash-input column read 0 for
    every archive, while the ubuntu and windows legs - both GNU find - passed. No run of this
    step on a GNU host can see that, which is why the guard is static.
    """
    workflow = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
    step = next(s for s in workflow["jobs"]["report"]["steps"] if s.get("name") == "Report every target's findings")
    commands = "\n".join(line for line in step["run"].splitlines() if not line.lstrip().startswith("#"))
    assert "-printf" not in commands, "GNU-only find primary: the macOS leg's BSD find rejects it"


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
