"""The no_std gate has to ask the same question in both directions.

`ci.yml`'s `no-std-check` job builds the foundation crates for `thumbv7em-none-eabi`, runs their unit
tests on the host with `--no-default-features`, and then re-checks the default (std) configuration to
prove that the no_std path was not "achieved" by deleting the std one. That last step is the one a
file edit can quiet: it names a crate list of its own, and a list narrower than the two above it
leaves a crate free to stop compiling with std while the job stays green - the asymmetry was real
here for a while, with three of the five foundation crates missing from the std re-check.

So the lists are compared, not trusted: each step's `-p <crate>` set is read out of the workflow text,
and the gate fails if the std re-check stops covering what the no_std steps cover. The foundation set
itself is read from the crate roots that declare `#![no_std]`, because a hand-copied list can only ever
check the workflow against itself - it cannot notice a crate that joined no_std and was never built for
a bare-metal target, or one that quietly lost the attribute. The negative cases are run on synthetic
bodies, because a guard that has never been seen to bite is a comment.
"""

from __future__ import annotations

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "ci.yml"

BUILD_STEP = "Build no_std foundation crates"
TEST_STEP = "Run no_std unit tests on the host"
STD_STEP = "Build with the default std feature"

# The foundation is what the sources claim, not what this file remembers. A crate joins by writing the
# attribute itself, so adding a no_std crate puts the burden on ci.yml to follow; and dropping the
# attribute takes the crate out of the gate's scope without an edit here. The hand-written list this
# replaced could only ever check the workflow against itself.
_ATTR_RE = re.compile(r"^#!\[no_std\][ \t]*$", re.MULTILINE)


def _no_std_foundation(crates_dir: Path) -> set[str]:
    """The crates whose own root declares `#![no_std]`.

    Anchored to the start of a line because the crate roots also *talk about* the attribute in doc and
    plain comments, and an unanchored search would count a sentence as a declaration.
    """
    found = set()
    for source in crates_dir.glob("*/src/lib.rs"):
        if _ATTR_RE.search(source.read_text(encoding="utf-8")):
            found.add(source.parent.parent.name)
    return found


FOUNDATION = _no_std_foundation(REPO_ROOT / "crates")

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
    assert built == FOUNDATION, f"the sources declare {sorted(FOUNDATION)}, the bare-metal build covers {sorted(built)}"
    assert tested == FOUNDATION, (
        f"the sources declare {sorted(FOUNDATION)}, the host no_std tests cover {sorted(tested)}"
    )


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


def test_the_foundation_is_read_from_the_attribute(tmp_path: Path) -> None:
    """The reader, on crate roots that separate the three readings it can be given."""
    crates = tmp_path / "crates"
    bodies = {
        "declares": "#![no_std]\n\nextern crate alloc;\n",
        "talks-about-it": "// `#![no_std]` drops std from the extern prelude\nfn f() {}\n",
        "doc-says-it": "//! The crate is `#![no_std]` by default.\nfn f() {}\n",
        "indented": "    #![no_std]\n",
        "silent": "fn f() {}\n",
    }
    for name, body in bodies.items():
        root = crates / name / "src"
        root.mkdir(parents=True)
        (root / "lib.rs").write_text(body, encoding="utf-8", newline="\n")
    assert _no_std_foundation(crates) == {"declares"}, "a comment is not a declaration"


def test_the_foundation_the_gate_asks_for_is_not_empty() -> None:
    # Without this, deleting the attribute from every crate would make both file assertions compare
    # an empty set against an empty set and report a passing, meaningless gate.
    assert FOUNDATION, "no crate root declares #![no_std], so the gate has nothing to cover"
    for crate in sorted(FOUNDATION):
        assert (REPO_ROOT / "crates" / crate / "Cargo.toml").is_file(), crate
