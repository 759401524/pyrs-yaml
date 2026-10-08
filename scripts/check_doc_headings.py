"""Refuse a Markdown heading that is really a sentence the formatter split in two.

The prose here is hard-wrapped, and a paragraph that continues on the next line sometimes
begins with an issue reference: `#292 (resolve mapping keys) is closed, and ...`. To a
Markdown formatter that is not a continuation, it is an ATX heading, so `rumdl fmt` promoted
it, blanked around it, and left a heading out of the middle of a sentence. Three of those
were in the tree at once — two in `ROADMAP.md`, one in `QUALITY_MATRIX.md` — and every gate
stayed green, because a document with a bogus heading is still valid Markdown: nothing asks
whether a heading was *meant*.

The signature is tight enough to assert on: a heading whose text begins with two or more
digits, where the digits are not followed by a dot. Measured over every tracked `.md`, that
pattern matches exactly the three damaged lines and none of the legitimate digit headings in
the repository — `### 1-D array` (one digit), `#### 10. メタデータの操作` (digits then a dot),
`## 1. Test matrix coverage`. So the rule is stated, the three lines are repaired, and a
future line-initial `#1234 …` fails the hook instead of quietly restructuring a page.

Headings inside fenced code blocks are not headings, so the scan tracks fences: the
repository documents shell sessions whose comments (`# 2025 timings`) match the pattern.

A second signature was added after the first was already in the tree: a run of dashes or equals
sitting directly under a prose line, which is a setext heading to CommonMark. `rumdl fmt` produced
exactly that while answering a transient `MD003`, promoting a whole paragraph into a heading and
leaving every gate green. The first attempt at a rule here measured the width of heading text
against the 100-column prose convention - falsified immediately, because twelve legitimate
`### (xx)` lead-ins on these pages are wider than that. So the rule names the shape instead: no
paragraph may end on the line above a dash run. A `---` separator is fine; it needs its blank line.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

# The trees that are not this repository's prose: an upstream checkout, generated site
# output, build and virtualenv directories.
EXCLUDED_PREFIXES = ("Reference/", "site/", "target/", "node_modules/", ".venv", ".cache/", ".git/")

HEADING = re.compile(r"^(#{1,6}) (\d{2,})([^\d.])")
FENCE = re.compile(r"^\s{0,3}(?:`{3,}|~{3,})")
SETEXT = re.compile(r"^\s{0,3}(?:-{2,}|={2,})\s*$")
TABLE = re.compile(r"^\s{0,3}\|")


def front_matter_end(lines) -> int:
    """Index just past a document's YAML front matter, or 0 when it has none.

    The closing `---` of front matter sits under a content line by definition, so a rule about
    dash runs has to know it is not prose. Only a first line of exactly `---` opens it.
    """
    if not lines or lines[0].strip() != "---":
        return 0
    for index in range(1, len(lines)):
        if lines[index].strip() in ("---", "..."):
            return index + 1
    return len(lines)


def heading_damage(lines, name):
    """Findings for one document, as one-line messages.

    `lines` is the document split into lines (no trailing newline), so the check is testable
    without a repository on disk.
    """
    findings = []
    in_fence = False
    for number, line in enumerate(lines, 1):
        if FENCE.match(line):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        match = HEADING.match(line)
        if match:
            text = line.strip()
            findings.append(
                f"{name}:{number}: {text[:72]}\n"
                "    a heading whose text starts with digits is almost always a wrapped sentence that\n"
                "    began with an issue reference and got promoted by the formatter. Join it back to\n"
                "    the paragraph, or start the line with a word (`PR #292 …`) so the reference cannot\n"
                "    lead the line."
            )
    return findings


def setext_damage(lines, name) -> list:
    """Findings for a paragraph that ends on the line above a dash or equals run.

    CommonMark reads that pair as a setext heading, so the paragraph *becomes* a title - and a
    formatter asked to reconcile the two will rewrite the sentence as an ATX heading, which is what
    happened to a `ROADMAP.md` paragraph here. A thematic break is legal, it needs its blank line.
    """
    findings = []
    in_fence = False
    start = front_matter_end(lines)
    for index in range(start, len(lines)):
        line = lines[index]
        if FENCE.match(line):
            in_fence = not in_fence
            continue
        if in_fence or index < 1:
            continue
        above = lines[index - 1]
        if not SETEXT.match(line) or not above.strip():
            continue
        if HEADING.match(above) or TABLE.match(above) or above.strip() in ("---", "..."):
            continue
        findings.append(
            f"{name}:{index + 1}: {line.strip()[:40]!r} sits under prose\n"
            f"    {above.strip()[:72]}\n"
            "    a dash run directly under a paragraph is a setext heading, not a separator: the\n"
            "    sentence above it turns into a title, and the formatter will rewrite it that way.\n"
            "    Put a blank line above the run."
        )
    return findings


def is_scoped(rel: str) -> bool:
    """Whether a repository-relative path is a page this rule speaks for.

    Split out of the file listing so the exclusions are testable without a repository, the
    same way `check_line_endings.py` exposes its scope: an upstream checkout and generated
    output are not this repository's prose, and rewriting them is not our call to make.
    """
    return rel.endswith(".md") and not rel.startswith(EXCLUDED_PREFIXES)


def tracked_markdown():
    """Every tracked `.md` outside the excluded trees."""
    out = subprocess.run(
        ["git", "ls-files", "-z", "--", "*.md"], capture_output=True, check=True, cwd=REPO
    ).stdout.decode("utf-8")
    for rel in out.split("\0"):
        rel = rel.strip().replace("\\", "/")
        if rel and is_scoped(rel):
            yield rel


def main(argv=()) -> int:
    paths = list(argv) or sorted(set(tracked_markdown()))
    findings: list[str] = []
    for rel in paths:
        path = REPO / rel
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        findings.extend(heading_damage(text.splitlines(), rel))
        findings.extend(setext_damage(text.splitlines(), rel))
    if findings:
        print(f"markdown headings that are really split sentences ({len(findings)}):")
        for finding in findings:
            print(f"  {finding}")
        return 1
    print(f"OK: no split-sentence heading among {len(paths)} Markdown files")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
