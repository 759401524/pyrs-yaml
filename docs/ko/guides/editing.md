title: 제자리 편집
lang: ko

# 제자리 편집

pyrs-yaml는 파싱된 문서를 **제자리에서 편집**할 수 있게 해주며, 모든 포맷 메타데이터(주석, 앵커, 태그, 스칼라 스타일, 흐름/블록 스타일)를 보존합니다 — 수동 문자열 조작 없이, 충실도 손실 없이.

## 개요

편집은 문서 트리에 대한 **JSONPath 스타일 경로**로 표현됩니다:

```python
import pyrs_yaml

doc = pyrs_yaml.parse("""
db:
  host: localhost
  port: 5432
""")

doc.set("$.db.host", "db.example.com")  # set by path
doc.set("$.db.port", 5433)
print(doc.to_yaml())
# db:
#   host: db.example.com
#   port: 5433
```

모든 편집 메서드는 **원자적**입니다: 실패 시 문서 리비전을 포함해 아무것도 변경되지 않습니다. 성공 시 문서는 dirty로 표시되며, 다음 `source()` / `to_yaml()` / `to_yaml_with_options()` / `reparse()` 호출 시 업데이트된 트리에서 재직렬화됩니다.

## 경로 구문

경로는 `$`로 시작하며 점으로 구분된 키(매핑) 또는 `[N]` 인덱스(시퀀스)가 이어집니다:

| 경로 | 의미 |
|------|------|
| `$.host` | 루트 매핑의 `host` 키 |
| `$.a.b.c` | 중첩 키 |
| `$.items[0]` | 시퀀스 `items`의 첫 번째 요소 |
| `$` | 루트 노드 자체 |

- **음수 인덱스** (`[-1]`)는 **지원되지 않습니다** — 오류를 발생시킵니다
- 키는 **값 기준**으로 일치하므로(메타데이터 무관), 따옴표 키 `"host"`는 일반 키 `host`와 일치합니다

편집 경로는 정확히 하나의 노드를 대상으로 해야 합니다 — **와일드카드** (`[*]`)와 **딥 스캔** (`..`)은 `YamlPathError`를 발생시킵니다. (쿼리 전용 `find()`는 이를 지원합니다. [find()로 쿼리하기](#find로-쿼리하기) 참조)

**발생:** 잘못된 경로에 대해 `YamlPathError`, 경로 단계를 적용할 수 없을 때(예: 스칼라로의 탐색, 별칭을 통한 편집) `YamlEditError`.

## 값 설정

### `set()` — 경로로 교체

```python
set(path: str, value: Any) -> None
```

```python
doc = pyrs_yaml.parse("a:\n  b: 1\nitems: [1, 2, 3]")

doc.set("$.a.b", 42)  # scalar → scalar, metadata preserved
doc.set("$.items[1]", "two")  # sequence index
doc.set("$.a.c", True)  # add a new key to a mapping (last position)
doc.set("$", {"x": 1})  # replace the entire root
```

값 변환 규칙:

| Python 값 | YAML 노드 |
|-----------|-----------|
| `str`, `int`, `float`, `bool`, `None` | 새 스칼라 (값은 *재파싱되지 않음*) |
| `dict` | 새 매핑 (일반 스타일) |
| `list` | 새 시퀀스 (일반 스타일) |
| `tuple` | ❌ 지원되지 않음 — `YamlEditError` 발생 |

기존 스칼라를 교체할 때 대상의 메타데이터(인라인 주석, 앵커, 태그, 따옴표 스타일)는 **보존**됩니다 — 새 값이 매핑/시퀀스인 경우는 예외로, 새 노드 자체의 서식을 따릅니다.

### `__setitem__` — 루트 슈가

```python
doc["b"] = 2  # equivalent to doc.set("$.b", 2)
```

### `Node.set_value()` — Node를 통한 편집

```python
node = doc.node().find("$.a.b")  # see "Working with Nodes"
node.set_value(42)
```

## 삽입 및 추가

둘 다 **시퀀스에만** 적용됩니다; 경로는 시퀀스 노드를 가리켜야 합니다.

### `insert()` — 인덱스에 삽입

```python
insert(path: str, index: int, value: Any) -> None
```

`index`는 현재 길이까지 허용됩니다 (`len`에 삽입하면 추가됨); 그보다 크면 `YamlEditError`가 발생합니다.

```python
doc = pyrs_yaml.parse("items:\n  - a\n  - c")

doc.insert("$.items", 1, "b")  # items: [a, b, c]
doc.insert("$.items", 0, "first")
doc.insert("$.items", 3, "last")  # index == len appends
```

### `append()` — 끝에 추가

```python
append(path: str, value: Any) -> None
```

```python
doc.append("$.items", "d")
```

### `Node.append()` / `Node.insert()`

동일한 작업을 `Node` 객체에서도 사용할 수 있습니다:

```python
node = doc.node().find("$.items")
node.append("d")
node.insert(1, "x")
```

## 삭제

### `delete()` — 경로로 제거

```python
delete(path: str) -> None
```

```python
doc = pyrs_yaml.parse("a: 1\nb: 2\nc: 3")
doc.delete("$.b")
print(doc.to_yaml())  # a: 1\nc: 3\n — order preserved
```

매핑 순서는 항상 보존됩니다; 시퀀스 삭제는 빈자리를 메웁니다.

### `__delitem__` — 루트 슈가

```python
del doc["b"]  # equivalent to doc.delete("$.b")
```

### `Node.delete()`

```python
node = doc.node().find("$.b")
node.delete()
```

## 이름 변경

### `rename()` — 매핑 키 제자리 이름 변경

```python
rename(path: str, new_key: str) -> None
```

경로는 **매핑 키**를 가리켜야 합니다 (그 아래 값이 메타데이터를 유지합니다):

```python
doc = pyrs_yaml.parse("old: value  # keep me\nnext: 1")
doc.rename("$.old", "new")
print(doc.to_yaml())  # new: value  # keep me\nnext: 1
```

- **위치가 보존됩니다** — 이름이 변경된 키는 제자리에 유지됩니다
- **메타데이터가 보존됩니다** — 키의 인라인 주석, 스타일, 앵커가 이름 변경과 함께 이동합니다
- 루트 또는 복합(비스칼라) 키의 이름 변경, 그리고 **기존 키로의** 이름 변경은 `YamlEditError`를 발생시킵니다 (자기 자신으로의 이름 변경은 no-op)

### `Node.rename()`

```python
node = doc.node().find("$.old")
node.rename("new")
```

## Node 작업

`doc.node()`는 문서 루트의 `Node`를 반환합니다; `Node.find(path)`는 하위 트리로 이동합니다:

```python
node = doc.node()  # root node
node = doc.node().find("$.db.host")  # navigate by path
print(node.value)  # "localhost"
node.set_value("other")  # edit through the node
print(node.root_type)  # "scalar" | "mapping" | "sequence" | "null"
```

Node는 트리 API를 제공합니다: `node.parent`, `node.children`, `node.walk()` (깊이 우선 반복자), `node.filter(predicate)`, `node.to_yaml()`.

### `find()`로 쿼리하기

`find()`는 **읽기 지향적**이며 와일드카드와 딥 스캔을 지원합니다 — 경로가 여러 노드를 선택하면 리스트를 반환합니다:

```python
doc.node().find("$.items[*]")  # all items of a sequence (list of Nodes)
doc.node().find("$..timeout")  # deep search for any key named "timeout"
```

와일드카드/딥 스캔 결과는 **직접 편집할 수 없습니다** — 경로를 찾는 데 사용한 후 `set()`/`insert()` 등으로 편집하세요.

## 별칭과 병합 키

별칭 노드 (`*name`)는 자체 경로가 설정될 때 **제자리에서 교체**됩니다:

```python
yaml = "defaults: &defaults\n  timeout: 30\nprod: *defaults\n"
doc = pyrs_yaml.YAML(typ="safe").parse(yaml)  # resolve_merges=false keeps the alias node

doc.set("$.prod", {"timeout": 99})  # replaces the alias node — prod.timeout: 99
```

- 별칭 **을 통한** 설정 (병합된 키에 도달하기 위해 `*defaults`를 탐색)은 `YamlEditError`를 발생시킵니다 — 참조된 노드는 다른 곳에 존재합니다
- 병합 키가 해석된 경우(기본값), 병합 확장 키는 클론입니다; 편집 시 클론만 편집됩니다
- 앵커된 노드 삭제는 허용됩니다 (앵커가 더 이상 참조되지 않을 뿐)

## 뷰 vs AST

`doc.get()` / `doc.to_dict()`는 **뷰**(해석된 값)를 반환합니다. 편집은 항상 **AST**에서 수행됩니다:

```python
doc = pyrs_yaml.parse("on: yes")
print(doc.get("on"))  # True   — view (core schema resolution)
doc.set("$.on", "off")  #         — edits the AST scalar
print(doc.to_yaml())  # on: off — serialized verbatim, no re-resolution
```

편집된 값은 **그대로** 출력됩니다; 뷰는 활성 스키마에 따라 이를 해석합니다.

## 오래된 Node

`Node`는 노드 생성 시 기록된 문서 **리비전**에 연결됩니다. 어떤 문서 편집(다른 노드를 통한 편집도 포함)이든 리비전을 올리므로, 이전에 얻은 노드는 오래된(stale) 상태가 됩니다:

```python
node = doc.node().find("$.a")
doc.set("$.b", 2)  # bumps the revision
node.set_value(99)  # RuntimeWarning + YamlDocumentError (stale)
```

편집 후에는 노드를 다시 찾아 작업을 계속하세요. `node.is_valid()`는 유효성을 확인합니다; `node.release()`는 노드를 문서에서 명시적으로 분리합니다.

## 오류 처리

| 오류 | 시점 |
|------|------|
| `YamlPathError` | 잘못된 경로, 편집 경로에 와일드카드/`..` 사용 |
| `YamlEditError` | 지원되지 않는 값 타입 (`tuple`), 음수 인덱스, 별칭을 통한 편집, 루트/복합/기존 키 이름 변경, 스칼라로의 탐색, 인덱스 범위 초과 |
| `YamlDocumentError` | 문서 편집 후 오래된 `Node` 사용 |

모든 편집은 원자적입니다 — 실패한 편집은 문서(및 리비전)를 변경하지 않습니다.

## 전체 예시

```python
import pyrs_yaml

doc = pyrs_yaml.parse("""
# server config
server:
  host: localhost  # bind address
  ports:
    - 8080
    - 9090
""")

doc.set("$.server.host", "0.0.0.0")
doc.insert("$.server.ports", 0, 80)
doc.append("$.server.ports", 443)
doc.rename("$.server", "srv")

print(doc.to_yaml())
# server config
# srv:
#   host: 0.0.0.0  # bind address
#   ports:
#     - 80
#     - 8080
#     - 9090
#     - 443
```

주석, 앵커, 태그, 스칼라 스타일 및 흐름/블록 서식이 전체 과정에서 보존됩니다.
