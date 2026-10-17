"""The release build compiles with the compiler the instruction-count floors were measured on.

`publish.yml` builds every wheel through `PyO3/maturin-action`, whose `rust-toolchain` input
defaults to current `stable`. The Ir gate does not use `stable`: it installs the compiler recorded
in `.ci/ir-baseline.json`, because a codegen move is not a code change — measured 2026-10-16 on one
tree and one host with only `RUSTUP_TOOLCHAIN` differing, `rustc 1.99.0` costs +6.4% counted
instructions across the `serialize_*` family (worst `serialize_medium` +9.08%) while the floors stay
green on `rustc 1.97.1`. Left as it was, that gap meant the committed floors described a build
nobody installs.

So both workflows pin the recorded version and this file fails the drift. Both directions matter: a
runner image that moves `stable` silently changes the shipped artifact, and a baseline refresh that
moves the pin silently changes what the workflow is supposed to use. The exemption the file encodes
is the `sdist` step, which packages source and compiles nothing.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
BASELINE = REPO_ROOT / ".ci" / "ir-baseline.json"
PUBLISH = REPO_ROOT / ".github" / "workflows" / "publish.yml"
CI = REPO_ROOT / ".github" / "workflows" / "ci.yml"

MATURIN = "uses: PyO3/maturin-action@v1"


def maturin_steps(text: str) -> list[dict[str, str]]:
    """Every `maturin-action` step's `with:` map, in file order.

    Read line by line rather than through a YAML parser because the answer needs the *steps*, and a
    parse would fold the matrix definitions and the anchors into one document where a step's keys are
    no longer addressable. Indentation is the workflow's own structure: `with:` sits at 8 spaces and
    its keys at 10, which is how the block ends.
    """
    lines = text.splitlines()
    steps: list[dict[str, str]] = []
    for index, line in enumerate(lines):
        if line.strip() != MATURIN:
            continue
        keys: dict[str, str] = {}
        for follow in lines[index + 1 :]:
            if follow.strip() == "with:":
                continue
            match = re.match(r"^ {10}([a-z-]+):\s*(.*)$", follow)
            if match is None:
                break
            keys[match.group(1)] = match.group(2)
        steps.append(keys)
    return steps


def pin() -> str:
    return json.loads(BASELINE.read_text(encoding="utf-8"))["toolchain"].split()[1]


def test_the_baseline_records_a_compiler_version():
    # Everything else here compares against this, so a malformed field must fail loudly rather than
    # quietly pinning the empty string everywhere.
    assert re.fullmatch(r"\d+\.\d+(\.\d+)?", pin()), pin()


def test_every_release_wheel_build_uses_the_recorded_compiler():
    wheel_steps = [s for s in maturin_steps(PUBLISH.read_text(encoding="utf-8")) if s.get("command") != "sdist"]
    assert len(wheel_steps) == 7, f"publish.yml's wheel builds changed shape: {len(wheel_steps)} steps"
    for step in wheel_steps:
        assert step.get("rust-toolchain") == pin(), step


def test_the_sdist_step_is_deliberately_not_pinned():
    sdist = [s for s in maturin_steps(PUBLISH.read_text(encoding="utf-8")) if s.get("command") == "sdist"]
    assert len(sdist) == 1, sdist
    assert "rust-toolchain" not in sdist[0], "an sdist compiles nothing; a pin here would be noise"


def test_the_cli_asset_build_pins_its_compiler_and_matrix_target():
    text = PUBLISH.read_text(encoding="utf-8")
    assert "dtolnay/rust-toolchain@stable" not in text, (
        "`@stable` resolves to whatever the image ships: the release binary would be built by a "
        "compiler no measurement describes"
    )
    assert re.search(rf"dtolnay/rust-toolchain@master\s*\n\s*with:\s*\n\s*toolchain: {re.escape(pin())}\b", text)
    # A single brace is not an expression: GitHub passes `${ matrix.platform.std }` through
    # literally and rustup receives the three words `${`, `matrix.platform.std`, `}` as target
    # names. PR CI never runs `publish.yml`, so this is the only place that can fail before a tag
    # push does.
    assert "targets: ${{ matrix.platform.std }}" in text


def test_ci_builds_the_same_recipe_with_the_same_compiler():
    # The canary: `publish.yml` runs only on a tag push, so this leg is where the input name and the
    # version are proven on three operating systems before a release depends on them.
    steps = [s for s in maturin_steps(CI.read_text(encoding="utf-8")) if "--find-interpreter" in s.get("args", "")]
    assert len(steps) == 1, steps
    assert steps[0].get("rust-toolchain") == pin(), steps[0]
