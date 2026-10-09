#!/usr/bin/env python3
"""Assert that the instruction files describe the defence the matrix actually measures.

Why this exists, named: `AGENTS.md` said for a whole milestone that `mkdocstrings` "is configured, installed
and invoked by the build - **and renders nothing**", with the gap registered as `docs-generation:plugin-unused`.
#321 closed it - the API pages in four locales now carry `:::` directives and the registry is empty - and the
sentence survived, still pointing the next agent the wrong way, because nothing in the tree reads that file.
`tests/test_quality_matrix.py` already guards the registry and the derived document against the measurement in
both directions; the prose that tells an agent how the defence works was the artefact left outside the gate.

The rule is narrow on purpose. An id of the shape `<kind>:<name>` is a hole id only when `kind` is one the
probe can emit, which is read off `scripts/quality_matrix.py` rather than listed here: that keeps `line:column`
(a pair of field names in a changelog entry) out of the vocabulary, and keeps the gate from silently widening to
every colon in prose. An id that is named must then be one the matrix measures *now*.

Scope is the two agent instruction files. The dated records - `CHANGELOG.md`, its four mirrors,
`docs/dev/quality-ledger.md`, and `QUALITY_MATRIX.md`'s own narrative - are history, and history is allowed to
name a hole that has since closed. An instruction file is present tense, and a present-tense sentence about a
closed hole is the defect this measures.

Usage:
    python scripts/check_hole_claims.py                 # the instruction files
    python scripts/check_hole_claims.py path [...]      # explicit, for tests

Exit code 0 = every named id is a measured hole; 1 = a stale id is named; 2 = the checker could not measure
anything, which is never allowed to read as a pass.
"""

from __future__ import annotations

import importlib.util
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
PROBE = REPO / "scripts" / "quality_matrix.py"
INSTRUCTION_FILES = ("AGENTS.md", "CLAUDE.md")

APPEND_SITE = re.compile(r"holes\.append\(")
KIND_AT_SITE = re.compile(r'holes\.append\(\s*\[\s*"([a-z][a-z0-9-]*)"')
NAMED_ID = re.compile(r"\b([a-z][a-z0-9-]*):([a-z][a-z0-9-]*)\b")


def emittable_kinds() -> set[str]:
    """Every hole kind the probe can report, read off its own source.

    The count guard is what stops this decaying into a no-op. A new reporting site whose first element is not a
    string literal would leave the vocabulary shorter than the probe, and a short vocabulary waves every
    sentence through - so a mismatch is an error rather than a smaller set of things to check.
    """
    source = PROBE.read_text(encoding="utf-8")
    found = KIND_AT_SITE.findall(source)
    sites = len(APPEND_SITE.findall(source))
    if len(found) != sites:
        raise RuntimeError(
            f"only {len(found)} of {sites} hole-reporting sites in {PROBE.name} begin with a kind "
            "literal, so the vocabulary cannot be trusted"
        )
    kinds = set(found)
    if not kinds:
        raise RuntimeError(f"no hole kinds found in {PROBE.name}")
    return kinds


def measured_ids() -> set[str]:
    """The ids the matrix measures right now, from the same probe the CI job runs."""
    spec = importlib.util.spec_from_file_location("quality_matrix", PROBE)
    assert spec is not None and spec.loader is not None, f"cannot load {PROBE}"
    probe = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(probe)
    return {f"{kind}:{name}" for kind, name, _why in probe.measure()["holes"]}


def stale_claims(text: str, kinds: set[str], measured: set[str]) -> list[tuple[int, str, str]]:
    """Every `(line, id, context)` naming a hole kind the measurement does not reproduce."""
    findings = []
    for number, line in enumerate(text.splitlines(), start=1):
        for match in NAMED_ID.finditer(line):
            token = match.group(0)
            if match.group(1) in kinds and token not in measured:
                findings.append((number, token, line.strip()))
    return findings


def main(argv: list[str]) -> int:
    paths = [pathlib.Path(arg) for arg in argv] or [REPO / name for name in INSTRUCTION_FILES]
    try:
        kinds = emittable_kinds()
        measured = measured_ids()
    except (OSError, RuntimeError) as exc:
        print(f"cannot measure: {exc}", file=sys.stderr)
        return 2
    findings = []
    for path in paths:
        try:
            text = path.read_text(encoding="utf-8")
        except OSError as exc:
            # An instruction file the checker cannot read is not a clean tree.
            print(f"unreadable: {path} ({exc})", file=sys.stderr)
            return 2
        findings.extend((path, *finding) for finding in stale_claims(text, kinds, measured))
    if not findings:
        print(
            f"OK: {len(paths)} instruction file(s) name only holes the matrix measures "
            f"({len(measured)} measured, {len(kinds)} kinds known)"
        )
        return 0
    print(f"{len(findings)} stale hole claim(s):", file=sys.stderr)
    for path, number, token, line in findings:
        print(f"  {path}:{number} names {token!r}, which the matrix does not measure", file=sys.stderr)
        print(f"      {line[:100]}", file=sys.stderr)
    print(
        "Remedy: the dated records (CHANGELOG.md, docs/dev/quality-ledger.md) keep the history - an "
        "instruction file states the defence as it is now, so name the pull request that closed the gap "
        "instead of the hole id, or register the id in .ci/quality-holes.json if the measurement really "
        "does still reproduce it.",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
