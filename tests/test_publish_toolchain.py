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

import yaml

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
    # Three free-threaded builds now sit beside the GIL ones: a cp314t wheel and a cp315-abi3t
    # wheel on each of Linux x86_64, Windows (x64/x86/arm64) and macOS, plus the two on the
    # native Linux arm64 job. The exact number is the tripwire: a new build that forgets the
    # pin, or a deletion that silently drops a platform, both change it.
    assert len(wheel_steps) == 12, f"publish.yml's wheel builds changed shape: {len(wheel_steps)} steps"
    for step in wheel_steps:
        assert step.get("rust-toolchain") == pin(), step


def test_both_free_threaded_wheels_are_built_for_every_platform_that_ships_one():
    """The docs promise free-threaded wheels for 3.14t and 3.15t; the matrix must produce both.

    `abi3t-py315` starts at 3.15, so the two artifacts come from different interpreters and
    neither substitutes for the other: a 3.14t interpreter cannot load the `cp315-abi3t` wheel,
    and the abi3t artifact only exists when a 3.15 interpreter runs the build. Linux arm64 must
    be a native runner because maturin executes the target interpreter for both builds, and
    under qemu that exec fails with `Exec format error (os error 8)` - the reason the platform
    had no free-threaded wheel. Reading the interpreter counts keeps the two host-interpreter
    builds (windows, macos) distinct from the two image-interpreter builds (linux, arm64).
    """
    text = PUBLISH.read_text(encoding="utf-8")
    args = [step.get("args", "") for step in maturin_steps(text)]
    assert sum("/opt/python/cp314-cp314t/bin/python" in a for a in args) == 2, args
    assert sum("/opt/python/cp315-cp315t/bin/python" in a for a in args) == 2, args
    assert sum("-i python3.14t" in a for a in args) == 2, args
    assert sum("-i python3.15t" in a for a in args) == 2, args
    document = yaml.safe_load(text)
    assert document["jobs"]["linux-arm64-free-threaded"]["runs-on"] == "ubuntu-24.04-arm", (
        "the arm64 free-threaded legs must run natively; an x86_64 runner under qemu cannot "
        "execute the target interpreter the build needs"
    )
    assert "linux-arm64-free-threaded" in document["jobs"]["release"]["needs"], (
        "Release must wait for the arm64 free-threaded wheels; otherwise PyPI is published "
        "without them while every job that ran is green"
    )


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


def test_every_containerised_suite_step_installs_the_tools_the_suite_calls():
    """The wheel tests run the whole suite, which shells out to git and imports a docs dependency.

    Both gaps were latent for the same reason: host runners already carry git, and `ci.yml`'s
    `uv run` installs the dev group (tomli-w included), while the release steps install only
    `.ci/requirements-test.txt` inside bare Alpine/Ubuntu containers. The first tag pushed after
    the repository gates and the script-import test landed failed every wheel job with
    `No module named 'tomli_w'` and nine `FileNotFoundError: git`. `publish.yml` cannot report
    this before a tag exists, so the container steps' inputs are checked here instead.

    The suite body is checked in as `.ci/pytest-wheel.sh` rather than embedded in the workflow:
    the first repair kept it inside `docker run ... sh -c '...'`, and the apostrophe in the comment
    `container's` closed the host shell's quote - Alpine ran `apk add` and nothing else, while the
    runner's toolcache python3 ran `-m virtualenv` and failed. A body that cannot hold a comment is
    not a body to keep, so each container leg is required to call the script.
    """
    document = yaml.safe_load(PUBLISH.read_text(encoding="utf-8"))
    steps = [step for job in document["jobs"].values() for step in job.get("steps", [])]
    # `run-on-arch` keeps its commands under `with.run`, so a step-level read would see no suite.
    bodies = [(step, str(step.get("run", "")) + "\n" + str((step.get("with") or {}).get("run", ""))) for step in steps]
    suite_steps = [(step, body) for step, body in bodies if ".ci/pytest-wheel.sh" in body or "pytest tests/" in body]
    container_steps = [
        (step, body)
        for step, body in suite_steps
        if str(step.get("uses", "")).startswith("uraimo/run-on-arch-action") or "docker run" in body
    ]
    assert len(container_steps) == 3, [step.get("name") for step, _ in container_steps]

    for step, body in container_steps:
        action_inputs = step.get("with") or {}
        install = str(action_inputs.get("install", ""))
        # `run-on-arch` declares its packages under `install:`; the plain `docker run` step's apk
        # line is inside `run`. Only the install command counts - a comment that names git is not
        # an installation.
        install_lines = install or "\n".join(
            line for line in body.splitlines() if re.match(r"\s*(apk|apt-get)\b", line)
        )
        assert re.search(r"\bgit\b", install_lines), f"{step.get('name')}: install lacks git: {install_lines!r}"
        assert "virtualenv" in install_lines, (
            f"{step.get('name')}: the script creates its venv with `python3 -m virtualenv`: {install_lines!r}"
        )
        assert ".ci/pytest-wheel.sh" in body, (
            f"{step.get('name')}: the suite body must be the checked-in script; an embedded "
            "`sh -c '...'` body ends at the first apostrophe in a comment"
        )

    script = (REPO_ROOT / ".ci" / "pytest-wheel.sh").read_text(encoding="utf-8")
    assert re.search(r"^git config --global --add safe\.directory", script, re.M), (
        "the mounted checkout is not marked safe for the container's user"
    )
    assert re.search(r"^\.venv/bin/pytest tests/ -v$", script, re.M), (
        "the script must run the whole suite against the installed wheel"
    )
    assert "source .venv" not in script, "Alpine's /bin/sh has no `source`; call the venv's binaries by path"

    requirements = (REPO_ROOT / ".ci" / "requirements-test.txt").read_text(encoding="utf-8")
    assert re.search(r"^tomli-w\b", requirements, re.M), (
        "build-docs.py imports tomli-w and the suite imports every script; the release test "
        "environment installs this file, not the docs group"
    )
