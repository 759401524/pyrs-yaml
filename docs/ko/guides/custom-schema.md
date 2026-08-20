---
title: 사용자 정의 스키마
description: 사용자 정의 YAML 스키마를 정의하고 사용하여 타입 해석을 제어합니다.
tags:
  - docs
status: new
---

## 사용자 정의 스키마

기본적으로 pyrs-yaml은 YAML 1.2 Core 스키마로 암시적 타입 해석을 수행합니다.
YAML Schema Language를 사용하면 평문 스칼라를 Python 타입으로 해석하는 방식을
제어하는 사용자 정의 스키마를 정의할 수 있습니다.

### 사용자 정의 스키마가 필요한 이유

Core 스키마는 `0xFF`를 `int(255)`로, `2026-08-11`을 `int(2026)`으로,
`hello`를 `"hello"`로 해석합니다. 때로는 다른 동작이 필요합니다:

- 날짜를 문자열로 유지 (`2026` 대신 `"2026-08-11"`)
- 16진수/2진수 리터럴을 정수로 해석
- YAML 1.1 스타일의 불리언 어휘(`yes`/`no`) 추가
- 순수 JSON 서브셋 사용(`inf`, `nan`, `0x` 없음)

### 스키마 정의 형식

스키마는 `rules` 목록을 가진 YAML 파일로 정의합니다. 각 규칙은 `pattern`(정규식)과
`type`(`null`, `bool`, `int`, `float`, `str` 중 하나)을 가집니다.

```yaml
# hex_schema.yaml
name: hex
extends: core
rules:
  - pattern: ^0x[0-9a-fA-F]+$
    type: int
  - pattern: ^0b[01]+$
    type: int
```

### 등록 및 사용

```python title="스키마 등록 및 사용"
import pyrs_yaml

pyrs_yaml.register_schema(
    "hex",
    """
name: hex
extends: core
rules:
  - pattern: ^0x[0-9a-fA-F]+$
    type: int
""",
)

y = pyrs_yaml.YAML(schema="hex")
doc = y.parse("addr: 0xFF")
assert doc.get("addr") == 255

d = pyrs_yaml.safe_load("addr: 0x1F", schema="hex")
assert d["addr"] == 31
```

#### 파일에서 스키마 로드

`load_schema()`는 파일 경로에서 스키마 정의를 읽어 등록합니다:

```python title="파일에서 스키마 로드"
# hex.yaml에 위 스키마 YAML이 포함되어 있음
pyrs_yaml.load_schema("hex", "path/to/hex.yaml")
```

#### 등록된 스키마 나열

`list_schemas()`는 등록된 모든 스키마 이름(내장 + 사용자 정의)을 반환합니다:

```python title="등록된 스키마 나열"
print(pyrs_yaml.list_schemas())
# ['failsafe', 'json', 'core', 'yaml1.1', 'hex', ...]
```

#### 구조 검증

스키마 정의에 `validate` 섹션을 추가하면 스칼라 타입 해석 위에 구조 검사를 추가할 수 있습니다. `validate_against_schema()`로 문서를 사용하기 전에 검증합니다:

```yaml title="검증이 포함된 스키마"
name: app
extends: core
validate:
  - path: $.port
    type: int
    required: true
  - path: $.tags[*]
    type: str
  - path: $.numbers
    sequence_of: int
  - path: $.config
    mapping_of: str
```

```python
import pyrs_yaml

schema = """\
name: app
extends: core
validate:
  - path: $.port
    type: int
    required: true
"""

pyrs_yaml.validate_against_schema("port: 80\n", schema)  # OK
# 모든 실패를 나열하며 YamlValidateError를 발생:
pyrs_yaml.validate_against_schema("port: abc\n", schema)
```

- `path` — JSONPath 형식 위치(`$.key`, `$.a.b`, `$.tags[*]`); 생략 시 모든 스칼라
- `type` — 스칼라가 이 YAML 타입(`null`/`bool`/`int`/`float`/`str`)으로 해석되어야 함
- `sequence_of` / `mapping_of` — 모든 요소 / 값이 지정 타입이어야 함
- `required` — 경로가 존재하고 null이 아니어야 함(`type`과 조합 가능)

### 인라인 Dict 스키마

```python title="인라인 dict 스키마"
d = pyrs_yaml.safe_load(
    "addr: 0xFF",
    schema={
        "extends": "core",
        "rules": [{"pattern": "^0x[0-9a-fA-F]+$", "type": "int"}],
    },
)
assert d["addr"] == 255
```

### 일반적인 패턴

=== "날짜를 문자열로 유지"

    ```python
    schema = {
        "extends": "core",
        "rules": [{"pattern": "^\\d{4}-\\d{2}-\\d{2}$", "type": "str"}],
    }
    ```

=== "YAML 1.1 불리언 추가"

    ```python
    schema = {
        "extends": "core",
        "rules": [{"pattern": "^(yes|no|Yes|No|YES|NO)$", "type": "bool"}],
    }
    ```

=== "엄격 JSON 모드"

    ```python
    schema = {
        "extends": "failsafe",
        "rules": [
            {"pattern": "^null$|^~$", "type": "null"},
            {"pattern": "^(true|false)$", "type": "bool"},
            {"pattern": "^-?\\d+$", "type": "int"},
            {"pattern": "^-?\\d+\\.\\d+$", "type": "float"},
        ],
    }
    ```

### 성능

사용자 정의 스키마는 정규식 기반 규칙 엔진을 사용합니다. 최상의 성능을 위해:

- 규칙 수를 20개 이하로 유지
- 가장 자주 사용되는 패턴을 앞에 배치
- `extends: core`를 사용하여 Core 해석을 재구현하지 않음

내장 Core 스키마는 영향을 받지 않으며, 계속 제로 오버헤드 `match` 디스패치를 사용합니다.

---

### 참고 항목

- [플러그인 개발](plugin-development.md) — 사용자 정의 태그 핸들러 구축
- [i18n 오류 메시지](i18n.md) — 스키마 오류 메시지 지역화
- [스키마 API 참조](../api/reference.md#yaml-schema-language) — `register_schema()` 및 인라인 스키마 문서
