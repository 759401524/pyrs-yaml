---
title: YamlDocument 클래스
description: pyrs-yaml의 핵심 클래스 — 파싱된 YAML 문서, 순환 보존, 편집 메서드, 더더 메서드
tags:
  - docs
status: new
---

## 개요

`YamlDocument`는 pyrs-yaml의 핵심 클래스로, 파싱된 YAML 문서를 보유합니다. `IndexMap` 기반의 사용자 정의 AST를 사용하여 **100% 순환 보존**, **완전한 키 순서 유지**, **중첩 주석 보유**, **상세 메타데이터**를 구현합니다.

```python
class YamlDocument:
    """pyrs-yaml의 핵심 클래스."""

    # ... C 확장으로 구현 ...
```

## 생성자

### `YamlDocument()`

내부 생성자. 사용자가 직접 호출하지 않습니다. `pyrs_yaml.parse()`에서 반환됩니다.

## 프로퍼티

- `version` — YAML 문서 버전
- `schema` — 스키마 (`core`, `failsafe`, `json`)
- `tags` — 태그 목록
- `anchors` — 앵커 목록
- `source` — YAML 소스 텍스트

## 메서드

### `to_yaml()`

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

### `to_yaml_with_options()`

사용자 지정 옵션으로 YAML로 변환합니다.

```python
to_yaml_with_options(
    indent_size: int = 2,
    explicit_start: bool = False,
    explicit_end: bool = False,
    sort_keys: bool = False,
    note_placement: str = "note_first",
) -> str
```

**매개변수:**

- `indent_size` — 들여쓰기 수준당 공백 수 (기본값: 2)
- `explicit_start` — 문서 시작에 `---` 추가 (기본값: False)
- `explicit_end` — 문서 끝에 `...` 추가 (기본값: False)
- `sort_keys` — 키를 알파벳순으로 정렬 (기본값: False)
- `note_placement` — 주석이 자기 줄에 실리지 못하고 바로 위가 `&a` / `!tag` 헤더 줄일 때의 자리: `note_first`(기본)는 헤더 위에 씁니다. 읽기도 거기 두므로 첫 출력에서 수렴합니다. `header_first`는 헤더와 본문 사이에 둡니다. 다른 값은 거부됩니다

**예시:**

```python
yaml_str = doc.to_yaml_with_options(
    indent_size=4,
    explicit_start=True,
    sort_keys=True,
)
```

### `to_dict()`

Python dict/list로 변환합니다. 별칭 참조를 해석하여 네이티브 Python 타입을 반환합니다. 같은 해석은 mapping key에도 똑같이 적용됩니다 — `~: 1`의 key는 `None`, `1: a`의 key는 정수 1. 같은 텍스트라면 `:`의 어느 쪽에 있든 같은 타입이 됩니다.

```python
to_dict() -> dict[Any, Any] | list[Any]
```

**반환값:** 딕셔너리 또는 리스트

**예시:**

```python
doc = pyrs_yaml.parse("key: value")
data = doc.to_dict()  # {'key': 'value'}
```

### `get()`

최상위 매핑 키로 값을 가져옵니다(리터럴 키 조회, `__getitem__`/`__setitem__`과 일관). 키에 `.`, `[`, `]`, `$`이 포함되어도 항상 그대로의 키로 취급되며 경로로 해석되지 않습니다.

```python
get(key: str, default: Any = None) -> Any
```

**반환값:** 값, 못 찾으면 기본값

!!! note "경로 접근"
    JSONPath 스타일 접근은 [`find()`](#find) / [`node()`](#node) /
    [`set()`](#set)을 사용하세요(`$.a.b`, `$.items[-1]` 등).

### `root_type()`

루트 노드 타입을 문자열로 가져옵니다.

```python
type() -> str
```

**반환값:** `"scalar"`, `"mapping"`, `"sequence"`, `"null"` 중 하나

`"alias"`는 별칭 루트에 대해 바인딩이 갖고 있는 철자이지만, 문서를 읽어서 거기에 닿는 일은
없습니다: 별칭은 같은 문서 앞에 자기 앵커가 있어야 하므로 루트 별칭은 파서가 거부합니다.
`pyrs_yaml.parse("*x\n")`는
`YamlParseError: YAML parse error: while parsing node, found unknown anchor at char 0 line 1 column 1`로
답하고, 문서 사이를 넘는 형태인 `"--- &a 1\n--- *a\n"`도 2번째 줄에서 같은 방식으로 거부됩니다.

### `to_json()`

문서를 JSON 문자열로 직렬화합니다.

```python
to_json(indent: int = 2) -> str
```

**반환값:** JSON 문자열

### `validate()`

JSON Schema를 기반으로 문서 내용을 검증합니다.

```python
validate(schema: dict[str, Any]) -> None
```

**발생:**

- `YamlValidateError` — 문서가 스키마에 맞지 않습니다. `str`도 매핑도 아닌 스키마도 여전히 이 클래스로 와서 엔진 자신의 거부 문장을 텍스트로 실습니다: `doc.validate(3)`는 `TypeError: argument of type 'int' is not iterable`로 답합니다

두 거부는 이 클래스 밖에서 옵니다 — 평가가 Python의 `jsonschema` 패키지에 위임되는데, 이 프로젝트가 배포하는 어떤 extra도 그 패키지를 설치하지 않기 때문입니다:

- `jsonschema`를 import할 수 없으면 `doc.validate({"type": "object"})`는 맨 `ModuleNotFoundError: No module named 'jsonschema'`로 답합니다
- `str` 스키마는 평가 엔진 앞에 `json.loads`를 거치므로, YAML로 철자한 스키마 텍스트는 `json.decoder.JSONDecodeError: Expecting value: line 1 column 1 (char 0)`로 답합니다 — `jsonschema`가 설치되어 있든 없든 같습니다

둘 다 재현과 함께 [예외 — YamlValidateError](exceptions.md#yamlvalidateerror)에 기록되어 있습니다.

### `flush_source()`

```python
flush_source() -> None
```

편집이 일어난 뒤 AST를 `source`에 지연 재직렬화하여 현재 트리를 `source`에 반영합니다. 바꿀 때마다 전체 내보내기를 강요하지 않습니다.

### `reparse()`

저장된 소스 텍스트를 제자리에서 재파싱하여 스키마 또는 병합 동작 변경을 허용합니다.

```python
reparse(resolve_merges: bool = True, schema: str | None = None, honor_version_directive: bool | None = None) -> None
```

**매개변수:**

- `resolve_merges` — `<<: *alias` 병합 키를 해석할지 여부 (기본값: `True`)
- `schema` — 타입 해석 스키마: `"core"`, `"json"`, `"failsafe"`, `"yaml1.1"`. `None`이면 문서를 읽을 때 쓴
  스키마를 그대로 유지합니다.
- `honor_version_directive` — 선언된 `%YAML`에 대한 버전 협상. `True`면 `%YAML 1.1`을 선언한 문서를 1.1
  프로파일로 해석하고 지시문도 다시 씁니다. `False`면 선언을 무시하고 고른 스키마를 지킵니다. `None`이면
  문서의 현재 설정을 그대로 둡니다.

**발생:**

- `TypeError` — 저장된 소스 텍스트가 없음
- `YamlParseError` — 재파싱 실패

**예시:**

```python
doc = pyrs_yaml.parse("x: on")
print(doc.get("x"))  # "on" (문자열, core 스키마)

doc.reparse(schema="yaml1.1")
print(doc.get("x"))  # True (bool, yaml1.1 스키마)
```

### `source()`

이 문서를 만드는 데 사용된 원본 YAML 소스 텍스트를 반환합니다.

```python
source_text() -> str
```

**반환값:** YAML 소스 문자열

## 편집 메서드

!!! note "원자적 편집"
    모든 편집 작업은 원자적입니다 — 실패 시 문서(리비전 포함)가 변경되지 않습니다.

문서를 제자리에서 편집하면서 모든 메타데이터(주석, 앵커, 태그, 스타일)를 보존합니다. 편집은 JSONPath 스타일 경로(`$.a.b`, `$.items[0]`)로 노드를 찾으며, 모든 작업은 **원자적**입니다 — 실패 시 문서(리비전 포함)가 변경되지 않습니다.

### `set()`

경로로 값을 교체합니다.

```python
set(path: str, value: Any, create_missing: bool = False) -> None
```

- 스칼라, `dict`, `list`, `tuple` 지원 — `tuple`은 `list`가 만드는 것과 같은 시퀀스 노드입니다
- 기존 스칼라를 교체하면 대상의 메타데이터가 보존됩니다; 경로가 없으면 매핑 끝에 새 키 추가
- 빈 문서(`""`에서 파싱)에 경로를 설정하면 매핑 루트가 자동 생성됩니다
- `create_missing=True`인 경우 누락된 중간 매핑 키가 자동으로 생성됩니다

**예시:**

```python
doc = pyrs_yaml.parse("a:\n  b: 1")
doc.set("$.a.b", 42)
doc.set("$.a.c", True)  # 새 키 추가
doc.set("$", {"x": 1})  # 루트 전체 교체

# 누락된 키 생성
doc.set("$.x.y.z", 3, create_missing=True)
```

**발생:**

- `YamlPathError` — 잘못된 경로 (와일드카드/`..` 거부)
- `YamlEditError` — 적용할 수 없는 경로 단계이며 메시지에 이름이 적혀 있습니다: 별칭을 **통한** 단계는 `edit failed: cannot-edit-alias`, 스칼라로의 단계이거나 모양이 다른 컨테이너에 올라간 단계는 `create-needs-mapping`, `create_missing=True`가 스칼라를 만나면 `cannot-descend-into-scalar:<segment>`, 쓰고 있는 키 **이전**의 키가 누락된 경우 `missing-path:<key>`, 시퀀스를 넘어난 인덱스는 `index-out-of-range-edit:<index>`. 맨 **끝** 키가 없는 것은 거부가 아닙니다 — `parse("a: 1\n").set("$.zz", 3)`은 `a: 1\nzz: 3\n`를 발출합니다 — 살아 있는 앵커의 원천에 쓰는 것도 거부가 아닙니다: `base: &x 1\nuse: *x\n`에서 `set("$.base", 2)`는 `base: &x 2\nuse: *x\n`를 다시 발출하므로 별칭이 아직 찾아가는 자리에 `&x`가 그대로 있습니다
- `YamlTypeError` — 작성자가 모르는 Python 타입의 값; `tuple`은 허용되어 시퀀스로 쓰이고, `bytes`는 `!!binary`로 허용됩니다

호출마다 측정된 어휘 전체는 [편집 이유 토큰](exceptions.md#edit-reason-tokens)에 있습니다.

#### `insert()`

시퀀스의 지정된 인덱스에 값을 삽입합니다.

```python
insert(path: str, index: int, value: Any) -> None
```

`index`는 시퀀스의 현재 길이까지 허용됩니다(`len`에 삽입하면 추가와 동일). 음수 인덱스는 끝에서부터 셉니다(`-1`은 마지막 요소 앞에 삽입). 경로는 시퀀스 노드를 가리켜야 합니다.

**발생:**

- `YamlEditError` — 경로가 매핑이나 스칼라로 해결되면 `edit failed: not-a-sequence`, `len`보다 크거나 `-len`보다 작은 인덱스는 `edit failed: index-out-of-range-edit`, 해석되지 않는 경로는 `edit failed: missing-path:<key>`, 별칭을 지나는 경로는 `edit failed: cannot-edit-alias`, 스칼라 값 아래로의 경로는 `edit failed: cannot-descend-into-scalar:<segment>`

### `append()`

시퀀스 끝에 값을 추가합니다.

```python
append(path: str, value: Any) -> None
```

**발생:** `insert()`와 같은 `YamlEditError` 이유들입니다 — 측정 결과 시퀀스 아닌 대상은 `edit failed: not-a-sequence`, 별칭 부모는 `edit failed: cannot-edit-alias`로 답합니다.

### `delete()`

경로로 노드를 제거합니다. 매핑 순서가 유지됩니다.

```python
delete(path: str, on_duplicate: str = "one") -> None
```

**발생:**

- `YamlEditError` — 문서가 갖고 있지 않은 키는 `edit failed: missing-path`(`del doc["zz"]`도 같은 답입니다), 시퀀스를 벗어난 인덱스는 `edit failed: index-out-of-range-edit`, 노드의 앵커가 아직 참조되고 있으면 `edit failed: anchor-is-referenced`, `on_duplicate="error"`에서 그 키를 적는 짝이 둘 이상이면 `edit failed: duplicate-key-target`, 그 밖의 `on_duplicate` 값은 `edit failed: unknown on_duplicate "<value>"; expected one, all or error`, 별칭을 지나는 경로는 `edit failed: cannot-edit-alias`, 그리고 `delete("$")`는 `edit failed: edit-error` — 루트에는 제거할 세그먼트가 남아 있지 않습니다

### `rename()`

매핑 키를 제자리에서 이름 변경합니다(위치와 메타데이터 보존).

```python
rename(path: str, new_key: str) -> None
```

루트 또는 복합(비스칼라) 키의 이름 변경은 `YamlEditError`를 발생시킵니다.

**발생:**

- `YamlEditError` — `$`에 대해서는 `edit failed: cannot-rename-root`, 복합 키나 시퀀스 요소에 대해서는 `edit failed: cannot-rename-complex-key`, 매핑이 이미 갖고 있는 `new_key`에 대해서는 `edit failed: rename-key-exists`, 해석되지 않는 경로에 대해서는 `edit failed: missing-path`, 별칭을 지나는 경로에 대해서는 `edit failed: cannot-edit-alias`

자기 자신으로의 이름 변경은 거부가 아닙니다: `a: 1`에서 측정하면 `rename("$.a", "a")`는 문서를
`a: 1` 그대로 둡니다.

### `node()`

문서 루트의 `Node`를 반환합니다.

```python
node() -> Node
```

### `walk()`

AST의 깊이 우선, 전위 순회를 수행하며 `Node` 객체를 생성합니다.

```python
walk() -> Generator[Node, None, None]
```

`Node.walk()`와 달리 이 메서드는 **Rust 기반**입니다 — AST를 Python dict로 변환하지 않고 직접 순회하므로 대규모 문서에서 훨씬 빠릅니다.

**생성:** 문서 트리의 모든 노드(루트 포함)에 대한 `Node` 객체.

**예시:**

```python
doc = pyrs_yaml.parse("a:\n  b: 1\n  c: 2\n")
for node in doc.walk():
    print(node._path, node.root_type)
# ()       mapping
# ('a',)   mapping
# ('a', 'b') scalar
# ('a', 'c') scalar
```

### `scalars()`

`walk()`와 유사하지만 스칼라/null 노드만 생성합니다.

```python
scalars() -> Generator[Node, None, None]
```

**생성:** 문서 트리의 모든 스칼라 또는 null 노드에 대한 `Node` 객체.

**예시:**

```python
doc = pyrs_yaml.parse("a: hello\nb: null\n")
for node in doc.scalars():
    print(node._path, node.value)
# ('a',) hello
# ('b',) None
```

### `find()`

경로로 노드를 찾습니다. 와일드카드(`[*]`)와 딥 스캔(`..`)을 지원 — 이 경우 노드 목록을 반환합니다.

```python
find(path: str) -> Node | list[Node]
```

**발생:**

- `YamlPathError` — 경로가 잘못되었거나 편집 경로에 와일드카드/`..` 사용
- `YamlEditError` — 편집을 적용할 수 없음(별칭을 통한 편집, 루트/복합 키 이름 변경, 스칼라로의 탐색, 인덱스 범위 초과)
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

## 더더 메서드

### `__getitem__()`

키 (매핑) 또는 인덱스 (시퀀스)로 접근합니다.

```python
doc = pyrs_yaml.parse("key: value")
value = doc["key"]  # 'value'
```

**발생:**

- `KeyError` — 매핑에서 키를 찾지 못했습니다. 측정된 텍스트는 `Key 'zz' not found`이고, 이것은 내장 `KeyError`이므로 `str(exc)`는 겉따옴표까지 보여 `"Key 'zz' not found"`로 나타납니다
- `IndexError` — 시퀀스에서 인덱스가 범위를 벗어났습니다. `pyrs_yaml.parse("- a\n- b\n")[99]`에서 측정하면 `Index 99 out of range (len=2)`입니다
- `TypeError` — 키 종류가 잘못됐거나 첨자 접근 자체가 불가능한 루트에 대한 `YamlTypeError` 절이며, 각각 측정했습니다: `parse("- a\n")["k"]` → `Index must be an integer`, `parse("a: 1\n")[99]` → `Key must be a string`, `parse("just a scalar")[0]` → `YamlDocument is not subscriptable`

위 텍스트는 모두 `pyrs_yaml.set_language()`를 따릅니다 — `zh-CN`에서 측정하면 네 절에 대해
각각 `索引必须是整数`, `键必须是字符串`, `YamlDocument 不支持下标操作`,
`索引 99 超出范围（长度=2）`가 답합니다. [예외 — i18n 지원](exceptions.md#i18n-support)을
참고하세요.

### `__setitem__()`

루트 매핑 키를 설정합니다(`doc.set()`의 루트 슈가).

```python
doc["key"] = value
```

### `__delitem__()`

루트 매핑 키를 삭제합니다(`doc.delete()`의 루트 슈가).

```python
del doc["key"]
```

### `__contains__()`

키가 존재하는지 확인합니다.

```python
"key" in doc  # True
```

### `__len__()`

항목 수를 가져옵니다.

```python
len(doc)
```

### `__iter__()`

키 (매핑) 또는 값 (시퀀스)을 반복합니다.

```python
for key in doc:
    print(key)
```

### `__repr__()`

디버그 표현.

```python
repr(doc)  # "YamlDocument({key: value})"
```

### `__str__()`

문자열 표현.

```python
str(doc)  # "YamlDocument({key: value})"
```

### `__eq__()`

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

## 사용예

```python
import pyrs_yaml

doc = pyrs_yaml.parse("""
name: Alice
age: 30
""")

print(doc.get("name"))  # Alice
print(doc.root_type())  # mapping
print(len(doc))  # 2
print("name" in doc)  # True
for key in doc:
    print(key, doc[key])  # name Alice, age 30
```

## 자동 생성 API 참조

아래 내용은 `mkdocstrings`가 라이브러리 자체 docstring과 생성된 형식 stub에서 그려 낸 것이므로, 서명은 코드에서 온 것이지 손으로 옮긴 것이 아닙니다.

::: pyrs_yaml.YamlDocument
