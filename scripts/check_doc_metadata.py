"""Refuse a documentation page whose metadata the site generator cannot read.

Every page under `docs/{en,ja,ko,zh}` opens with a YAML block that decides the page title, the
`<meta description>`, the tags and the changelog's place in the navigation. Four changelog pages lost
theirs: `title: Changelog description: … tags:` on one line is not a mapping.

What that does depends on the generator's version, and both halves are measured. The locked 0.0.56 - what
CI and the deployed site build - publishes the page anyway, with its `<meta name="description">` replaced
by the site-wide description and its tags gone; the `<title>` is unchanged, because the damaged line still
begins with the title's own value, which is exactly why the damage survived three merges unseen. The newer
0.0.69 refuses the build outright: `error reading page metadata 'changelog.md'`. A green deploy therefore
proved nothing about the page, and the next generator upgrade turns the same bytes into a red deploy.

The width fixer caused it and no gate could see it. `paragraph_blocks` was written to skip front matter
and skipped only its opening `---`, so the block's *body* was treated as a paragraph - joined, balanced,
re-wrapped. The result satisfied every rule the tree checks: it was inside the width limit, it had no
bogus heading, the mirror checker read the version headings fine, and `rumdl` does not care. The site is
built by a workflow that runs on push to `main` rather than on pull requests, so three merged pull
requests carried the damage before anyone opened a rendered page.

Two rules, both stated structurally so the hook needs nothing but the standard library (PyYAML is used
as a confirmation when it happens to be installed, never as a requirement):

- a metadata line carries one key. A second `key:` on the same line is the join, and it is reported with
  the offending text rather than as a YAML byte offset.
- a raw `<details>` block is invisible to the renderer's Markdown. `md_in_html` parses the body of an
  HTML block only when the opening tag carries the `markdown` attribute, so a fold written without it
  prints its own `####` headings, bullet lists and code fences as text - which is what the deployed
  changelog pages showed for all 21 folded releases. GitHub parses `<details>` bodies either way, which
  is why the shape looked correct everywhere a human read it.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
LOCALES = ("en", "ja", "ko", "zh")
KEY = re.compile(r"^\s*([A-Za-z_][\w-]*):")
# A second `key:` after the first value starts is the join this damage produces. Hyphenated words
# inside a sentence (`well-known:`) are excluded by requiring the colon to close a value-ish run, and
# a false positive costs only a message a human can read.
JOINED = re.compile(r"^\s*[A-Za-z_][\w-]*:\s+\S.*\s[A-Za-z_][\w-]*:\s")
FOLD = re.compile(r"^\s*<details(?![^>]*\bmarkdown=)")


def pages(paths: list) -> list:
    """The files this gate owns: the hook's arguments, or every tracked page of the published tree.

    Listing the locale *directories* rather than a `docs/<locale>/**/*.md` pattern is deliberate: the
    glob form skips a locale's own top level, which is exactly where the four changelog pages live, and
    the first version of this function passed its own coverage test only because the test asked a
    different question. `git ls-files` returns every tracked path under the directory, so a page cannot
    hide at either depth.
    """
    if paths:
        return [path for path in paths if path.endswith(".md")]
    tracked = subprocess.run(
        ["git", "ls-files", "--", *(f"docs/{locale}" for locale in LOCALES)],
        cwd=REPO,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.split()
    return [path for path in tracked if path.endswith(".md")]


def block_of(lines: list) -> tuple:
    """The metadata block's lines and the index just past it, or `(None, 0)` when absent."""
    if not lines or lines[0].strip() != "---":
        return None, 0
    for index in range(1, len(lines)):
        if lines[index].strip() in ("---", "..."):
            return lines[1:index], index + 1
    return None, 0


def findings(rel: str, text: str) -> list:
    """Every reason this page would fail the build or mis-render, as `(file, line, message)`."""
    out = []
    lines = text.split("\n")
    block, after = block_of(lines)
    if block is None:
        out.append((rel, 1, "no YAML front matter: the page has no title, description or tags"))
    else:
        seen = []
        for offset, line in enumerate(block, start=2):
            if not line.strip() or line.lstrip().startswith(("#", "- ", "* ")):
                continue
            keys = [match.group(1) for match in KEY.finditer(line)]
            if not keys:
                out.append((rel, offset, f"metadata line is not a key: {line.strip()[:60]!r}"))
                continue
            if JOINED.match(line):
                out.append(
                    (
                        rel,
                        offset,
                        "two keys on one line, so this is not a mapping (a formatter joined them):"
                        f" {line.strip()[:80]}",
                    )
                )
            for key in keys:
                if key in seen:
                    out.append((rel, offset, f"key {key!r} appears twice in the metadata block"))
                seen.append(key)
        if "title" not in seen:
            out.append((rel, 1, "the metadata block has no `title`, so the page renders without one"))
    body_start = after - 1 if block is not None else 0
    for offset, line in enumerate(lines[body_start:], start=body_start + 1):
        if FOLD.match(line):
            out.append((rel, offset, 'a raw <details> hides its body from the renderer; write <details markdown="1">'))
    return out


def _confirmed_by_yaml(rel: str, text: str) -> str:
    """Ask PyYAML the same question the generator asks, when it is installed.

    The structural rules above are what the hook can rely on. When a YAML reader is available it is
    used to catch shapes the regexes were not written for, and its message is appended verbatim.
    """
    try:
        import yaml
    except ModuleNotFoundError:
        return ""
    block, _after = block_of(text.split("\n"))
    if block is None:
        return ""
    try:
        yaml.safe_load("\n".join(block))
    except Exception as error:
        return f"{rel}: front matter does not parse: {str(error).replace(chr(10), ' ')[:160]}"
    return ""


def main(argv: list) -> int:
    checked = pages(argv)
    if not checked:
        print(
            "check_doc_metadata: no pages selected - a renamed docs tree would silently disable this gate",
            file=sys.stderr,
        )
        return 1
    found = []
    for rel in checked:
        path = REPO / rel
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8")
        found.extend(findings(rel, text))
        extra = _confirmed_by_yaml(rel, text)
        if extra:
            found.append((rel, 1, extra.split(": ", 1)[1]))
    for rel, line, message in found:
        print(f"{rel}:{line}: {message}", file=sys.stderr)
    if found:
        print(f"FAIL: {len(found)} metadata problem(s) across {len(checked)} page(s)", file=sys.stderr)
        return 1
    print(f"OK: {len(checked)} page(s) carry metadata the site generator can read")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
