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
| :material-clock-outline: `!timestamp` | `datetime` | ISO 8601 일시 라운드트립 |
| :material-calendar: `!date` | `datetime.date` | ISO 8601 날짜(시간 없음) |
| :material-clock: `!time` | `datetime.time` | ISO 8601 시간(날짜 없음) |
| :material-binary: `!uuid` | `uuid.UUID` | UUID 문자열 ↔ 객체 |
| :material-decimal: `!decimal` | `decimal.Decimal` | 임의 정밀도 십진수 |
| :material-binary: `!binary` | `bytes` | Base64 인코딩 바이너리 데이터 |
| :material-language-python: `!regex` | `re.Pattern` | 컴파일된 정규식 |
| :material-format-list-bulleted: `!set` | `str` | YAML 세트(키 없는 매핑) |

### 사용자 정의 타입 생성

`CustomType`을 상속하고 `from_yaml()`과 `to_yaml()`을 구현합니다:

```python title="CustomType 서브클래스"
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

```python title="명령형 등록"
pyrs_yaml.register_type("!timestamp", TimestampType())
```

**데코레이터 형식:**

```python title="데코레이터 등록"
@pyrs_yaml.register_type("!timestamp")
class TimestampType(pyrs_yaml.CustomType): ...
```

### 사용

**태그 스칼라 로드:**

```python title="태그 스칼라 파싱"
doc = pyrs_yaml.parse("when: !timestamp 2026-08-11T10:30:00")
val = doc.get("when")
assert isinstance(val, datetime)
```

**Python 객체 덤프:**

```python title="Python 객체 덤프"
data = {"ts": datetime(2026, 8, 11, 10, 30)}
out = pyrs_yaml.safe_dump(data)
# out에 포함: ts: !timestamp 2026-08-11T10:30:00
```

### API 참조

| 메서드 | 설명 |
|--------|------|
| :material-function: `can_parse(node)` | 이 타입이 AST 노드를 처리하는지 여부 |
| :material-swap-horizontal: `from_yaml(value)` | YAML 문자열을 Python 객체로 변환 |
| :material-swap-horizontal: `to_yaml(obj)` | Python 객체를 YAML 문자열로 변환 |
| :material-check-decagram: `validate(obj)` | Python 객체 검증(`bool` 반환) |

### 예제: UUID 타입

```python title="uuid_plugin.py"
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

---

### 참고 항목

- [플러그인 개발](plugin-development.md) — 나만의 사용자 정의 타입 구축
- [사용자 정의 스키마](custom-schema.md) — 스칼라 타입 해석 제어
