---

title: 예외
lang: ko

## 예외

pyrs-yaml는 오류 처리를 위해 사용자 정의 예외 클래스를 정의합니다.

### YamlParseError

YAML 파싱이 실패할 때 발생합니다.

```python
class YamlParseError(ValueError):
    """YAML 파싱 오류 (ValueError 상속)."""
```

**상속:** `ValueError`

**예시:**

```python
try:
    doc = pyrs_yaml.parse("invalid: yaml: [")
except pyrs_yaml.YamlParseError as e:
    print(f"파싱 오류: {e}")
```

**오류 메시지 예시:**

- `Invalid YAML: line 1, column 15: did not find expected key`
- `YAML parse error at line 2, column 1: mapping values are not allowed here`

### YamlSerializeError

YAML 직렬화가 실패할 때 발생합니다.

```python
class YamlSerializeError(ValueError):
    """YAML 직렬화 오류 (ValueError 상속)."""
```

**상속:** `ValueError`

**예시:**

```python
try:
    result = pyrs_yaml.safe_dump(float("inf"))
except pyrs_yaml.YamlSerializeError as e:
    print(f"직렬화 오류: {e}")
```

### YamlTypeError

타입 변환 오류가 발생할 때 발생합니다.

```python
class YamlTypeError(TypeError):
    """타입 변환 오류 (TypeError 상속)."""
```

**상속:** `TypeError`

**예시:**

```python
try:
    result = pyrs_yaml.safe_dump(object())  # 변환 불가능한 타입
except pyrs_yaml.YamlTypeError as e:
    print(f"타입 오류: {e}")
```

### YamlValidateError

JSON Schema 검증이 실패할 때 발생합니다.

```python
class YamlValidateError(ValueError):
    """JSON Schema 검증 오류 (ValueError 상속)."""
```

**상속:** `ValueError`

**예시:**

```python
try:
    doc = pyrs_yaml.parse("age: not_a_number")
    doc.validate(schema={"type": "object", "properties": {"age": {"type": "number"}}})
except pyrs_yaml.YamlValidateError as e:
    print(f"검증 오류: {e}")
```

### YamlEditError

제자리 편집을 적용할 수 없을 때 발생합니다: 지원되지 않는 값 타입 (`tuple`), 음수 인덱스, 별칭을 통한 편집, 루트 또는 복합 키 이름 변경, 스칼라로의 탐색, 인덱스 범위 초과.

```python
class YamlEditError(ValueError):
    """제자리 편집 오류 (ValueError 상속)."""
```

**상속:** `ValueError`

**예시:**

```python
doc = pyrs_yaml.parse("a:\n  b: 1")

try:
    doc.set("$.a.b.c", 2)  # 스칼라로의 탐색
except pyrs_yaml.YamlEditError as e:
    print(f"편집 오류: {e}")
```

### YamlPathError

JSONPath 스타일 경로가 잘못되었거나 편집할 수 없을 때 발생합니다: `$`로 시작하지 않는 경로, 편집 작업에서 와일드카드 (`[*]`) 또는 딥 스캔 (`..`) 세그먼트 사용.

```python
class YamlPathError(ValueError):
    """YAML 경로 오류 (ValueError 상속)."""
```

**상속:** `ValueError`

**예시:**

```python
doc = pyrs_yaml.parse("items: [1, 2]")

try:
    doc.set("$.items[*]", 3)  # 와일드카드는 편집 불가
except pyrs_yaml.YamlPathError as e:
    print(f"경로 오류: {e}")
```

### YamlDocumentError

`Node`가 오래된(stale) 상태일 때 발생합니다 — 노드 생성 후 문서가 수정(또는 해제)됨.

```python
class YamlDocumentError(Exception):
    """노드의 부모 YamlDocument가 오래된 경우 발생."""
```

**상속:** `Exception`

**예시:**

```python
node = doc.node().find("$.a")
doc.set("$.b", 2)  # 문서 리비전 증가
node.set_value(99)  # RuntimeWarning + YamlDocumentError
```

### 오류 메시지 형식

모든 오류 메시지에 컨텍스트 정보가 포함됩니다.

| 필드 | 설명 |
|------|------|
| 메시지 | 사람이 읽을 수 있는 오류 설명 |
| Line | 오류가 발생한 줄 번호 |
| Column | 오류가 발생한 열 번호 |
| offset | 줄 내 바이트 오프셋 |

**편집 오류 형식:**

| 오류 | 형식 |
|------|------|
| 편집 실패 | `YAML edit error: <detail>` |
| 경로 오류 | `YAML path error: <detail>` |

### i18n 지원

오류 메시지를 현지화할 수 있습니다:

```python
import pyrs_yaml

pyrs_yaml.set_language("zh-CN")  # 중국어
try:
    pyrs_yaml.parse("invalid: yaml: [")
except pyrs_yaml.YamlParseError as e:
    print(e)  # 중국어 오류 메시지
```

### 모범 사례

```python
# 구체적인 예외 캡처
try:
    doc = pyrs_yaml.parse(yaml_content)
except pyrs_yaml.YamlParseError as e:
    logger.error(f"YAML 파싱 오류: {e}")
    # 오류 메시지 파싱
    error_str = str(e)  # "Invalid YAML: line 1, column 15: ..."
except pyrs_yaml.YamlTypeError as e:
    logger.error(f"타입 오류: {e}")
```

**참고:** 모든 사용자 정의 예외는 `ValueError`를 상속하므로 `except ValueError`로 일괄 캡처할 수 있습니다. 하지만 더 세밀한 오류 처리를 위해서는 구체적인 예외 클래스를 사용하는 것이 좋습니다.
