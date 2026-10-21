"""The engine's CI legs skip the one crate that needs an interpreter, and the gate licenses that.

Two measured facts make this rule worth having. First, the cost is real and countable:
`cargo test --workspace --no-run --unit-graph -Z unstable-options` resolves 250 compilation units
over 153 packages on this tree, and that package set names `pyo3`, `pyo3-ffi`, `pyo3-macros`,
`pyo3-macros-backend`, `pyo3-build-config` and `numpy`; the same command with `--exclude pyrs-yaml`
resolves 220 units over 139 packages and names none of them. Second, what is given up is nothing:
`cargo nextest run --all` and `cargo nextest run --workspace --exclude pyrs-yaml` both run 748
tests, and the extension has no doctests to lose (`cargo test -p pyrs-yaml --doc` reports 0).

The exclusion is therefore licensed by `scripts/check_rust_python_decoupling.py` rule R2, which
fails the moment a `#[cfg(test)]` block, a `tests/` directory or a `[[test]]` target reappears in
the extension crate. This file closes the other half of the loop, because a licence nobody checks
expires silently: it reddens if a workspace-wide test step stops excluding the crate the gate names
as the extension, if such a step still sets `PYO3_PYTHON` after excluding it, or if the MSRV leg -
whose entire purpose is the declared floor for *every* crate - starts excluding it.

The extension crate is never named here. It is read out of the gate's own measurement, so a crate
that becomes the extension has to move the workflow and this rule together rather than edit a
string in a test.
"""

from __future__ import annotations

import importlib.util
import re
import sys
from pathlib import Path

import yaml

REPO_ROOT = Path(__file__).resolve().parent.parent
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci.yml"
ROOT_MANIFEST = REPO_ROOT / "Cargo.toml"
GATE = REPO_ROOT / "scripts" / "check_rust_python_decoupling.py"

# `cargo test ... --workspace` and `cargo nextest run ... --workspace`; `cargo check` and
# `cargo build` are not test steps and are deliberately out of scope.
_TEST_COMMAND = re.compile(r"\bcargo\s+(?:nextest\s+run|test)\b")
# The value of the flag, in either spelling cargo takes. Matching `--exclude pyrs-yaml` as a
# substring would accept `--exclude pyrs-yaml-core` and call the leg decoupled while it still
# builds all of pyo3 - a shape this file's own control found.
_EXCLUDE = re.compile(r"--exclude(?:=|\s)([A-Za-z0-9._-]+)")
_TOOLCHAIN_ACTION = "dtolnay/rust-toolchain@"


def _load_gate():
    spec = importlib.util.spec_from_file_location("rust_leg_decoupling_gate", GATE)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def declared_floor() -> str:
    """The `rust-version` every workspace member declares, after checking they agree."""
    members = re.search(r"^members = \[([^\]]+)\]", ROOT_MANIFEST.read_text(encoding="utf-8"), re.M)
    assert members, "Cargo.toml no longer lists its workspace members in one line"
    versions: set = set()
    for name in members.group(1).split(","):
        manifest = REPO_ROOT / name.strip().strip('"') / "Cargo.toml"
        match = re.search(r'^rust-version = "([^"]+)"', manifest.read_text(encoding="utf-8"), re.M)
        assert match, f"{manifest.relative_to(REPO_ROOT)} declares no rust-version"
        versions.add(match.group(1))
    assert len(versions) == 1, f"the workspace disagrees about its floor: {sorted(versions)}"
    return versions.pop()


def extension_crate(gate) -> str:
    """The crate the decoupling gate names as the extension, and the licence to skip it."""
    findings, measurements = gate.find_violations(REPO_ROOT)
    blocked = [found for found in findings if found[0] in ("R2", "R3")]
    assert blocked == [], (
        "the CI legs skip the extension crate only because it holds no Rust test tier, and the "
        "gate no longer agrees: " + "; ".join("{} {}: {}".format(*found) for found in blocked)
    )
    name = measurements.get("extension")
    assert name, "the decoupling gate found no extension crate, so there is nothing to exclude"
    return name


def workspace_test_steps(document: dict) -> list:
    """(job id, step name, run text, env keys) for every step that tests the whole workspace."""
    found = []
    for job_id, job in (document.get("jobs") or {}).items():
        for step in (job or {}).get("steps") or []:
            run = step.get("run") or ""
            if _TEST_COMMAND.search(run) and "--workspace" in run:
                found.append((job_id, str(step.get("name", "<unnamed>")), run, sorted(step.get("env") or {})))
    return found


def floor_jobs(document: dict, floor: str) -> set:
    """Jobs that install the declared floor - the ones that must keep testing every crate."""
    jobs = set()
    for job_id, job in (document.get("jobs") or {}).items():
        for step in (job or {}).get("steps") or []:
            uses = str(step.get("uses") or "")
            toolchain = str((step.get("with") or {}).get("toolchain") or "")
            if (uses.startswith(_TOOLCHAIN_ACTION) and uses.rsplit("@", 1)[-1] == floor) or toolchain == floor:
                jobs.add(job_id)
    return jobs


def excluded_names(run: str) -> set:
    """The crate names this command passes to `--exclude`, in either spelling."""
    return set(_EXCLUDE.findall(run))


def findings(document: dict, extension: str, floor: str) -> list:
    """Every way this workflow's Rust test legs can stop matching what the gate says about them."""
    steps = workspace_test_steps(document)
    if not steps:
        return ["no step runs cargo test/nextest with --workspace, so this rule measures nothing"]
    floors = floor_jobs(document, floor)
    engine = [step for step in steps if step[0] not in floors]
    whole = [step for step in steps if step[0] in floors]
    problems = []
    if not engine:
        problems.append(
            "every workspace test step installs the MSRV, so no leg is left to prove the engine "
            "builds without an interpreter"
        )
    for job_id, name, run, env in engine:
        where = f"{job_id} / {name}"
        skipped = excluded_names(run)
        if extension not in skipped:
            problems.append(
                "{} / {}: tests --workspace while excluding {}, so it builds the Python-runtime "
                "crates the gate says nothing here depends on".format(
                    job_id, name, ", ".join(sorted(skipped)) or "nothing"
                )
            )
        if "PYO3_PYTHON" in env:
            problems.append(
                f"{where}: sets PYO3_PYTHON while excluding {extension}, so the interpreter it "
                "points at is probed by nothing in the step"
            )
    for job_id, name, run, _env in whole:
        if extension in excluded_names(run):
            problems.append(
                f"{job_id} / {name}: excludes {extension}, which makes the declared floor a claim about every crate "
                "except the one that ships"
            )
    return problems


def test_the_workflow_agrees_with_the_gate() -> None:
    """The real file, read through the same function the controls below mutate."""
    gate = _load_gate()
    document = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
    problems = findings(document, extension_crate(gate), declared_floor())
    assert problems == [], "\n".join(problems)


def test_the_rule_sees_a_step_that_stops_excluding_it() -> None:
    """The main control: drop the flag from one leg and the leg is named."""
    gate = _load_gate()
    extension = extension_crate(gate)
    yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
    text = WORKFLOW.read_text(encoding="utf-8").replace(
        f"run: cargo test --workspace --exclude {extension} --locked",
        "run: cargo test --workspace --locked",
        1,
    )
    assert text != WORKFLOW.read_text(encoding="utf-8"), "the control did not change the workflow"
    problems = findings(yaml.safe_load(text), extension, declared_floor())
    assert any("rust-lint" in problem and "while excluding nothing" in problem for problem in problems), problems


def test_the_rule_sees_a_dead_interpreter_variable() -> None:
    """`PYO3_PYTHON` on an excluded leg is a symptom, not a bug: it says the leg still thinks it
    builds pyo3."""
    gate = _load_gate()
    extension = extension_crate(gate)
    text = WORKFLOW.read_text(encoding="utf-8").replace(
        "      - name: Run cargo tests\n        env:\n",
        "      - name: Run cargo tests\n        env:\n          PYO3_PYTHON: python\n",
        1,
    )
    problems = findings(yaml.safe_load(text), extension, declared_floor())
    assert any("PYO3_PYTHON" in problem for problem in problems), problems


def test_the_rule_refuses_to_let_the_floor_leg_narrow() -> None:
    """The exclusion is not a free win: applied to the MSRV leg it quietly drops a crate from the
    floor's own proof, so the same rule has to refuse it there."""
    gate = _load_gate()
    extension = extension_crate(gate)
    floor = declared_floor()
    text = WORKFLOW.read_text(encoding="utf-8").replace(
        "        run: cargo test --workspace --locked\n",
        f"        run: cargo test --workspace --exclude {extension} --locked\n",
        1,
    )
    problems = findings(yaml.safe_load(text), extension, floor)
    assert any("floor a claim about every crate except" in problem for problem in problems), problems


def test_the_floor_leg_is_identified_by_the_toolchain_not_by_its_name() -> None:
    """Renaming the job must not move the boundary: the classification reads the compiler it
    installs."""
    gate = _load_gate()
    extension = extension_crate(gate)
    floor = declared_floor()
    text = WORKFLOW.read_text(encoding="utf-8").replace("  msrv-check:\n", "  floor-check:\n", 1)
    document = yaml.safe_load(text)
    assert "floor-check" in document["jobs"], "the control renamed the wrong key"
    assert "msrv-check" not in floor_jobs(document, floor), floor_jobs(document, floor)
    assert floor_jobs(document, floor) == {"floor-check"}, floor_jobs(document, floor)
    problems = findings(document, extension, floor)
    assert problems == [], "\n".join(problems)


def test_an_exclusion_naming_the_wrong_crate_is_a_finding() -> None:
    """The flag is only meaningful against the crate the gate names; excluding some other member
    would read as a decoupling move while building all of pyo3."""
    gate = _load_gate()
    extension = extension_crate(gate)
    other = next(name for name in ("pyrs-yaml-core", "pyrs-json", "pyrs-toml") if name != extension)
    text = WORKFLOW.read_text(encoding="utf-8").replace(f"--exclude {extension}", f"--exclude {other}")
    problems = findings(yaml.safe_load(text), extension, declared_floor())
    assert problems, f"the rule accepted an exclusion of {other} instead of {extension}"


def test_the_other_flag_spelling_means_the_same_thing() -> None:
    """`--exclude NAME` and `--exclude=NAME` are one instruction to cargo, so they must be one
    instruction to this rule; reading only the first would turn a reformat into a false red."""
    gate = _load_gate()
    extension = extension_crate(gate)
    text = WORKFLOW.read_text(encoding="utf-8").replace(f"--exclude {extension}", f"--exclude={extension}")
    document = yaml.safe_load(text)
    excluded_runs = [step[2] for step in workspace_test_steps(document) if "--exclude" in step[2]]
    assert len(excluded_runs) == 2, excluded_runs
    assert all(f"--exclude={extension}" in run for run in excluded_runs), excluded_runs
    assert findings(document, extension, declared_floor()) == []
