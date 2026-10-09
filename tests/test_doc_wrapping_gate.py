"""Gate the prose width of the six pages that grow continuously.

`ROADMAP.md` had reached one line of 21,850 characters and `docs/ja/changelog.md` one of 884, and
no tool objected: Markdown does not care about physical line length, and neither does a renderer.
What cares is a reader - and the localized pages proved that a width rule written for Latin text is
no rule at all, because a formatter that only breaks at spaces leaves a CJK paragraph on one line
forever. So the convention is measured in **display columns** (a fullwidth glyph counts as two) and
the wrapper can break between CJK characters.

`scripts/check_doc_wrapping.py` holds both halves in one file deliberately: `--fix` and the check
share the wrap and the same two reported classes, so the gate cannot reject what the formatter
writes. These tests are what proves the wrap is safe to run over 11,000 lines of history - each rule
that protects *structure* (a code span, a list marker, a fence, a table row, an issue reference) is
fired by an injection, because a reflow tool that silently rewrites meaning is the worst kind of
formatting commit.
"""

from __future__ import annotations

import importlib.util
import pathlib
import re
import sys

import pytest

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
CHECKER = REPO_ROOT / "scripts" / "check_doc_wrapping.py"
GOVERNED = [
    "CHANGELOG.md",
    "ROADMAP.md",
    "docs/en/changelog.md",
    "docs/ja/changelog.md",
    "docs/ko/changelog.md",
    "docs/zh/changelog.md",
    # The three development documents the roadmap's history moved into: they grow one entry per fix
    # like the release notes do, so the same typographic rule has to hold them or they drift back to a
    # 21,850-character line.
    "docs/dev/quality-ledger.md",
    "docs/dev/boundaries.md",
    "docs/dev/perf.md",
]

CJK_RUN = "这是一段很长的中文说明用来验证换行器能不能在没有任何空格的情况下断行" * 3


def _load():
    spec = importlib.util.spec_from_file_location("check_doc_wrapping", CHECKER)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def checker():
    return _load()


def squeeze(value: str) -> str:
    return "".join(value.split())


# ── the tree ──────────────────────────────────────────────────────────────────────


def test_the_governed_pages_fit_the_convention(checker):
    assert checker.check(GOVERNED) == []


def test_the_wrap_is_idempotent(checker):
    """`--fix` twice must be the same as `--fix` once, or the hook can never settle."""
    for rel in GOVERNED:
        once = checker.format_text((REPO_ROOT / rel).read_text(encoding="utf-8"))
        assert checker.format_text(once) == once, rel


def test_a_latin_line_over_the_limit_is_reported_and_fixed(checker):
    """`rumdl fmt` is not a width authority, so nobody may leave a 587-character English line.

    This reverses a claim this file used to make ("rumdl re-wraps Latin prose at ~102 columns, so
    those lines are its decision"). Measured through stdin instead of assumed: a 587-character
    English line and a 319-column Korean one both come back unchanged, and `line-length = 80` does
    nothing to either because that key configures the linter while `.rumdl.toml` disables `MD013`.
    Structure is rumdl's; width is unowned unless it is owned here.
    """
    latin = "word " * 20 + "another " * 10
    assert len(checker.offenders([latin.rstrip()], "page.md")) == 1, latin
    fixed = checker.format_text(latin.rstrip())
    assert len(fixed.split("\n")) > 1, fixed
    assert checker.offenders(fixed.split("\n"), "page.md") == []
    cjk = "这是一段中文" * 30
    assert len(checker.offenders([cjk], "page.md")) == 1, cjk
    assert checker.offenders(checker.format_text(cjk).split("\n"), "page.md") == []


def test_a_space_between_han_or_kana_glyphs_is_residue_and_is_reported(checker):
    """Chinese and Japanese have no word spaces, so a gap between two glyphs has no right reading.

    This is the one class the other formatter cannot see at all: `rumdl fmt` has no notion of
    display columns or scripts, so it neither introduces nor removes `は すべて`. It is reported
    separately from width because the two are orthogonal - the gap sits on an ordinary short line.
    """
    latin_neighbour = "機構・seed と docs.md を問うて見つかった穴"
    assert checker.spacing_artifacts([latin_neighbour], "page.md") == [], "a gap beside Latin or a file name is text"
    injected = "クラッシュ番号・機構・seed は すべて残す"
    found = checker.spacing_artifacts([injected], "page.md")
    assert len(found) == 1 and found[0][2] == 1, found
    out = checker.format_text(injected)
    assert "は すべて" not in out and "はすべて" in out, out
    assert checker.spacing_artifacts(out.split("\n"), "page.md") == []


def test_korean_word_spacing_is_never_residue(checker):
    """The narrower rule is the whole point: `인용 「열기」는` is correct Korean, not a wrap artifact.

    An earlier draft scoped the pattern to "any space beside a CJK punctuation or wide run" and
    measured 208 hits across the six governed pages, of which zero were real: leading indentation
    and Korean spacing between a word and an opening bracket both matched. Hangul is excluded here
    so that the class can stay a gate instead of becoming a list of exceptions.
    """
    korean = "이제 인용 「열기」는 토큰 경계(행두 또는 `\\t:,[]{}-` 에서만 인정한다."
    assert checker.spacing_artifacts([korean], "page.md") == [], korean
    assert checker.format_text(korean) == korean


# ── structure the wrap must not touch ────────────────────────────────────────────


def test_a_line_no_formatter_can_fix_is_excluded_not_reported(checker):
    """The exemption is the fixer's capability, measured rather than asserted.

    Four lines in the changelogs open with a 101-127 column code span (a `cargo build --target
    thumbv7em-none-eabi ...` invocation) and then continue in prose. Every legal cut leaves the
    first piece over budget, so `--fix` emits them unchanged - which means a rule that reported
    them could never go green and would train people to bypass a gate. Splitting the span is not an
    option: it would corrupt the command that makes the line long.
    """
    span = (
        "`cargo build --locked --no-default-features --target thumbv7em-none-eabi "
        "-p pyrs-ast -p pyrs-schema -p pyrs-json`"
    )
    line = span + " is green locally, which is the gate that job runs."
    assert checker.display_width(span) > checker.MAX_WIDTH, checker.display_width(span)
    assert checker.offenders([line], "page.md") == [], line
    assert checker.format_text(line) == line


def test_the_gate_is_satisfiable_by_its_own_fix(checker):
    """Whatever `--fix` writes must satisfy the check, or the hook rejects its own output.

    The failure this pins was measured on the way here: the check asked for "some legal break
    exists" while the fixer needed "some legal break fits", so four lines stayed reported forever
    and every re-flowed page came back red after `--fix` had already touched them.
    """
    text = "\n".join(
        [
            "这是一段中文" * 40,  # no whitespace at all: must still be breakable
            "word " * 30,  # ordinary Latin prose: the common case
            "`a b c d e f g h i j k l m n o p q r s t u v w x y z 0 1 2 3 4 5 6 7 8 9` tail " * 3,
            "",
        ]
    )
    once = checker.format_text(text)
    assert checker.offenders(once.split("\n"), "page.md") == [], once
    assert checker.format_text(once) == once


def test_rewrapping_preserves_every_non_whitespace_character(checker):
    text = "\n\n".join(
        [
            "Prose with a `long inline code span that has spaces inside it and goes on forever` "
            + "and then ordinary words " * 4,
            "- a list item " + "body " * 60,
            "> a quote " + "body " * 60,
            "| table | " + "cell | " * 40,
            "```text",
            "a code line " + "x" * 200,
            "```",
        ]
    )
    assert squeeze(checker.wrap_text(text).replace(">", "")) == squeeze(text.replace(">", ""))


def test_a_code_span_is_never_split(checker):
    line = "start " + "`a b c d e f g h i j k l m n o p q r s t u v w x y z` " * 6
    for piece in checker.wrap_text(line).split("\n"):
        assert piece.count("`") % 2 == 0, piece


def test_a_list_item_stays_one_item(checker):
    line = "- first " + "body " * 60
    pieces = checker.wrap_text(line).split("\n")
    assert len(pieces) > 1
    assert sum(1 for piece in pieces if re.match(r"^\s*-\s", piece)) == 1, pieces
    assert all(piece.startswith("  ") for piece in pieces[1:]), pieces


def test_a_continuation_never_starts_with_a_list_marker(checker):
    """A plus sign that joins two words must not be promoted to a bullet by a line break.

    Injected rather than asserted: this is the exact shape the Korean changelog had (`abi3t` +
    `abi3t-py315`), where the wrap cut in front of the `+` and the file gained a list item.
    """
    line = (
        "- **Python 3.13, 3.14, 3.15 지원** — PyO3 `abi3-py38` 휠이 Python 3.8-3.15 커버 (GIL 빌드); "
        "`abi3t` + `abi3t-py315`는 free-threaded 안정 ABI 제공 " + "본문 " * 30
    )
    pieces = checker.wrap_text(line).split("\n")
    assert len(pieces) > 1, pieces
    for piece in pieces[1:]:  # pieces[0] is the item's own marker
        assert not re.match(r"^\s*(?:[-*+]|\d{1,3}[.)])\s", piece), piece


def test_a_block_quote_keeps_its_marker_on_every_line(checker):
    pieces = checker.wrap_text("> quoted " + "body " * 60).split("\n")
    assert len(pieces) > 1 and all(piece.startswith("> ") for piece in pieces), pieces


def test_table_rows_and_fenced_code_are_left_alone(checker):
    row = "| a | " + "b | " * 60
    fenced = "\n".join(["```text", "code " + "x" * 200, "```"])
    assert checker.wrap_text(row) == row
    assert checker.wrap_text(fenced) == fenced
    # …and a long line inside a fence is not reported as a violation of a prose rule.
    assert checker.offenders(fenced.split("\n"), "page.md") == []


def test_an_issue_reference_cannot_land_at_the_start_of_a_line(checker):
    """This is how the wrapper damaged `ROADMAP.md` once, and the gate caught it."""
    line = "…the run went green and it went in as #283, whose fuzz jobs then replayed the corpus " + "tail " * 30
    for piece in checker.wrap_text(line).split("\n"):
        assert not piece.strip().startswith("#"), piece


def test_a_line_never_begins_with_closing_punctuation(checker):
    pieces = [piece for piece in checker.wrap_text(CJK_RUN).split("\n") if piece]
    assert len(pieces) > 1, pieces
    for piece in pieces:
        assert piece[0] not in "，。、）】」", piece  # noqa: RUF001 - the fullwidth glyphs are the test


# ── width is measured in columns, not characters ─────────────────────────────────


def test_fullwidth_glyphs_count_as_two_columns(checker):
    assert checker.display_width("abcd") == 4
    assert checker.display_width("中文") == 4
    assert checker.display_width("a中") == 3


def test_cjk_wraps_without_any_space(checker):
    pieces = [piece for piece in checker.wrap_text(CJK_RUN).split("\n") if piece]
    assert len(pieces) >= 2, pieces
    assert all(checker.display_width(piece) <= checker.MAX_WIDTH for piece in pieces), pieces


def test_korean_wraps_only_at_spaces(checker):
    """Hangul is written with spaces between words, so a wrap never has to cut one in half.

    The first version broke between syllables and shipped `내` / `보내는` in the Korean changelog -
    Han and kana may be broken anywhere and Korean may not, which is a distinction only a reader of
    the affected page would notice. Rejoining the pieces with single spaces must give the original.

    This exercises the whole `format_text` path: a Korean line has spaces, so it is reported like
    any other over-length prose and re-flowed - but only where a word boundary allows.
    """
    korean = " " + " ".join(
        [
            "게이트가",
            "번호를",
            "매긴",
            "쪽은",
            "내보내는",
            "쪽뿐인데",
            "키의",
            "의미가",
            "정해지는",
            "것은",
            "되읽는",
            "쪽이다",
        ]
        * 3
    )
    pieces = [piece for piece in checker.format_text(korean).split("\n") if piece.strip()]
    assert len(pieces) > 1, pieces
    assert " ".join(piece.strip() for piece in pieces) == korean.strip(), pieces
    assert len(checker.offenders([korean], "page.md")) == 1, "Korean prose is prose too"


def test_a_space_inside_a_code_span_is_text_not_residue(checker):
    """`` `中文 日文` `` holds a real space; the residue sweep must not edit a sample.

    Found while widening the class to fullwidth punctuation: the sweep reads whole paragraphs, so a
    space that belongs to the code became a "typography artifact" and got deleted - changing what the
    documented command says, which is the exact corruption atomic spans exist to prevent.
    """
    line = "说明 `中文 日文` 与代码里的空格"
    assert checker.spacing_artifacts([line], "page.md") == [], line
    assert checker.strip_residue(line) == line, line
    injected = "说明， 所以代码 `中文 日文` 保留"  # noqa: RUF001 - the fullwidth comma is the residue
    assert len(checker.spacing_artifacts([injected], "page.md")) == 1, injected
    out = checker.format_text(injected)
    assert "，所以" in out, out  # noqa: RUF001
    assert "`中文 日文`" in out, out


def test_a_paragraph_is_balanced_rather_than_cascading(checker):
    """Re-flowing physical lines one at a time eats one word per line, every pass.

    Measured in `ROADMAP.md` after the first run of this hook: `and`, `one`, `what`, `been` and
    `first` each ended up alone on a line, because a line just past the limit hands its last word to
    a continuation and the next pass takes another. The paragraph is the unit that must be balanced,
    so `format_text` joins one and re-cuts it - a whitespace-only change.
    """
    paragraph = (
        "Markdown does not care about physical line length and neither does a renderer, so nothing "
        "objected when a line grew to two hundred columns and the reader paid for it silently. "
    ) * 2
    wrapped = checker.format_text("\n".join(paragraph[i : i + 101] for i in range(0, len(paragraph), 101)))
    pieces = [piece for piece in wrapped.split("\n") if piece.strip()]
    assert len(pieces) > 2, pieces
    assert all(checker.display_width(piece) <= checker.MAX_WIDTH for piece in pieces), pieces
    # Only the final line may be short; a mid-paragraph sliver means the cascade is back.
    for piece in pieces[:-1]:
        assert checker.display_width(piece) > checker.MAX_WIDTH - 20, piece
    assert "".join(wrapped.split()) == "".join(paragraph.split())


def test_sibling_list_items_never_merge_into_one(checker):
    """The localized changelogs put entries back to back with no blank line, and joining them once
    destroyed 170 entries. A list marker always opens a new block.
    """
    first = "- **台账重排** — " + "这是一段中文说明" * 14
    second = "- **门禁新增** — " + "这是另一段中文说明" * 14
    out = checker.format_text(first + "\n" + second)
    assert sum(1 for line in out.split("\n") if line.startswith("- ")) == 2, out
    assert "台账重排" in out and "门禁新增" in out, out
    assert squeeze(out) == squeeze(first + "\n" + second)


def test_a_double_space_between_han_glyphs_is_residue(checker):
    """Previous wraps ate one side of a break and left the other visible: `义 是`.

    Latin and Korean double spaces are not this rule's business - "sentence.  Next" is a habit of
    these pages, and Korean spacing is grammatical - so the pattern needs a Han or kana glyph on
    both sides of the run.
    """
    line = "这是一段中文  说明换行器  能不能在没有任何空格的情况下断行"
    assert len(checker.spacing_artifacts([line], "page.md")) == 1, line
    out = checker.format_text(line)
    assert "  " not in out, out
    korean = "게이트가  번호를  매긴  쪽은  내보내는"
    assert checker.spacing_artifacts([korean], "page.md") == [], korean


def test_reflowing_preserves_the_document_structure(checker):
    """Blank lines, list items and headings are structure, and a formatting pass may not spend them.

    Two measured reasons this test exists. A first draft joined hand-wrapped paragraphs back
    together before cutting them, and in the localized changelogs that merged *sibling entries* into
    one block - 170 lines of separate bullets collapsed into one paragraph, which read as a
    formatting diff. A later pass then invented an item: a wrap cut in front of a literal `+` that
    joined two ABI names, and the continuation line began with `+ `. Counting only lines that start
    with `- ` at column zero missed it, so the marker regex below is indented-any-depth and covers
    `+`, `*` and ordered markers too.
    """
    marker = re.compile(r"^\s*(?:[-*+]|\d{1,3}[.)])\s")
    for rel in GOVERNED:
        text = (REPO_ROOT / rel).read_text(encoding="utf-8")
        lines = text.split("\n")
        blanks = sum(1 for line in lines if not line.strip())
        bullets = sum(1 for line in lines if marker.match(line))
        heads = sum(1 for line in lines if re.match(r"^#{1,6} ", line))
        out = checker.format_text(text).split("\n")
        assert sum(1 for line in out if not line.strip()) == blanks, rel
        assert sum(1 for line in out if marker.match(line)) == bullets, rel
        assert sum(1 for line in out if re.match(r"^#{1,6} ", line)) == heads, rel


def test_an_unbreakable_token_overflows_without_being_reported(checker):
    line = "x" * 160
    assert checker.wrap_text(line) == line
    assert checker.offenders([line], "page.md") == []


def test_the_hook_is_wired(checker):
    prek = (REPO_ROOT / "prek.toml").read_text(encoding="utf-8")
    assert 'id = "doc-line-width"' in prek, prek
    assert "python scripts/check_doc_wrapping.py" in prek, prek


# ── page metadata ────────────────────────────────────────────────────────────────

# A page whose metadata block holds the shape that reached `main`: the `description:` line is over the
# limit, so it is exactly the input that got joined, and the prose below is over the limit too, so the
# pass has work to do either way.
LONG_DESCRIPTION = (
    "description: All notable changes to pyrs-yaml are recorded here, version by version, "
    + "including the fixes, the gates and the measurements that proved them. " * 2
)
METADATA_PAGE = "\n".join(
    [
        "---",
        "title: Changelog",
        LONG_DESCRIPTION,
        "tags:",
        "- docs",
        "status: new",
        "---",
        "",
        "## [Unreleased]",
        "",
        "Prose that has to be re-flowed by this pass, so the test cannot pass by doing nothing: "
        + "and here is the rest of the sentence." * 3,
    ]
)


def test_the_metadata_range_covers_the_block_and_not_the_body(checker):
    lines = METADATA_PAGE.split("\n")
    assert checker.front_matter(lines) == set(range(1, 7)), checker.front_matter(lines)
    assert checker.front_matter(["# no block", "", "prose"]) == set()
    # An unterminated block is not metadata: treating it as such would ignore the whole document.
    assert checker.front_matter(["---", "title: x"]) == set()


def test_the_fixer_leaves_metadata_alone_and_still_wraps_the_prose(checker):
    """The block is data the generator parses; a re-flow of it is a broken page, not a wide line.

    `paragraph_blocks` skipped only the opening `---` and then treated the body as a paragraph, so this
    input came out with `title:`, `description:` and `tags:` on one line - valid-looking Markdown, an
    invalid mapping, and a site build that stops with `error reading page metadata`. The prose is
    asserted to change in the same call, because a pass that skipped everything would pass the first
    half of this test by accident.
    """
    fixed = checker.format_text(METADATA_PAGE)
    head = lambda text: text[: text.index("\n---\n") + 1]  # noqa: E731
    assert head(fixed) == head(METADATA_PAGE), "metadata was rewritten"
    assert len(fixed.split("\n")) > len(METADATA_PAGE.split("\n")), "the pass never ran on the prose"
    assert checker.format_text(fixed) == fixed, "not idempotent"


def test_the_check_says_nothing_about_a_line_it_forbids_moving(checker):
    """Reporting a line the fixer must not re-flow would make the gate impossible to satisfy.

    This is the reason no hook ever caught the damage: after the join, every metadata line fit inside
    100 columns, so the width rule was *satisfied by the thing it caused*.
    """
    lines = METADATA_PAGE.split("\n")
    reported = {number for _name, number, _width, _snippet in checker.offenders(lines, "page.md")}
    assert 3 not in reported, "the over-long `description:` line was reported although it may not move"
    assert reported, "the over-long prose line was missed"


def test_the_joiner_never_writes_the_gap_the_checker_reports(checker):
    """A re-joined paragraph must not contain the artifact the same script then reports.

    The glue used to be decided from a script class, and U+3001 (ideographic comma) is General
    Punctuation - neither Han nor kana - so a line ending in `、` was joined to the following kana with a
    space, and `spacing_artifacts` reported the very gap the joiner had written. Measured on
    `docs/ja/changelog.md`, where `--fix` could not converge because each pass recreated the finding.
    Asking the two residue patterns makes the disagreement structurally impossible.
    """
    joined = checker.join_lines(["文章が、", "その続きを書きます"])
    assert "が、その続き" in joined, joined
    assert checker._outside_spans(joined, checker.CJK_PUNCT_SPACE) == []
    assert checker._outside_spans(joined, checker.CJK_WORD_SPACE) == []


def test_a_boundary_touching_a_code_span_keeps_its_space(checker):
    """The other half of the same rule: `は `docs.md` にある` is normal typography."""
    joined = checker.join_lines(["YAML Core は", "`docs.md` にある"])
    assert joined == "YAML Core は `docs.md` にある"
