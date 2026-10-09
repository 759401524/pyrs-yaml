"""Wrap documentation prose to a fixed display width, and refuse pages that exceed it.

# Why this exists

`ROADMAP.md` had reached one line of 21,850 characters, and 308 of its 939 lines were wider
than a terminal window. The changelog mirrors were worse in the same way for a different
reason: `zh`/`ja`/`ko` cannot be re-flowed by any tool that only breaks at spaces, so every
long CJK paragraph stayed one physical line (884 columns in `docs/ja`, 779 in `docs/ko`).
Neither shape is a rendering bug — Markdown does not care — which is exactly why no gate had
noticed: the files were readable to a diff viewer only, and the ledger that carries this
repository's whole quality history is the one document people are meant to *read*.

# The convention

One paragraph per logical line group, wrapped at 100 **display columns**, where a
fullwidth character (`unicodedata.east_asian_width` in `F`/`W`) counts as two. CJK text
breaks between characters, Latin text at spaces, and a line never starts with closing
punctuation or ends with an opening one - a rule the CJK pages need and the Latin ones
mostly hit by accident. Code fences, tables, headings, front matter and admonition markers
are left alone: wrapping any of them changes what they are, not how they look. Inline
spans (`` `code` ``, link destinations, emphasis delimiters) are atomic, so a break can
never split `**bold**` from its own text or cut a URL in half.

# Who owns a line, and why width has no other authority

`rumdl fmt` is the repository's Markdown formatter and stays the one: it normalises list markers,
indentation and blank lines, which a width tool must not touch - measured, because excluding it left
`MD007`/`MD012` failing on six lines in the localized changelogs, and handing them back fixed all
six. But it is *not* an authority over width, and an earlier draft of this file assumed it was. Fed
a 587-character English line, `rumdl fmt` emits a 587-character line; fed a 319-column Korean one,
the same. A `line-length = 80` override changes nothing, because that key configures the linter -
and `.rumdl.toml` disables `MD013` with the comment "line length handled by general formatting",
which this measurement shows is not true. So the premise "a Latin line over the limit is the
formatter's decision, leave it" is false, and the 634 over-length lines it excused are simply
unowned prose, of which 294 were Korean and 184 Japanese.

The division is therefore by dimension, not by language: rumdl owns Markdown *structure*, this rule
owns *width* for every prose line it can re-flow:

- Over the limit with a break that fits: reported here and re-flowed by `--fix`, English, Korean and
  Chinese alike.
- Over the limit where the *shortest* legal cut is itself over budget: left alone, not reported. Four
  such lines exist (`CHANGELOG.md:694`, `:1578`, `ROADMAP.md:1175`, `docs/en/changelog.md:702`), each
  opening with a 101-127 column code span such as a `cargo build --target thumbv7em-none-eabi ...`
  invocation. Splitting the span would corrupt the very text that makes the line long, and a gate
  nobody can satisfy gets bypassed rather than obeyed - so the exemption is the fixer's own
  capability, not a list of exceptions.

`--fix` and the check read the same two lists, so the gate cannot drift from the fixer and reject
what `prek` just wrote. Convergence is measured, not assumed: repairing this tree's backlog made
`rumdl fmt` rewrite three files and this hook four in the first round, and nothing in the second or
third - the fixed point holds with either hook running first.

# The one thing here that is not about width

A space with a Han or kana glyph on both sides (`は すべて`) is residue from some wrap that ate a
break: Chinese and Japanese do not separate words with spaces, so the gap has no reading in which
it is right. `rumdl fmt` cannot see it - it has no notion of display columns or scripts - so
`spacing_artifacts` reports it. The rule is deliberately narrower than "any CJK space": Korean word
spacing is grammatical (`인용 「열기」는`), and a pattern that also matched leading indentation or a
space before an opening bracket was measured at 208 false positives across these pages against the
zero real ones they hold now.
"""

from __future__ import annotations

import re
import sys
import unicodedata
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
MAX_WIDTH = 100

# Lines matching these are never re-wrapped: they are structure, or their whitespace is
# meaningful (a table cell, a code sample, a heading an anchor is computed from).
SKIP_PATTERNS = (
    re.compile(r"^#{1,6} "),  # ATX heading
    re.compile(r"^\s{0,3}\|"),  # table row
    re.compile(r"^\s{0,3}(?:-{3,}|\*{3,}|_{3,})\s*$"),  # thematic break
    re.compile(r"^!!!\s"),  # admonition
    re.compile(r"^\?\?\?\s"),  # collapsible section
    # A standalone HTML tag line: the changelog folds released versions in <details>/<summary>, and
    # re-flowing one of those would break the tag across two lines and produce markup that renders as
    # literal text - the failure this whole tool is written to avoid, so structure wins over width.
    re.compile(r"^\s*</?[a-z][a-z0-9-]*(?:\s[^>]*)?>\s*$"),
    re.compile(r"^\s{0,3}\[\^\d+\]:"),  # footnote definition body start
)
# An opener is a fence run with at most an info string after it, and it only opens a block
# where one can start: after a blank line. That second condition is what keeps a prose line
# that begins with a three-backtick code span from being read as a block opener and
# desynchronising the fence state for the rest of the file - which matters because wrapping
# can legitimately put a code span at the start of a continuation line, and refusing every
# such break left 1900-column lines behind.
FENCE_OPEN = re.compile(r"^\s{0,3}(`{3,}|~{3,})[^`\n]*$")
LIST_ITEM = re.compile(r"^(\s*)([-*+]\s+|\d{1,3}\.\s+|\[[ xX]\]\s+)")
QUOTE = re.compile(r"^(\s*>+\s?)")
# Anything that opens a new block of its own, and therefore closes the paragraph being collected.
# A fenced-code opener counts, so a block never swallows a code sample that starts without a blank
# line above it.
BLOCK_END = re.compile(r"^(?:\s*(?:[-*+]\s|\d{1,3}\.\s|\[[ xX]\]\s)|#{1,6} |\s{0,3}(?:`{3,}|~{3,}))")
# What cannot start a prose block at all: a heading owns a line of its own, and a fence opener does
# not belong to the paragraph above it. List markers are absent here on purpose - an item *is* a
# block, it just never joins with the item before it.
NOT_A_BLOCK = re.compile(r"^(?:#{1,6} |\s{0,3}(?:`{3,}|~{3,}))")
# Something that would *become* a list marker if it landed at the start of a continuation line. The
# Korean changelog gained a bullet when a wrap cut in front of a literal `+` joining two ABI names
# (`abi3t` + `abi3t-py315`): a renderer sees a new `+ ` item where the author wrote a plus sign.
# Refusing such a break costs almost nothing, because a marker is only a marker at line start.
MARKER_START = re.compile(r"^(?:[-*+]|\d{1,3}[.)])\s")

# Opening delimiters never end a line; closing punctuation never starts one. The sets cover
# CJK and Latin because both appear in these pages, often inside one sentence. Emphasis
# markers are absent on purpose: a line may end with a *closing* `**`, which `_emphasis_open`
# tells from an opening one - refusing both left 186-column lines in the probe. The fullwidth
# glyphs are the point of the two sets, so the ambiguity warning is what this file is for.
NO_LINE_END = set("([{（〔〈【《「『“‘«")  # noqa: RUF001
NO_LINE_START = set("-)]},），、．。：；！？〉〕」』】》”’…,.;:!?%)}]}\"'")  # noqa: RUF001


def display_width(text: str) -> int:
    """Columns `text` occupies, counting fullwidth glyphs as two."""
    return sum(2 if unicodedata.east_asian_width(ch) in ("F", "W") else 1 for ch in text)


def _atomic_spans(line: str) -> list:
    """Character ranges a wrap must not cut through: code spans and link destinations.

    Code spans follow CommonMark's rule: a run of k backticks is closed by the next run of
    exactly k, and if no such run exists on the line the opener is literal text, not a span.
    Pairing naively instead - first-backtick-then-next-whatever - lets one literal backtick in
    prose swallow every space that follows it, and the wrapper then reports a 1 973-column
    paragraph it cannot break. Both shapes are in `ROADMAP.md`: it mentions the backtick
    character as text *and* quotes code, in the same paragraph.
    """
    spans = []
    position = 0
    length = len(line)
    while position < length:
        if line[position] != "`":
            position += 1
            continue
        start = position
        run = 0
        while start + run < length and line[start + run] == "`":
            run += 1
        probe = start + run
        closer = None
        while probe < length:
            if line[probe] == "`":
                candidate = 0
                while probe + candidate < length and line[probe + candidate] == "`":
                    candidate += 1
                if candidate == run:
                    closer = (probe, candidate)
                    break
                probe += candidate
            else:
                probe += 1
        if closer is None:
            position = start + run  # literal backticks: keep the rest of the line breakable
            continue
        spans.append((start, closer[0] + closer[1]))
        position = closer[0] + closer[1]
    # `](dest)` plus the surrounding `[text]`: a URL has no space to break at anyway, but a
    # long link *text* does, and cutting inside the destination breaks the link outright.
    for match in re.finditer(r"\]\([^)\n]*\)", line):
        spans.append(match.span())
    for match in re.finditer(r"<[^>\s]+>", line):
        spans.append(match.span())
    return spans


def _in_any(position: int, spans: list) -> bool:
    """Whether a break at `position` would cut *through* a span.

    Strictly inside only: the boundary before an opening backtick and the one after a closing
    backtick are ordinary word ends. Treating them as inside (an earlier version did) left so few
    legal breaks that a wrapped paragraph came out with a one-word line - `and` alone - which is
    the exact ugliness this file exists to remove.
    """
    return any(start < position < end for start, end in spans)


def _emphasis_open(text: str) -> bool:
    """Whether `text` ends inside an unclosed emphasis run.

    Only that case makes a break illegal: a delimiter that opened emphasis cannot be followed
    by a line break, while a delimiter that just closed it is an ordinary word end. Counting
    the runs is enough here because the pages use `**bold**` symmetrically, and a code span is
    already an atomic range in `_break_points`.
    """
    for run in ("**", "__", "*", "_"):
        marker = run[0]
        if text.endswith(run) and text.count(marker) % 2 == 1:
            return True
    return False


def _is_hangul(character: str) -> bool:
    """Whether `character` is a Hangul syllable or jamo.

    Korean is written with spaces between words, so a wrap has somewhere to go without cutting a
    word in half; Han and kana do not, which is why they may be broken anywhere and Hangul may
    not. Found on the Korean changelog, where the first version produced `내` / `보내는`.
    """
    return "\u1100" <= character <= "\u11ff" or "\uac00" <= character <= "\ud7a3"


def _is_han_kana(character: str) -> bool:
    """Whether `character` is Han or kana, the scripts that carry no word spaces.

    Hangul is excluded: Korean words are space-separated, so gluing two Korean lines would fuse a
    particle onto the following noun.
    """
    return "\u3040" <= character <= "\u30ff" or "\u4e00" <= character <= "\u9fff"


def join_lines(body: list) -> str:
    """Join physical lines of one paragraph back into a single logical line.

    The glue is the whole point, and getting it wrong is how the first block-aware pass created 146
    residue findings in `docs/ja` and `docs/ko`: joining with a space is correct for Korean and Latin
    and *wrong* between two Han or kana glyphs, where a space is precisely the artifact
    `spacing_artifacts` reports. A boundary touching a Latin, a digit or a code span keeps its space,
    because `は `docs.md` にある` is normal typography.
    """
    out = body[0]
    for following in body[1:]:
        if not out or not following:
            out = out + following
            continue
        glue = "" if _is_han_kana(out[-1]) and _is_han_kana(following[0]) else " "
        out = out.rstrip() + glue + following.lstrip()
    return out


def _has_break_inside(line: str, room: int) -> bool:
    """Whether *some* legal cut of `line` would leave the first piece within `room` columns.

    This is the fixer's own capability expressed as a predicate, and the check has to use it:
    `_wrap_line` gives up and emits the line unchanged when no cut fits, so a rule that reported
    such a line would be red forever, and `--fix` would appear to do nothing to it. That is exactly
    what the four overlong-code-span lines in the changelogs were before this predicate existed.
    """
    return any(display_width(line[:point].rstrip()) <= room for point in _break_points(line))


def _break_points(line: str) -> list:
    """Offsets at which the line may be split, in ascending order.

    Two sources: a space outside the atomic spans, and a boundary between two
    fullwidth-capable characters (CJK runs have no spaces to work with). Both are then
    filtered for the punctuation rules, so the result is a list of *legal* breaks rather
    than merely possible ones.
    """
    spans = _atomic_spans(line)
    chars = list(line)
    candidates = []
    for index, char in enumerate(chars):
        if char in " \t\r" and not _in_any(index, spans):
            after = index + 1
            while after < len(chars) and chars[after] == " ":
                after += 1
            if after < len(chars):
                candidates.append(after)
    for index in range(1, len(chars)):
        left, right = chars[index - 1], chars[index]
        if _in_any(index, spans):
            continue
        if _is_hangul(left) or _is_hangul(right):
            continue
        if not (unicodedata.east_asian_width(left) in ("F", "W") or unicodedata.east_asian_width(right) in ("F", "W")):
            continue
        if left == " " or right == " ":
            continue
        candidates.append(index)

    legal = []
    for point in sorted(set(candidates)):
        if point in (0, len(line)):
            continue
        before = line[:point].rstrip()
        after = line[point:].lstrip()
        if not before or not after:
            continue
        if before[-1] in NO_LINE_END or after[0] in NO_LINE_START:
            continue
        if after.startswith("|") or before.endswith("|"):
            continue
        if MARKER_START.match(after):
            continue  # would invent a list item, which is a content change, not a wrap
        if _emphasis_open(before):
            continue
        if after.startswith("#"):
            # Never put an issue reference at the start of a continuation line: `as #283, whose
            # fuzz jobs ...` broken there is a heading to the Markdown formatter, which is the
            # exact damage `check_doc_headings.py` exists to catch. The wrapper tripped that gate
            # on this file, which is how the rule got here.
            continue
        legal.append(point)
    return legal


def _wrap_line(body: str, indent: str, width: int, cont: str | None = None) -> list:
    """Greedy wrap of one already-stripped logical line into pieces no wider than `width`.

    `cont` is the prefix for every piece after the first. It defaults to `indent`, which is
    right for a block quote (each line needs its `>`) and wrong for a list item, where
    repeating the marker would split one item into several - a structure change, not a wrap.

    Cut points are absolute offsets into `body`, kept in one place on purpose: an earlier
    version re-based them against a shrinking remainder and drifted, which cut the middle of
    an inline code span - the one corruption this function exists to avoid.
    """
    cont = indent if cont is None else cont
    budget = max(width - display_width(indent), 20)
    if display_width(body) <= budget:
        return [indent + body] if body else [indent.rstrip()]
    points = _break_points(body)
    if not points:
        return [indent + body]  # one unbreakable token: overflow rather than corruption
    out = []
    start = 0
    while start < len(body):
        prefix = indent if not out else cont
        room = max(width - display_width(prefix), 20)
        tail = body[start:].rstrip()
        if display_width(tail) <= room:
            out.append(prefix + tail)
            return out
        cut = None
        for point in points:
            if point <= start:
                continue
            # Piece widths grow monotonically with `point`, so the first overflow ends the scan.
            if display_width(body[start:point].rstrip()) > room:
                break
            cut = point
        if cut is None:
            out.append(prefix + tail)
            return out
        out.append(prefix + body[start:cut].rstrip())
        start = cut
        while start < len(body) and body[start] == " ":
            start += 1
    return out


def reflow_paragraph(line: str, width: int = MAX_WIDTH) -> list:
    """Wrap one physical line, preserving any list or blockquote prefix."""
    quote = QUOTE.match(line)
    if quote:
        prefix = quote.group(1)
        inner = _wrap_line(line[len(prefix) :].strip(), prefix, width)
        return [text if text.strip() else prefix.rstrip() for text in inner]
    item = LIST_ITEM.match(line)
    if item:
        indent, marker = item.group(1), item.group(2)
        # Continuation lines are indented to the item's text column, never re-marked: a
        # second `- ` would be a second list item, which is a content change.
        return _wrap_line(
            line[len(indent) + len(marker) :].strip(),
            indent + marker,
            width,
            cont=indent + " " * len(marker),
        )
    indent = re.match(r"^\s*", line).group(0)
    return _wrap_line(line[len(indent) :].strip(), indent, width)


def scan(text: str):
    """Yield ``(line, in_fence)`` for a document, with fence state tracked once.

    Shared by the wrapper and the check on purpose: if the two disagreed about which lines are
    code, the gate would reject exactly what `--fix` writes.
    """
    lines = text.split("\n")
    fence = None
    for index, line in enumerate(lines):
        if fence is None:
            opener = FENCE_OPEN.match(line)
            starts_a_block = index == 0 or not lines[index - 1].strip()
            if opener and starts_a_block:
                fence = (opener.group(1)[0], len(opener.group(1)))
                yield line, True
                continue
            yield line, False
            continue
        stripped = line.strip()
        if stripped and set(stripped) == {fence[0]} and len(stripped) >= fence[1]:
            fence = None
        yield line, True


def is_skipped(line: str) -> bool:
    return any(pattern.match(line) for pattern in SKIP_PATTERNS)


def wrap_text(text: str, width: int = MAX_WIDTH) -> str:
    """Re-wrap every prose line in a document, leaving structure untouched."""
    out = []
    in_front_matter = text.startswith("---\n")
    for line, in_fence in scan(text):
        if in_front_matter:
            out.append(line)
            if line.strip() == "---":
                in_front_matter = False
            continue
        if in_fence or not line.strip() or is_skipped(line):
            out.append(line)
            continue
        out.extend(reflow_paragraph(line, width))
    return "\n".join(out)


def offenders(lines: list, name: str = "") -> list:
    """Lines wider than the convention that a re-flow could bring inside it.

    Fence state comes from `scan`, the shared pass, so the gate cannot disagree with `--fix` about
    which lines are code. One exemption scopes the rule: a line whose shortest legal cut is itself
    over the limit cannot be repaired without splitting an atomic span, so it is left alone rather
    than reported. Everything else over 100 display columns belongs here - measured, because
    `rumdl fmt` preserves author line breaks and excuses none of them.
    """
    out = []
    for number, line, in_fence in _scan_list(lines):
        if in_fence or not line.strip() or is_skipped(line):
            continue
        width = display_width(line)
        if width <= MAX_WIDTH:
            continue
        body = line.strip()
        indent = re.match(r"^\s*", line).group(0)
        room = MAX_WIDTH - display_width(indent)
        if not _has_break_inside(body, room):
            continue  # no cut fits: overflow is physical here, so reporting it would be noise
        out.append((name, number, width, body[:60]))
    return out


def _scan_list(lines: list):
    """`scan` over an already-split document, numbered from one."""
    return [(number, line, fence) for number, (line, fence) in enumerate(scan("\n".join(lines)), 1)]


def paragraph_blocks(text: str):
    """Index ranges of prose that belongs to one logical paragraph, ``(start, stop, prefix)``.

    Wrapping a *physical* line at a time is what left `and`, `one` and `what` alone on their own
    lines in `ROADMAP.md`: a line just over the limit loses its last word to a continuation, and the
    next pass loses another, cascading until half the paragraph is orphans. A paragraph is the unit
    that has to be balanced.

    A block ends at a blank line, a fenced or skipped line, and - the part that matters - at any new
    list marker. The localized changelogs hold sibling entries with no blank line between them, and
    joining those once collapsed 170 separate bullets into one paragraph, so a marker always starts
    fresh. `prefix` is the list marker or blockquote marker of the line that opens the block, which
    the re-flow keeps and does not repeat.
    """
    blocks = []
    lines = text.split("\n")
    index = 0
    skip_next = text.startswith("---\n")
    while index < len(lines):
        line = lines[index]
        if skip_next:
            if line.strip() == "---":
                skip_next = False
            index += 1
            continue
        if not line.strip() or is_skipped(line) or NOT_A_BLOCK.match(line):
            index += 1
            continue
        start = index
        prefix = ""
        item = LIST_ITEM.match(line) or QUOTE.match(line)
        if item:
            prefix = item.group(0)
        index += 1
        while index < len(lines):
            following = lines[index]
            if not following.strip() or is_skipped(following) or BLOCK_END.match(following):
                break
            index += 1
        blocks.append((start, index, prefix))
    return blocks


WIDE_RUN = re.compile(
    r"(?<=[\u3040-\u30ff\u3130-\ud7a3\u4e00-\u9fff]) {2,}(?=[\u3040-\u30ff\u3130-\ud7a3\u4e00-\u9fff])"
)
# A space after closing CJK punctuation, or before an opening one, is the mark of a previous wrap
# eating the break there - the localized changelogs are full of them. Scoped to the punctuation
# rather than to every wide character, because Korean word spacing is grammatical and has to
# survive the pass.
CJK_PUNCT_SPACE = re.compile(
    r"([\u3001\u3002\uff0c\uff1a\uff1b\uff01\uff1f\u300d\u300f\uff09\u3011]) +| +([\uff08\u300c\u300e\u3010])"
)
# Chinese and Japanese carry no word spaces at all, so a gap with a Han or kana glyph on both sides
# can only be residue. Hangul is excluded on purpose: `인용 「열기」는` is correct Korean, and the
# wider pattern that also caught leading indentation measured 208 of those against zero real
# artifacts in these six pages. Fullwidth punctuation belongs in the class too: a gap after U+FF0C
# (ideographic comma) is exactly the shape the re-flow leaves behind on the localized pages.
HAN_KANA = "\u3040-\u30ff\u4e00-\u9fff"
CJK_TYPE = HAN_KANA + "\u3000-\u303f\uff01\uff08\uff09\uff0c\uff1a\uff1b\uff01\uff1f\uff3b\uff3d\uff5e"
CJK_WORD_SPACE = re.compile(rf"(?<=[{CJK_TYPE}]) +(?=[{CJK_TYPE}])")


def _outside_spans(line: str, pattern: re.Pattern) -> list:
    """Matches of `pattern` that are not inside an atomic span.

    A code span is literal text: `` `中文 日文` `` holds a real space, and rewriting it would change
    the sample rather than the typography - the same class of damage the atomic-span rule exists to
    prevent, met again because the residue sweep reads whole paragraphs.
    """
    spans = _atomic_spans(line)
    return [match for match in pattern.finditer(line) if not any(start <= match.start() < end for start, end in spans)]


def strip_residue(line: str) -> str:
    """Remove wrap residue from a line, leaving atomic spans byte-for-byte alone."""
    spans = sorted(s for s in _atomic_spans(line) if s[1] > s[0])
    pieces = []
    cursor = 0

    def clean(text: str) -> str:
        return CJK_PUNCT_SPACE.sub(_drop_match, WIDE_RUN.sub(" ", CJK_WORD_SPACE.sub("", text)))

    for start, end in spans:
        if start < cursor:
            continue
        pieces.append(clean(line[cursor:start]))
        pieces.append(line[start:end])
        cursor = end
    pieces.append(clean(line[cursor:]))
    return "".join(pieces)


def _drop_match(match: re.Match) -> str:
    """Keep whichever side of the artifact the pattern captured, and drop the space run."""
    return match.group(1) or match.group(2) or ""


def spacing_artifacts(lines: list, name: str = "") -> list:
    """Wrap residue the width rule cannot own and the other formatter cannot see.

    Reported as its own class rather than folded into `offenders` because the two are orthogonal:
    the gap in `は すべて` sits on an ordinary 60-column line, and a 300-column CJK paragraph may
    have no gap in it at all. Measured on the current tree: zero, because the pass that created
    these gaps was itself reverted - the class exists to keep it from coming back.
    """
    out = []
    for number, line, in_fence in _scan_list(lines):
        if in_fence or not line.strip() or is_skipped(line):
            continue
        matches = _outside_spans(line, CJK_WORD_SPACE)
        if matches:
            out.append((name, number, len(matches), line.strip()[:60]))
    return out


def reflow_block(body_lines: list, width: int) -> list:
    """Re-flow one logical paragraph given as several physical lines, balanced.

    The lines are joined (a whitespace-only change) and cut again from one buffer, which is what
    removes the orphan cascade: re-flowing each physical line on its own hands its last word to a
    continuation, and every pass takes one more.
    """
    first = body_lines[0]
    quote = QUOTE.match(first)
    if quote:
        prefix = quote.group(1)
        inner = join_lines([first[len(prefix) :].strip()] + [line.strip() for line in body_lines[1:]])
        pieces = _wrap_line(inner, prefix, width)
        return [piece if piece.strip() else prefix.rstrip() for piece in pieces]
    item = LIST_ITEM.match(first)
    if item:
        indent, marker = item.group(1), item.group(2)
        inner = join_lines([first[len(indent) + len(marker) :].strip()] + [line.strip() for line in body_lines[1:]])
        return _wrap_line(inner, indent + marker, width, cont=indent + " " * len(marker))
    indent = re.match(r"^\s*", first).group(0)
    inner = join_lines([first[len(indent) :].strip()] + [line.strip() for line in body_lines[1:]])
    return _wrap_line(inner, indent, width)


def format_text(text: str, width: int = MAX_WIDTH) -> str:
    """Re-flow every paragraph `offenders` reports in, and strip the gaps `spacing_artifacts` finds.

    Paragraphs, not lines: see `reflow_block`. A block is touched only when at least one of its lines
    was reported, so prose inside the limit keeps the breaks its author chose; `rumdl fmt` runs
    before this hook in `prek.toml`, normalises structure, and preserves the breaks written here.
    """
    lines = text.split("\n")
    reported = {number for _name, number, _width, _snippet in offenders(lines, "")}
    gapped = {number for _name, number, _count, _snippet in spacing_artifacts(lines, "")}
    dirty = reported | gapped
    fenced = {number for number, (_line, in_fence) in enumerate(scan(text), 1) if in_fence}
    reflowed = {}
    for start, stop, _prefix in paragraph_blocks(text):
        numbers = set(range(start + 1, stop + 1))
        if numbers & fenced or not numbers & dirty:
            continue
        block = [lines[index] for index in range(start, stop)]
        cleaned = [strip_residue(line) for line in block]
        reflowed[(start, stop)] = reflow_block(cleaned, width)
    # Splice the re-flowed paragraphs back into the document, leaving every other line untouched and
    # in order. `paragraph_blocks` yields disjoint ascending ranges, so the walk cannot overlap.
    out = []
    cursor = 0
    for (start, stop), new_lines in sorted(reflowed.items()):
        out.extend(lines[cursor:start])
        out.extend(new_lines)
        cursor = stop
    out.extend(lines[cursor:])
    return "\n".join(out)


def check(paths: list) -> list:
    found = []
    for rel in paths:
        path = REPO / rel
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8")
        found.extend(offenders(text.split("\n"), rel))
        found.extend(spacing_artifacts(text.split("\n"), rel))
    return found


def fix(paths: list) -> int:
    changed = 0
    for rel in paths:
        path = REPO / rel
        if not path.is_file():
            continue
        before = path.read_text(encoding="utf-8")
        after = format_text(before)
        if after != before:
            path.write_text(after, encoding="utf-8", newline="\n")
            changed += 1
            print(f"wrapped {rel}")
    return changed


def main(argv: list) -> int:
    fixing = "--fix" in argv
    paths = [a for a in argv if not a.startswith("--")]
    if not paths:
        print("usage: check_doc_wrapping.py [--fix] <file.md>...", file=sys.stderr)
        return 2
    if fixing:
        fix(paths)
    found = []
    for rel in paths:
        path = REPO / rel
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8").split("\n")
        found.extend(("too wide", entry) for entry in offenders(text, rel))
        found.extend(("wrap residue", entry) for entry in spacing_artifacts(text, rel))
    if found:
        print(f"{len(found)} problem line(s) in the governed pages ({MAX_WIDTH} display columns):")
        for kind, (name, number, measure, snippet) in found:
            detail = f"{measure} cols" if kind == "too wide" else f"{measure} gap(s)"
            print(f"  {name}:{number} ({detail}) [{kind}] {snippet}")
        print("Run: python scripts/check_doc_wrapping.py --fix <the files above>")
        return 1
    print(f"OK: {len(paths)} file(s) within {MAX_WIDTH} display columns, no wrap residue")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
