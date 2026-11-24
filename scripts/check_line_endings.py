"""List every tracked text file whose bytes are not LF-only, and fix that.

Skips the trees where bytes are the payload (fuzz inputs) or where the content is
generated and ignored (target/, site/, node_modules/, .venv*, .cache/). `Reference/` is
an upstream checkout, so it is left exactly as fetched.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
# The three fuzz *data* trees, not "fuzz/": `fuzz/fuzz_targets/*.rs` is Rust source and
# belongs to the policy. Listing the parent directory here excluded the targets as well,
# which is the same mistake the checker exists to catch — a wide exclusion written for a
# narrow reason. Kept in sync with the `prek.toml` globs by
# `tests/test_line_endings_gate.py`.
SKIP_PREFIXES = (
    "fuzz/seeds/",
    "fuzz/corpus/",
    "fuzz/artifacts/",
    "Reference/",
    "target/",
    "site/",
    "node_modules/",
    ".venv",
    ".cache/",
    ".git/",
)
CHECKED_IN = (".md", ".rs", ".py", ".toml", ".yml", ".yaml", ".json", ".ini", ".txt", ".pyi", ".cfg", ".lock")

# Files whose line endings are inside raw strings, i.e. they are benchmark input data and
# not formatting. `.ci/ir-baseline.json` was measured against these bytes on the Linux
# runner, so rewriting them changes the parsed fixture (CRLF is two bytes for the scanner,
# LF is one) and belongs in a baseline regeneration - the queued pillar-four item - rather
# than in a hygiene commit that would otherwise move no bytes the gate measures. Kept in
# sync with the `prek.toml` exclusion for the same files.
DATA_FIXTURES = (
    "crates/pyrs-yaml-core/src/bench_inputs.rs",
    "crates/pyrs-yaml-core/benches/ir_gate.rs",
)

# The CSV corpus holds one fixture whose record separator is CRLF, and those CR bytes are
# the input rather than the formatting: the reader is measured on a file whose rows end in
# CRLF, so normalising it here would replace the case under test with the LF case. The
# reason is the same as the fuzz seeds' - a byte stream that means something - and it is
# declared in `.gitattributes` as `-text` so a checkout does not convert it either.
#
# What this list carries today is stated plainly, because a defence that is registered
# rather than measured is the shape of hole this repository keeps refusing to build:
# `.csv` is not in `CHECKED_IN`, so this fixture was never in the scope of the absolute
# rule to begin with. The CRLF bytes are caught by the prek `mixed-line-ending` hook and by
# the `.gitattributes` declaration, not by the file listing below. The entry is here so
# that the two declarations cannot drift apart when the extension list changes, and so
# that the reason it would matter is written down rather than rediscovered.
#
# The XML fixture is the same kind of measured input: its CRLF separators are one of
# the six lexical normalisations the XML reader is tested against. It adds one honest
# caveat: `.xml` is not in `CHECKED_IN` either, so this listing carries no weight for
# it today. The load-bearing declarations for both fixtures are `.gitattributes`
# (`-text`, so a checkout cannot rewrite the bytes) and the two prek hooks.
CRLF_FIXTURES = (
    "crates/pyrs-csv/tests/fixtures/crlf.csv",
    "crates/pyrs-xml/tests/fixtures/crlf.xml",
)


def is_excluded(rel: str) -> bool:
    """The paths this policy does not own, whatever they are written in.

    One predicate for both callers, so the file listing and the scope test cannot
    drift apart: a path excluded from the listing but not from the scope test
    would be checked by one and skipped by the other, which is the shape of
    hole this gate exists to close.
    """
    return rel.startswith(SKIP_PREFIXES) or rel in DATA_FIXTURES or rel in CRLF_FIXTURES


def is_checked_path(rel: str) -> bool:
    """Whether a repository-relative path is in scope for the policy.

    Split out of the file listing so the exclusions are testable without a repository:
    the data fixtures are the part a future reader is most likely to "clean up" by
    mistake, and their CRLFs are benchmark input and corpus input, not formatting.
    """
    if not rel.endswith(CHECKED_IN):
        return False
    return not is_excluded(rel)


def line_ending_counts(data: bytes) -> tuple[int, int]:
    """(CRLF count, lone-CR count) for a file's bytes."""
    crlf = data.count(b"\r\n")
    return crlf, data.count(b"\r") - crlf


def offenders(rel_and_bytes: list[tuple[str, bytes]]) -> list[tuple[str, int, int]]:
    """Pure classification, so a test can feed it the shapes this gate exists for."""
    found = []
    for rel, data in rel_and_bytes:
        crlf, lone_cr = line_ending_counts(data)
        if crlf or lone_cr:
            found.append((rel, crlf, lone_cr))
    return found


def tracked_text_files() -> list[Path]:
    # `--others --exclude-standard` matters: the point of this gate is the moment a file
    # is created, and a fresh file is exactly when a Windows-side writer gets CRLF into
    # the tree. Listing only `git ls-files` would check the damage after it is committed.
    proc = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        cwd=REPO,
    )
    if proc.returncode != 0:
        raise RuntimeError(f"git ls-files exited {proc.returncode}: {proc.stderr.strip()[-200:]}")
    files: list[Path] = []
    for rel in proc.stdout.split("\n"):
        rel = rel.strip().replace("\\", "/")
        if not rel or not rel.endswith(CHECKED_IN):
            continue
        if is_excluded(rel):
            continue
        path = REPO / rel
        if path.is_file():
            files.append(path)
    return files


def main() -> int:
    mode = sys.argv[1] if len(sys.argv) > 1 else "list"
    try:
        scanned = [(str(path.relative_to(REPO)), path.read_bytes()) for path in tracked_text_files()]
    except RuntimeError as exc:
        print(f"cannot measure: {exc} (exit 2 is not a pass)", file=sys.stderr)
        return 2
    if not scanned:
        # Zero bytes read is not the same verdict as zero offenders: a broken index, an empty
        # checkout or a git that cannot run all answer with the empty list this line refuses.
        print("cannot measure: git listed no text file to read (exit 2 is not a pass)", file=sys.stderr)
        return 2
    found = offenders(scanned)
    for rel, crlf, lone in found:
        print(f"{rel:56s} CRLF={crlf:5d} loneCR={lone}")
    print(f"{len(found)} of {len(scanned)} tracked text file(s) have non-LF line endings")
    if mode != "fix":
        # Non-zero on findings: this is the CI step that reports the audit, and a checker
        # that only prints is a report, not a gate. `fix` mode is the repair path a human
        # or the `mixed-line-ending` hook takes.
        return 1 if found else 0
    for rel, _, _ in found:
        path = REPO / rel
        data = path.read_bytes()
        # Lone CRs are only rewritten where they sit next to an LF; a file that
        # legitimately contains a bare CR elsewhere is not a line-ending question.
        path.write_bytes(data.replace(b"\r\n", b"\n"))
    print("normalized to LF")
    return 0


if __name__ == "__main__":
    sys.exit(main())
