---

title: YamlDocument 클래스
lang: ko

## YamlDocument 클래스

### 개요

`YamlDocument`는 pyrs-yaml의 핵심 클래스로, 파싱된 YAML 문서를 보유합니다. `IndexMap` 기반의 사용자 정의 AST를 사용하여 **100% 순환 보존**, **완전한 키 순서 유지**, **중첩 주석 보유**, **상세 메타데이터**를 구현합니다.

```python
class YamlDocument:
    """pyrs-yaml의 핵심 클래스."""

    # ... C 확장으로 구현 ...
```

### 생성자

#### `YamlDocument()`

내부 생성자. 사용자가 직접 호출하지 않습니다. `pyrs_yaml.parse()`에서 반환됩니다.

### 프로퍼티

- `version` — YAML 문서 버전
- `schema` — 스키마 (`core`, `failsafe`, `json`)
- `tags` — 태그 목록
- `anchors` — 앵커 목록
- `source` — YAML 소스 텍스트

### 메서드

#### `to_yaml()`

문서를 YAML 문자열로 변환합니다.

```python
to_yaml(
    indent: int = 2,
    allow_unicode: bool = True,
    default_flow_style: bool = False,
    sort_keys: bool = False,
    width: int = 80,
    resolve_aliases: bool = True,
    strip_comments: bool = False,
    preserve_quotes: bool = True,
) -> str
```

**매개변수:**

- `indent` — 들여쓰기 공백 수 (기본값: 2)
- `allow_unicode` — Unicode 문자 허용 (기본값: True)
- `default_flow_style` — 기본 플로우 스타일 사용 (기본값: False)
- `sort_keys` — 키 정렬 (기본값: False)
- `width` — 줄 바꿈 너비 (기본값: 80)
- `resolve_aliases` — 별칭 해석 (기본값: True)
- `strip_comments` — 주석 제거 (기본값: False)
- `preserve_quotes` — 따옴표 유지 (기본값: True)

**반환값:** YAML 문자열

**예시:**

```python
doc = pyrs_yaml.parse("key: value\n# comment")
yaml_str = doc.to_yaml()
```

#### `to_dict()`

Python dict/list로 변환합니다. 별칭 참조를 해석하여 네이티브 Python 타입을 반환합니다.

```python
to_dict() -> dict[str, Any] | list[Any]
```

**반환값:** 딕셔너리 또는 리스트

**예시:**

```python
doc = pyrs_yaml.parse("key: value")
data = doc.to_dict()  # {'key': 'value'}
```

#### `get()`

키로 값을 가져옵니다 (매핑 루트용).

```python
get(key: str, default: Any = None) -> Any
```

**반환값:** 값, 못 찾으면 기본값

#### `type()`

루트 노드 타입을 문자열로 가져옵니다.

```python
type() -> str
```

**반환값:** 타입 이름 (`"mapping"`, `"sequence"`, `"scalar"`)

#### `to_json()`

문서를 JSON 문자열로 직렬화합니다.

```python
to_json(indent: int = 2) -> str
```

**반환값:** JSON 문자열

#### `validate()`

JSON Schema를 기반으로 문서 내용을 검증합니다.

```python
validate(schema: dict[str, Any]) -> None
```

**발생:** `YamlValidateError` — 검증 오류

#### `reload()`

저장된 소스 텍스트를 제자리에서 재파싱하여 스키마 또는 병합 동작 변경을 허용합니다.

```python
reload(schema: str = "core", resolve_merges: bool = True) -> None
```

#### `source_text()`

이 문서를 만드는 데 사용된 원본 YAML 소스 텍스트를 반환합니다.

```python
source_text() -> str
```

**반환값:** YAML 소스 문자열

### 편집 메서드

문서를 제자리에서 편집하면서 모든 메타데이터(주석, 앵커, 태그, 스타일)를 보존합니다. 편집은 JSONPath 스타일 경로(`$.a.b`, `$.items[0]`)로 노드를 찾으며, 모든 작업은 **원자적**입니다 — 실패 시 문서(리비전 포함)가 변경되지 않습니다.

#### `set()`

경로로 값을 교체합니다.

```python
set(path: str, value: Any) -> None
```

- 스칼라, `dict`, `list` 지원; `tuple`은 지원되지 않음(`YamlEditError` 발생)
- 기존 스칼라를 교체하면 대상의 메타데이터가 보존됩니다; 경로가 없으면 매핑 끝에 새 키 추가

**예시:**

```python
doc = pyrs_yaml.parse("a:\n  b: 1")
doc.set("$.a.b", 42)
doc.set("$.a.c", True)  # 새 키 추가
doc.set("$", {"x": 1})  # 루트 전체 교체
```

#### `insert()`

시퀀스의 지정된 인덱스에 값을 삽입합니다.

```python
insert(path: str, index: int, value: Any) -> None
```

`index`는 시퀀스의 현재 길이까지 허용됩니다(`len`에 삽입하면 추가와 동일). 경로는 시퀀스 노드를 가리켜야 합니다.

#### `append()`

시퀀스 끝에 값을 추가합니다.

```python
append(path: str, value: Any) -> None
```

#### `delete()`

경로로 노드를 제거합니다. 매핑 순서가 유지됩니다.

```python
delete(path: str) -> None
```

#### `rename()`

매핑 키를 제자리에서 이름 변경합니다(위치와 메타데이터 보존).

```python
rename(path: str, new_key: str) -> None
```

루트 또는 복합(비스칼라) 키의 이름 변경은 `YamlEditError`를 발생시킵니다.

#### `node()`

문서 루트의 `Node`를 반환합니다.

```python
node() -> Node
```

#### `find()`

경로로 노드를 찾습니다. 와일드카드(`[*]`)와 딥 스캔(`..`)을 지원 — 이 경우 노드 목록을 반환합니다.

```python
find(path: str) -> Node | list[Node]
```

**발생:**

- `YamlPathError` — 경로가 잘못되었거나 편집 경로에 와일드카드/`..` 사용
- `YamlEditError` — 편집을 적용할 수 없음(`tuple`, 음수 인덱스, 별칭을 통한 편집, 루트/복합 키 이름 변경, 스칼라로의 탐색, 인덱스 범위 초과)
- `YamlDocumentError` — 문서 편집 후 오래된 `Node` 사용

**참조:** [제자리 편집 가이드](../guides/editing.md)

**예시:**

```python
doc = pyrs_yaml.parse("items: [1, 2, 3]")
doc.set("$.items[1]", "two")
doc.insert("$.items", 1, "x")  # items: [1, x, 2, 3]
doc.append("$.items", 4)
doc.rename("$.items", "list")  # 매핑 키 이름 변경
del doc["list"]  # doc.delete("$.list")와 동일
```

### 더더 메서드

#### `__getitem__()`

키 (매핑) 또는 인덱스 (시퀀스)로 접근합니다.

```python
doc = pyrs_yaml.parse("key: value")
value = doc["key"]  # 'value'
```

#### `__setitem__()`

루트 매핑 키를 설정합니다(`doc.set()`의 루트 슈가).

```python
doc["key"] = value
```

#### `__delitem__()`

루트 매핑 키를 삭제합니다(`doc.delete()`의 루트 슈가).

```python
del doc["key"]
```

#### `__contains__()`

키가 존재하는지 확인합니다.

```python
"key" in doc  # True
```

#### `__len__()`

항목 수를 가져옵니다.

```python
len(doc)
```

#### `__iter__()`

키 (매핑) 또는 값 (시퀀스)을 반복합니다.

```python
for key in doc:
    print(key)
```

#### `__repr__()`

디버그 표현.

```python
repr(doc)  # "YamlDocument({key: value})"
```

#### `__str__()`

문자열 표현.

```python
str(doc)  # "YamlDocument({key: value})"
```

#### `__eq__()`

동등 비교. 두 `YamlDocument`가 동일한 내용을 가지면 true를 반환합니다.

```python
doc1 == doc2  # True or False
```

**예시:**

```python
import pyrs_yaml

# 매핑
doc = pyrs_yaml.parse("name: Alice\nage: 30")
print(doc["name"])  # Alice
print(len(doc))  # 2

# 시퀀스
doc = pyrs_yaml.parse("- item1\n- item2")
print(doc[0])  # item1

# 중첩 접근
doc = pyrs_yaml.parse("user:\n  name: Alice")
print(doc["user"]["name"])  # Alice
```
