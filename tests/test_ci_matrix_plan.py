"""The CI interpreter matrix is planned, and the plan has to stay sound.

`ci.yml` used to write its grid as `os x python-version`, 24 cells, and two things followed from that
shape. The first is cost: measured over 144 legs of six recent runs, a leg is 138 s on ubuntu, 186 s on
macOS and 251 s on Windows, 76% of it the same ~2 500 assertions re-run per interpreter - so 78 minutes
of runner time per pull request bought cross-product coverage nothing was using.

The second is the quiet one, and it is the reason this file exists rather than a comment in the
workflow. Two gates hang off specific cells by literal condition -

    ${{ matrix.os == 'ubuntu-latest' && matrix.python-version == '3.12' && '--cov-fail-under=78 ...' }}
    if: matrix.os == 'ubuntu-latest' && matrix.python-version == '3.14'

and a cell that stops existing makes both conditions false. The suite still passes. The coverage
threshold is never evaluated, the compliance summary never prints, and the run is green: a gate that
switches itself off by being edited away, with nothing to show that it happened. So the grid is
planned by `scripts/ci_matrix_plan.py`, the roles are properties of a leg rather than string
comparisons, and `--check` refuses a plan where either role is not carried exactly once.

The rules being held:

* the version list is read from `pyproject.toml`'s trove classifiers - the matrix cannot drift away
  from the range the package publishes, and cannot quietly drop a version added there;
* every declared version runs the suite on ubuntu; the first and the last run it on every OS, and the
  coverage version runs it on ubuntu alone - the role is one leg, and re-running ~2 500 assertions on a
  runner that bills 10x for the same document is the part this file's own measurement says yields;
* the remaining cells are not dropped but demoted to a wheel smoke that installs the real artefact and
  round-trips each dialect - so the cross product is still executed per pull request, at the cost of a
  setup rather than a suite;
* the weekly schedule runs the whole grid with the suite, which is what makes the demotion reviewable.

Every claim below is checked by mutation as well as by reading: `test_check_reports_*` delete a cell or a
role from a real plan and require `check()` to name it, because a validator that cannot fail is the same
defect as the conditional that cannot fire.
"""

from __future__ import annotations

import importlib.util
import json
import pathlib
import re

import pytest
import yaml

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
SCRIPT = REPO_ROOT / "scripts" / "ci_matrix_plan.py"
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci.yml"
PYPROJECT = REPO_ROOT / "pyproject.toml"


def _load():
    spec = importlib.util.spec_from_file_location(SCRIPT.stem, SCRIPT)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


plan_owner = _load()


def declared_versions() -> list:
    return plan_owner.supported_versions(PYPROJECT.read_text(encoding="utf-8"))


def pr_plan() -> list:
    return plan_owner.plan("pull_request", declared_versions())


def cells(legs: list) -> set:
    return {(leg["os"], leg["python-version"]) for leg in legs}


# ── the version list is the published one ────────────────────────────────────────


def test_the_matrix_reads_its_versions_from_the_package_metadata() -> None:
    """Classifiers, not a list in a workflow that can disagree with them."""
    classifiers = re.findall(
        r'"Programming Language :: Python :: (3\.\d+)"',
        PYPROJECT.read_text(encoding="utf-8"),
    )
    versions = declared_versions()
    assert sorted(set(classifiers), key=lambda v: tuple(map(int, v.split(".")))) == versions
    assert versions[0] == "3.8", "the floor of requires-python is what the 3.8 leg exists to test"
    assert "3.15" in versions


def test_adding_a_declared_version_adds_it_to_the_matrix() -> None:
    """The derivation is load-bearing: prove a new classifier flows through.

    Without this, a plan that reads the classifiers today can be replaced by a hard-coded list tomorrow
    and the only symptom is a version nobody tests - which is the defect #363 was written to remove.
    """
    text = PYPROJECT.read_text(encoding="utf-8")
    with_extra = text.replace(
        '"Programming Language :: Python :: 3.15",',
        '"Programming Language :: Python :: 3.15",\n    "Programming Language :: Python :: 3.99",',
    )
    assert with_extra != text, "the classifier line moved; this check no longer proves anything"
    versions = plan_owner.supported_versions(with_extra)
    assert "3.99" in versions
    legs = plan_owner.plan("pull_request", versions)
    assert ("ubuntu-latest", "3.99") in cells(legs), "a declared version runs no suite anywhere"


# ── what every pull request must run ─────────────────────────────────────────────


def test_every_declared_version_runs_the_suite_on_the_cheapest_runner() -> None:
    versions = declared_versions()
    ubuntu = {leg["python-version"] for leg in pr_plan() if leg["os"] == "ubuntu-latest" and leg["suite"]}
    assert set(versions) <= ubuntu, sorted(set(versions) - ubuntu)


def test_the_floor_and_the_top_run_the_suite_on_every_os() -> None:
    """The two cells an expensive runner owes: what the package promises, and where abi3 breaks.

    The floor is also what `check_matrix_verdict.py` itself runs on, and the top is where the
    stable-ABI claims fail first. Measured over the last 30 runs, no failure was confined to one
    interpreter version or to one OS, so the mid-range is the part that yields on a runner that bills
    10x (macOS) or 2x (Windows) for the same assertions.
    """
    versions = declared_versions()
    owed = plan_owner.expensive_suite_versions(versions)
    assert owed == [versions[0], versions[-1]], owed
    for version in owed:
        for os_name in plan_owner.OS_ALL:
            assert (os_name, version) in cells(pr_plan()), f"{os_name} lost the {version} suite leg"


def test_the_coverage_carrier_runs_the_suite_on_the_cheap_runner_only() -> None:
    """The role is one leg, and the same version is a smoke leg on the other two OSes.

    Demoted, not dropped: `(macos-latest, 3.12)` still installs the wheel that platform built and
    round-trips a document through every dialect, it just does not re-run the ~2 500 assertions that
    ubuntu runs with the threshold attached.
    """
    versions = declared_versions()
    carrier = [leg for leg in pr_plan() if leg["coverage"]]
    assert len(carrier) == 1 and carrier[0]["os"] == plan_owner.CHEAP_OS, carrier
    suite = cells(pr_plan())
    for os_name in plan_owner.OS_ALL:
        if os_name == plan_owner.CHEAP_OS:
            assert (os_name, plan_owner.COVERAGE_VERSION) in suite
            continue
        assert (os_name, plan_owner.COVERAGE_VERSION) not in suite, (
            f"{os_name} still runs the suite at the role version"
        )
        assert (os_name, plan_owner.COVERAGE_VERSION) in cells(plan_owner.smoke_plan(versions))


def test_the_remaining_cells_are_demoted_not_dropped() -> None:
    """Suite legs + smoke legs must be the whole cross product, with no cell in both."""
    versions = declared_versions()
    full = plan_owner.plan("schedule", versions)
    suite = cells(pr_plan())
    smoke = cells(plan_owner.smoke_plan(versions))
    assert suite & smoke == set(), sorted(suite & smoke)
    assert suite | smoke == cells(full), (
        f"missing: {sorted(cells(full) - (suite | smoke))}; extra: {sorted((suite | smoke) - cells(full))}"
    )
    assert all(leg["suite"] for leg in full), (
        "the scheduled tier runs the suite in every cell; that is what licenses the demotion"
    )


def test_the_scheduled_tier_is_the_full_cross_product() -> None:
    versions = declared_versions()
    assert len(plan_owner.plan("schedule", versions)) == len(plan_owner.OS_ALL) * len(versions)


# ── the two roles, which used to be silent ───────────────────────────────────────


def test_exactly_one_leg_carries_the_coverage_gate() -> None:
    carriers = [leg for leg in pr_plan() if leg["coverage"]]
    assert len(carriers) == 1, f"{len(carriers)} legs carry the coverage role: {carriers}"
    assert carriers[0]["suite"], "the coverage role has to sit on a leg that runs the suite"
    assert carriers[0]["python-version"] == plan_owner.COVERAGE_VERSION


def test_exactly_one_leg_carries_the_compliance_summary() -> None:
    carriers = [leg for leg in pr_plan() if leg["compliance"]]
    assert len(carriers) == 1, f"{len(carriers)} legs carry the compliance role: {carriers}"
    assert carriers[0]["python-version"] == plan_owner.COMPLIANCE_VERSION


def test_a_role_pointed_at_an_undeclared_version_is_an_error_not_an_empty_plan() -> None:
    """Refusing loudly is the whole difference from the old conditionals."""
    with pytest.raises(SystemExit) as caught:
        plan_owner.boundary_versions(["3.9", "3.10", "3.11"])
    assert "does not declare" in str(caught.value)


# ── check() has to be able to fail ───────────────────────────────────────────────


def test_the_committed_plan_is_sound() -> None:
    assert plan_owner.check(declared_versions()) == []


def test_check_reports_a_lost_coverage_role() -> None:
    """The control: this is the failure the literal conditionals used to hide.

    Strip the coverage flag from every leg - the cells all still exist and the suite would still pass,
    which is exactly what happened under `matrix.os == 'ubuntu-latest' && matrix.python-version ==
    '3.12'` once that cell was gone. `check()` has to name the role, not shrug.
    """
    versions = declared_versions()
    stripped = [{**leg, "coverage": False} for leg in plan_owner.plan("pull_request", versions)]
    problems = plan_owner.check(versions, legs=stripped)
    assert any("'coverage' role" in problem and "0" in problem for problem in problems), problems


def test_check_reports_a_version_that_runs_nothing() -> None:
    """A dropped cell that leaves a version untested is named, not averaged away.

    Removing ubuntu/3.9 from the grid is the edit a cost-cutting change makes happily; the version is
    still declared in `pyproject.toml`, and the only thing standing between that and silence is this
    rule.
    """
    versions = declared_versions()
    kept = [leg for leg in plan_owner.plan("pull_request", versions) if leg["python-version"] != "3.9"]
    assert ("ubuntu-latest", "3.9") not in cells(kept)
    problems = plan_owner.check(versions, legs=kept, smoke=plan_owner.smoke_plan(versions))
    assert any("3.9 runs no suite" in problem for problem in problems), problems


def test_check_reports_a_lost_boundary_cell_on_an_expensive_os() -> None:
    versions = declared_versions()
    legs = plan_owner.plan("pull_request", versions)
    for os_name, version in (("windows-latest", "3.8"), ("macos-latest", versions[-1])):
        kept = [leg for leg in legs if (leg["os"], leg["python-version"]) != (os_name, version)]
        assert len(kept) == len(legs) - 1, (os_name, version)
        problems = plan_owner.check(versions, legs=kept, smoke=plan_owner.smoke_plan(versions))
        assert any(
            problem.startswith(f"{os_name} does not run the suite on the {version} it owes") for problem in problems
        ), (os_name, version, problems)


def test_check_reports_a_gate_role_moved_onto_an_expensive_runner() -> None:
    """The invariant this change introduces, proven in both directions.

    `--cov-fail-under` counts exactly one leg, so moving the role to macOS keeps the count at one and
    would pass a validator that only counts. The runner it bills on has to be part of the rule.
    """
    versions = declared_versions()
    legs = [dict(leg) for leg in plan_owner.plan("pull_request", versions)]
    for leg in legs:
        leg["coverage"] = (leg["os"], leg["python-version"]) == ("macos-latest", versions[0])
    problems = plan_owner.check(versions, legs=legs, smoke=plan_owner.smoke_plan(versions))
    assert any("'coverage' role sits on macos-latest" in problem for problem in problems), problems
    assert plan_owner.check(versions) == [], plan_owner.check(versions)


def test_check_reports_two_tiers_overlapping_one_cell() -> None:
    """The bookkeeping rule that keeps `demoted, not dropped` from double-paying.

    If a cell appears in both tiers the totals still add up only by luck, and the grid quietly runs a
    version twice on one OS while another runs none.
    """
    versions = declared_versions()
    legs = plan_owner.plan("pull_request", versions)
    stolen = dict(legs[0])
    overlapping = [
        {
            "os": stolen["os"],
            "python-version": stolen["python-version"],
            "coverage": False,
            "compliance": False,
            "suite": False,
        }
    ]
    problems = plan_owner.check(versions, legs=legs, smoke=overlapping)
    assert any("in both tiers" in problem for problem in problems), problems


# ── the workflow is wired to the plan, not to a copy of it ───────────────────────


def workflow_document() -> dict:
    return yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))


def test_no_interpreter_version_is_hard_coded_in_the_test_matrix() -> None:
    """The grid must come from the plan; a literal list is how the two drift apart."""
    text = WORKFLOW.read_text(encoding="utf-8")
    job = workflow_document()["jobs"]["test"]
    matrix = job["strategy"]["matrix"]
    assert "include" in matrix and "python-version" not in matrix, matrix.keys()
    assert "fromJSON(needs.matrix-plan.outputs.suite-legs)" in str(matrix["include"])
    assert 'python-version: ["3.8"' not in text, "a literal version list is back in ci.yml"


def test_the_gate_conditions_read_the_roles_not_the_cells() -> None:
    """The specific defect: `matrix.python-version == '3.12'` cannot fail loudly when the cell goes.

    Read off the parsed workflow's `if:` and `run:` fields, not off the file: the first version of this
    check grepped the raw text and was tripped by the comment that explains why the literal is gone.
    """
    job = workflow_document()["jobs"]["test"]
    inspected = [str(job.get("if", ""))]
    for step in job.get("steps") or []:
        inspected.append(str(step.get("if", "")))
        inspected.append(str(step.get("run", "")))
    cell_keyed = [text for text in inspected if re.search(r"matrix\.python-version\s*==\s*[\"']3\.\d+[\"']", text)]
    assert not cell_keyed, f"a gate condition is keyed on a cell rather than a role: {cell_keyed}"
    assert any("matrix.coverage" in text for text in inspected), "nothing selects the coverage leg"
    assert any(text.strip() == "matrix.compliance" for text in inspected), (
        "the compliance summary is not selected by its role"
    )


def test_leg_names_are_still_the_shape_branch_protection_matches() -> None:
    """Check names are the merge contract; an explicit `name:` is required once the matrix has flags."""
    job = workflow_document()["jobs"]["test"]
    assert job["name"] == "test (${{ matrix.os }}, ${{ matrix.python-version }})", job.get("name")
    smoke = workflow_document()["jobs"]["wheel-smoke"]
    assert smoke["name"].startswith("wheel smoke ("), smoke["name"]


def test_the_fan_in_waits_on_the_new_jobs() -> None:
    """`test_matrix_verdict_gate.py` pins this too; asserted here with the plan's own vocabulary."""
    needs = workflow_document()["jobs"]["matrix-verdict"]["needs"]
    assert {"matrix-plan", "wheel-smoke", "test"} <= set(needs), needs


def test_the_smoke_job_does_not_claim_to_run_the_suite() -> None:
    """The corpus gate derives whole-suite jobs from the workflow; a false positive would dilute it.

    `tests/test_ci_corpus_gates.py` decides which jobs owe which corpus clones by looking for a
    whole-suite run. If the smoke tier ever grows a `pytest tests/`, it must also grow the clones - so
    this asserts the current shape rather than trusting a name.
    """
    import sys

    sys.path.insert(0, str(REPO_ROOT / "tests"))
    from test_ci_corpus_gates import suite_jobs

    names = {name for name, _steps in suite_jobs("ci.yml")}
    assert "test" in names and "test-freethreaded" in names, names
    assert "wheel-smoke" not in names, "the smoke job now looks like a whole-suite job: it owes the clones"


def test_the_plan_is_valid_json_for_a_matrix_include() -> None:
    """GitHub parses this at run time; a repr with single quotes is a red workflow, not a typo."""
    payload = plan_owner.plan("pull_request", declared_versions())
    assert json.loads(json.dumps(payload)) == payload
    for leg in payload:
        assert isinstance(leg["coverage"], bool) and isinstance(leg["suite"], bool)
        assert set(leg) == {"os", "python-version", "coverage", "compliance", "suite"}, leg


def test_the_workflow_checks_the_plan_before_spending_legs() -> None:
    """`--check` in the planner job: an unsound plan must fail before any leg is paid for."""
    WORKFLOW.read_text(encoding="utf-8")
    job = workflow_document()["jobs"]["matrix-plan"]
    runs = " ".join(str(step.get("run", "")) for step in job["steps"])
    assert "--check" in runs, "the planner emits without validating"
    assert "smoke-count" in json.dumps(job["outputs"]), "the empty-smoke case would launch a phantom leg"


def test_the_pinned_versions_and_the_plan_agree() -> None:
    """The floor CI imports checkers on and the floor the package declares are the same fact.

    `matrix-verdict` runs `check_matrix_verdict.py` on 3.8 on purpose - it is the job that must never be
    the one that breaks - and that choice is only meaningful while 3.8 is still the declared floor.
    """
    verdict = workflow_document()["jobs"]["matrix-verdict"]
    pinned = [
        step["with"]["python-version"]
        for step in verdict["steps"]
        if isinstance(step.get("with"), dict) and "python-version" in step["with"]
    ]
    assert pinned == [declared_versions()[0]], (
        f"the fan-in runs on {pinned}; the declared floor is {declared_versions()[0]}"
    )
