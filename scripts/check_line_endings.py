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


def is_checked_path(rel: str) -> bool:
    """Whether a repository-relative path is in scope for the policy.

    Split out of the file listing so the exclusions are testable without a repository:
    the two data fixtures are the part a future reader is most likely to "clean up" by
    mistake, and their CRLFs are benchmark input, not formatting.
    """
    if not rel.endswith(CHECKED_IN):
        return False
    return not (rel.startswith(SKIP_PREFIXES) or rel in DATA_FIXTURES)


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
    out = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        cwd=REPO,
    ).stdout.split("\n")
    files: list[Path] = []
    for rel in out:
        rel = rel.strip().replace("\\", "/")
        if not rel or not rel.endswith(CHECKED_IN):
            continue
        if rel.startswith(SKIP_PREFIXES) or rel in DATA_FIXTURES:
            continue
        path = REPO / rel
        if path.is_file():
            files.append(path)
    return files


def main() -> int:
    mode = sys.argv[1] if len(sys.argv) > 1 else "list"
    scanned = [(str(path.relative_to(REPO)), path.read_bytes()) for path in tracked_text_files()]
    found = offenders(scanned)
    for rel, crlf, lone in found:
        print(f"{rel:56s} CRLF={crlf:5d} loneCR={lone}")
    print(f"{len(found)} tracked text file(s) with non-LF line endings")
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
