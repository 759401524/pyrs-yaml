---
title: MergedView 클래스
description: pyrs-yaml의 MergedView 클래스 — 병합 키가 해석된 읽기 전용 dict-like 뷰
tags:
  - docs
status: new
---

## MergedView 클래스

`MergedView` 클래스는 병합 키(`<<: *anchor`)가 해석된 `YamlDocument`의 읽기 전용 뷰를 제공합니다. `doc.merged()`를 통해 접근합니다.

### Overview

```python
class MergedView(Mapping):
    """Read-only view of a YAML document with merge keys resolved."""
```

이 뷰는 `YamlDocument.to_dict()`에서 지연(lazy) 방식으로 구성되며, 직렬화 중에 앵커와 병합 키를 해석합니다. 원본 AST는 절대 변경되지 않습니다.

### Constructor

#### `MergedView.__init__()`

```python
MergedView.__init__(document: YamlDocument) -> None
```

**매개변수:**

- `document` — `YamlDocument` 인스턴스

문서 루트가 시퀀스인 경우, 뷰는 이를 정수 키 매핑(`{0: item0, 1: item1, ...}`)으로 변환합니다.

### Methods

#### `__getitem__()`

키로 값을 접근합니다.

```python
__getitem__(key: str | int) -> Any
```

자식 dict와 list는 각각 `MergedView._DictView`와 `MergedView._ListView`로 재귀적으로 감싸집니다.

**예제:**

```python
doc = pyrs_yaml.parse("""
defaults: &defaults
  timeout: 30
  retries: 3

config:
  <<: *defaults
  timeout: 60
""")

view = doc.merged()
print(view["config"]["timeout"])  # 60 (overrides merged value)
print(view["config"]["retries"])  # 3 (inherited from merge)
```

#### `__len__()`

최상위 항목의 개수를 반환합니다.

```python
__len__() -> int
```

#### `__iter__()`

최상위 키를 순회합니다.

```python
__iter__() -> Iterator[str | int]
```

#### `__repr__()`

```python
__repr__() -> str
```

내부 dict 표현을 사용해 `MergedView({...})`를 반환합니다.

#### `get()`

`get()`은 `collections.abc.Mapping`에서 상속됩니다 — `get(key, default=None)`을 제공합니다.

```python
get(key: str | int, default: Any = None) -> Any
```

### Merge Key Resolution

키는 다음 우선순위로 해석됩니다(높을수록 우선):

1. 병합하는 문서에서 직접 정의된 키
2. 병합된 앵커에서 온 키(`<<:`에 나타난 순서)
3. 나중의 앵커가 앞선 앵커를 덮어씁니다

### Root Type Support

| 루트 타입 | 동작 |
| --- | --- |
| Mapping | 키는 매핑의 키입니다 |
| Sequence | 키는 정수 인덱스입니다(`0`, `1`, ...) |
| Scalar/Null | `__len__()`은 `0`을 반환하고, `__getitem__()`은 `KeyError`를 발생시킵니다 |

### Example

```python
import pyrs_yaml

doc = pyrs_yaml.parse("""
base: &base
  host: localhost
  port: 8080

prod:
  <<: *base
  host: prod.example.com
  debug: false
""")

merged = doc.merged()
assert merged["base"]["host"] == "localhost"
assert merged["prod"]["host"] == "prod.example.com"  # overridden
assert merged["prod"]["port"] == 8080  # inherited
assert merged["prod"]["debug"] is False  # own key
```
