"""The extension under test must be built from the sources it is tested against.

The built module inside `python/pyrs_yaml/` belongs to whichever sources were checked out when it was
last compiled, so a Python run after `jj edit` or `jj rebase` can silently test another branch's binary.
That has already produced three misleading results in one session: a JSON5 test failing against a build
that predated its fix, a schema test failing against a build that predated another, and a passing run
that had proved nothing. This file is why AGENTS.md says to rebuild after moving the working copy.

Until now that obligation was prose. Ignoring it produces a *result*, not an error, which is what makes
it expensive — a wrong answer looks exactly like an answer. The check below asks the one question that
decides whether a pytest run is evidence at all: is the loaded binary newer than every source file that
was compiled into it? It names the offenders, so the response is a rebuild rather than a bisect.

Scope is the compiled tree, not the repository. A comment edit under `docs/` does not make a binary
stale, and Rust modules gated by `#[cfg(test)]` never reach the shipped extension, so editing a property
test and re-running pytest without rebuilding is correct behaviour rather than a hole. The excluded
names are derived from the crate roots' own `#[cfg(test)] mod` declarations instead of kept as a list,
because a list rots into excluding nothing.

Where it bites: on a development box whose working copy holds a built extension, and on the CI legs that
run `maturin develop`, where the copy is written after the checkout and the rule holds trivially. A
wheel installed outside the working copy is skipped with a reason — those sources did not build it.
"""

from __future__ import annotations

import re
from pathlib import Path

import pytest

from pyrs_yaml import pyrs_yaml as _native

REPO_ROOT = Path(__file__).resolve().parent.parent

_MOD_DECL = re.compile(r"^(?:pub\s+)?mod\s+(\w+)\s*;$")


def _cfg_test_modules(root: Path) -> set[str]:
    """Module names a crate root declares under a `#[cfg(test)]` attribute.

    Those modules are compiled for `cargo test` only, so their source dates say nothing about the
    shipped binary.
    """
    names: set[str] = set()
    for crate_root in sorted((root / "crates").glob("*/src/lib.rs")):
        lines = crate_root.read_text(encoding="utf-8").splitlines()
        for index, line in enumerate(lines[:-1]):
            if line.strip() != "#[cfg(test)]":
                continue
            declared = _MOD_DECL.match(lines[index + 1].strip())
            if declared:
                names.add(declared.group(1))
    return names


def _shipped_sources(root: Path) -> list[Path]:
    """Every file whose bytes the built extension is made of."""
    test_only = _cfg_test_modules(root)
    crates = root / "crates"
    sources: list[Path] = []
    for path in sorted(crates.glob("*/src/**/*.rs")):
        crate = path.relative_to(crates).parts[0]
        inside_src = path.relative_to(crates / crate / "src")
        if test_only & (set(inside_src.parts[:-1]) | {inside_src.stem}):
            continue
        sources.append(path)
    sources.extend(sorted(crates.glob("*/Cargo.toml")))
    sources.extend(p for name in ("Cargo.toml", "Cargo.lock") if (p := root / name).is_file())
    return sources


def _stale(sources: list[Path], binary_mtime: float) -> list[Path]:
    return [path for path in sources if path.stat().st_mtime > binary_mtime]


def _loaded_binary() -> Path | None:
    """The native module this interpreter actually loaded, when it lives in this working copy."""
    origin = getattr(_native, "__file__", None)
    if not origin:
        return None
    path = Path(origin)
    return path if path.is_file() and REPO_ROOT in path.parents else None


def test_the_loaded_extension_is_newer_than_the_sources_that_built_it() -> None:
    binary = _loaded_binary()
    if binary is None:
        pytest.skip("the loaded extension is outside this working copy, so these sources did not build it")

    binary_mtime = binary.stat().st_mtime
    offenders = _stale(_shipped_sources(REPO_ROOT), binary_mtime)
    if not offenders:
        return

    listed = ", ".join(f"{path.relative_to(REPO_ROOT).as_posix()} (written later)" for path in offenders[:8])
    more = f", and {len(offenders) - 8} more" if len(offenders) > 8 else ""
    raise AssertionError(
        f"{binary.name} predates {len(offenders)} source file(s) compiled into it: {listed}{more}. "
        "Whatever this interpreter just reported describes an older working copy. "
        "Rebuild with `uv run maturin develop --release`, then re-run."
    )


def test_the_gate_names_the_compiled_sources_and_nothing_else(tmp_path: Path) -> None:
    """A newer source must be named, and a file that never reaches the binary must not be.

    The working copy is rebuilt here as a fixture because the interesting cases are the exclusions:
    they are what keeps the gate from asking for a rebuild after every Rust test edit.
    """
    src = tmp_path / "crates" / "demo" / "src"
    src.mkdir(parents=True)
    (src / "lib.rs").write_text(
        "#[cfg(test)]\nmod pbt;\n#[cfg(test)]\nmod integration;\n\npub mod serialize;\n",
        encoding="utf-8",
    )
    (src / "serialize.rs").write_text("pub fn emit() {}", encoding="utf-8")
    (src / "pbt.rs").write_text("#[test]\nfn round_trip() {}", encoding="utf-8")
    nested = src / "integration"
    nested.mkdir()
    (nested / "mod.rs").write_text("#![cfg(test)]\nmod cases;", encoding="utf-8")
    (nested / "cases.rs").write_text("#[test]\nfn merges() {}", encoding="utf-8")
    (tmp_path / "crates" / "demo" / "Cargo.toml").write_text("[package]", encoding="utf-8")
    (tmp_path / "Cargo.toml").write_text("[workspace]", encoding="utf-8")
    (tmp_path / "Cargo.lock").write_text("version = 3", encoding="utf-8")
    outside = tmp_path / "crates" / "demo" / "benches"
    outside.mkdir()
    (outside / "ir_gate.rs").write_text("fn bench() {}", encoding="utf-8")
    (tmp_path / "docs").mkdir()
    (tmp_path / "docs" / "notes.md").write_text("prose", encoding="utf-8")

    shipped = _shipped_sources(tmp_path)
    names = {path.relative_to(tmp_path).as_posix() for path in shipped}
    assert names == {
        "crates/demo/src/lib.rs",
        "crates/demo/src/serialize.rs",
        "crates/demo/Cargo.toml",
        "Cargo.toml",
        "Cargo.lock",
    }
    assert _cfg_test_modules(tmp_path) == {"pbt", "integration"}

    # Everything above was written now, so a binary stamped in the past is stale against all of it,
    # and a binary stamped in the future is stale against none of it.
    assert _stale(shipped, 1_000_000.0) == shipped
    assert _stale(shipped, shipped[-1].stat().st_mtime + 86_400.0) == []


def test_the_derived_exclusion_still_excludes_something() -> None:
    """A derivation that silently matches nothing would re-open the rebuild-on-every-edit friction."""
    names = _cfg_test_modules(REPO_ROOT)
    assert names, "no crate root declares a #[cfg(test)] mod, so nothing is excluded"
    every_source = set((REPO_ROOT / "crates").glob("*/src/**/*.rs"))
    assert every_source - set(_shipped_sources(REPO_ROOT)), f"{sorted(names)} excludes no file on disk"
