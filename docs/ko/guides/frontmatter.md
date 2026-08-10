---
title: Markdown Front Matter
description: Markdown 파일과 문자열에서 YAML Front Matter를 추출하는 read_markdown 및 read_markdown_str 함수
tags:
  - docs
status: new
---

Markdown 파일과 문자열에서 YAML Front Matter를 추출합니다.

## Front Matter란?

Front Matter는 Markdown 파일 상단에 `---` 구분자로 감싼 YAML 블록입니다. 블로그 플랫폼, 정적 사이트 생성기, 콘텐츠 관리 시스템에서 흔히 사용됩니다.

```markdown
---

title: 블로그 포스트
author: Alice
date: 2024-01-15
tags: [yaml, python, rust]
---

# 안녕하세요

콘텐츠입니다.
```

## `read_markdown()`

Markdown 파일에서 Front Matter를 파싱합니다:

```python
import pyrs_yaml

# (frontmatter_dict, content_string) 반환
frontmatter, content = pyrs_yaml.read_markdown("post.md")

print(frontmatter)
# {'title': '블로그 포스트', 'author': 'Alice', 'date': '2024-01-15', 'tags': ['yaml', 'python', 'rust']}

print(content)
# "# 안녕하세요\n\n콘텐츠입니다.\n"
```

## `read_markdown_str()`

Markdown 문자열에서 Front Matter를 파싱합니다:

```python
markdown_text = """
---
title: 내 포스트
tags: [tech]
---

여기에 콘텐츠.
"""

frontmatter, content = pyrs_yaml.read_markdown_str(markdown_text)

if frontmatter:
    print(f"제목: {frontmatter['title']}")
    print(f"태그: {frontmatter['tags']}")
    print(f"콘텐츠: {content}")
else:
    print("Front Matter를 찾을 수 없습니다")
```

## Front Matter가 없는 경우

파일/문자열에 Front Matter가 없으면:

```python
frontmatter, content = pyrs_yaml.read_markdown("no-frontmatter.md")

# frontmatter는 None, content는 전체 텍스트
assert frontmatter is None
assert content == "일반 Markdown 콘텐츠."
```

## 일반적인 사용 예시

### 블로그 플랫폼

```python
# 블로그 목록용 메타데이터 추출
frontmatter, _ = pyrs_yaml.read_markdown("draft.md")
if frontmatter.get("published", False):
    print(f"발행된 포스트: {frontmatter['title']}")
else:
    print("초안 포스트")
```

#### 정적 사이트 생성기

```python
# 모든 Markdown 파일 처리
import glob

for path in glob.glob("posts/*.md"):
    meta, content = pyrs_yaml.read_markdown(path)
    # 메타데이터와 콘텐츠로 템플릿 렌더링
```

#### 콘텐츠 관리

```python
# Front Matter 구조 검증
required_fields = ["title", "author", "date"]
frontmatter, _ = pyrs_yaml.read_markdown("article.md")

for field in required_fields:
    assert field in frontmatter, f"필수 필드 누락: {field}"
```
