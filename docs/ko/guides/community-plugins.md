---
title: 커뮤니티 플러그인
description: Community Plugins API로 pyrs-yaml을 확장하여 사용자 정의 노드 타입을 등록합니다.
tags:
  - docs
status: new
---

## 커뮤니티 플러그인

Community Plugins API를 사용하면 사용자 정의 YAML 노드 타입을 정의하여 pyrs-yaml의
직렬화 및 역직렬화와 통합할 수 있습니다. 사용자 정의 타입은 YAML 태그 스칼라와
임의의 Python 객체를 상호 변환할 수 있습니다.

### 내장 플러그인

pyrs-yaml에는 임포트 시 자동 등록되는 내장 플러그인이 포함되어 있습니다:

| 태그 | Python 타입 | 설명 |
|-----|-------------|------|
| `!timestamp` | `datetime` | ISO 8601 타임스탬프 라운드트립 |
| `!set` | `str` | YAML 세트(키 없는 매핑) |

### 사용자 정의 타입 생성

`CustomType`을 상속하고 `from_yaml()`과 `to_yaml()`을 구현합니다:

```python
import pyrs_yaml
from datetime import datetime


class TimestampType(pyrs_yaml.CustomType):
    python_type = datetime

    def from_yaml(self, value: str):
        return datetime.fromisoformat(value)

    def to_yaml(self, obj) -> str:
        return obj.isoformat()
```

### 등록

**명령형:**

```python
pyrs_yaml.register_type("!timestamp", TimestampType())
```

**데코레이터 형식:**

```python
@pyrs_yaml.register_type("!timestamp")
class TimestampType(pyrs_yaml.CustomType):
    ...
```

### 사용

**태그 스칼라 로드:**

```python
doc = pyrs_yaml.parse("when: !timestamp 2026-08-11T10:30:00")
val = doc.get("when")
assert isinstance(val, datetime)
```

**Python 객체 덤프:**

```python
data = {"ts": datetime(2026, 8, 11, 10, 30)}
out = pyrs_yaml.safe_dump(data)
# out에 포함: ts: !timestamp 2026-08-11T10:30:00
```

### API 참조

| 메서드 | 설명 |
|--------|------|
| `can_parse(node)` | 이 타입이 AST 노드를 처리하는지 여부 |
| `from_yaml(value)` | YAML 문자열을 Python 객체로 변환 |
| `to_yaml(obj)` | Python 객체를 YAML 문자열로 변환 |
| `validate(obj)` | Python 객체 검증(`bool` 반환) |

### 예제: UUID 타입

```python
import uuid
import pyrs_yaml


class UUIDType(pyrs_yaml.CustomType):
    python_type = uuid.UUID

    def from_yaml(self, value):
        return uuid.UUID(value)

    def to_yaml(self, obj):
        return str(obj)


pyrs_yaml.register_type("!uuid", UUIDType())

doc = pyrs_yaml.parse("id: !uuid 550e8400-e29b-41d4-a716-446655440000")
assert isinstance(doc.get("id"), uuid.UUID)
```
