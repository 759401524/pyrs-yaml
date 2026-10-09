"""Gate the metadata block of every published page, and the fold the renderer has to be able to open.

Four changelog pages lost their YAML front matter. The width fixer was written to skip it and skipped
only the opening `---`, so the block's body was re-flowed as prose: `title:`, `description:` and
`tags:` arrived on one line, which is not a mapping. The locked generator published those pages anyway
with the page description replaced by the site's; a newer one refuses the build with
`error reading page metadata 'changelog.md'`. Every source-level gate stayed green either way, because
the joined line satisfied the width rule, the heading rule, the mirror checker and `rumdl` - the damage
was *inside* the limit, so the width rule was satisfied by the thing it caused. And the site is built by
a workflow that runs on push to `main`, so nothing on a pull request ever rendered the page.

The second half is what the same release did to the rendered page. A raw `<details>` block is opaque to
Python-Markdown: `md_in_html` parses the body of an HTML block only when the opening tag carries the
`markdown` attribute, so 21 folded releases arrived with their `####` headings, bullet lists and code
fences printed as text. GitHub parses `<details>` bodies regardless, which is why the pattern is
copy-pasted from README advice and why the source looked fine everywhere it was read.

Both are properties of the page that only the renderer cares about, so this gate asks the renderer's
questions without running it: does the block parse, and does every fold carry the attribute.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
CHECKER = REPO_ROOT / "scripts" / "check_doc_metadata.py"

# The damaged block, exactly as it stood on `main` after the width fixer ran and before this repair:
# three keys glued onto one line, then re-wrapped to fit the limit.
DAMAGED_EN = """---
title: Changelog description: All notable changes to pyrs-yaml, formatted per Keep a Changelog and
Semantic Versioning. tags:
- docs status: new
---

## [Unreleased]
"""
REPAIRED_EN = """---
title: Changelog
description: All notable changes to pyrs-yaml, formatted per Keep a Changelog and Semantic Versioning.
tags:
  - docs
status: new
---

## [Unreleased]
"""


@pytest.fixture(scope="module")
def checker():
    spec = importlib.util.spec_from_file_location("check_doc_metadata", CHECKER)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_the_join_the_width_fixer_produced_is_a_finding(checker):
    """The damage as shipped reddens the check, and the repair does not.

    Copied from Git rather than written from memory, because a guard tested against a fictional input
    proves nothing about the input that actually reached `main`.
    """
    found = checker.findings("docs/en/changelog.md", DAMAGED_EN)
    assert found, "the joined block parsed as clean"
    reasons = " | ".join(message for _f, _l, message in found)
    # The structural rule has to be enough on its own: the hook runs with whatever interpreter `prek`
    # finds, and a gate that only speaks when a third-party reader is installed is a gate that is
    # occasionally absent. PyYAML is used to *add* detail, never to decide.
    assert "two keys on one line" in reasons, reasons
    assert checker.findings("docs/en/changelog.md", REPAIRED_EN) == []


def test_a_page_without_metadata_is_a_finding(checker):
    found = checker.findings("docs/zh/changelog.md", "## 变更日志\n\n正文。\n")
    assert any("no YAML front matter" in message for _f, _l, message in found), found


def test_a_fold_the_renderer_cannot_open_is_a_finding(checker):
    """A fold is only a fold if the body is parsed; otherwise the page prints its own markup."""
    bare = REPAIRED_EN + "\n<details>\n<summary>One sentence</summary>\n\n#### 新增\n\n- item\n\n</details>\n"
    found = checker.findings("docs/en/changelog.md", bare)
    assert any("hides its body" in message for _f, _l, message in found), found

    attributed = bare.replace("<details>", '<details markdown="1">')
    assert checker.findings("docs/en/changelog.md", attributed) == [], attributed


def test_the_gate_reads_the_tree_it_was_built_from(checker):
    """The committed pages pass, and the selection includes every page that broke.

    A gate that silently selects nothing is worse than no gate, and the first version of `pages` did
    select nothing at a locale's top level - a `docs/en/**/*.md` glob skips `docs/en/changelog.md`,
    which is the file this whole check exists for. So the four changelogs are named here rather than
    inferred from a count.
    """
    selected = checker.pages([])
    for locale in checker.LOCALES:
        assert f"docs/{locale}/changelog.md" in selected, selected[:6]
    # A floor, not a snapshot: the count is the one the directory listing yields, and a selection that
    # narrows - back to a glob that misses a locale's top level - drops well below it.
    assert len(selected) >= 160, len(selected)
    assert checker.main([]) == 0, "a published page carries metadata the generator cannot read"


def test_an_empty_selection_fails_rather_than_passing(checker, monkeypatch):
    monkeypatch.setattr(checker, "pages", lambda _argv: [])
    assert checker.main([]) == 1


def test_the_hook_and_the_pr_gate_are_both_wired(checker):
    """Commit time and merge time.

    The prek hook stops the damage entering; the CI job stops it surviving on a path no hook reads -
    the fixer is a hook, so a page it rewrites is caught before it is staged, but a page damaged by
    *another* tool has to be caught by the pipeline that builds the site.
    """
    prek = (REPO_ROOT / "prek.toml").read_text(encoding="utf-8")
    assert 'id = "doc-metadata"' in prek, prek
    assert "python scripts/check_doc_metadata.py" in prek, prek
    workflow = (REPO_ROOT / ".github" / "workflows" / "validate.yml").read_text(encoding="utf-8")
    assert "scripts/check_doc_metadata.py" in workflow, workflow
