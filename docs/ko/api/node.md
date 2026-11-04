---
title: Node 클래스
description: pyrs-yaml의 AST 노드 클래스 — 탐색, 편집, 트리 API 및 오래된 노드 동작
tags:
  - docs
status: new
---

## Node 클래스

`Node` 클래스는 `YamlDocument`의 AST에 대한 대여된 뷰(borrowed view)를 제공하여 트리 탐색, 쿼리 및 변경 작업을 가능하게 합니다. 노드는 `doc.node()`, `doc.find("$.path")` 또는 `doc.walk()`을 통해 생성됩니다.

### 개요

```python
class Node:
    """A node in the YAML AST, backed by a YamlDocument and a path."""
```

각 `Node`는 부모 `YamlDocument`에 대한 참조와 문서의 AST 내에서 대상 노드로 이동하는 경로 튜플을 저장합니다. 문서가 수정되거나 해제되면 노드는 만료(stale)됩니다.

### 생성자

#### `Node.__init__()`

```python
Node.__init__(document: YamlDocument, path: tuple = ()) -> None
```

**매개변수:**

- `document` — 부모 `YamlDocument`
- `path` — 대상 노드로 이동하는 경로 세그먼트(키/인덱스)의 튜플

### 속성

#### `value`

이 노드의 스칼라 값을 가져옵니다.

```python
value -> Any | None
```

스칼라가 아닌 노드(매핑, 시퀀스)에 대해서는 `None`을 반환합니다.

읽기에 실패하면 예외가 납니다. 경로가 없으면 `KeyError`, 서로 다른 두 YAML 키가 같은
Python 키로 투영되면 `YamlTypeError`입니다(`to_dict()`가 같은 지점에서 거부합니다). 뒤의
거부는 경로가 아니라 문서의 성질이므로, 그런 문서에서는 어떤 경로를 쓰든 `Node` 읽기가
예외를 냅니다. 쓰기는 이 읽기를 거치지 않습니다. `set_value()`는 AST를 직접 고치므로,
이름이 겹치면 그 이름으로 접히는 첫 쌍에 반영됩니다.

#### `root_type`

이 노드의 타입을 가져옵니다.

```python
root_type -> str
```

`"scalar"`, `"mapping"`, `"sequence"`, `"null"` 중 하나를 반환합니다.

별칭 노드는 자기 **대상**의 타입을 알릴 뿐 자기만의 철자는 갖지 않습니다:
`defaults: &d\n  timeout: 30\nprod: *d\n`와 `src: &s\n  - a\nref: *s\n`에서 측정하면
`$.prod`와 `$.ref` 노드는 `"mapping"`과 `"sequence"`로 답하고, `base: &x 1\nuse: *x\n`의
`$.use`에 있는 스칼라 별칭은 `"scalar"`로 답합니다.

#### `_path`

문서의 AST 내에서 이 노드로 이동하는 경로 튜플입니다.

```python
_path -> tuple
```

#### `path`

Get this node's path segments (tuple of keys and indices). The public accessor for logging, error messages, and re-finding a node after it becomes stale.

```python
path -> tuple
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

#### `comment`

이 노드의 주석 텍스트를 가져옵니다. 주석이 없으면 `None`을 반환합니다.

```python
comment -> str | None
```

#### `anchor`

이 노드의 앵커 이름을 가져옵니다. 앵커가 없으면 `None`을 반환합니다.

```python
anchor -> str | None
```

#### `tag`

이 노드의 YAML 태그 문자열(예: `!!str`)을 가져옵니다. 태그가 없으면 `None`을 반환합니다.

```python
tag -> str | None
```

#### `scalar_style`

스칼라 스타일을 가져옵니다(`"plain"`, `"single_quoted"`, `"double_quoted"`, `"literal"`, `"folded"`). 비스칼라 노드는 `None`을 반환합니다.

```python
scalar_style -> str | None
```

#### `flow_style`

흐름 스타일을 가져옵니다(`True` = 흐름 `{}`/`[]`, `False` = 블록). 비컨테이너 노드는 `None`을 반환합니다.

```python
flow_style -> bool | None
```

#### `chomping`

chomping 표시자를 가져옵니다(`"strip"`, `"clip"`, `"keep"`). 비스칼라 노드는 `None`을 반환합니다.

```python
chomping -> str | None
```

### 메서드

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

#### `find_first()`

Find the first matching node by JSONPath-like path. Always returns a single `Node` or `None` (never a list) — for wildcard paths (`[*]`), returns the first element.

```python
find_first(path: str) -> Node | None
```

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

측정된 모서리들:

- `find()`가 해석하지 못한 노드 — `Node(invalid)` 객체 — 도 여전히 씁니다: `doc.node().find("$.zz").set_value(3)`은 `zz: 3`을 추가합니다. 거부되는 쪽은 쓰기가 아니라 그 `value` 읽기(`KeyError`)입니다
- **별칭** 노드에서도 이 호출은 거부가 아니라 별칭을 제자리에서 다시 씁니다: `parse("base: &x 1\nuse: *x\n").node().find("$.use").set_value(42)`는 `base: &x 1\nuse: 42\n`를 발출합니다
- 별칭 **을 통한** 쓰기는 거부됩니다: `edit failed: cannot-edit-alias`
- 만료된 노드에서는 `RuntimeWarning`을 낸 뒤 `YamlDocumentError: document has been modified; re-find the node`를 발생시킵니다
- 작성자가 알지 못하는 Python 값은 편집 오류가 아니라 `YamlTypeError`(`Unsupported Python type for YAML conversion`)입니다

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
delete(on_duplicate: str = "one") -> None
```

측정된 거부들: 루트 노드는 `YamlEditError: edit failed: edit-error`로 답하고, 별칭이 아직
참조하는 앵커를 가진 노드는 `edit failed: anchor-is-referenced`로 답하며, 같은 키를 두 짝이
철자하는 문서에서 `on_duplicate="error"`는 `edit failed: duplicate-key-target`으로 답합니다.
**별칭** 짝을 지우는 것은 거부가 아닙니다 —
`defaults: &d\n  timeout: 30\nprod: *d\n`에서 `find("$.prod").delete()`는
`defaults: &d\n  timeout: 30\n`만 남깁니다. 만료된 노드는 다음 접근에서 `YamlDocumentError`로
답하지만 `is_valid()`는 여전히 `True`로 답합니다.

#### `rename()`

이 노드의 매핑 키를 변경합니다. 노드는 매핑 값이어야 합니다.

```python
rename(new_key: str) -> None
```

측정된 거부들: 루트는 `YamlEditError: edit failed: cannot-rename-root`로 답하고, 키가 없는
시퀀스 요소는 `edit failed: cannot-rename-complex-key`로 답하며, 매핑이 이미 갖고 있는
`new_key`는 `edit failed: rename-key-exists`로 답하고, 키 자체가 복합 노드인 노드는 엔진까지
닿지 않습니다: `parse("? [1, 2]\n: value\n").node().children[0].rename("b")`는
`YamlEditError: ValueError: segment must be str or int`로 답합니다. 같은 키로 이름 변경하는
것은 no-op이고, 별칭 짝에서는 이 호출이 거부되지 않고 키의 이름을 바꿉니다 —
`defaults: &d\n  timeout: 30\nprod: *d\n`에서 `find("$.prod").rename("prod2")`는
`defaults: &d\n  timeout: 30\nprod2: *d\n`를 남깁니다.

#### `set_comment()`

이 노드의 주석을 설정(또는 교체)합니다. `standalone=True`(기본)면 주석이 노드 위의 독립된 줄에 출력되고, `standalone=False`면 노드 뒤에 인라인으로 출력됩니다.

```python
set_comment(text: str, standalone: bool = True) -> None
```

#### `remove_comment()`

이 노드의 주석을 제거합니다.

```python
remove_comment() -> None
```

#### `set_anchor()`

이 노드의 앵커를 설정(또는 교체)합니다.

```python
set_anchor(name: str) -> None
```

별칭이 아직 현재 앵커를 가리키는 동안은 `YamlEditError: edit failed: anchor-is-referenced`로
거부됩니다 — `base: &x 1\nuse: *x\n`에서 측정하면 `find("$.base").set_anchor("renamed")`는
실패하고, 별칭을 지운 뒤 같은 호출을 `base: &x 1\n`에 넘기면 `base: &renamed 1`을 발출합니다.
별칭 노드 자신에 대해서는 `edit failed: cannot-edit-alias`로 답합니다.

#### `remove_anchor()`

이 노드의 앵커를 제거합니다.

```python
remove_anchor() -> None
```

`set_anchor()`와 같은 방어막입니다: `base: &x 1\nuse: *x\n`에서 측정하면
`find("$.base").remove_anchor()`는 `YamlEditError: edit failed: anchor-is-referenced`로
답합니다.

#### `set_tag()`

이 노드의 YAML 태그를 설정(또는 교체)합니다. `"!custom"`은 로컬 태그, `"!!int"`는 프라이머리(`!!`) 태그, `"!<tag:yaml.org,2002:str>"`는 verbatim 태그가 됩니다.

```python
set_tag(tag: str) -> None
```

#### `remove_tag()`

이 노드의 YAML 태그를 제거합니다.

```python
remove_tag() -> None
```

#### `set_scalar_style()`

스칼라 스타일을 설정(또는 교체)합니다. 비스칼라 노드에서는 no-op. 값: `"plain"`, `"single_quoted"`, `"double_quoted"`, `"literal"`, `"folded"`.

```python
set_scalar_style(style: str) -> None
```

#### `set_flow_style()`

흐름 스타일을 설정(또는 교체)합니다. `True`는 흐름(`{}`/`[]`), `False`는 블록. 비컨테이너 노드에서는 no-op.

```python
set_flow_style(flow: bool) -> None
```

#### `set_chomping()`

chomping 표시자를 설정(또는 교체)합니다. 값: `"strip"`(`-`), `"clip"`(기본값), `"keep"`(`+`). 비스칼라 노드에서는 no-op.

```python
set_chomping(chomp: str) -> None
```

#### `sort_keys()`

Sort the keys of this mapping node in place. No-op on non-mapping nodes.

```python
sort_keys() -> None
```

#### `move()`

Move this subtree to a new path in the same document (copies the subtree to `new_path`, then removes the source). `new_path` is an absolute JSONPath.

```python
move(new_path: str) -> None
```

측정된 모서리들: 부모가 존재하는 목적지는 자기 자신이 없어도 됩니다 —
`src:\n  x: 1\n`에서 `find("$.src").move("$.nope")`는 `nope:\n  x: 1\n`를 남깁니다 — 반면
중간 키가 없는 목적지는 `YamlEditError: edit failed: missing-path:deep`으로 답합니다. 스칼라
목적지를 교체하는 것은 동작합니다: `dst: 2`가 `dst:\n  x: 1\n`가 됩니다. 루트를 이동하면
`edit failed: edit-error`가 답하고, 하위 트리를 자기 경로로 옮기면 문서가 비워집니다
(`find("$.src").move("$.src")`은 `{}\n`를 남깁니다), 그리고 값을 참조하는 별칭이 아직
남아 있는 목적지는 `edit failed: anchor-is-referenced`로 답합니다.

#### `to_yaml()`

이 서브트리를 YAML 문자열로 직렬화합니다.

```python
to_yaml() -> str
```

#### `copy()`

이 하위 트리를 문서에서 분리된 독립 Python 값(dict/list/scalar)으로 깊은 복사합니다. `set_value()`로 다른 곳에 붙여넣을 때 유용합니다.

```python
copy() -> Any
```

#### `is_valid()`

부모 문서가 여전히 살아있는지 확인합니다.

```python
is_valid() -> bool
```

이 질문은 문서에 대한 것이지 경로에 대한 것이 아닙니다. 경로가 아무것도 해석하지
못하는 노드(키가 없거나, 두 YAML 키가 같은 Python 이름으로 접히거나, 이후 편집으로
노드가 만료된 경우)도 여전히 유효하므로, `is_valid()`으로는 `value`가 응답할지를
확인할 수 없습니다. 그 신호는 `__repr__`이 전달합니다.

#### `release()`

부모 문서에 대한 참조를 해제하여 이 노드를 만료 상태로 표시합니다.

```python
release() -> None
```

`release()`를 호출한 후에는 이 노드에 접근하면 `RuntimeWarning`이 발생하고 `YamlDocumentError`가 발생합니다.

### 던더 메서드

#### `__repr__()`

```python
__repr__() -> str
```

경로가 해석되면 `Node(root_type=<type>, path=<path>)`, `release()`를 호출한 뒤에는
`Node(released)`, 해석되지 않으면 `Node(invalid)`를 반환합니다. 후자는 문서가 수정되었거나,
경로가 아무것도 가리키지 않거나, 값 읽기가 거부된 경우(`value` 참조)에 나타납니다.

#### `__eq__()`

```python
__eq__(other: object) -> bool
```

두 `Node` 인스턴스는 동일한 문서, 경로 및 활성 상태를 공유하는 경우 동일합니다.

#### `value_eq()`

Compare this node's resolved value with another node or Python value. Unlike `__eq__` (reference identity), this compares the actual YAML value.

```python
value_eq(other: object) -> bool
```

### 무효 노드 동작

!!! warning "만료된 노드"
    `Node`는 생성 시점의 문서 리비전에 연결됩니다. 문서 편집은 리비전을 증가시키므로,
    이전에 얻은 노드는 만료됩니다. 문서를 편집한 후에는 항상 노드를 다시 찾아야 합니다.

노드는 다음 경우에 만료됩니다:

- 부모 `YamlDocument`가 가비지 컬렉션된 경우
- `release()`가 명시적으로 호출된 경우
- 노드가 생성된 후 문서가 수정된 경우

만료된 노드에 접근하면 `RuntimeWarning`이 발생하고 `YamlDocumentError`가 발생합니다:

```python title="만료 노드 예제"
>>> node = doc.node()
>>> doc.set("$.key", "new_value")
>>> node.value  # (1)!
RuntimeWarning: Node is stale: the document was modified after this node was created
YamlDocumentError: document has been modified; re-find the node  # (2)!
```

1. :material-alert: 만료된 노드에 접근하면 먼저 `RuntimeWarning`이 트리거됩니다.
2. :material-close: 그런 다음 `YamlDocumentError`가 발생합니다. `doc.node().find(path)`로 노드를 다시 획득하세요.

### 예제

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

## 자동 생성 API 참조

아래 내용은 `mkdocstrings`가 라이브러리 자체 docstring과 생성된 형식 stub에서 그려 낸 것이므로, 서명은 코드에서 온 것이지 손으로 옮긴 것이 아닙니다.

::: pyrs_yaml.Node
