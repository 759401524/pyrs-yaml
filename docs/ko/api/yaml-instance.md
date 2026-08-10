---
title: YAML 클래스
description: pyrs-yaml의 YAML 인스턴스 클래스 — 재사용 가능한 구성, 파싱 및 직렬화 메서드
tags:
  - docs
status: new
---

## YAML 클래스

`YAML` 클래스는 `typ`, `schema`, `max_depth`, `allow_duplicate_keys` 설정을 통해 파싱 동작을 제어하는 구성된 파서 인스턴스입니다. 라운드트립(`rt`), 안전(safe), 전체(full) YAML 파싱 모드를 지원합니다.

### Overview

```python
class YAML:
    """Configured YAML parser instance (rt / safe / full)."""
```

### Constructor

#### `__init__()`

구성된 YAML 파서 인스턴스를 생성합니다.

```python
__init__(
    typ: str = "rt",
    schema: str = "core",
    max_depth: int = 1000,
    allow_duplicate_keys: bool = False,
) -> None
```

**매개변수:**

| 매개변수 | 타입 | 기본값 | 설명 |
|-----------|------|---------|-------------|
| `typ` | `str` | `"rt"` | 파서 타입입니다. `"rt"`(라운드트립), `"safe"`, `"full"` 중 하나입니다. |
| `schema` | `str` | `"core"` | YAML 스키마입니다. `"core"`, `"yaml1.1"`, `"failsafe"`, `"json"` 중 하나입니다. |
| `max_depth` | `int` | `1000` | 파싱을 위한 최대 중첩 깊이입니다. |
| `allow_duplicate_keys` | `bool` | `False` | 중복 매핑 키를 허용할지 여부입니다. |

**발생:** `typ` 또는 `schema`가 유효하지 않으면 `YamlTypeError`가 발생합니다.

**예제:**

```python
from pyrs_yaml import YAML

# Round-trip parser (default)
yaml = YAML()

# Safe parser (no merge resolution)
yaml_safe = YAML(typ="safe")

# Full parser with YAML 1.1 schema
yaml_full = YAML(typ="full", schema="yaml1.1")
```

### Methods

#### `parse()`

YAML 문자열을 파싱하고 전체 메타데이터가 보존된 `YamlDocument`를 반환합니다.

```python
parse(yaml: str | bytes) -> YamlDocument
```

**매개변수:**

| 매개변수 | 타입 | 설명 |
|-----------|------|-------------|
| `yaml` | `str \| bytes` | 파싱할 YAML 콘텐츠입니다. |

**반환값:** 라운드트립 편집 지원, 주석 보존, 소스 추적이 포함된 `YamlDocument`입니다.

**참고:**

- `typ`이 `"rt"` 또는 `"full"`인 경우 병합 해석(`<<`)이 활성화됩니다.
- 반환된 문서는 주석, 앵커 및 포맷을 보존합니다.

**예제:**

```python
yaml = YAML(typ="rt")
doc = yaml.parse("name: Alice\nage: 30\n")
print(doc.root_type())  # mapping
print(doc["name"])  # Alice
```

#### `safe_load()`

YAML을 일반 Python `dict` 또는 `list`로 파싱하고 앵커와 병합을 해석합니다.

```python
safe_load(yaml: str) -> dict[str, Any] | list[Any]
```

**매개변수:**

| 매개변수 | 타입 | 설명 |
|-----------|------|-------------|
| `yaml` | `str` | 파싱할 YAML 콘텐츠입니다. |

**반환값:** 모든 YAML 앵커가 해석된 일반 Python `dict` 또는 `list`입니다.

**참고:**

- 이 메서드는 주석, 포맷 또는 소스 추적을 보존하지 않습니다.
- 모든 앵커 참조가 해석됩니다 — 결과는 일반 Python 객체입니다.
- 파싱 오류 시 `YamlTypeError`를 발생시킵니다.

**예제:**

```python
yaml = YAML(typ="safe")
data = yaml.safe_load("""
person: &ref
  name: Alice
alias: *ref
""")
# data == {"person": {"name": "Alice"}, "alias": {"name": "Alice"}}
```

#### `safe_loads()`

다중 문서 YAML 문자열을 `dict`/`list` 객체의 리스트로 파싱합니다.

```python
safe_loads(yaml: str) -> list[dict[str, Any] | list[Any]]
```

**매개변수:**

| 매개변수 | 타입 | 설명 |
|-----------|------|-------------|
| `yaml` | `str` | 다중 문서 YAML 콘텐츠입니다. |

**반환값:** 문서당 하나씩의 일반 Python `dict` 또는 `list` 객체 리스트입니다.

**참고:**

- 문서는 `---` 마커로 구분됩니다.
- 앵커와 병합은 각 문서 내에서 해석됩니다.
- 주석과 포맷은 보존되지 않습니다.

**예제:**

```python
yaml = YAML(typ="safe")
docs = yaml.safe_loads("""
---
a: 1
---
b: 2
""")
# docs == [{"a": 1}, {"b": 2}]
```

#### `parse_file()`

YAML 파일을 파싱하고 전체 메타데이터가 보존된 `YamlDocument`를 반환합니다.

```python
parse_file(path: str) -> YamlDocument
```

**매개변수:**

| 매개변수 | 타입 | 설명 |
|-----------|------|-------------|
| `path` | `str` | 읽고 파싱할 파일 경로입니다. |

**반환값:** 라운드트립 편집 지원이 포함된 `YamlDocument`입니다.

**발생:** 파일을 읽을 수 없으면 `IOError`가 발생합니다.

**참고:**

- 파일은 Rust의 `std::fs::read_to_string`을 사용해 디스크에서 읽습니다 — GIL 차단이 없습니다.
- 라운드트립 충실도를 위해 소스가 문서에 저장됩니다.

**예제:**

```python
yaml = YAML(typ="rt")
doc = yaml.parse_file("config.yaml")
print(doc["database"]["host"])
```

#### `parse_all_docs()`

다중 문서 YAML 문자열을 파싱하고 `YamlDocument` 객체 리스트를 반환합니다.

```python
parse_all_docs(yaml: str) -> list[YamlDocument]
```

**매개변수:**

| 매개변수 | 타입 | 설명 |
|-----------|------|-------------|
| `yaml` | `str` | 다중 문서 YAML 콘텐츠입니다. |

**반환값:** 문서당 하나씩의 `YamlDocument` 객체 리스트입니다.

**참고:**

- 문서는 `---` 마커로 구분됩니다.
- 각 문서는 전체 라운드트립 지원(주석, 앵커, 포맷)을 유지합니다.
- `typ`이 `"rt"` 또는 `"full"`인 경우 병합 해석이 활성화됩니다.

**예제:**

```python
yaml = YAML(typ="rt")
docs = yaml.parse_all_docs("""
---
a: 1
---
b: 2
""")
for doc in docs:
    print(doc.root_type())
```

#### `dump_stream()`

스트리밍 writer: Python 객체를 파일류 객체로 직렬화하며 상수 메모리를 사용합니다.

```python
dump_stream(
    file_obj: Any,
    iterable: Any,
    explicit_start: bool = False,
    explicit_end: bool = False,
    sort_keys: bool = False,
) -> None
```

**매개변수:**

| 매개변수 | 타입 | 기본값 | 설명 |
|-----------|------|---------|-------------|
| `file_obj` | `Any` | — | `write(str)` 메서드가 있는 쓰기 가능한 파일류 객체입니다. |
| `iterable` | `Any` | — | 직렬화할 Python 객체의 이터러블입니다. |
| `explicit_start` | `bool` | `False` | 각 문서 시작에 `---`를 출력할지 여부입니다. |
| `explicit_end` | `bool` | `False` | 각 문서 끝에 `...`를 출력할지 여부입니다. |
| `sort_keys` | `bool` | `False` | 매핑 키를 알파벳순으로 정렬할지 여부입니다. |

**발생:** `file_obj`에 `write` 메서드가 없으면 `YamlTypeError`가 발생합니다.

**참고:**

- 상수 메모리를 사용합니다 — 전체 출력을 메모리에 보관할 필요가 없습니다.
- Rust 직렬화 단계 동안 GIL이 해제됩니다.
- 이터러블의 각 항목은 별도의 YAML 문서가 됩니다.

**예제:**

```python
import io
from pyrs_yaml import YAML

yaml = YAML()
buf = io.StringIO()
yaml.dump_stream(buf, [{"a": 1}, {"b": 2}], explicit_start=True)
print(buf.getvalue())
# ---
# a: 1
# ---
# b: 2
```

#### `dump_file()`

스트리밍 writer: Python 객체를 디스크의 파일로 직접 직렬화합니다.

```python
dump_file(
    path: str,
    iterable: Any,
    explicit_start: bool = False,
    explicit_end: bool = False,
    sort_keys: bool = False,
) -> None
```

**매개변수:**

| 매개변수 | 타입 | 기본값 | 설명 |
|-----------|------|---------|-------------|
| `path` | `str` | — | 쓸 파일 경로입니다. |
| `iterable` | `Any` | — | 직렬화할 Python 객체의 이터러블입니다. |
| `explicit_start` | `bool` | `False` | 각 문서 시작에 `---`를 출력할지 여부입니다. |
| `explicit_end` | `bool` | `False` | 각 문서 끝에 `...`를 출력할지 여부입니다. |
| `sort_keys` | `bool` | `False` | 매핑 키를 알파벳순으로 정렬할지 여부입니다. |

**발생:** 파일을 만들거나 쓸 수 없으면 `IOError`가 발생합니다.

**참고:**

- Rust의 `std::fs::File`을 직접 사용합니다 — I/O 중 GIL 차단이 없습니다.
- 이터러블의 각 항목은 별도의 YAML 문서가 됩니다.
- 상수 메모리를 사용하며 대용량 출력에 적합합니다.

**예제:**

```python
from pyrs_yaml import YAML

yaml = YAML()
yaml.dump_file("output.yaml", [{"x": 2}, {"x": 3}], sort_keys=True)
```

#### `load_stream()`

지연 이벤트 이터레이터: 파일류 객체에서 점진적으로 읽습니다.

```python
load_stream(file_obj: Any) -> YamlStream
```

**매개변수:**

| 매개변수 | 타입 | 설명 |
|-----------|------|-------------|
| `file_obj` | `Any` | `str` 또는 `bytes`를 반환하는 `read()` 메서드가 있는 읽기 가능한 파일류 객체입니다. |

**반환값:** 파싱된 이벤트 dict를 지연 생성하는 `YamlStream` 이터레이터입니다.

**발생:** `file_obj`에 `read` 메서드가 없으면 `YamlTypeError`가 발생합니다.

**참고:**

- 스트림은 점진적으로 파싱됩니다 — 전체 파일을 메모리에 로드할 필요가 없습니다.
- 각 생성된 이벤트는 `"type"`, `"key"`, `"value"`, `"start_mark"`, `"end_mark"` 같은 키를 가진 `dict`입니다.
- `__next__`가 `None`을 반환하면 스트림이 끝납니다.

**예제:**

```python
import io
from pyrs_yaml import YAML

yaml = YAML()
buf = io.StringIO("key: value\n")
stream = yaml.load_stream(buf)
for event in stream:
    if event is None:
        break
    print(event["type"])
```

#### `load_stream_file()`

지연 이벤트 이터레이터: 파일 경로에서 점진적으로 읽습니다.

```python
load_stream_file(path: str) -> YamlStream
```

**매개변수:**

| 매개변수 | 타입 | 설명 |
|-----------|------|-------------|
| `path` | `str` | 점진적으로 읽을 파일 경로입니다. |

**반환값:** 파싱된 이벤트 dict를 지연 생성하는 `YamlStream` 이터레이터입니다.

**발생:** 파일을 열 수 없으면 `IOError`가 발생합니다.

**참고:**

- 버퍼링된 I/O와 함께 Rust의 `std::fs::File`을 사용합니다 — 읽기 중 GIL 차단이 없습니다.
- 파일을 점진적으로 파싱하므로 대용량 YAML 파일에 이상적입니다.

**예제:**

```python
from pyrs_yaml import YAML

yaml = YAML()
stream = yaml.load_stream_file("large.yaml")
for event in stream:
    if event is None:
        break
    print(event)
```

### Usage Examples

#### 구성된 인스턴스로 라운드트립 편집

```python
from pyrs_yaml import YAML

yaml = YAML(typ="rt", schema="core")
doc = yaml.parse("""
# User configuration
user:
  name: Alice
  age: 30
  tags: [admin, user]
""")

# Edit the document
doc["user"]["age"] = 31
doc["user"]["tags"].append("staff")

# Serialize back — comments and formatting are preserved
print(doc.to_yaml())
```

#### JSON 스키마로 안전 파싱

```python
from pyrs_yaml import YAML

yaml = YAML(typ="safe", schema="json")
data = yaml.safe_load("{name: Bob, age: 25}")
print(data["name"])  # Bob
```

#### 다중 문서 스트림 처리

```python
from pyrs_yaml import YAML

yaml = YAML(typ="rt")
docs = yaml.parse_all_docs("""
---
doc: first
---
doc: second
""")
for doc in docs:
    print(doc["doc"])

# Or dump multiple documents
yaml.dump_file("multi.yaml", [{"id": 1}, {"id": 2}], explicit_start=True)
```

### See Also

- [`YamlDocument`](yaml-document.md) — 라운드트립으로 편집 가능한 문서 객체
- [`YamlStream`](reference.md#yamlstream) — 지연 이벤트 스트림 이터레이터
- [`parse()`](reference.md#parse) — 모듈 수준 편의 함수
- [`safe_load()`](reference.md#safe_load) — 모듈 수준 편의 함수
