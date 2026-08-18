---
title: Markdown Front Matter
description: 使用 pyrs-yaml 从 Markdown 文件和字符串中提取 YAML Front Matter 的指南。
tags:
  - docs
status: new
---

从 Markdown 文件和字符串中提取 YAML Front Matter。

## 什么是Front Matter？

Front Matter是 Markdown 文件顶部用 `---` 分隔符包裹的 YAML 块。常用于博客平台、静态网站生成器和内容管理系统。

```markdown title="post.md"
---
title: 博客文章
author: Alice
date: 2024-01-15
tags: [yaml, python, rust]
---

# 你好世界

这是内容。
```

## `read_markdown()`

从 Markdown 文件解析Front Matter：

```python title="从文件解析"
import pyrs_yaml

# 返回 (frontmatter_dict, content_string)
frontmatter, content = pyrs_yaml.read_markdown("post.md")

print(frontmatter)
# {'title': '博客文章', 'author': 'Alice', 'date': '2024-01-15', 'tags': ['yaml', 'python', 'rust']}

print(content)
# "# 你好世界\n\n这是内容。\n"
```

## `read_markdown_str()`

从 Markdown 字符串解析Front Matter：

```python title="从字符串解析"
markdown_text = """
---
title: 我的文章
tags: [tech]
---

这里是内容。
"""

frontmatter, content = pyrs_yaml.read_markdown_str(markdown_text)

if frontmatter:
    print(f"标题: {frontmatter['title']}")
    print(f"标签: {frontmatter['tags']}")
    print(f"内容: {content}")
else:
    print("未找到Front Matter")
```

## 没有Front Matter的情况

如果文件/字符串没有Front Matter：

```python title="无 Front Matter"
frontmatter, content = pyrs_yaml.read_markdown("no-frontmatter.md")

# frontmatter 为 None，content 为全文
assert frontmatter is None
assert content == "普通 Markdown 内容。"
```

## 常见使用场景

=== "博客平台"

    ```python title="提取博客列表的元数据"
    # 提取博客列表的元数据
    frontmatter, _ = pyrs_yaml.read_markdown("draft.md")
    if frontmatter.get("published", False):
        print(f"已发布文章: {frontmatter['title']}")
    else:
        print("草稿文章")
    ```

=== "静态网站生成器"

    ```python title="处理所有 Markdown 文件"
    # 处理所有 Markdown 文件
    import glob

    for path in glob.glob("posts/*.md"):
        meta, content = pyrs_yaml.read_markdown(path)
        # 使用元数据和内容渲染模板
    ```

=== "内容管理"

    ```python title="验证 Front Matter 结构"
    # 验证Front Matter结构
    required_fields = ["title", "author", "date"]
    frontmatter, _ = pyrs_yaml.read_markdown("article.md")

    for field in required_fields:
        assert field in frontmatter, f"缺少必需字段: {field}"
    ```
