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

**등록명의 표기도 계약에 포함됩니다.** `register_type("!x", ...)`는 로컬 태그 `!x`에,
`register_type("!!x", ...)`는 YAML의 표준 태그에 응답합니다. 후자는 두 표기를 모두 받습니다.
`!!x`와 완전 형식 `!<tag:yaml.org,2002:x>`은 YAML 1.2 §6.8에서 같은 태그의 두 표현이기 때문입니다.
`tag:yaml.org,2002:` 밖의 URI는 YAML이 이름을 정하는 영역이 아니므로, 그 태그를 단 스칼라는 태그가
없는 값과 같이 해석됩니다.

### API 참조

| 함수 | 설명 |
|---------|------|
| :material-code-braces: `register_type(name, handler)` | `CustomType` 인스턴스 등록 |
| :material-close: `clear_type_handlers()` | 모든 등록된 타입 제거 |
| :material-close: `remove_type(name)` | 특정 타입 제거 |
| :material-check-decagram: `validate_custom_types(obj)` | 모든 등록된 타입에 대해 객체 검증 |

---

### 참고 항목

- [커뮤니티 플러그인](community-plugins.md) — 확장할 수 있는 내장 타입
- [사용자 정의 스키마](custom-schema.md) — 타입 해석 규칙 정의
- [태그 레지스트리 API](../api/reference.md#tag-registry) — `register_tag()` 및 관련 함수
