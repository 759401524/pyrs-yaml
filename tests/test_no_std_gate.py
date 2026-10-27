"""The no_std gate has to ask the same question in both directions.

`ci.yml`'s `no-std-check` job builds the foundation crates for `thumbv7em-none-eabi`, runs their unit
tests on the host with `--no-default-features`, and then re-checks the default (std) configuration to
prove that the no_std path was not "achieved" by deleting the std one. That last step is the one a
file edit can quiet: it names a crate list of its own, and a list narrower than the two above it
leaves a crate free to stop compiling with std while the job stays green - the asymmetry was real
here for a while, with three of the five foundation crates missing from the std re-check. The
foundation was five crates then; the YAML engine joined it later, and this file's history keeps the
number it had rather than the number it is.

So the lists are compared, not trusted: each step's `-p <crate>` set is read out of the workflow text,
and the gate fails if the std re-check stops covering what the no_std steps cover. The foundation set
itself is read from the crate roots that declare `#![no_std]`, because a hand-copied list can only ever
check the workflow against itself - it cannot notice a crate that joined no_std and was never built for
a bare-metal target, or one that quietly lost the attribute. The negative cases are run on synthetic
bodies, because a guard that has never been seen to bite is a comment.

The third question is the same one asked of the prose, and it is the one the gate was missing: the job
can be green and the documents can still be wrong. `docs/dev/boundaries.md` pointed at "AGENTS.md's
no_std story" - a story that did not exist - and named five foundation crates in a paragraph that the
YAML engine had just overtaken. A reader of the documents is not served by a green gate, so the crates
the live documents name are read out of them and compared with the derived foundation as well. The
dated records - `CHANGELOG.md`, its four mirrors, `docs/dev/quality-ledger.md` - are history and are
exempt: they may name a five-crate foundation, because that is what was true when they were written.

The fourth question is about the command rather than the list. A step named "Run no_std unit tests on
the host" only earns that name if the units it compiles are std-free, and feature unification across a
multi-`-p` invocation can take `std` back on through a sibling's manifest. That is measured here from
the manifests, with the proof and the numbers in the comment above the reader.

The fifth question is which host the answer came from. While the job was `runs-on: ubuntu-latest`, its
unit tests, its std re-check and the two opt-in `mmap` and `parallel` legs certified one operating
system - and no other job in this workflow runs a Rust test off Linux, while wheels ship for three. The
split is kept asymmetric on purpose: the cross-target build is gated to one host because its answer
cannot move, and a job that collapses the host steps back to a single OS is reported as "runs on exactly
one OS" rather than read as a pass.
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


# The live surface for the no_std story: the agent instruction file and the developer document that
# points at it. `CLAUDE.md` is a Comet-managed block whose interior the tool may replace, so it is not
# part of this surface; the dated records are history, and history may name a five-crate foundation.
LIVE_DOCS = ("AGENTS.md", "docs/dev/boundaries.md")

_NO_STD_RE = re.compile(r"no[_ -]std", re.IGNORECASE)
_CRATE_RE = re.compile(r"\bpyrs-[a-z0-9]+(?:-[a-z0-9]+)*\b")
_POINTER_RE = re.compile(r"`([A-Za-z0-9_./-]+\.md)`'s `no_std` story")


def _blocks(text: str) -> list[str]:
    """The document's blocks: a blank line ends one, and so does a markdown list item.

    `AGENTS.md` writes its facts as a bullet list with no blank line between the items, so a
    paragraph-only split would fuse the whole list into one block and the no_std paragraph would
    carry every crate the file happens to name.
    """
    blocks: list[str] = []
    current: list[str] = []
    for line in text.splitlines():
        blank = not line.strip()
        if current and (blank or line.startswith("- ")):
            blocks.append("\n".join(current))
            current = []
        if not blank:
            current.append(line)
    if current:
        blocks.append("\n".join(current))
    return blocks


def _no_std_blocks(text: str) -> list[str]:
    """The blocks that talk about `no_std`, which is where the story lives."""
    return [block for block in _blocks(text) if _NO_STD_RE.search(block)]


def _named_crates(blocks: list[str]) -> set[str]:
    """Every crate the blocks name."""
    named: set[str] = set()
    for block in blocks:
        named.update(_CRATE_RE.findall(block))
    return named


def _foundation_gap(named: set[str], foundation: set[str]) -> tuple[set[str], set[str]]:
    """The crates the prose leaves out, and the ones it names that are not in the foundation."""
    return foundation - named, named - foundation


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
    # The negative control, and it is the shape the file had before this change: two crates kept in
    # the std step while both no_std steps name the whole foundation.
    narrowed = {
        "pyrs-ast",
        "pyrs-schema",
    }
    no_std = FOUNDATION
    missing = _std_narrower_than_no_std(no_std, narrowed)
    assert missing == {"pyrs-event", "pyrs-json", "pyrs-toml", "pyrs-yaml-core"}, sorted(missing)


_JOB_RE = re.compile(r"^  no-std-check:\n((?:.*\n)*?)(?=^  [a-z][a-z0-9-]*:\n)", re.MULTILINE)
_OS_MATRIX_RE = re.compile(r"os:\s*\[([^\]]*)\]")

# The runners this project publishes wheels for, so a host outside them certifies nothing users run.
SHIPPED_RUNNERS = ("ubuntu-latest", "windows-latest")


def _no_std_job(text: str) -> str:
    body = _JOB_RE.search(text)
    assert body, "the `no-std-check` job is not in ci.yml"
    return body.group(1)


def _step_body(text: str, step_name: str) -> str:
    """One step's own lines, from its `- name:` to the next step or the end of its job."""
    block = _no_std_job(text)
    marks = [(m.group(1), m.start()) for m in _STEP_RE.finditer(block)]
    for idx, (name, start) in enumerate(marks):
        if name != step_name:
            continue
        end = marks[idx + 1][1] if idx + 1 < len(marks) else len(block)
        return block[start:end]
    raise AssertionError(f"step {step_name!r} is not in the no_std job")


def _host_oses(text: str) -> set[str]:
    """Every OS the no_std job's host steps run on."""
    block = _no_std_job(text)
    matrix = _OS_MATRIX_RE.search(block)
    assert matrix, "no_std has no `os:` matrix, so its host steps run on exactly one OS"
    assert "runs-on: ${{ matrix.os }}" in block, "the job declares an `os:` matrix it does not run on"
    return {entry.strip().strip("\"'") for entry in matrix.group(1).split(",") if entry.strip()}


def test_the_no_std_host_steps_run_on_the_hosts_the_project_ships_for() -> None:
    """The unit tests are the OS-dependent half, so one host cannot certify them.

    The job used to be `runs-on: ubuntu-latest` alone: the only evidence that a `#![no_std]` crate's
    logic survives without std came from one OS, while wheels are published for three. The bare-metal
    *build* genuinely is host-independent - it links for `thumbv7em-none-eabi` identically wherever it
    runs - so the asymmetry this test protects is deliberate in one direction only: the tests spread,
    the compile does not have to.
    """
    hosts = _host_oses(WORKFLOW.read_text(encoding="utf-8"))
    missing = sorted(name for name in SHIPPED_RUNNERS if name not in hosts)
    assert not missing, f"the no_std host steps never run on {missing}, which ship wheels: {sorted(hosts)}"


def test_the_bare_metal_build_is_gated_to_one_host_and_the_tests_are_not() -> None:
    """The one step whose answer cannot move between hosts is the one pinned to a single host."""
    text = WORKFLOW.read_text(encoding="utf-8")
    build = _step_body(text, BUILD_STEP)
    assert re.search(r"^\s+if: matrix\.os == ", build, re.MULTILINE), (
        "`Build no_std foundation crates` runs on every matrix leg, so the job pays a dependency "
        "build per host for a link check that cannot differ between them"
    )
    assert "if: matrix.os" not in _step_body(text, TEST_STEP), "the host unit tests are gated back to one OS"


def test_the_gate_reports_a_job_whose_host_steps_run_on_one_os() -> None:
    """The control: collapsing the matrix has to be named, not read as a pass."""
    collapsed = "  no-std-check:\n    runs-on: ubuntu-latest\n    steps:\n      - name: x\n        run: y\n\n  other:\n    runs-on: ubuntu-latest\n"
    try:
        _host_oses(collapsed)
    except AssertionError as exc:
        assert "exactly one OS" in str(exc), exc
    else:
        raise AssertionError("a single-host no_std job passed the gate that exists to catch it")


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
        with (root / "lib.rs").open("w", encoding="utf-8", newline="\n") as handle:
            handle.write(body)
    assert _no_std_foundation(crates) == {"declares"}, "a comment is not a declaration"


def test_the_foundation_the_gate_asks_for_is_not_empty() -> None:
    # Without this, deleting the attribute from every crate would make both file assertions compare
    # an empty set against an empty set and report a passing, meaningless gate.
    assert FOUNDATION, "no crate root declares #![no_std], so the gate has nothing to cover"
    for crate in sorted(FOUNDATION):
        assert (REPO_ROOT / "crates" / crate / "Cargo.toml").is_file(), crate


def test_the_live_documents_name_the_whole_foundation() -> None:
    """The prose and the sources must name the same foundation.

    A crate that joins the group without a line in the documents leaves the next reader with a
    foundation that is one crate short of the real one - which is exactly what happened to
    `docs/dev/boundaries.md` when the YAML engine joined.
    """
    for relative in LIVE_DOCS:
        text = (REPO_ROOT / relative).read_text(encoding="utf-8")
        blocks = _no_std_blocks(text)
        assert blocks, f"{relative} has no no_std story, so the foundation is documented nowhere"
        missing, extra = _foundation_gap(_named_crates(blocks), FOUNDATION)
        assert not missing, f"{relative} does not name {sorted(missing)} in its no_std story"
        assert not extra, f"{relative} names {sorted(extra)}, which do not build without std"


def test_the_no_std_pointer_resolves() -> None:
    """A document that points at another document's no_std story must point at a real one.

    `docs/dev/boundaries.md` pointed at "AGENTS.md's no_std story" for as long as there was no such
    story, and the pointer read as a cross-reference the whole time.
    """
    pointers = []
    for relative in LIVE_DOCS:
        text = (REPO_ROOT / relative).read_text(encoding="utf-8")
        for target in _POINTER_RE.findall(text):
            path = REPO_ROOT / target
            assert path.is_file(), f"{relative} points at {target}, which does not exist"
            assert _no_std_blocks(path.read_text(encoding="utf-8")), (
                f"{relative} points at {target}'s no_std story, and {target} has none"
            )
            pointers.append((relative, target))
    assert pointers, "no live document points at another document's no_std story, so the rule is empty"


def test_the_gate_reports_a_stale_foundation_list() -> None:
    # The negative control, and it is the shape boundaries.md had: a no_std paragraph that names the
    # foundation minus the crate that had just joined it.
    body = (
        "the `no_std` job builds `pyrs-ast`, `pyrs-event`, `pyrs-schema`, `pyrs-json` and\n"
        "`pyrs-toml` for `thumbv7em-none-eabi`.\n"
    )
    named = _named_crates(_no_std_blocks(body))
    missing, extra = _foundation_gap(named, FOUNDATION)
    assert missing == FOUNDATION - {"pyrs-ast", "pyrs-event", "pyrs-schema", "pyrs-json", "pyrs-toml"}
    assert not extra, sorted(extra)


def test_the_gate_reports_a_crate_that_does_not_build_without_std() -> None:
    # The other direction: a no_std paragraph that names a crate the sources do not put in the
    # foundation is a claim nobody can check, so it is a finding too.
    body = "the `no_std` job builds `pyrs-ast` and `pyrs-yaml`.\n"
    named = _named_crates(_no_std_blocks(body))
    missing, extra = _foundation_gap(named, FOUNDATION)
    assert extra == {"pyrs-yaml"}, sorted(extra)
    assert missing == FOUNDATION - {"pyrs-ast"}


def test_the_reader_only_counts_the_no_std_paragraph() -> None:
    # The scoping: a paragraph that does not talk about no_std names crates for other reasons, and
    # counting those would make every file in the repository a no_std claim.
    body = (
        "`pyrs-yaml-core` is the YAML engine and `pyrs-json` the JSON one.\n"
        "\n"
        "The `no_std` job builds `pyrs-ast` and `pyrs-toml` for `thumbv7em-none-eabi`.\n"
    )
    assert _named_crates(_no_std_blocks(body)) == {"pyrs-ast", "pyrs-toml"}


def test_the_reader_separates_adjacent_bullets() -> None:
    # `AGENTS.md` writes its facts as a bullet list with no blank line between the items, so a
    # paragraph-only split would read the whole list as one block.
    body = "- `pyrs-json` is the JSON engine.\n- The `no_std` job builds `pyrs-toml`.\n"
    assert _named_crates(_no_std_blocks(body)) == {"pyrs-toml"}


# The fourth question: does the command the step runs actually mean what its name says?
#
# Cargo unifies features across the packages selected in ONE invocation, so a foundation crate whose
# manifest names a sibling without `default-features = false` re-enables that sibling's `std` even
# when the same command also selects the sibling as a root and asks for no defaults. Measured on this
# tree with `cargo test -v --no-run` and the `--cfg feature="..."` set read out of each rustc line:
# `cargo test --no-default-features -p pyrs-toml` alone compiles `pyrs_toml`'s test unit with no
# feature cfg at all, while the six-crate command the `no_std` job runs compiled it with
# `--cfg feature="default" --cfg feature="std"` - and `pyrs_json` and `pyrs_ast` came out the same
# way. The edge that did it was `pyrs-yaml-core`'s three dev-dependency lines, so the step named
# "Run no_std unit tests on the host" was running 58 pyrs-json, 54 pyrs-toml and the whole pyrs-ast
# suite in the std configuration, with every one of them passing there.
#
# The rule is therefore read from the manifests and asks about the *target* of each edge: an edge
# into a sibling that declares `default = ["std"]` must close it, and an edge into a sibling with no
# features at all (`pyrs-event`, `pyrs-schema`) is harmless and is not demanded. Requiring the clause
# everywhere would be noise; missing it where it matters is the silent case.

_SECTION_RE = re.compile(r"^\[\s*([a-zA-Z0-9_.\"'-]+dependencies)\s*\]")
_PATH_RE = re.compile(r'path = "\.\./([^"]+)"')


def _declares_std_default(manifest: Path) -> bool:
    """Whether a crate's own defaults carry `std`, which is what makes an edge load-bearing."""
    default = re.search(r"^default = \[([^\]]*)\]", manifest.read_text(encoding="utf-8"), re.M)
    return bool(default) and '"std"' in default.group(1)


def _std_reopening_edges(manifest_text: str, std_defaulting: dict[str, bool]) -> list[str]:
    """Every path edge in one manifest that leaves a std-defaulting sibling's defaults on."""
    edges: list[str] = []
    section = "(head)"
    for line in manifest_text.splitlines():
        head = _SECTION_RE.match(line)
        if head:
            section = head.group(1)
            continue
        target = _PATH_RE.search(line)
        if target is None:
            continue
        name = target.group(1)
        if not std_defaulting.get(name):
            continue
        if "default-features = false" not in line:
            edges.append(f"[{section}] {name}")
    return edges


def test_no_foundation_manifest_reopens_a_siblings_std() -> None:
    crates_dir = REPO_ROOT / "crates"
    std_defaulting = {name: _declares_std_default(crates_dir / name / "Cargo.toml") for name in sorted(FOUNDATION)}
    offenders = {
        name: _std_reopening_edges((crates_dir / name / "Cargo.toml").read_text(encoding="utf-8"), std_defaulting)
        for name in sorted(FOUNDATION)
    }
    found = {name: edges for name, edges in offenders.items() if edges}
    assert not found, (
        f"the `no_std` job selects {sorted(FOUNDATION)} in one `cargo test --no-default-features` "
        f"invocation, and these manifests re-enable a sibling's std inside it: {found}"
    )


def test_the_gate_reports_an_edge_that_leaves_std_on() -> None:
    # The falsification control, on the exact edge shape the fix removed: without the clause the
    # reader must name the section and the sibling, and the same line with the clause must be clean.
    std_defaulting = {"pyrs-json": True, "pyrs-toml": True, "pyrs-schema": False}
    broken = '[dev-dependencies]\npyrs-json = { path = "../pyrs-json", version = "0.18.3" }\n'
    fixed = '[dev-dependencies]\npyrs-json = { path = "../pyrs-json", version = "0.18.3", default-features = false }\n'
    sibling_without_std = '[dependencies]\npyrs-schema = { path = "../pyrs-schema", version = "0.18.3" }\n'
    assert _std_reopening_edges(broken, std_defaulting) == ["[dev-dependencies] pyrs-json"]
    assert _std_reopening_edges(fixed, std_defaulting) == []
    assert _std_reopening_edges(sibling_without_std, std_defaulting) == []


def test_the_gate_reads_a_synthetic_manifest_section_list() -> None:
    # A manifest whose edges are spread over three sections has to be read section by section: a
    # reader that stopped at the first one would pass the widest class of reopening edge.
    body = (
        "[dependencies]\n"
        'pyrs-ast = { path = "../pyrs-ast", default-features = false }\n'
        "[dev-dependencies]\n"
        'pyrs-toml = { path = "../pyrs-toml" }\n'
        "[build-dependencies]\n"
        'pyrs-json = { path = "../pyrs-json" }\n'
    )
    std_defaulting = {"pyrs-ast": True, "pyrs-toml": True, "pyrs-json": True}
    assert _std_reopening_edges(body, std_defaulting) == [
        "[dev-dependencies] pyrs-toml",
        "[build-dependencies] pyrs-json",
    ]
