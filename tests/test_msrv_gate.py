"""The MSRV job installs the compiler the crates declare as their floor.

`msrv-check` is the only job that can notice a dependency raising the minimum supported Rust
version. `rust-version` in a manifest documents a floor; it does not enforce it, so compiling the
workspace on current `stable` proves only that it builds on current `stable`. The job installed
`dtolnay/rust-toolchain@stable` from f6b590b3 (2026-08-16) onward, which made that the whole of its
check: a dependency needing a newer compiler than 1.96 would have passed the job and failed at a
user's install.

Measured on 2026-10-19 before the pin changed: all eight crates declare `1.96`, and
`cargo +1.96.0 check --workspace --locked` and `cargo +1.96.0 test --workspace --locked` both pass
on this tree, so the declared floor is real and only the workflow was not asking about it.

Both directions are checked, because both are silent: a workflow pin that moves away from the
manifests, and a manifest that moves away from the pin. The install command may name the version in
the action ref or in `with.toolchain` - `publish.yml` uses the second spelling - but the effective
toolchain is what the job runs, so that is what is compared.
"""

from __future__ import annotations

import re
from pathlib import Path

import yaml

REPO_ROOT = Path(__file__).resolve().parent.parent
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci.yml"
ROOT_MANIFEST = REPO_ROOT / "Cargo.toml"
INSTALLER = "dtolnay/rust-toolchain@"


def workspace_manifests() -> list[Path]:
    """Every workspace member's `Cargo.toml`, read from the root `members` list.

    The list rather than a glob over `crates/`: the MSRV job compiles `--workspace`, so the crates
    it actually builds are the set whose floors must agree. A member added outside `crates/` is
    covered by this reading and would be missed by a glob.
    """
    text = ROOT_MANIFEST.read_text(encoding="utf-8")
    members = re.search(r"^members = \[([^\]]+)\]", text, re.M)
    assert members, "Cargo.toml no longer lists its workspace members in one line"
    paths = [REPO_ROOT / name.strip().strip('"') / "Cargo.toml" for name in members.group(1).split(",")]
    assert paths, "the members list parsed empty"
    return paths


def declared_msrv() -> str:
    """The single `rust-version` the workspace declares, after checking there is one."""
    versions: dict[str, list[str]] = {}
    for manifest in workspace_manifests():
        text = manifest.read_text(encoding="utf-8")
        match = re.search(r'^rust-version = "([^"]+)"', text, re.M)
        assert match, f"{manifest.relative_to(REPO_ROOT)} declares no rust-version"
        versions.setdefault(match.group(1), []).append(str(manifest.relative_to(REPO_ROOT)))
    assert len(versions) == 1, f"the workspace disagrees about its floor: {versions}"
    msrv = next(iter(versions))
    assert re.fullmatch(r"\d+\.\d+(?:\.\d+)?", msrv), f"rust-version is not a plain version: {msrv!r}"
    return msrv


def msrv_job_steps() -> list[dict]:
    document = yaml.safe_load(WORKFLOW.read_text(encoding="utf-8"))
    assert "msrv-check" in document["jobs"], "ci.yml has no msrv-check job"
    return document["jobs"]["msrv-check"].get("steps", [])


def test_every_workspace_crate_declares_one_numeric_floor():
    """A member without the field, or two members disagreeing, leaves the workflow pin unanchored."""
    declared_msrv()


def test_the_msrv_job_installs_the_declared_floor():
    installers = [step for step in msrv_job_steps() if str(step.get("uses", "")).startswith(INSTALLER)]
    assert len(installers) == 1, installers
    install = installers[0]
    ref = install["uses"][len(INSTALLER) :]
    effective = str((install.get("with") or {}).get("toolchain") or ref)
    assert effective == declared_msrv(), (
        f"msrv-check installs {effective!r}, and the crates declare {declared_msrv()!r}: the job "
        "would compile on a compiler the floor does not describe"
    )


def test_the_msrv_job_builds_and_tests_the_workspace():
    """An installer with nothing behind it proves the compiler downloads, not that the tree builds."""
    runs = "\n".join(str(step.get("run", "")) for step in msrv_job_steps())
    assert re.search(r"^cargo check --workspace --locked$", runs, re.M), runs
    assert re.search(r"^cargo test --workspace --locked$", runs, re.M), runs
