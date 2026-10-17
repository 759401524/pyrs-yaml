"""Every standalone cargo workspace needs its own formatting hook.

`cargo fmt` at the repository root formats the main workspace only; a nested manifest that declares
`[workspace]` is invisible to it. `fuzz/` is such a workspace, and the targets added by 39b1b9eb and
5f5262aa were unformatted while every gate stayed green (measured 2026-10-17: three files differed
under `cargo fmt --manifest-path fuzz/Cargo.toml -- --check`). The root `cargo-fmt` hook cannot be
extended with `&&` because a hook `entry` is executed without a shell, so each workspace needs a
hook of its own. This test derives the workspaces instead of naming `fuzz`, so a second one cannot
slip past the same way.
"""

from __future__ import annotations

import os
import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
PRUNED = {"target", ".git", ".jj", ".venv", "node_modules", "__pycache__"}


def standalone_workspaces() -> set[str]:
    """Repository-relative directories whose manifest declares `[workspace]`, excluding the root."""
    found: set[str] = set()
    for root, dirs, files in os.walk(REPO_ROOT):
        dirs[:] = [name for name in dirs if name not in PRUNED]
        if Path(root) == REPO_ROOT or "Cargo.toml" not in files:
            continue
        manifest = Path(root) / "Cargo.toml"
        text = manifest.read_text(encoding="utf-8")
        if re.search(r"^\[workspace\]\s*$", text, re.MULTILINE):
            found.add(manifest.parent.relative_to(REPO_ROOT).as_posix())
    return found


def test_every_standalone_workspace_has_its_own_fmt_hook() -> None:
    prek = (REPO_ROOT / "prek.toml").read_text(encoding="utf-8")
    missing = [
        workspace
        for workspace in sorted(standalone_workspaces())
        if f"cargo fmt --manifest-path {workspace}/Cargo.toml -- --check" not in prek
    ]
    assert not missing, (
        f"{missing}: standalone cargo workspaces with no formatting hook. The root `cargo fmt` "
        "reaches the main workspace only, and a hook entry runs without a shell, so each workspace "
        "needs its own hook."
    )
