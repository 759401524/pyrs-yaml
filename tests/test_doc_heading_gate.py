"""Gate the shape a Markdown formatter creates and no other gate can see.

A wrapped paragraph whose continuation line begins with an issue reference is, to a Markdown
formatter, an ATX heading. `rumdl fmt` promotes it, blanks around it, and the sentence is left
split across a heading — which is valid Markdown, so it passes the linter, the mirror checker,
the i18n checker and the link checker: nothing asks whether a heading was *meant*. Three such
headings sat in the tree at once (two in `ROADMAP.md`, one in `QUALITY_MATRIX.md`), and the
mechanism was reproduced rather than inferred — a scratch file containing

    Intro line one, and the
    #292 branch as written, and more
    text continues here.

came out of `rumdl fmt` as

    Intro line one, and the

    ## 292 branch as written, and more

    text continues here.

so `scripts/check_doc_headings.py` asserts the shape: a heading whose text begins with two or
more digits that are not followed by a dot. The digit-headings this repository *does* mean
(`### 1-D array`, `#### 10. メタデータの操作`, `## 1. Test matrix coverage`) are in the tests
below, because a rule that reddes legitimate writing is a rule that gets disabled.

A second shape came from the re-flow work, and its first rule was falsified before it shipped.
Re-flowing a paragraph can leave a run of dashes as the next line, which CommonMark reads as a
setext underline: the paragraph *is* a heading then, and `rumdl fmt` answered its own `MD003` by
rewriting a whole sentence into a 126-column `## ` one. The rule tried first — flag any heading
wider than the 100-column prose convention — matched twelve legitimate `### (xx)` lead-ins in
`ROADMAP.md`, so it was discarded and the shape is named directly instead.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
CHECKER = REPO_ROOT / "scripts" / "check_doc_headings.py"

# The damaged line, copied from `ROADMAP.md` as merged by PR #307's follow-up audit.
DAMAGED = [
    "so the hook did not fire and the comparison had nothing to say. Meanwhile",
    "",
    '## 293\'s tier of the same class — "hygiene hooks run in CI" — did',
    "",
    "get its entries. One of the two judgements is inconsistent with the other,",
]
# The same text after joining it back and letting a word lead the line.
REPAIRED = [
    "so the hook did not fire and the comparison had nothing to say. Meanwhile",
    'PR #293\'s tier of the same class — "hygiene hooks run in CI" — did',
    "get its entries. One of the two judgements is inconsistent with the other,",
]


def _load():
    spec = importlib.util.spec_from_file_location("check_doc_headings", CHECKER)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


@pytest.fixture(scope="module")
def checker():
    return _load()


# ── the tree ──────────────────────────────────────────────────────────────────────


def test_every_tracked_page_is_clean(checker):
    assert checker.main() == 0


@pytest.mark.parametrize(
    ("path", "scoped"),
    [
        ("ROADMAP.md", True),
        ("docs/ja/guides/numpy.md", True),
        ("README.md", True),
        # Upstream, generated, and vendored trees are not this repository's prose.
        ("Reference/yaml-test-suite/Readme.md", False),
        ("site/en/changelog/index.html", False),
        ("docs/en/features.md", True),
    ],
)
def test_the_scope_predicate_names_what_the_rule_speaks_for(checker, path, scoped):
    assert checker.is_scoped(path) is scoped, path


# ── the shape, both ways ─────────────────────────────────────────────────────────


def test_a_promoted_issue_reference_is_a_finding(checker):
    findings = checker.heading_damage(DAMAGED, "ROADMAP.md")
    assert len(findings) == 1, findings
    assert "ROADMAP.md:3:" in findings[0], findings[0]
    assert "## 293's tier" in findings[0], findings[0]


def test_the_same_sentence_joined_back_is_clean(checker):
    """Discriminator: the rule is about the shape, not about issue references.

    Removing the promotion has to silence it, otherwise the fix would be to stop citing pull
    requests in the ledger - which is the opposite of what this repository records.
    """
    assert checker.heading_damage(REPAIRED, "ROADMAP.md") == []


@pytest.mark.parametrize(
    "line",
    [
        # Every sample is copied verbatim from the page named in the trailing comment, which
        # is why two of them carry fullwidth punctuation that ruff rightly flags as ambiguous
        # in code - the ambiguity is the point, since these are prose headings, not literals.
        "### 1-D array",  # docs/en/features.md
        "#### 4 言語すべてをビル드",
        "#### 2-D matrix",  # docs/ja/features.md
        "#### 0-D Scalar Arrays",  # docs/en/guides/numpy.md
        "## 1. Test matrix coverage",  # QUALITY_MATRIX.md
        "#### 10. メタデータの操作（comment, anchor, tag）",  # noqa: RUF001 - docs/ja/quick-start.md
        "#### 13. 深度编辑（批量设置、排序、移动、复制）",  # noqa: RUF001 - docs/zh/quick-start.md
    ],
)
def test_the_digit_headings_this_repository_means_are_not_findings(checker, line):
    """Every sample is a real line in the tree, taken while sizing the rule.

    A rule over prose has to be measured against the prose that exists: one digit (`1-D`),
    digits-then-a-dot (`10.`) and a shell comment inside a fenced block are all writing this
    repository does, and a guard that reddened them is a guard that gets switched off.
    """
    assert checker.heading_damage([line], "page.md") == [], line


def test_a_comment_inside_a_code_fence_is_not_a_heading(checker):
    """The rule reads prose; fenced shell sessions are full of `# 2025 …` comments."""
    lines = ["```console", "$ run", "# 2025 timings were recorded here", "```"]
    assert checker.heading_damage(lines, "page.md") == []
    # …and the fence tracking has to end, or everything after the block goes unchecked.
    assert len(checker.heading_damage([*lines, "## 2025 timings"], "page.md")) == 1


def test_the_finding_tells_the_reader_how_to_comply(checker):
    """A guard whose message does not name the fix gets bypassed rather than obeyed."""
    findings = checker.heading_damage(DAMAGED, "ROADMAP.md")
    assert "Join it back" in findings[0], findings[0]
    assert "PR #292" in findings[0], findings[0]


def test_a_dash_run_under_a_paragraph_is_a_promoted_paragraph(checker):
    """The setext shape, fired by injection: prose on one line, dashes on the next.

    This is the damage the re-flow caused and no gate saw. A separator is legal, so the finding is
    the missing blank line, not the dashes themselves.
    """
    damaged = ["A paragraph that ends here, and the sentence continues on no line at all.", "---"]
    found = checker.setext_damage(damaged, "page.md")
    assert len(found) == 1, found
    assert "setext heading" in found[0], found[0]
    separated = [damaged[0], "", damaged[1]]
    assert checker.setext_damage(separated, "page.md") == [], separated


def test_front_matter_closing_dashes_are_not_findings(checker):
    """A front matter block ends with `---` under a content line by definition.

    Measured reason: the localized changelog pages all carry mkdocs front matter, and a rule that
    reddes on it is a rule that gets disabled rather than a rule that gets obeyed.
    """
    page = ["---", "title: Changelog", "---", "", "## 0.15.0", "", "Prose here.", "", "---"]
    assert checker.setext_damage(page, "page.md") == [], page


def test_a_dash_run_inside_a_fenced_block_is_not_a_finding(checker):
    fenced = ["```text", "a shell session line", "-----", "```"]
    assert checker.setext_damage(fenced, "page.md") == [], fenced


def test_the_heading_gate_fires_on_the_tree_it_was_built_from(checker):
    """The promoted paragraph, as it was in the tree before repair, reddens the check.

    Copying the damaged line here rather than trusting the story is what keeps the gate honest: the
    width-based rule that came first passed this file and reddened twelve legitimate headings.
    """
    promoted = [
        "### (au) A Markdown formatter had been rewriting the ledger's sentences into headings",
        "",
        "A third finding was about the hook tier rather than the text. Two formatters on one page",
        "converge: `rumdl fmt` re-wraps prose at its own width, so",
        "-----",
    ]
    assert checker.setext_damage(promoted, "ROADMAP.md") != [], promoted
    assert checker.heading_damage(promoted, "ROADMAP.md") == [], "the digit rule sees nothing here"


def test_the_hook_is_wired(checker):
    """The rule has to run at the moment the damage is created: commit time."""
    prek = (REPO_ROOT / "prek.toml").read_text(encoding="utf-8")
    assert 'id = "doc-heading-integrity"' in prek, prek
    assert "python scripts/check_doc_headings.py" in prek, prek
