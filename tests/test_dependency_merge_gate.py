"""The dependency-update policy, checked as a wiring contract rather than read as prose.

`.github/dependabot.yml` opens pull requests, `scripts/dependency_merge_gate.py` decides which of
them may merge themselves, and `scripts/check_changelog_coupling.py --allow-lockfile-bumps` is the
gate they all have to pass first. Each half is easy to write and easy to leave disagreeing, which is
the failure this file is about: a bump that can never go green because the release-note gate will not
exempt it, or one that merges with a supported interpreter floor inside it.

The rules themselves are exercised by the script's own `--selftest` and by rule 3 in
`tests/test_changelog_coupling_gate.py`; what is pinned here is that the three files name the same
check, the same labels and the same allowlist, and that nothing on the path to a merge is a bypass.
"""

from __future__ import annotations

import importlib.util
import re
import sys
from pathlib import Path

import pytest
import yaml

REPO_ROOT = Path(__file__).resolve().parent.parent
DEPENDABOT = REPO_ROOT / ".github" / "dependabot.yml"
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "deps-automerge.yml"
SCRIPT = REPO_ROOT / "scripts" / "dependency_merge_gate.py"
COUPLING = REPO_ROOT / "scripts" / "check_changelog_coupling.py"


def _load(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def gate():
    return _load(SCRIPT, "dependency_merge_gate")


@pytest.fixture(scope="module")
def config() -> dict:
    return yaml.safe_load(DEPENDABOT.read_text(encoding="utf-8"))


def _entries(config: dict, ecosystem: str) -> list[dict]:
    return [e for e in config["updates"] if e["package-ecosystem"] == ecosystem]


def _ignored_names(entry: dict) -> set[str]:
    return {i["dependency-name"] for i in entry.get("ignore", [])}


def _allowed_names(entry: dict) -> set[str]:
    return {a["dependency-name"] for a in entry.get("allow", [])}


def _bot_pr(**overrides) -> dict:
    pr = {"user": {"login": "dependabot[bot]"}, "state": "open", "mergeable": True, "labels": []}
    return {**pr, **overrides}


# ── what is tracked, how often, and on which branch ────────────────────────────────


def test_the_three_pinned_ecosystems_are_all_tracked(config):
    """Cargo, uv and the CI actions: the three places this repository pins versions."""
    assert {e["package-ecosystem"] for e in config["updates"]} == {"cargo", "uv", "github-actions"}


def test_the_fuzz_workspace_lock_is_covered_too(config):
    """`fuzz/` carries its own `Cargo.lock`; untracked, it rots into the "why does replay differ"
    class of problem the fuzz corpus has already been caught hiding once."""
    directories = {d for e in _entries(config, "cargo") for d in e.get("directories", [e.get("directory")])}
    assert {"/", "/fuzz"} <= directories, directories


def test_every_entry_targets_main(config):
    """One branch strategy: bumps land on `main`, never on an open feature branch."""
    assert all(e["target-branch"] == "main" for e in config["updates"])


def test_every_entry_is_scheduled_and_bounded(config):
    """Weekly version updates, and a per-entry ceiling on open pull requests - the ceiling is what
    keeps a Friday rebuild from eating the review queue."""
    for entry in config["updates"]:
        assert entry["schedule"]["interval"] == "weekly", entry["package-ecosystem"]
        assert 1 <= entry["open-pull-requests-limit"] <= 10, entry


def test_security_updates_are_not_suppressed(config):
    """The only `ignore` forms here are deliberate pins and a floor, never an advisory class."""
    for entry in config["updates"]:
        assert entry.get("secure") is not False
        for ignored in entry.get("ignore", []):
            assert set(ignored) <= {"dependency-name", "update-types", "versions"}, ignored


def test_the_config_uses_only_what_the_hosted_schema_documents(config):
    """A key or value the hosted schema rejects does not fail loudly - it stops updates for that
    ecosystem, the silent shape this repository refuses elsewhere (`AGENTS.md`: a check that never
    reports is not a pass). So the boundary a `versioning-strategy` would express - resolve, do not
    widen - is enforced in the merge gate instead, where a test can see it."""
    documented = {
        "package-ecosystem",
        "directory",
        "directories",
        "target-branch",
        "schedule",
        "open-pull-requests-limit",
        "labels",
        "commit-message",
        "ignore",
        "allow",
        "groups",
    }
    for entry in config["updates"]:
        assert set(entry) <= documented, sorted(set(entry) - documented)


def test_an_unmergeable_bump_is_refused_rather_than_hand_resolved(gate):
    """Conflict policy: nothing merges a pull request whose head does not apply onto its base. A
    hand-resolved lockfile under a moved manifest is what the Ir floor cannot be read against, so the
    answer is to fix the conflict, not to merge through it."""
    assert not gate.decide(_bot_pr(mergeable=False), ["Cargo.lock"], "success").merge


# ── the versions that are pinned on purpose, and the interpreter floor ─────────────


def test_the_stub_generator_is_not_bumped_by_a_bot(config):
    """`AGENTS.md` locks maturin because the committed stub and `check_stub_drift.py` describe it."""
    assert "maturin" in _ignored_names(_entries(config, "uv")[0])


def test_the_docs_toolchain_is_not_bumped_by_a_bot(config):
    uv = _entries(config, "uv")[0]
    assert {"zensical", "mkdocstrings-python"} <= _ignored_names(uv)


def test_a_parser_major_is_a_migration_not_a_bump(config):
    """granit 1.3 -> 2.0 was a measured migration of spans and comment placement; a major must not
    arrive as a lockfile line, while patch and minor updates still do."""
    majors = [i for i in _entries(config, "cargo")[0].get("ignore", [])]
    granit = [i for i in majors if i.get("dependency-name") == "granit-parser"]
    assert granit and granit[0]["update-types"] == ["version-update:semver-major"]


def test_pyo3_is_tracked_by_exactly_one_entry_that_carries_a_refused_label(config, gate):
    """pyo3 0.30+ raises the interpreter floor to 3.9 while the shipped wheels are `abi3-py38`.

    Two halves, both needed. The routine cargo entry must ignore pyo3, or the same bump arrives twice
    and one copy is unlabelled; and the entry that does track it must carry a label `decide()`
    refuses, or an auto-merge answers the Python 3.8 question by itself.
    """
    assert "pyo3" in _ignored_names(_entries(config, "cargo")[0])
    tracked = [e for e in config["updates"] if "pyo3" in _allowed_names(e)]
    assert len(tracked) == 1, "pyo3 must be tracked by exactly one entry"
    refused = set(tracked[0]["labels"]) & set(gate.REFUSED_LABELS)
    assert refused, f"no label on the pyo3 entry stops the merge gate: {tracked[0]['labels']}"


# ── the auto-merge wiring ──────────────────────────────────────────────────────────


def test_the_verdict_name_is_one_name_across_the_files_that_use_it(gate):
    """The required check's name is the contract. A rename in one place silently un-gates the rest,
    which is the same failure `tests/test_matrix_verdict_gate.py` watches for inside CI."""
    text = WORKFLOW.read_text(encoding="utf-8") + SCRIPT.read_text(encoding="utf-8")
    assert gate.REQUIRED_CHECK in text
    assert gate.REQUIRED_CHECK in (REPO_ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
    assert gate.REQUIRED_CHECK in (REPO_ROOT / "scripts" / "check_matrix_verdict.py").read_text(encoding="utf-8")


def test_the_workflow_triggers_on_the_ci_verdict_not_on_its_own_checks():
    text = WORKFLOW.read_text(encoding="utf-8")
    assert "workflow_run" in text, "a PR-scoped workflow cannot merge the PR its own checks gated"
    assert "- CI" in text or "workflows: [CI]" in text, "it must trigger on the CI workflow"
    assert "conclusion == 'success'" in text, "only a CI run that finished successfully may merge"
    assert "event == 'pull_request'" in text, "a push or schedule run has no pull request to merge"


def test_nothing_on_the_merge_path_is_a_bypass(gate):
    """The prose is allowed to say `--admin` to explain why it never does; the calls are not.

    Checked on the `gh` argument lists and on the workflow's non-comment lines, because a
    substring test over whole files only ever finds the sentence warning about the bypass.
    """
    calls = re.findall(r"\[[^\]]*\"gh\"[^\]]*\]", SCRIPT.read_text(encoding="utf-8"))
    assert calls, "the script shells out to gh; if it stops doing that this check is stale"
    assert all("--admin" not in call and "--no-verify" not in call for call in calls), calls
    steps = [line for line in WORKFLOW.read_text(encoding="utf-8").splitlines() if not line.lstrip().startswith("#")]
    joined = "\n".join(steps)
    for bypass in ("--admin", "--no-verify", "skip-checks"):
        assert bypass not in joined, f"{bypass} is how an unverified dependency reaches main"
    assert '"--rebase"' in " ".join(calls), "this repository merges by rebase"
    assert gate.DEPENDABOT == "dependabot[bot]", "GitHub's bot login is the author check itself"


def test_permissions_are_the_minimum_for_a_merge():
    loaded = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
    assert loaded["permissions"] == {"contents": "read", "pull-requests": "write"}
    assert loaded["concurrency"]["cancel-in-progress"] is False, (
        "cancelling between the verdict and the merge leaves a bump approved and unmerged"
    )


def test_the_workflow_defers_every_decision_to_the_script():
    text = WORKFLOW.read_text(encoding="utf-8")
    assert "scripts/dependency_merge_gate.py --pr" in text
    for rule in ("Cargo.lock", "python-floor"):
        assert rule not in text.split("jobs:")[1], f"{rule} belongs to the script, where it can be table-tested"


# ── the path rule, exercised ───────────────────────────────────────────────────────


@pytest.mark.parametrize(
    "path",
    ["Cargo.lock", "uv.lock", "fuzz/Cargo.lock", ".github/workflows/versions.yml"],
)
def test_a_resolution_only_bump_may_merge_itself(gate, path):
    assert gate.decide(_bot_pr(), [path], "success").merge, path


@pytest.mark.parametrize(
    "path",
    [
        "Cargo.toml",
        "crates/pyrs-yaml/Cargo.toml",
        "pyproject.toml",
        "python/pyrs_yaml/pyrs_yaml.pyi",
        "rust-toolchain.toml",
        "crates/pyrs-yaml-core/src/parser/mod.rs",
    ],
)
def test_a_range_a_floor_or_the_shipped_stub_waits_for_a_human(gate, path):
    assert not gate.decide(_bot_pr(), [path], "success").merge, path


def test_a_source_file_smuggled_into_a_lockfile_bump_is_refused(gate):
    assert not gate.decide(_bot_pr(), ["Cargo.lock", "src/lib.rs"], "success").merge


@pytest.mark.parametrize("verdict", ["failure", "skipped", "cancelled", "action_required", ""])
def test_every_non_success_verdict_is_a_refusal(gate, verdict):
    """`skipped` and `cancelled` are refusals, not passes - the reading
    `scripts/check_matrix_verdict.py` already enforces for the verdict job itself."""
    assert not gate.decide(_bot_pr(), ["Cargo.lock"], verdict).merge, verdict


def test_a_human_pull_request_is_not_this_gates_business(gate):
    decision = gate.decide(_bot_pr(user={"login": "someone"}), ["Cargo.lock"], "success")
    assert not decision.merge and "not the dependency bot" in decision.reason


def test_a_label_stops_the_merge_even_with_everything_else_green(gate):
    for label in gate.REFUSED_LABELS:
        pr = _bot_pr(labels=[{"name": label}])
        assert not gate.decide(pr, ["Cargo.lock"], "success").merge, label


def test_the_selftest_covers_the_rules_the_docstring_claims(gate, capsys):
    assert gate.selftest() == 0
    printed = capsys.readouterr().out
    for name in ("not the dependency bot", "skipped", "floor label", "declared range"):
        assert name in printed, f"{name} is promised in prose and absent from the fixtures"


def test_the_lockfile_allowlist_and_the_release_note_exemption_agree(gate):
    """One judgement, two gates. If the auto-merge allowlist and the coupling exemption ever name
    different files, a bump is either stuck unable to go green or lands unannounced."""
    coupling = _load(COUPLING, "check_changelog_coupling")
    names = {path.rsplit("/", 1)[-1] for path in gate.AUTOMERGE_ALLOWLIST}
    assert set(coupling.LOCKFILE_NAMES) <= names, coupling.LOCKFILE_NAMES
    assert all(coupling.is_lockfile_bump(p) for p in gate.AUTOMERGE_ALLOWLIST if "lock" in p)
