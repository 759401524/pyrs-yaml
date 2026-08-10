---
title: Markdown Front Matter
description: Markdown ファイルや文字列から YAML Front Matter を抽出する方法を説明します。
tags:
  - docs
status: new
---

Markdown ファイルや文字列から YAML Front Matterを抽出します。

## Front Matterとは？

Front Matterは、Markdown ファイルの先頭にある `---` 区切りで囲まれた YAML ブロックです。ブログプラットフォーム、静的サイトジェネレーター、コンテンツ管理システムでよく使われます。

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

Markdown ファイルからFront Matterをパースします：

```python
import pyrs_yaml

# (frontmatter_dict, content_string) を返す
frontmatter, content = pyrs_yaml.read_markdown("post.md")

print(frontmatter)
# {'title': 'ブログ記事', 'author': 'Alice', 'date': '2024-01-15', 'tags': ['yaml', 'python', 'rust']}

print(content)
# "# こんにちは\n\nコンテンツです。\n"
```

## `read_markdown_str()`

Markdown 文字列からFront Matterをパースします：

```python
markdown_text = """
---
title: 記事
tags: [tech]
---

コンテンツここ。
"""

frontmatter, content = pyrs_yaml.read_markdown_str(markdown_text)

if frontmatter:
    print(f"タイトル: {frontmatter['title']}")
    print(f"タグ: {frontmatter['tags']}")
    print(f"コンテンツ: {content}")
else:
    print("Front Matterが見つかりません")
```

## Front Matterがない場合

ファイル/文字列にFront Matterがない場合：

```python
frontmatter, content = pyrs_yaml.read_markdown("no-frontmatter.md")

# frontmatter は None、content は全文
assert frontmatter is None
assert content == "通常の Markdown コンテンツ。"
```

## 一般的な使用例

### ブログプラットフォーム

```python
# ブログ一覧用のメタデータを抽出
frontmatter, _ = pyrs_yaml.read_markdown("draft.md")
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
    meta, content = pyrs_yaml.read_markdown(path)
    # meta とコンテンツでテンプレートをレンダリング
```

#### コンテンツ管理

```python
# Front Matterの構造を検証
required_fields = ["title", "author", "date"]
frontmatter, _ = pyrs_yaml.read_markdown("article.md")

for field in required_fields:
    assert field in frontmatter, f"必須フィールドがありません: {field}"
```
