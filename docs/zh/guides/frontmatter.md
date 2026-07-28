---

title: Markdown 前端元数据
lang: zh

## Markdown 前端元数据

从 Markdown 文件和字符串中提取 YAML 前端元数据。

### 什么是前端元数据？

前端元数据是 Markdown 文件顶部用 `---` 分隔符包裹的 YAML 块。常用于博客平台、静态网站生成器和内容管理系统。

```markdown
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

从 Markdown 文件解析前端元数据：

```python
import pyyaml_rs

# 返回 (frontmatter_dict, content_string)
frontmatter, content = pyyaml_rs.read_markdown("post.md")

print(frontmatter)
# {'title': '博客文章', 'author': 'Alice', 'date': '2024-01-15', 'tags': ['yaml', 'python', 'rust']}

print(content)
# "# 你好世界\n\n这是内容。\n"
```

### `read_markdown_str()`

从 Markdown 字符串解析前端元数据：

```python
markdown_text = """
---
title: 我的文章
tags: [tech]
---

这里是内容。
"""

frontmatter, content = pyyaml_rs.read_markdown_str(markdown_text)

if frontmatter:
    print(f"标题: {frontmatter['title']}")
    print(f"标签: {frontmatter['tags']}")
    print(f"内容: {content}")
else:
    print("未找到前端元数据")
```

### 没有前端元数据的情况

如果文件/字符串没有前端元数据：

```python
frontmatter, content = pyyaml_rs.read_markdown("no-frontmatter.md")

# frontmatter 为 None，content 为全文
assert frontmatter is None
assert content == "普通 Markdown 内容。"
```

### 常见使用场景

#### 博客平台

```python
# 提取博客列表的元数据
frontmatter, _ = pyyaml_rs.read_markdown("draft.md")
if frontmatter.get("published", False):
    print(f"已发布文章: {frontmatter['title']}")
else:
    print("草稿文章")
```

#### 静态网站生成器

```python
# 处理所有 Markdown 文件
import glob

for path in glob.glob("posts/*.md"):
    meta, content = pyyaml_rs.read_markdown(path)
    # 使用元数据和内容渲染模板
```

#### 内容管理

```python
# 验证前端元数据结构
required_fields = ["title", "author", "date"]
frontmatter, _ = pyyaml_rs.read_markdown("article.md")

for field in required_fields:
    assert field in frontmatter, f"缺少必需字段: {field}"
```
