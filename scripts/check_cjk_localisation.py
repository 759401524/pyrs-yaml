#!/usr/bin/env python3
"""Guard the localized docs against mixed-script intrusions.

Each locale page must stay in its own writing system:

* `zh` — no Japanese kana, no Hangul.
* `ja` — no Hangul, and no *simplified-Chinese-only* Han (the forms that Japanese
  spells with a different codepoint, e.g. 積 vs 积, 連 vs 链, 視 vs 视).
* `ko` — no Japanese kana, and no Han at all. Modern Korean prose is written in
  Hangul; a Han glyph in a Korean sentence is never a legitimate variant, it is
  either a Japanese kanji or a Chinese character that leaked in with the draft.

The `ko` rule used to be the narrower "simplified-only Han" list, and that list is
a set of thirteen hand-picked codepoints. Measured consequence: the Korean
changelog shipped fifteen lines of mixed Chinese prose (whole clauses such as
`热点 样本로 指定。以前 inline 만 计量`) while every checker in the tree reported OK,
because 热, 点, 样, 指 and 定 are simply not on that list. Keying on "any Han"
removes the curation burden entirely: the rule has no list to fall behind.

Technical text is exempt, per codepoint rather than per line: fenced blocks,
inline code spans and link targets. A Korean page documenting i18n legitimately
shows `title: 文档标题` inside a ```yaml block, and a Korean JSON5 entry lists
`{ é: 1, 名: 2, हिन्दी: 3 }` as parser input. A gate that reddens those teaches
people to bypass it, so exemption is decided by position, not by whether the
neighbouring line happens to contain prose as well.

Scope is the whole page tree under `docs/<locale>/`, not just the changelog's
`[Unreleased]` block. Measured cost of widening it: `zh` and `ja` produce zero
findings across their 41 pages each, `ko` produces findings only in
`changelog.md`. Older entries used to be skipped because "historical entries stay
English" (`AGENTS.md`), and English cannot violate a script-purity rule, so the
narrow scope was protecting nothing.

The scan refuses to call an empty result a pass. The first version of this gate
pinned its heading pattern to one nesting depth, matched nothing on the localized
pages, and printed a green OK over zero scanned lines.

Usage:
    uv run python scripts/check_cjk_localisation.py [paths...]

With no paths it scans every locale page. Paths are accepted because prek hands
changed filenames to `language: system` hooks; a path outside a policed locale is
simply not this hook's business.
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Sequence

# Han codepoints that only exist in Simplified Chinese. The Japanese/Korean
# (and Traditional) forms are separate codepoints, so these never appear in
# correctly localized text. Only unambiguous pairs belong here: a gate that
# reddes legitimate text is worse than no gate, so glyphs that are real Japanese
# kanji (柱, 梅, 陀, ...) are deliberately absent even when one of them once
# carried the wrong meaning in a sentence — that is a review question, not a
# script violation.
SIMPLIFIED_ONLY = {
    "\u79ef": "积 (Japanese: 積)",
    "\u94fe": "链 (Japanese: 鎖)",
    "\u8d39": "费 (Japanese: 費)",
    "\u5047": "假 (Japanese: 仮)",
    "\u5f52": "归 (Japanese: 帰)",
    "\u89c6": "视 (Japanese: 視)",
    "\u7ea6": "约 (Japanese: 約)",
    "\u7ee9": "绩 (Japanese: 績)",
    "\u8ba1": "计 (Japanese: 計)",
    "\u5c42": "层 (Japanese: 層)",
    "\u89c1": "见 (Japanese: 見)",
    "\u53e0": "叠 (Japanese: 畳)",
    "\u8fde": "连 (Japanese: 連)",
}

# U+3400-4DBF CJK extension A, U+4E00-9FFF main block, U+F900-FAFF compatibility
# ideographs, plus the two iteration marks that only Japanese writes (U+3005 the
# repeats-previous-kanji mark, U+3007 the zero ideograph).
HAN = re.compile(r"[\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff\u3005\u3007]")
HIRAGANA_KATAKANA = re.compile(r"[\u3041-\u309f\u30a1-\u30ff]")
HANGUL = re.compile(r"[\uac00-\ud7a3]")

INLINE_CODE = re.compile(r"`[^`\n]*`")
LINK_TARGET = re.compile(r"\]\([^)]*\)")
FENCE_LINE = re.compile(r"^\s*(?P<fence>`{3,}|~{3,})\s*(?P<info>\S[^`]*)?\s*$")


@dataclass(frozen=True)
class Rules:
    """Which scripts a locale is not allowed to contain."""

    kana: bool
    hangul: bool
    simplified: bool
    han: bool


# Only the `han` column differs from the historical table, and only for `ko`,
# where it subsumes `simplified`: forbidding every Han forbids the thirteen
# listed ones too, so the curated list is not consulted for Korean at all.
LOCALES: dict[str, Rules] = {
    "zh": Rules(kana=True, hangul=True, simplified=False, han=False),
    "ja": Rules(kana=False, hangul=True, simplified=True, han=False),
    "ko": Rules(kana=True, hangul=False, simplified=False, han=True),
}


def technical_regions(line: str) -> list[tuple[int, int]]:
    """Offset ranges of `line` that are code or a URL rather than prose."""
    return [(m.start(), m.end()) for pattern in (INLINE_CODE, LINK_TARGET) for m in pattern.finditer(line)]


def prose_lines(text: str) -> Iterable[tuple[int, str]]:
    """Yield (1-based line number, prose-only content) for scannable lines.

    Fenced blocks are tracked with a real state machine: a closing fence must use
    the same character and be at least as long as the opening one, and an opening
    fence carries the language info string. A naive toggle would go out of phase on
    the first nested or longer fence and then either skip prose or report code.
    """
    open_fence: str | None = None
    for number, line in enumerate(text.splitlines(), start=1):
        match = FENCE_LINE.match(line)
        if open_fence is None:
            if match:
                open_fence = match.group("fence")
                continue
            yield number, line
            continue
        closes = (
            match is not None
            and match.group("fence")[0] == open_fence[0]
            and len(match.group("fence")) >= len(open_fence)
            and not match.group("info")
        )
        if closes:
            open_fence = None
        # Inside a fenced block: technical by construction, never prose.
    # An unterminated fence is the markdown's problem, not this gate's; the
    # lines after it stay suppressed, which is the conservative direction.


def describe(glyphs: str) -> str:
    """Render offending glyphs with their codepoints, so a reader can confirm."""
    seen: list[str] = []
    for c in glyphs:
        if c not in seen:
            seen.append(c)
    return " ".join(f"{c}(U+{ord(c):04X})" for c in seen[:8])


def check_line(line: str, rules: Rules) -> list[tuple[str, str]]:
    """Return (kind, detail) for every script violation in one prose line."""
    findings: list[tuple[str, str]] = []
    if rules.kana and HIRAGANA_KATAKANA.search(line):
        findings.append(("kana in a non-Japanese page", "kana"))
    if rules.hangul and HANGUL.search(line):
        findings.append(("Hangul in a non-Korean page", "Hangul"))
    if rules.simplified:
        hits = sorted({SIMPLIFIED_ONLY[c] for c in line if c in SIMPLIFIED_ONLY})
        if hits:
            findings.append((f"simplified-only Han {'/'.join(hits)}", "Han"))
    if rules.han:
        glyphs = "".join(m.group() for m in HAN.finditer(line))
        if glyphs:
            findings.append((f"Han in a Hangul-only page: {describe(glyphs)}", "Han"))
    return findings


def check_page(relative: str, text: str, rules: Rules) -> list[str]:
    findings: list[str] = []
    for number, raw in prose_lines(text):
        line = LINK_TARGET.sub(lambda m: " " * len(m.group()), raw)
        regions = technical_regions(line)
        prose = "".join(c if not any(start <= i < end for start, end in regions) else " " for i, c in enumerate(line))
        for detail, _ in check_line(prose, rules):
            findings.append(f"{relative}:{number}: {detail} | {prose.strip()[:70]}")
    return findings


def collect_pages(root: Path, paths: Sequence[str] | None) -> list[tuple[str, Path]]:
    """Resolve what to police: every locale page, or just the changed ones."""
    wanted = None
    if paths:
        wanted = {Path(p).as_posix() for p in paths}
    pages: list[tuple[str, Path]] = []
    for locale in sorted(LOCALES):
        base = root / "docs" / locale
        for page in sorted(base.rglob("*.md")) if base.is_dir() else []:
            relative = page.relative_to(root).as_posix()
            if wanted is None or relative in wanted:
                pages.append((relative, page))
    return pages


def scan(root: Path, paths: Sequence[str] | None = None) -> tuple[list[str], int]:
    """Return (findings, number of prose lines scanned)."""
    findings: list[str] = []
    scanned = 0
    for relative, page in collect_pages(root, paths):
        text = page.read_text(encoding="utf-8")
        lines = list(prose_lines(text))
        scanned += len(lines)
        findings.extend(check_page(relative, text, LOCALES[Path(relative).parts[1]]))
    return findings, scanned


def main(argv: Sequence[str] | None = None, *, root: Path | None = None) -> int:
    """Run the gate. `root` is injectable so the gate itself has a unit test."""
    parser = argparse.ArgumentParser(description="Check localized docs for mixed-script intrusions")
    parser.add_argument("paths", nargs="*", help=argparse.SUPPRESS)
    args = parser.parse_args(argv)

    if root is None:
        root = Path(__file__).resolve().parent.parent
    selected = collect_pages(root, args.paths or None)
    if args.paths and not selected:
        print(f"OK: nothing to check among {len(args.paths)} file(s)")
        return 0

    findings, scanned = scan(root, args.paths or None)
    if scanned == 0:
        print("FAIL: no prose lines were scanned - the gate is checking nothing")
        return 1
    if findings:
        print("\n".join(findings))
        print(f"FAIL: {len(findings)} mixed-script intrusion(s) in localized docs")
        return 1
    scope = f"{len(selected)} page(s)" if not args.paths else f"{len(selected)} changed page(s)"
    print(f"OK: ja/zh/ko docs stay in their own script ({scope}, {scanned} prose lines scanned)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
