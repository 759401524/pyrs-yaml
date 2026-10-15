"""The no_std gate has to ask the same question in both directions.

`ci.yml`'s `no-std-check` job builds the foundation crates for `thumbv7em-none-eabi`, runs their unit
tests on the host with `--no-default-features`, and then re-checks the default (std) configuration to
prove that the no_std path was not "achieved" by deleting the std one. That last step is the one a
file edit can quiet: it names a crate list of its own, and a list narrower than the two above it
leaves a crate free to stop compiling with std while the job stays green - the asymmetry was real
here for a while, with three of the five foundation crates missing from the std re-check.

So the lists are compared, not trusted: each step's `-p <crate>` set is read out of the workflow text,
and the gate fails if the std re-check stops covering what the no_std steps cover. The negative case is
run on a synthetic workflow body, because a guard that has never been seen to bite is a comment.
"""

from __future__ import annotations

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci.yml"

BUILD_STEP = "Build no_std foundation crates"
TEST_STEP = "Run no_std unit tests on the host"
STD_STEP = "Build with the default std feature"

# The crates the matrix treats as the no_std foundation: everything below the engine that a
# bare-metal target could carry. A crate joins this list only with its own no_std build in place.
FOUNDATION = {"pyrs-ast", "pyrs-event", "pyrs-schema", "pyrs-json", "pyrs-toml"}

_STEP_RE = re.compile(r"^      - name: (.+?)$", re.MULTILINE)
_PKG_RE = re.compile(r"-p\s+([A-Za-z0-9_.-]+)")


def _crates_in_step(text: str, step_name: str) -> set[str]:
    """The `-p <crate>` set of one step, read from the raw workflow text.

    The file is parsed rather than loaded as YAML because the steps this asserts on are written as
    folded scalars (`run: >`), and the assertion is about the text a maintainer edits - a YAML round
    trip would normalise exactly the layout differences worth catching.
    """
    marks = [(m.group(1), m.start()) for m in _STEP_RE.finditer(text)]
    for idx, (name, start) in enumerate(marks):
        if name != step_name:
            continue
        end = marks[idx + 1][1] if idx + 1 < len(marks) else len(text)
        return set(_PKG_RE.findall(text[start:end]))
    raise AssertionError(f"step {step_name!r} is not in {WORKFLOW.name}")


def _std_narrower_than_no_std(no_std: set[str], std: set[str]) -> set[str]:
    return no_std - std


def test_the_no_std_steps_cover_the_foundation_crates() -> None:
    text = WORKFLOW.read_text(encoding="utf-8")
    built = _crates_in_step(text, BUILD_STEP)
    tested = _crates_in_step(text, TEST_STEP)
    assert built == FOUNDATION, f"bare-metal build covers {sorted(built)}"
    assert tested == FOUNDATION, f"host no_std tests cover {sorted(tested)}"


def test_the_std_recheck_names_every_no_std_crate() -> None:
    text = WORKFLOW.read_text(encoding="utf-8")
    no_std = _crates_in_step(text, BUILD_STEP) | _crates_in_step(text, TEST_STEP)
    std = _crates_in_step(text, STD_STEP)
    missing = _std_narrower_than_no_std(no_std, std)
    assert not missing, f"{sorted(missing)} compile without std but are not re-checked with it"


def test_the_gate_reports_a_narrowed_recheck() -> None:
    # The negative control, and it is the shape the file had before this change: three crates dropped
    # from the std step while both no_std steps still named five.
    narrowed = {
        "pyrs-ast",
        "pyrs-schema",
    }
    no_std = FOUNDATION
    missing = _std_narrower_than_no_std(no_std, narrowed)
    assert missing == {"pyrs-event", "pyrs-json", "pyrs-toml"}, sorted(missing)


def test_the_gate_reads_a_synthetic_step_list() -> None:
    # Same reader the file assertions use, on a body that keeps the step names and changes one list -
    # so the guard is a function, not a hand-read of this repository's own YAML.
    text = (
        "      - name: Build no_std foundation crates\n"
        "        run: >\n          cargo build\n          -p pyrs-ast\n\n"
        "      - name: Run no_std unit tests on the host\n"
        "        run: >\n          cargo test\n          -p pyrs-ast\n\n"
        "      - name: Build with the default std feature\n"
        "        run: cargo check --locked -p pyrs-ast -p pyrs-event\n"
    )
    assert _crates_in_step(text, BUILD_STEP) == {"pyrs-ast"}
    assert _crates_in_step(text, STD_STEP) == {"pyrs-ast", "pyrs-event"}
    assert not _std_narrower_than_no_std(_crates_in_step(text, BUILD_STEP), _crates_in_step(text, STD_STEP))
