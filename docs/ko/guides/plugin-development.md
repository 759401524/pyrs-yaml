---
title: 플러그인 개발
description: Community Plugins API로 pyrs-yaml 서드파티 플러그인을 만듭니다.
tags:
  - docs
status: new
---

## 플러그인 개발

이 가이드에서는 Community Plugins API를 사용하여 pyrs-yaml의 서드파티 플러그인을 만드는 방법을 설명합니다.

### 플러그인 구조

플러그인은 `CustomType` 서브클래스를 정의하고 등록하는 Python 모듈입니다.

```python title="my_timestamp_plugin.py"
import pyrs_yaml
from datetime import datetime


class MyTimestampType(pyrs_yaml.CustomType):
    python_type = datetime

    def from_yaml(self, value):
        return datetime.fromisoformat(value)

    def to_yaml(self, obj):
        return obj.isoformat()


def register():
    pyrs_yaml.register_type("!mytimestamp", MyTimestampType())
```

### 통합 TagHandler API

`TagHandler`는 태그 레지스트리(`register_tag`)와 커뮤니티 플러그인(`register_type`)을 단일 등록 지점으로 통합합니다. 핸들러는 스칼라, 컨테이너, 라이프사이클 메서드를 임의 조합으로 구현할 수 있습니다.

```python
class HexInt(pyrs_yaml.TagHandler):
    python_type = int

    def from_yaml(self, value, tag):
        return int(value, 16)

    def to_yaml(self, obj, tag):
        return hex(obj)


pyrs_yaml.register_tag_handler("!hex", HexInt())
```

컨테이너 태그(`from_yaml_mapping`/`from_yaml_sequence`/`to_yaml_mapping`/`to_yaml_sequence`)는 AST 라운드트립 경로에서만 유효. `safe_dump` 고속 경로는 항상 일반 컨테이너를 출력합니다.

### API 참조

| 함수 | 설명 |
|---------|------|
| :material-code-braces: `register_type(name, handler)` | `CustomType` 인스턴스 등록 |
| :material-close: `clear_type_handlers()` | 모든 등록된 타입 제거 |
| :material-close: `remove_type(name)` | 특정 타입 제거 |
| :material-check-decagram: `validate_custom_types(obj)` | 모든 등록된 타입에 대해 객체 검증 |
| :material-link-variant: `register_tag_handler(name, handler)` | 통합 `TagHandler` 등록 |
| :material-close: `clear_tag_handlers()` | 모든 태그 핸들러 제거 |
| :material-close: `remove_tag_handler(name)` | 지정 태그의 핸들러 제거 |
| :material-code-braces: `list_tag_handlers()` | 등록된 태그 핸들러 나열 |
| :material-code-braces: `get_tag_handler(name)` | 태그의 핸들러 인스턴스 조회 |
| :material-export: `serialize(doc, ...)` | 라이프사이클 훅 포함 직렬화 |

### 참고 항목

- [커뮤니티 플러그인](community-plugins.md) — 확장할 수 있는 내장 타입
- [사용자 정의 스키마](custom-schema.md) — 타입 해석 규칙 정의
- [태그 레지스트리 API](../api/reference.md#tag-registry) — `register_tag()` 및 관련 함수
