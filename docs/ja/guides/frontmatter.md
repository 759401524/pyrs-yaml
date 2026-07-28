---

title: Markdown フロントメータ
lang: ja

## Markdown フロントメータ

Markdown ファイルや文字列から YAML フロントメータを抽出します。

### フロントメータとは？

フロントメータは、Markdown ファイルの先頭にある `---` 区切りで囲まれた YAML ブロックです。ブログプラットフォーム、静的サイトジェネレーター、コンテンツ管理システムでよく使われます。

```markdown
---

title: ブログ記事
author: Alice
date: 2024-01-15
tags: [yaml, python, rust]
---

# こんにちは

コンテンツです。
```

## `read_markdown()`

Markdown ファイルからフロントメータをパースします：

```python
import pyyaml_rs

# (frontmatter_dict, content_string) を返す
frontmatter, content = pyyaml_rs.read_markdown("post.md")

print(frontmatter)
# {'title': 'ブログ記事', 'author': 'Alice', 'date': '2024-01-15', 'tags': ['yaml', 'python', 'rust']}

print(content)
# "# こんにちは\n\nコンテンツです。\n"
```

### `read_markdown_str()`

Markdown 文字列からフロントメータをパースします：

```python
markdown_text = """
---
title: 記事
tags: [tech]
---

コンテンツここ。
"""

frontmatter, content = pyyaml_rs.read_markdown_str(markdown_text)

if frontmatter:
    print(f"タイトル: {frontmatter['title']}")
    print(f"タグ: {frontmatter['tags']}")
    print(f"コンテンツ: {content}")
else:
    print("フロントメータが見つかりません")
```

### フロントメータがない場合

ファイル/文字列にフロントメータがない場合：

```python
frontmatter, content = pyyaml_rs.read_markdown("no-frontmatter.md")

# frontmatter は None、content は全文
assert frontmatter is None
assert content == "通常の Markdown コンテンツ。"
```

### 一般的な使用例

#### ブログプラットフォーム

```python
# ブログ一覧用のメタデータを抽出
frontmatter, _ = pyyaml_rs.read_markdown("draft.md")
if frontmatter.get("published", False):
    print(f"公開済み記事: {frontmatter['title']}")
else:
    print("下書き記事")
```

#### 静的サイトジェネレーター

```python
# すべての Markdown ファイルを処理
import glob

for path in glob.glob("posts/*.md"):
    meta, content = pyyaml_rs.read_markdown(path)
    # meta とコンテンツでテンプレートをレンダリング
```

#### コンテンツ管理

```python
# フロントメータの構造を検証
required_fields = ["title", "author", "date"]
frontmatter, _ = pyyaml_rs.read_markdown("article.md")

for field in required_fields:
    assert field in frontmatter, f"必須フィールドがありません: {field}"
```
