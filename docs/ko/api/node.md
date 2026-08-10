---
title: Node 클래스
description: pyrs-yaml의 AST 노드 클래스 — 탐색, 편집, 트리 API 및 오래된 노드 동작
tags:
  - docs
status: new
---

## Node 클래스

`Node` 클래스는 `YamlDocument`의 AST에 대한 대여된 뷰(borrowed view)를 제공하여 트리 탐색, 쿼리 및 변경 작업을 가능하게 합니다. 노드는 `doc.node()`, `doc.find("$.path")` 또는 `doc.walk()`을 통해 생성됩니다.

### Overview

```python
class Node:
    """A node in the YAML AST, backed by a YamlDocument and a path."""
```

각 `Node`는 부모 `YamlDocument`에 대한 참조와 문서의 AST 내에서 대상 노드로 이동하는 경로 튜플을 저장합니다. 문서가 수정되거나 해제되면 노드는 만료(stale)됩니다.

### Constructor

#### `Node.__init__()`

```python
Node.__init__(document: YamlDocument, path: tuple = ()) -> None
```

**매개변수:**

- `document` — 부모 `YamlDocument`
- `path` — 대상 노드로 이동하는 경로 세그먼트(키/인덱스)의 튜플

### Properties

#### `value`

이 노드의 스칼라 값을 가져옵니다.

```python
value -> Any | None
```

스칼라가 아닌 노드(매핑, 시퀀스)에 대해서는 `None`을 반환합니다.

#### `root_type`

이 노드의 타입을 가져옵니다.

```python
root_type -> str
```

`"scalar"`, `"mapping"`, `"sequence"`, `"null"` 중 하나를 반환합니다.

#### `_path`

문서의 AST 내에서 이 노드로 이동하는 경로 튜플입니다.

```python
_path -> tuple
```

#### `children`

이 노드의 자식 노드들을 가져옵니다.

```python
children -> list[Node]
```

스칼라/null 노드에 대해서는 빈 리스트를 반환합니다.

#### `parent`

부모 `Node`를 가져오거나, 루트인 경우 `None`을 반환합니다.

```python
parent -> Node | None
```

### Methods

#### `find()`

JSONPath-like 경로로 노드를 찾습니다.

```python
find(path: str) -> Node | list[Node]
```

**지원하는 경로 문법:**

| 패턴 | 설명 |
| --- | --- |
| `$.key` | 루트 키 |
| `$.key.subkey` | 중첩 키 |
| `$.arr[0]` | 시퀀스 인덱스 |
| `$.arr[*]` | 시퀀스의 모든 항목 |
| `$..key` | 모든 깊이에서 키를 검색 |
| `$..*` | 모든 하위 노드 |

**반환값:** 정확한 경로에 대해서는 단일 `Node`, 와일드카드/깊이 검색 쿼리에 대해서는 `list[Node]`를 반환합니다.

#### `walk()`

모든 하위 노드를 순회합니다(깊이 우선 전위 순회).

```python
walk() -> Iterator[Node]
```

**생성(yield):** 노드 자신, 그 다음 모든 하위 노드를 재귀적으로 반환합니다.

#### `filter()`

술어 함수로 하위 노드를 필터링합니다.

```python
filter(predicate: Callable[[Node], bool]) -> list[Node]
```

**매개변수:**

- `predicate` — `Node`를 받아 `bool`을 반환하는 함수

**예제:**

```python
scalars = root.filter(lambda n: n.root_type == "scalar")
```

#### `set_value()`

이 노드의 값을 교체하며, 메타데이터(주석, 앵커, 태그, 스타일)를 보존합니다.

```python
set_value(value: Any, create_missing: bool = False) -> None
```

`create_missing=True`를 사용하면 경로상의 누락된 중간 매핑 키가 중첩 매핑으로 생성됩니다. 인덱스 세그먼트가 누락된 경우는 여전히 오류입니다.

#### `append()`

시퀀스 노드에 값을 추가합니다.

```python
append(value: Any) -> None
```

#### `insert()`

시퀀스 노드의 특정 인덱스에 삽입합니다.

```python
insert(index: int, value: Any) -> None
```

#### `delete()`

이 노드와 그 주석을 제거합니다. 이후 노드는 만료됩니다.

```python
delete() -> None
```

#### `rename()`

이 노드의 매핑 키를 변경합니다. 노드는 매핑 값이어야 합니다.

```python
rename(new_key: str) -> None
```

#### `to_yaml()`

이 서브트리를 YAML 문자열로 직렬화합니다.

```python
to_yaml() -> str
```

#### `is_valid()`

부모 문서가 여전히 살아있고 수정되지 않았는지 확인합니다.

```python
is_valid() -> bool
```

#### `release()`

부모 문서에 대한 참조를 해제하여 이 노드를 만료 상태로 표시합니다.

```python
release() -> None
```

`release()`를 호출한 후에는 이 노드에 접근하면 `RuntimeWarning`이 발생하고 `YamlDocumentError`가 발생합니다.

### Dunder Methods

#### `__repr__()`

```python
__repr__() -> str
```

유효한 노드에 대해서는 `Node(root_type=<type>, path=<path>)`, 해제된 노드에 대해서는 `Node(released)`, 만료된 노드에 대해서는 `Node(invalid)`를 반환합니다.

#### `__eq__()`

```python
__eq__(other: object) -> bool
```

두 `Node` 인스턴스는 동일한 문서, 경로 및 활성 상태를 공유하는 경우 동일합니다.

### Stale Node Behavior

!!! warning "만료된 노드"
    `Node`는 생성 시점의 문서 리비전에 연결됩니다. 문서 편집은 리비전을 증가시키므로,
    이전에 얻은 노드는 만료됩니다. 문서를 편집한 후에는 항상 노드를 다시 찾아야 합니다.

노드는 다음 경우에 만료됩니다:

- 부모 `YamlDocument`가 가비지 컬렉션된 경우
- `release()`가 명시적으로 호출된 경우
- 노드가 생성된 후 문서가 수정된 경우

만료된 노드에 접근하면 `RuntimeWarning`이 발생하고 `YamlDocumentError`가 발생합니다:

```python
>>> node = doc.node()
>>> doc.set("$.key", "new_value")
>>> node.value
RuntimeWarning: Node is stale: the document was modified after this node was created
YamlDocumentError: document has been modified; re-find the node
```

### Example

```python
import pyrs_yaml

doc = pyrs_yaml.parse("""
a:
  b: 1
  c: [2, 3, 4]
d: hello
""")

# Get root node
root = doc.node()
print(root.root_type)  # "mapping"

# Navigate
node = root.find("$.a.c[1]")
print(node.value)  # 3

# Walk
for n in root.walk():
    print(n._path, n.root_type)

# Filter
numbers = root.filter(lambda n: n.root_type == "scalar" and isinstance(n.value, int))
for n in numbers:
    print(n._path, n.value)  # ('a', 'b') 1, ('a', 'c', 0) 2, ...

# Mutate
root.find("$.a.b").set_value(42)
root.find("$.a.c").append(5)
root.find("$.d").rename("greeting")
root.find("$.a.c[0]").delete()

print(doc.to_yaml())
# a:
#   b: 42
#   c: [3, 4, 5]
# greeting: hello
```
