#!/usr/bin/env python3
"""Guard the localized changelogs against mixed-script intrusions.

Each locale page must stay in its own writing system:

* `zh` — no Japanese kana, no Hangul.
* `ja` — no Hangul, and no *simplified-Chinese-only* Han (the forms that Japanese
  spells with a different codepoint, e.g. 積 vs 积, 連 vs 链, 視 vs 视).
* `ko` — no Japanese kana, no simplified-Chinese-only Han.

The check deliberately reads only the `[Unreleased]` block: historical entries stay
in English in every mirror (see "Changelog Mirrors" in `AGENTS.md`), so scanning
them would report the intended content as a defect. A translated entry that drifts
into another script is not a style nit — it silently changes what a reader of that
locale is told, and it is exactly the class of mistake this file was written to
catch after four occurrences in one review cycle.

Usage:
    uv run python scripts/check_cjk_localisation.py [--full]

`--full` scans every `[Unreleased]` *and* the first released section, for use when
auditing older translations.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

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

HIRAGANA_KATAKANA = re.compile(r"[\u3041-\u309f\u30a1-\u30ff]")
HANGUL = re.compile(r"\uac00-\ud7a3")

PAGES = {
    "docs/zh/changelog.md": {"kana": True, "hangul": True, "simplified": False},
    "docs/ja/changelog.md": {"kana": False, "hangul": True, "simplified": True},
    "docs/ko/changelog.md": {"kana": True, "hangul": False, "simplified": True},
}

# The root `CHANGELOG.md` puts versions at `##`, while the localized pages nest them
# one level down (`## <title>` + `### [Unreleased]`). A pattern pinned to one depth
# silently matched nothing - the first version of this gate reported OK over zero
# scanned lines, which is how a green guard can be worth less than none. Accept both
# depths and refuse to call an empty scan a pass.
UNRELEASED = re.compile(r"^#{2,4} \[Unreleased\]", re.MULTILINE)
NEXT_SECTION = re.compile(r"^#{2,4} \[", re.MULTILINE)


def unreleased_block(text: str, *, include_first_release: bool) -> tuple[str, int]:
    """Return the block to police plus the line number it starts on (1-based).

    Raises when no `[Unreleased]` header exists at all, so a renamed heading fails
    the run loudly instead of disabling it.
    """
    match = UNRELEASED.search(text)
    if match is None:
        raise ValueError("no `[Unreleased]` section header found")
    start_line = text.count("\n", 0, match.start()) + 1
    rest = text[match.end() :]
    nxt = NEXT_SECTION.search(rest)
    if nxt is None:
        return rest, start_line
    end = nxt.end() if include_first_release else nxt.start()
    block = rest[:end]
    if not block.strip():
        raise ValueError("the `[Unreleased]` section is empty - refusing to scan nothing")
    return block, start_line


def check(path: Path, rules: dict[str, bool], *, full: bool) -> list[str]:
    findings: list[str] = []
    text = path.read_text(encoding="utf-8")
    block, offset = unreleased_block(text, include_first_release=full)
    for index, line in enumerate(block.splitlines()):
        number = offset + index
        if rules["kana"] and HIRAGANA_KATAKANA.search(line):
            findings.append(f"{path}:{number}: kana in a non-Japanese page | {line.strip()[:70]}")
        if rules["hangul"] and HANGUL.search(line):
            findings.append(f"{path}:{number}: Hangul in a non-Korean page | {line.strip()[:70]}")
        if rules["simplified"]:
            hits = sorted({SIMPLIFIED_ONLY[c] for c in line if c in SIMPLIFIED_ONLY})
            if hits:
                findings.append(f"{path}:{number}: simplified-only Han {'/'.join(hits)} | {line.strip()[:70]}")
    return findings


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--full",
        action="store_true",
        help="also police the first released section (for auditing old translations)",
    )
    # prek hands the changed files to `language: system` hooks; honour that instead
    # of ignoring it, so a run over one page reports only that page.
    parser.add_argument("paths", nargs="*", help=argparse.SUPPRESS)
    args = parser.parse_args()

    root = Path(__file__).resolve().parent.parent
    selected = PAGES
    if args.paths:
        wanted = {str(Path(p).as_posix()) for p in args.paths}
        selected = {rel: rules for rel, rules in PAGES.items() if rel in wanted}
        if not selected:
            print(f"OK: nothing to check among {len(args.paths)} file(s)")
            return 0

    findings: list[str] = []
    scanned = 0
    for rel, rules in selected.items():
        page = root / rel
        if page.exists():
            block_lines, _ = unreleased_block(page.read_text(encoding="utf-8"), include_first_release=args.full)
            scanned += len(block_lines.splitlines())
            findings.extend(check(page, rules, full=args.full))

    if findings:
        print("\n".join(findings))
        print(f"FAIL: {len(findings)} mixed-script intrusion(s) in localized changelogs")
        return 1
    scope = "Unreleased + first release" if args.full else "Unreleased"
    print(f"OK: ja/zh/ko changelogs stay in their own script ({scope}, {scanned} lines scanned)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
