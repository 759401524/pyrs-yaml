"""The Ir harnesses are feature-gated, so a lint scope that omits the feature never sees them.

`--all-targets` is CI's clippy scope, and the repository adopted it precisely so that a green PR
could not leave `tests/` and `benches/` unlinted. It cannot reach these two targets by construction:
both `crates/pyrs-yaml-core/benches/ir_gate.rs` and `crates/pyrs-yaml/benches/ir_gate.rs` declare
`required-features = ["ir-gate"]` - and for the extension crate that gate is mandatory, rule R5 of
`docs/dev/boundaries.md`, because an ungated bench there would make every workspace bench command
build an interpreter probe. cargo drops a target whose required features are off instead of checking
it, so the wider scope skips exactly the files the instruction floors are made of.

Measured 2026-10-29 by planting a type error in each harness in turn. `cargo clippy --all
--all-targets -- -D warnings` reported a clean tree both times; the narrow feature-on command exited
101 naming `crates/pyrs-yaml-core/benches/ir_gate.rs:325` and then
`crates/pyrs-yaml/benches/ir_gate.rs:147`. The `ir` job already *builds* the harness, so a compile
error never walked past - what nothing read was a lint. `QUALITY_MATRIX.md` said so in the present
tense ("checked by building it, not by the lint job") until this file's rule made it a gate.

Nothing here names a crate. The gated set is read out of the manifests and the hook set out of
`prek.toml`, so a third gated harness cannot join the tree unlinted and the CI skip list cannot fall
behind a clippy hook whose job installs no `clippy` component and could only fail on the tool.
"""

from __future__ import annotations

import re
from pathlib import Path

import yaml

REPO_ROOT = Path(__file__).resolve().parent.parent
CI = REPO_ROOT / ".github" / "workflows" / "ci.yml"
PREK = REPO_ROOT / "prek.toml"
CRATES = REPO_ROOT / "crates"

# The bench target both instruction-count harnesses declare themselves as.
HARNESS_TARGET = "ir_gate"


def crate_dir(crate: str) -> Path:
    return CRATES / crate


def _required_features(manifest: Path, target: str) -> tuple[str, ...]:
    """The `required-features` of the `[[bench]]` blocks naming `target`, as declared.

    Split on block headers rather than matching up to the next `[`: `required-features =
    ["ir-gate"]` carries a `[` itself, so a `[^[]*` span stops inside the value and reports an
    ungated target - which would make this file's own rule read green on a gated harness.
    """
    found: list[str] = []
    for block in re.split(r"(?m)^\[\[", manifest.read_text(encoding="utf-8"))[1:]:
        kind, _, body = block.partition("]]")
        if kind.strip() != "bench":
            continue
        name = re.search(r'name\s*=\s*"([^"]+)"', body)
        if not name or name.group(1) != target:
            continue
        gate = re.search(r"required-features\s*=\s*\[([^\]]*)\]", body)
        if gate:
            found.extend(re.findall(r'"([^"]+)"', gate.group(1)))
    return tuple(found)


def gated_harness_crates() -> dict[str, tuple[str, ...]]:
    """crate -> the features its `ir_gate` bench requires, for crates carrying that bench."""
    out: dict[str, tuple[str, ...]] = {}
    for bench in sorted(CRATES.glob(f"*/benches/{HARNESS_TARGET}.rs")):
        crate = bench.parent.parent.name
        features = _required_features(crate_dir(crate) / "Cargo.toml", HARNESS_TARGET)
        if features:
            out[crate] = features
    return out


def clippy_commands(workflow_text: str) -> list[str]:
    """Every `run:` command in the workflow that invokes clippy, whitespace-normalised."""
    commands = []
    for job in yaml.safe_load(workflow_text)["jobs"].values():
        for step in job.get("steps") or []:
            run = step.get("run") or ""
            if re.search(r"\bcargo\s+clippy\b", run):
                commands.append(" ".join(run.split()))
    return commands


def clippy_hook_ids(config_text: str) -> list[str]:
    """The ids of every `prek.toml` hook whose entry runs clippy."""
    ids = []
    for block in re.split(r"(?m)^\[\[repos\.hooks\]\]", config_text)[1:]:
        entry = re.search(r'entry\s*=\s*"([^"]*)"', block)
        ident = re.search(r'id\s*=\s*"([^"]*)"', block)
        if entry and ident and "cargo clippy" in entry.group(1):
            ids.append(ident.group(1))
    return ids


def _flags(command: str) -> tuple[set[str], set[str], set[str]]:
    """The packages, bench targets and features a cargo command names, as tokens.

    Not substring tests: `-p pyrs-yaml` is a substring of `-p pyrs-yaml-core`, so a match by text
    would credit a command that never selects the binding crate - the same trap the exclusion rule
    in `tests/test_ci_rust_leg_exclusion.py` records for `--exclude`.
    """
    tokens = command.split()
    packages: set[str] = set()
    benches: set[str] = set()
    features: set[str] = set()
    for index, token in enumerate(tokens):
        name, _, inline = token.partition("=")
        following = tokens[index + 1 : index + 2]
        values = [inline] if inline else [value for value in following if not value.startswith("-")]
        if name in ("-p", "--package"):
            packages.update(values)
        elif name == "--bench":
            benches.update(values)
        elif name == "--features":
            features.update(part for value in values for part in re.split(r"[,\s]+", value) if part)
    return packages, benches, features


def _lint_covers(gated: dict[str, tuple[str, ...]], commands: list[str]) -> dict[str, str]:
    """crate -> the clippy command that lints its gated harness, or a missing key.

    A command counts only if it selects the crate, selects the harness target, and turns on every
    feature that target requires: `--features` with some other name is the wide scope wearing the
    narrow one's clothes.
    """
    covering: dict[str, str] = {}
    for crate, features in gated.items():
        for command in commands:
            packages, benches, enabled = _flags(command)
            if crate in packages and HARNESS_TARGET in benches and set(features) <= enabled:
                covering[crate] = command
                break
    return covering


def test_the_instruction_harnesses_are_feature_gated():
    """The premise, read off the tree: two gated harnesses, each needing a named feature.

    An empty answer would make every rule below vacuous, so the count is asserted before any
    command is compared - the reason the matrix probes refuse to pass on a glob that found nothing.
    """
    gated = gated_harness_crates()
    assert len(gated) >= 2, gated
    for crate, features in gated.items():
        assert features, crate
        assert (crate_dir(crate) / "benches" / f"{HARNESS_TARGET}.rs").exists(), crate


def test_ci_lints_every_gated_harness_with_its_feature_on():
    gated = gated_harness_crates()
    commands = clippy_commands(CI.read_text(encoding="utf-8"))
    assert commands, "no clippy step anywhere in ci.yml"
    covering = _lint_covers(gated, commands)
    assert sorted(covering) == sorted(gated), (sorted(gated), sorted(covering), commands)


def test_the_wide_scope_is_still_there_beside_the_narrow_one():
    """The narrow step is an addition, not a swap.

    `--all-targets` is what lints every ungated `tests/` and `benches/` target - the blind spot the
    repository closed once already - so a fix that traded it for the feature-gated command would
    leave the tree less covered than it found it.
    """
    commands = clippy_commands(CI.read_text(encoding="utf-8"))
    assert any("--all" in command and "--all-targets" in command for command in commands), commands


def test_every_clippy_hook_is_skipped_by_the_job_that_installs_no_clippy():
    workflow = yaml.safe_load(CI.read_text(encoding="utf-8"))["jobs"]["hooks"]
    skip = (workflow.get("env") or {}).get("SKIP") or ""
    skipped = {part.strip() for part in skip.split(",") if part.strip()}
    hooks = set(clippy_hook_ids(PREK.read_text(encoding="utf-8")))
    assert hooks, "no clippy hook in prek.toml - the hook tier stopped linting Rust entirely"
    steps = yaml.safe_dump(workflow.get("steps") or "")
    assert "components: clippy" not in steps.replace(" ", "").replace('"', "").replace("'", ""), (
        "the hooks job now installs clippy, so this rule has to be re-read rather than kept"
    )
    assert hooks <= skipped, (sorted(hooks - skipped), sorted(skipped))


def test_injection_each_way_the_coverage_rule_can_be_misled():
    """The positive shape, then each of the three ways of faking it.

    A rule that only ever saw the whole command would also pass on `--all-targets` alone, which is
    the reading that let the gap live: the payloads drop the feature, drop the target, and drop the
    crate from the `-p` list, one at a time.
    """
    gated = {"pyrs-yaml-core": ("ir-gate",), "pyrs-yaml": ("ir-gate",)}
    whole = """
jobs:
  rust-lint:
    steps:
      - run: cargo clippy --all --all-targets -- -D warnings
      - run: |
          cargo clippy -p pyrs-yaml-core -p pyrs-yaml --bench ir_gate --features ir-gate -- -D warnings
"""
    assert sorted(_lint_covers(gated, clippy_commands(whole))) == sorted(gated)
    # The wide scope alone is what the gap wore: a clippy step exists and lints neither harness.
    assert _lint_covers(gated, ["cargo clippy --all --all-targets -- -D warnings"]) == {}
    assert _lint_covers(gated, clippy_commands(whole.replace(" --features ir-gate", ""))) == {}
    assert _lint_covers(gated, clippy_commands(whole.replace(" --bench ir_gate", ""))) == {}
    dropped = whole.replace("-p pyrs-yaml-core -p pyrs-yaml", "-p pyrs-yaml-core")
    assert sorted(_lint_covers(gated, clippy_commands(dropped))) == ["pyrs-yaml-core"]
