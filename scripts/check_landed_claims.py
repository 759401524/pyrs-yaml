#!/usr/bin/env python3
"""Assert that a live design document's "this landed" heading names code the tree actually contains.

Why this exists, named: `docs/dev/event-stream.md` carried the heading "What the note flag split
answered (2026-10-21, decision 6 landed)" against a tree that did not have the split. The documentation
had reached trunk through a docs-only commit while the implementation sat in an unmerged local change, so
the sentence described a defence that was not there. `scripts/check_hole_claims.py` guards the instruction
files against the measured matrix and `tests/test_quality_matrix.py` guards the registry against the
measurement, but nothing compared a design doc's present tense with the code, and that unchecked class is
exactly where this slipped through.

The rule is narrow, because a gate over prose only earns trust by being precise about what it refuses. A
heading whose text contains `landed` or `shipped` claims something about the tree, and the next non-blank
line must be a lock:

    <!-- landed: crates/pyrs-event/src/lib.rs::NoteOwner -->
    <!-- landed: crates/pyrs-yaml-core/tests/event_notes.rs -->

Several references are several consecutive lock lines. A comma-separated list is legal to this checker but
the `doc-line-width` hook owns the same pages at 100 display columns, and the three-ref line it replaced
here measured 141 - the fold shape is the one that survives both gates.

A lock reference is a repository path, optionally `::` plus one identifier that must appear as a whole word
in that file. Locks are accepted under headings that do not need one, and a lock that does not resolve is an
error wherever it sits - so adding evidence is always allowed, and stale evidence is always caught.

Scope is the live design documents under `docs/dev/`, derived by glob so a new one is covered the day it
lands. `docs/dev/quality-ledger.md` is exempt by name: it is a dated record, and a dated record describes
what was true on its date. The locale changelogs are exempt for the same reason.

Usage:
    python scripts/check_landed_claims.py                # every live design document
    python scripts/check_landed_claims.py path [...]     # explicit, for tests

Exit code 0 = every claim carries a lock and every lock resolves; 1 = a claim is unlocked or a lock is
stale; 2 = the checker could not measure anything, which is never allowed to read as a pass.
"""

from __future__ import annotations

import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
LIVE_DOCS_DIR = REPO / "docs" / "dev"
# Dated records state what was true on their date; a present-tense claim is the thing under gate.
EXEMPT = {"quality-ledger.md"}

HEADING = re.compile(r"^(#{1,6})\s+(.*)$")
CLAIM_WORD = re.compile(r"\b(?:landed|shipped)\b")
LOCK = re.compile(r"^<!--\s*landed:\s*(?P<refs>[^>]*?)\s*-->$")
REF = re.compile(r"^(?P<path>[^\s:]+)(?:::(?P<symbol>[A-Za-z0-9_]+))?$")


def live_docs() -> list[pathlib.Path]:
    """Every live design document, derived from the directory rather than a list beside it."""
    if not LIVE_DOCS_DIR.is_dir():
        raise RuntimeError(f"{LIVE_DOCS_DIR} is not a directory, so there is nothing to measure")
    found = sorted(path for path in LIVE_DOCS_DIR.glob("*.md") if path.name not in EXEMPT and path.is_file())
    if not found:
        raise RuntimeError(f"no live design documents under {LIVE_DOCS_DIR}")
    return found


def lock_refs(line: str) -> list[str] | None:
    """The references of a lock comment line, or None when the line is not a lock."""
    match = LOCK.match(line.strip())
    if match is None:
        return None
    return [ref.strip() for ref in match.group("refs").split(",") if ref.strip()]


def unresolved(ref: str, root: pathlib.Path) -> str | None:
    """Why one lock reference does not resolve, or None when the tree carries it."""
    match = REF.match(ref)
    if match is None:
        return f"malformed reference {ref!r}, expected `path` or `path::symbol`"
    path = root / match.group("path")
    if not path.is_file():
        return f"{match.group('path')} is not a file in this tree"
    symbol = match.group("symbol")
    if symbol is None:
        return None
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        return f"{path.name} cannot be read ({exc})"
    if not re.search(rf"\b{re.escape(symbol)}\b", text):
        return f"{symbol} does not appear in {match.group('path')}"
    return None


def scan(text: str, root: pathlib.Path) -> tuple[list[tuple[int, str]], int, int]:
    """`(findings, claims, locks)` for one document: what must be answered, and how much was looked at."""
    findings: list[tuple[int, str]] = []
    lines = text.splitlines()
    claims = 0
    locks = 0
    for index, line in enumerate(lines):
        heading = HEADING.match(line)
        if heading is not None and CLAIM_WORD.search(heading.group(2)):
            claims += 1
        refs = lock_refs(line)
        if refs is not None:
            locks += 1
            for ref in refs:
                reason = unresolved(ref, root)
                if reason is not None:
                    findings.append((index + 1, reason))
            continue
        if heading is None or not CLAIM_WORD.search(heading.group(2)):
            continue
        following = next((n for n in lines[index + 1 :] if n.strip()), "")
        if lock_refs(following) is None:
            findings.append(
                (index + 1, f"claims something landed or shipped with no lock under it: {line.strip()[:80]}")
            )
    return findings, claims, locks


def main(argv: list[str]) -> int:
    if argv:
        scope = [pathlib.Path(arg) for arg in argv]
    else:
        try:
            scope = live_docs()
        except (OSError, RuntimeError) as exc:
            print(f"cannot measure: {exc}", file=sys.stderr)
            return 2
    findings: list[tuple[pathlib.Path, int, str]] = []
    total_claims = 0
    total_locks = 0
    for path in scope:
        try:
            text = path.read_text(encoding="utf-8")
        except OSError as exc:
            # A document the checker cannot read is not a document that passed.
            print(f"unreadable: {path} ({exc})", file=sys.stderr)
            return 2
        found, claims, locks = scan(text, REPO)
        total_claims += claims
        total_locks += locks
        findings.extend((path, line, reason) for line, reason in found)
    if not findings:
        print(
            f"OK: {len(scope)} document(s) read, {total_claims} landed/shipped heading(s), "
            f"{total_locks} lock(s), every lock resolves"
        )
        return 0
    print(f"{len(findings)} landed-claim problem(s):", file=sys.stderr)
    for path, line, reason in findings:
        print(f"  {path}:{line}: {reason}", file=sys.stderr)
    print(
        "Remedy: name the code that makes the sentence true, as `<!-- landed: <path>[::<symbol>] -->` on the "
        "line under the heading - or move the claim into a dated record (docs/dev/quality-ledger.md, "
        "CHANGELOG.md), where a past tense is the point.",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
