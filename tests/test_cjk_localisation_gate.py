"""Self-test for the localized-script purity gate (`scripts/check_cjk_localisation.py`).

The gate exists because assistant-drafted translations leak glyphs from a neighbour
script, and a reader of that locale usually cannot see it. Until now the gate had no
test of its own, which is exactly how it stayed blind: the Korean page was policed by
a hand-curated list of thirteen simplified-only codepoints, so `経路` (a Japanese
shinjitai form) and `행內` (a traditional form) both passed while the page was wrong.
Every case below is a shape the previous version accepted.

Fixtures are written into a temporary repo so the rules are exercised in isolation;
one test then runs the gate over the real tree, which is what makes the shipped docs
an assertion rather than a hope.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
GATE_PATH = REPO_ROOT / "scripts" / "check_cjk_localisation.py"


def _load_gate():
    spec = importlib.util.spec_from_file_location("check_cjk_localisation", GATE_PATH)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    # @dataclass resolves its owning module through sys.modules at class-creation
    # time, so a module that was never registered fails to import at all.
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


gate = pytest.fixture(scope="module")(_load_gate)


def make_tree(root: Path, pages: dict[str, str]) -> Path:
    for relative, text in pages.items():
        page = root / relative
        page.parent.mkdir(parents=True, exist_ok=True)
        page.write_text(text, encoding="utf-8")
    return root


def findings_for(gate, root: Path, paths=None) -> list[str]:
    return gate.scan(root, paths)[0]


def test_korean_page_rejects_a_japanese_kanji_form(gate, tmp_path: Path) -> None:
    """`経` is Japanese shinjitai, not simplified-only Han, so the old list missed it."""
    root = make_tree(tmp_path, {"docs/ko/changelog.md": "# Korean\n\n이 経路 는 고정됩니다.\n"})
    hits = findings_for(gate, root)
    assert len(hits) == 1, hits
    assert "Han in a Hangul-only page" in hits[0]
    assert "経(U+7D4C)" in hits[0]


def test_korean_page_rejects_a_traditional_chinese_form(gate, tmp_path: Path) -> None:
    """`內` is the traditional twin of `내`; it is Korean text only as Hangul."""
    root = make_tree(tmp_path, {"docs/ko/changelog.md": "# Korean\n\n주석은 행內 에 남는다.\n"})
    hits = findings_for(gate, root)
    assert len(hits) == 1, hits
    assert "Han in a Hangul-only page" in hits[0]


def test_korean_page_rejects_a_whole_chinese_clause(gate, tmp_path: Path) -> None:
    """The defect class actually shipped: Korean scaffolding around Chinese prose."""
    root = make_tree(
        tmp_path,
        {"docs/ko/changelog.md": "# Korean\n\n- 热点 样本로 指定。以前 计量 만 했습니다.\n"},
    )
    hits = findings_for(gate, root)
    assert len(hits) == 1, hits
    assert "热(U+70ED)" in hits[0]


def test_korean_page_still_rejects_kana(gate, tmp_path: Path) -> None:
    root = make_tree(tmp_path, {"docs/ko/changelog.md": "# Korean\n\n이 경로는 安定します.\n"})
    kinds = "\n".join(findings_for(gate, root))
    assert "kana in a non-Japanese page" in kinds
    assert "Han in a Hangul-only page" in kinds


def test_han_inside_technical_text_is_not_a_violation(gate, tmp_path: Path) -> None:
    """Korean i18n docs must be able to show a Chinese sample without tripping the gate.

    Both exemption shapes matter: a fenced block and an inline span, and the inline
    span matters on a line that also carries prose, because a line-level check would
    report that line and point at nothing.
    """
    text = """# Korean

```yaml title="frontmatter 예시"
title: 文档标题
lang: zh-CN
```

`{ é: 1, 名: 2 }` 키를 받아들인다.

링크는 [중국어 페이지](/pyrs-yaml/zh/docs/标题) 를 가리킨다.
"""
    root = make_tree(tmp_path, {"docs/ko/guides/i18n.md": text})
    assert findings_for(gate, root) == []


def test_unterminated_fence_does_not_leak_code_as_prose(gate, tmp_path: Path) -> None:
    """A fence that never closes suppresses the rest of the file, conservatively."""
    root = make_tree(tmp_path, {"docs/ko/changelog.md": "# Korean\n\n```text\n标题\n"})
    assert findings_for(gate, root) == []


def test_chinese_page_rejects_kana_and_hangul(gate, tmp_path: Path) -> None:
    root = make_tree(
        tmp_path,
        {"docs/zh/changelog.md": "# 中文\n\n- 键的注释留在 行内。\n- 仮名が混じる。\n- 한국어도混入。\n"},
    )
    kinds = "\n".join(findings_for(gate, root))
    assert "kana in a non-Japanese page" in kinds
    assert "Hangul in a non-Korean page" in kinds
    assert "行内" not in kinds  # Han is the Chinese page's own script


def test_japanese_page_rejects_hangul_and_simplified_han(gate, tmp_path: Path) -> None:
    root = make_tree(
        tmp_path,
        {"docs/ja/changelog.md": "# 日本語\n\n注釈は行内に残る。\n- 한국語\n- 折叠 の規則。\n"},
    )
    kinds = "\n".join(findings_for(gate, root))
    assert "Hangul in a non-Korean page" in kinds
    assert "simplified-only Han" in kinds


def test_japanese_page_keeps_its_own_kanji(gate, tmp_path: Path) -> None:
    root = make_tree(tmp_path, {"docs/ja/changelog.md": "# 日本語\n\n畳み込みと往復契約の不動点。\n"})
    assert findings_for(gate, root) == []


def test_paths_filter_scopes_the_run(gate, tmp_path: Path) -> None:
    """prek passes changed filenames; a Korean defect must not surface for a ja run."""
    root = make_tree(
        tmp_path,
        {
            "docs/ko/changelog.md": "# Korean\n\n이 経路 는 고정됩니다.\n",
            "docs/ja/changelog.md": "# 日本語\n\n- 折叠 の規則。\n",
        },
    )
    hits = findings_for(gate, root, ["docs/ja/changelog.md"])
    assert len(hits) == 1, hits
    assert hits[0].startswith("docs/ja/changelog.md:")


def test_unmatched_paths_are_a_no_op_not_a_failure(gate, tmp_path: Path) -> None:
    """The hook fires on `docs/**`; a page outside a policed locale is not its business."""
    make_tree(tmp_path, {"docs/en/changelog.md": "# English\n\nNothing here.\n"})
    assert gate.main(["docs/en/changelog.md"], root=tmp_path) == 0


def test_a_gate_that_scans_nothing_fails(gate, tmp_path: Path) -> None:
    """Renamed locale dirs used to turn the guard into a green no-op."""
    make_tree(tmp_path, {"documentation/ko/changelog.md": "# Korean\n\n이 経路 는.\n"})
    assert gate.main([], root=tmp_path) == 1


def test_the_shipped_docs_pass_the_gate(gate) -> None:
    """Dogfood: the repository's own localized pages satisfy the rule they enforce."""
    assert gate.main([], root=REPO_ROOT) == 0
