---
title: Markdown Frontmatter
description: Extract YAML frontmatter from Markdown files and strings, with use cases for blog platforms and static site generators.
tags:
  - docs
status: new
---

## Markdown Frontmatter

Extract YAML frontmatter from Markdown files and strings.

### What is Frontmatter?

Frontmatter is a YAML block at the top of Markdown files, wrapped between `---` delimiters. Commonly used in blog platforms, static site generators, and content management systems.

```markdown title="post.md"
---
title: My Blog Post
author: Alice
date: 2024-01-15
tags: [yaml, python, rust]
---

# Hello World

This is the content.
```

### read_markdown()

Parse frontmatter from a Markdown file:

```python title="Parse from a file"
import pyrs_yaml

# Returns (frontmatter_dict, content_string)
frontmatter, content = pyrs_yaml.read_markdown("post.md")

print(frontmatter)
# {'title': 'My Blog Post', 'author': 'Alice', 'date': '2024-01-15', 'tags': ['yaml', 'python', 'rust']}

print(content)
# "# Hello World\n\nThis is the content.\n"
```

### read_markdown_str()

Parse frontmatter from a Markdown string:

```python title="Parse from a string"
markdown_text = """
---
title: My Post
tags: [tech]
---

Content here.
"""

frontmatter, content = pyrs_yaml.read_markdown_str(markdown_text)

if frontmatter:
    print(f"Title: {frontmatter['title']}")
    print(f"Tags: {frontmatter['tags']}")
    print(f"Content: {content}")
else:
    print("No frontmatter found")
```

### No Frontmatter

If the file/string has no frontmatter:

```python title="No frontmatter"
frontmatter, content = pyrs_yaml.read_markdown("no-frontmatter.md")

# frontmatter is None, content is the full text
assert frontmatter is None
assert content == "Just regular markdown content."
```

### Common Use Cases

=== "Blog Platforms"

    ```python title="Extract metadata for blog listing"
    # Extract metadata for blog listing
    frontmatter, _ = pyrs_yaml.read_markdown("draft.md")
    if frontmatter.get("published", False):
        print(f"Published post: {frontmatter['title']}")
    else:
        print("Draft post")
    ```

=== "Static Site Generators"

    ```python title="Process all markdown files"
    # Process all markdown files
    import glob

    for path in glob.glob("posts/*.md"):
        meta, content = pyrs_yaml.read_markdown(path)
        # Render template with meta and content
    ```

=== "Content Management"

    ```python title="Validate frontmatter structure"
    # Validate frontmatter structure
    required_fields = ["title", "author", "date"]
    frontmatter, _ = pyrs_yaml.read_markdown("article.md")

    for field in required_fields:
        assert field in frontmatter, f"Missing required field: {field}"
    ```
