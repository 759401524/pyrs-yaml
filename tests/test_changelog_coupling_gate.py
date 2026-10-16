"""Self-test for the changelog coupling gate (`scripts/check_changelog_coupling.py`).

The rule: a changeset that moves the product moves the release notes, and it moves *all five*
mirrors or none. Two failures this gate is named for, both real and both invisible to every
checker that existed before it:

* PR #296 shipped two fuzz targets, two deterministic corpus gates and a `fuzz.yml` matrix row
  without touching any changelog - `check_changelog_mirrors.py` compares version headers, which
  only move at release time, and `prek.toml` runs it on a `files:` pattern that a changeset
  without a changelog never matches.
* The partial-update rule in `AGENTS.md` ("never commit partial updates") was prose: nothing
  compared *which* of the five mirrors a changeset touched.

So the cases below are the two shapes, plus the exclusions that keep the gate from reddening
dependency bumps, plus the pin that the two changelog checkers still agree on the mirror list.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
GATE_PATH = REPO_ROOT / "scripts" / "check_changelog_coupling.py"

FIVE = [
    "CHANGELOG.md",
    "docs/en/changelog.md",
    "docs/ja/changelog.md",
    "docs/ko/changelog.md",
    "docs/zh/changelog.md",
]

# `git show --name-only 39b1b9eb`, reduced to one path per component it touched. The commit is
# the defect this gate was written for, so the fixture is the real file list rather than a
# paraphrase of it.
PR_296 = [
    ".ci/quality-holes.json",
    ".github/workflows/fuzz.yml",
    "QUALITY_MATRIX.md",
    "ROADMAP.md",
    "crates/pyrs-json/tests/roundtrip_corpus.rs",
    "crates/pyrs-toml/tests/roundtrip_corpus.rs",
    "fuzz/Cargo.toml",
    "fuzz/fuzz_targets/json_roundtrip.rs",
    "fuzz/fuzz_targets/toml_roundtrip.rs",
    "fuzz/seeds/json_roundtrip/shape-deep-nesting.seed",
    "scripts/quality_matrix.py",
]

# Dependabot's `chore(deps): bump docker/setup-qemu-action` and the ruff/rumdl hook bump: the two
# commits the first draft of the trigger list would have reddened for no reason.
DEPENDENCY_BUMPS = [
    ".github/workflows/publish.yml",
    "prek.toml",
    "README.md",
]


def _load_gate():
    return _load(GATE_PATH, "check_changelog_coupling")


def _load(path: Path, name: str):
    """Import a `scripts/` module by path: they are scripts, not an installed package."""
    spec = importlib.util.spec_from_file_location(name, path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def gate():
    return _load_gate()


# ── rule 1: coupling ──────────────────────────────────────────────────────────────


def test_pr_296_shape_reddens(gate):
    """A defence-tier changeset with no changelog anywhere is the named failure."""
    found = gate.findings(PR_296)
    assert len(found) == 1, found
    assert "no release note" in found[0], found[0]
    assert "none of 5 changelogs" in found[0], found[0]


def test_the_same_changeset_with_the_changelogs_is_green(gate):
    assert gate.findings(PR_296 + FIVE) == []


def test_dependency_bumps_are_out_of_scope(gate):
    """Calibration: including workflows and hook pins reddened dependabot for no reason."""
    assert gate.findings(DEPENDENCY_BUMPS) == []


@pytest.mark.parametrize(
    ("path", "trigger"),
    [
        ("crates/pyrs-toml/src/writer.rs", True),
        ("crates/pyrs-json/tests/roundtrip_corpus.rs", True),
        ("python/pyrs_yaml/editing.py", True),
        ("fuzz/fuzz_targets/parse_yaml.rs", True),
        ("scripts/check_changelog_mirrors.py", True),
        ("tests/test_route_parity.py", True),
        ("Cargo.lock", True),
        ("pyproject.toml", True),
        # Not product changes: prose, translation, and the gate's own bookkeeping.
        ("docs/en/api.md", False),
        ("docs/ja/changelog.md", False),
        ("README.md", False),
        ("QUALITY_MATRIX.md", False),
        ("ROADMAP.md", False),
        (".ci/quality-holes.json", False),
        (".github/workflows/ci.yml", False),
        ("prek.toml", False),
        ("Reference/yaml-test-suite/data.yaml", False),
        ("site/en/index.html", False),
    ],
)
def test_trigger_predicate(gate, path, trigger):
    assert gate.is_trigger(path) is trigger, path


def test_backslashes_are_normalised(gate):
    """The hook and the CI runner hand over platform-native separators."""
    assert gate.findings(["crates\\pyrs-ast\\src\\ast.rs"]) != []


# ── rule 2: completeness ──────────────────────────────────────────────────────────


def test_partial_mirror_update_names_what_is_missing(gate):
    paths = ["crates/pyrs-ast/src/ast.rs", "CHANGELOG.md", "docs/en/changelog.md"]
    found = gate.findings(paths)
    assert len(found) == 1, found
    for missing in ("docs/ja/changelog.md", "docs/ko/changelog.md", "docs/zh/changelog.md"):
        assert missing in found[0], (missing, found[0])


def test_a_changelog_only_changeset_still_has_to_be_complete(gate):
    """A translation catch-up committed alone is exactly the partial update the rule forbids."""
    assert gate.findings(["docs/ko/changelog.md"]) != []


def test_docs_only_changesets_are_green(gate):
    assert gate.findings(["docs/en/api.md", "site/en/index.html"]) == []


# ── the two changelog gates agree ─────────────────────────────────────────────────


def test_the_mirror_list_is_one_declaration_not_two(gate):
    """`check_changelog_mirrors.py` polices the same five files; a divergence would be silent."""
    spec = importlib.util.spec_from_file_location(
        "check_changelog_mirrors", REPO_ROOT / "scripts" / "check_changelog_mirrors.py"
    )
    assert spec is not None and spec.loader is not None
    mirrors = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = mirrors
    spec.loader.exec_module(mirrors)
    assert [Path(p).relative_to(REPO_ROOT).as_posix() for p in mirrors.FILES] == list(gate.MIRRORS)


def test_the_gate_is_wired_where_a_merge_can_be_blocked(gate):
    """A checker no CI step runs is prose. `quality_matrix.py`'s orphan-gate probe agrees."""
    workflow = (REPO_ROOT / ".github" / "workflows" / "hygiene.yml").read_text(encoding="utf-8")
    assert "scripts/check_changelog_coupling.py" in workflow, "not invoked by any job"
    assert "--base" in workflow
    assert "pull_request" in workflow
    # Rule 3 has to be granted by author, or the exemption becomes a hole: anyone who names their
    # branch like the bot would ship a bump with no note.
    assert "--allow-lockfile-bumps" in workflow, "the bot's exemption is not wired"
    assert "github.actor == 'dependabot[bot]'" in workflow, (
        "the exemption must be conditional on the pull request's author, not unconditional"
    )


def test_base_mode_reads_a_real_diff(gate):
    """The CI entry point resolves its path list from Git rather than from argv."""
    head = subprocess.run(["git", "rev-parse", "HEAD"], capture_output=True, text=True, check=True).stdout.strip()
    paths, how = gate.staged_or_base(["--base", head])
    assert how.startswith("--base")
    assert paths == [], "a diff of HEAD against HEAD should be empty"


def test_cli_exit_codes(gate):
    assert gate.main(["--paths", *PR_296]) == 1
    assert gate.main(["--paths", *PR_296, *FIVE]) == 0
    assert gate.main(["--paths"]) == 0, "an empty changeset is not a violation"


# ── rule 3: the dependency bot's lockfile exemption ────────────────────────────────

LOCKFILE_BUMPS = [
    ["Cargo.lock"],
    ["uv.lock"],
    ["Cargo.lock", "uv.lock"],
    ["fuzz/Cargo.lock"],
]


@pytest.mark.parametrize("paths", LOCKFILE_BUMPS)
def test_a_lockfile_only_bump_is_exempt_when_the_caller_says_so(gate, paths):
    assert gate.findings(paths, allow_lockfile_bumps=True) == []


@pytest.mark.parametrize("paths", LOCKFILE_BUMPS)
def test_the_same_bump_needs_a_note_from_anyone_else(gate, paths):
    """The flag is the exemption, so the default stays strict for the hook and for a human."""
    assert len(gate.findings(paths)) == 1


def test_a_bump_that_also_moves_code_still_owes_the_note(gate):
    found = gate.findings(["Cargo.lock", "crates/pyrs-yaml-core/src/parser/mod.rs"], True)
    assert len(found) == 1, found
    assert "no release note" in found[0], found[0]


def test_a_manifest_bump_is_not_a_lockfile_bump(gate):
    """`Cargo.toml` states a range; moving a range is the decision this exemption is not for."""
    assert len(gate.findings(["Cargo.toml", "Cargo.lock"], True)) == 1


def test_the_exemption_does_not_reach_the_completeness_rule(gate):
    """Rule 2 still bites: an exempt bump that carries one changelog must carry all five."""
    found = gate.findings(["Cargo.lock", "CHANGELOG.md"], allow_lockfile_bumps=True)
    assert len(found) == 1, found
    assert "partial changelog update" in found[0], found[0]


def test_the_cli_flag_reaches_the_decision(gate, capsys):
    assert gate.main(["--paths", "Cargo.lock"]) == 1
    assert gate.main(["--paths", "Cargo.lock", "--allow-lockfile-bumps"]) == 0
    out = capsys.readouterr().out
    assert "lockfile-only bump" in out, "the exemption has to be visible in the CI log"


def test_lockfile_names_are_one_declaration_across_the_two_gates(gate):
    """The bump needs both decisions to agree, or it either never merges or merges unannounced."""
    merge = _load(REPO_ROOT / "scripts" / "dependency_merge_gate.py", "dependency_merge_gate")
    allowed = {p.rsplit("/", 1)[-1] for p in merge.AUTOMERGE_ALLOWLIST}
    assert set(gate.LOCKFILE_NAMES) <= allowed, gate.LOCKFILE_NAMES
